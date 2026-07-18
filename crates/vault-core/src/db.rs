use crate::crypto::{
    decrypt, derive_key, encrypt, DerivedKey, KeySlot, VaultHeader, VaultMetaFile,
};
use crate::error::{Result, VaultError};
use crate::paths;
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};
use std::fs;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use zeroize::Zeroize;

/// Leading bytes of a v2 vault file. Eight bytes so that a v1 blob — which
/// begins with a random nonce — cannot plausibly be mistaken for one.
const MAGIC: &[u8; 8] = b"VAULT-R2";
const FORMAT_VERSION: u32 = 2;

/// How the vault on disk is laid out.
///
/// **V2** is the current format: a single self-describing file whose header
/// holds one key slot per way of unlocking (master password, optional recovery
/// code), each wrapping the same random data key that actually encrypts the
/// database. **V1** is the original layout — the database encrypted directly
/// under `Argon2id(password)`, with the salt in a plaintext `vault.meta.json`
/// sidecar. V1 vaults are still opened and written, and are upgraded in place
/// the next time they are unlocked with a password (see [`Vault::open_in`]);
/// a vault unlocked from the OS keychain has no password to build a slot from,
/// so it stays V1 until the user types one.
#[derive(Debug)]
enum Format {
    V1,
    V2 { header: VaultHeader },
}

/// A vault file split into "how to get the data key" and "the encrypted image".
enum VaultFile {
    V1 { blob: Vec<u8> },
    V2 { header: VaultHeader, payload: Vec<u8> },
}

/// Parses a vault file already in memory, so callers that need the
/// pre-migration bytes for a backup (see `Vault::from_plaintext`) can read
/// the file once and reuse the buffer, rather than reading it a second time.
fn parse_vault_bytes(bytes: &[u8]) -> Result<VaultFile> {
    if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
        return Ok(VaultFile::V1 { blob: bytes.to_vec() });
    }
    let len_at = MAGIC.len();
    let body_at = len_at + 4;
    if bytes.len() < body_at {
        return Err(VaultError::Crypto("vault file header is truncated".into()));
    }
    let header_len = u32::from_le_bytes([
        bytes[len_at],
        bytes[len_at + 1],
        bytes[len_at + 2],
        bytes[len_at + 3],
    ]) as usize;
    let payload_at = body_at
        .checked_add(header_len)
        .ok_or_else(|| VaultError::Crypto("vault file header length is invalid".into()))?;
    if bytes.len() < payload_at {
        return Err(VaultError::Crypto("vault file header is truncated".into()));
    }
    let header: VaultHeader = serde_json::from_slice(&bytes[body_at..payload_at])?;
    Ok(VaultFile::V2 {
        header,
        payload: bytes[payload_at..].to_vec(),
    })
}

/// Serializes a v2 vault file: magic, header length, header JSON, then the
/// encrypted database image.
fn encode_v2(header: &VaultHeader, payload: &[u8]) -> Result<Vec<u8>> {
    let header_json = serde_json::to_vec(header)?;
    let header_len = u32::try_from(header_json.len())
        .map_err(|_| VaultError::Crypto("vault header too large".into()))?;
    let mut out = Vec::with_capacity(MAGIC.len() + 4 + header_json.len() + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&header_len.to_le_bytes());
    out.extend_from_slice(&header_json);
    out.extend_from_slice(payload);
    Ok(out)
}

/// An unlocked vault: a live **in-memory** SQLite database, backed by an
/// AES-256-GCM encrypted blob on disk that is the only artifact meant to
/// persist across app restarts.
///
/// The decrypted database exists solely in this process's memory. Nothing is
/// ever written to disk in plaintext: [`persist`](Vault::persist) serializes
/// the in-memory image and encrypts it before writing, so a stolen disk (or a
/// crash) never exposes secrets. The connection — and the derived key — are
/// freed when the `Vault` is dropped or [`locked`](Vault::lock).
#[derive(Debug)]
pub struct Vault {
    pub(crate) conn: Connection,
    /// The key the database image is encrypted under. In a v2 vault this is a
    /// random data key recovered from a key slot; in a legacy v1 vault it is
    /// `Argon2id(password)` itself.
    key: DerivedKey,
    format: Format,
    blob_path: PathBuf,
    dir: PathBuf,
}

fn meta_file_path(dir: &Path) -> PathBuf {
    dir.join("vault.meta.json")
}
fn blob_file_path(dir: &Path) -> PathBuf {
    dir.join("vault.db.enc")
}

/// Reads at most `head.len()` leading bytes, returning how many were read.
/// A file shorter than the buffer is not an error — it just is not a v2 vault.
fn read_head(path: &Path, head: &mut [u8]) -> std::io::Result<usize> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut filled = 0;
    while filled < head.len() {
        match file.read(&mut head[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

/// Number of automatic backups on disk. A missing directory counts as zero
/// rather than an error, so a vault that has never been backed up still
/// reports a status.
fn count_backups(dir: &Path) -> usize {
    fs::read_dir(dir.join("backups"))
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .count()
        })
        .unwrap_or(0)
}

/// Earlier builds decrypted the vault into this plaintext working file. It is
/// no longer created, but a stale one from a prior version would still contain
/// every secret in the clear — delete it whenever we touch the vault.
fn remove_legacy_session_file(dir: &Path) {
    let _ = fs::remove_file(dir.join("vault.session.db"));
}

/// Copies `bytes` into a buffer allocated by `sqlite3_malloc` and wraps it as
/// an [`OwnedData`], which is what `sqlite3_deserialize` requires — it takes
/// ownership of the memory and frees/resizes it with SQLite's own allocator,
/// so a plain Rust `Vec` cannot be handed over directly.
fn owned_data_from(bytes: &[u8]) -> Result<OwnedData> {
    let sz = c_int::try_from(bytes.len())
        .map_err(|_| VaultError::Crypto("vault image too large to load".into()))?;
    // sqlite3_malloc(0) legitimately returns NULL; a valid DB image is never
    // empty, so treat a null return as an allocation/corruption failure.
    let ptr = unsafe { rusqlite::ffi::sqlite3_malloc(sz) } as *mut u8;
    let nn = NonNull::new(ptr)
        .ok_or_else(|| VaultError::Crypto("could not allocate vault image".into()))?;
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), nn.as_ptr(), bytes.len());
        Ok(OwnedData::from_raw_nonnull(nn, bytes.len()))
    }
}

fn configure_connection(conn: &Connection) -> Result<()> {
    // Enforce FK cascades (repo -> env -> variable deletes). Journaling and
    // synchronous pragmas are irrelevant for an in-memory database.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

/// Current on-disk schema version, tracked via SQLite's own
/// `PRAGMA user_version` rather than a bootstrap table -- it lives inside the
/// database image itself, so it travels with the encrypted blob and with
/// every backup for free.
const SCHEMA_VERSION: i32 = 4;

/// One ordered step in the schema's history. `noop` marks a step guaranteed
/// not to alter existing data (e.g. `CREATE TABLE IF NOT EXISTS` against
/// tables that already exist), which [`apply_migrations`] uses to decide
/// whether a pre-migration backup is warranted.
struct Migration {
    version: i32,
    sql: &'static str,
    noop: bool,
}

/// Migration 1 is the schema exactly as it originally shipped, written so it
/// is a no-op against any vault that already has these tables. Every vault
/// that predates this versioning scheme reports `user_version = 0` (nothing
/// ever set it) and is simply stamped to 1 without a byte of data changing.
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    noop: true,
    sql: r#"
        CREATE TABLE IF NOT EXISTS repos (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS environments (
            id TEXT PRIMARY KEY,
            repo_id TEXT NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(repo_id, name)
        );
        CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS variables (
            id TEXT PRIMARY KEY,
            env_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            group_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(env_id, key)
        );
        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            env_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL,
            summary TEXT NOT NULL,
            payload TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS members (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            role TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
}, Migration {
    // A genuinely new table: not destructive to any existing row, but this
    // is the first schema change that actually alters structure rather than
    // just stamping a version, so it is not marked `noop` -- a pre-migration
    // backup is taken for it like any real change.
    version: 2,
    noop: false,
    sql: r#"
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            env_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL
        );
        "#,
}, Migration {
    // Each versioned step runs at most once (gated by `user_version`), so
    // unlike migration 1 these plain `ADD COLUMN`s don't need an `IF NOT
    // EXISTS` guard of their own.
    version: 3,
    noop: false,
    sql: r#"
        ALTER TABLE variables ADD COLUMN description TEXT;
        ALTER TABLE variables ADD COLUMN required INTEGER NOT NULL DEFAULT 0;
        "#,
}, Migration {
    // NULL means "no rotation policy", which is every pre-existing row, so
    // this changes no data -- but it is still a structural change and takes a
    // pre-migration backup like any other.
    version: 4,
    noop: false,
    sql: r#"
        ALTER TABLE variables ADD COLUMN rotate_after_days INTEGER;
        "#,
}];

/// Applies whichever of `migrations` have a version greater than `conn`'s
/// current `user_version`, all inside one transaction, then stamps the new
/// version and commits. A connection whose `user_version` is already ahead of
/// `target_version` belongs to a newer build and is refused outright rather
/// than silently operated on. Returns whether any applied step was *not*
/// marked `noop`, so callers can decide whether a pre-migration backup was
/// warranted.
fn apply_migrations(conn: &Connection, migrations: &[Migration], target_version: i32) -> Result<bool> {
    let current: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > target_version {
        return Err(VaultError::FutureSchema {
            found: current,
            supported: target_version,
        });
    }
    let pending: Vec<&Migration> = migrations.iter().filter(|m| m.version > current).collect();
    if pending.is_empty() {
        return Ok(false);
    }
    let non_trivial = pending.iter().any(|m| !m.noop);

    let tx = conn.unchecked_transaction()?;
    for m in &pending {
        tx.execute_batch(m.sql)?;
    }
    tx.execute_batch(&format!("PRAGMA user_version = {target_version};"))?;
    tx.commit()?;
    Ok(non_trivial)
}

fn migrate(conn: &Connection) -> Result<bool> {
    apply_migrations(conn, MIGRATIONS, SCHEMA_VERSION)
}

/// What the app can learn about the vault on disk *without* unlocking it.
///
/// This exists so the lock screen can never again claim there is no vault
/// without also showing the directory it looked in and what it found there —
/// a silent false negative used to send users straight to "create a vault"
/// with no way back.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    /// Directory the vault is read from, for display and troubleshooting.
    pub dir: String,
    pub file_name: String,
    pub exists: bool,
    /// On-disk format: 1 for a legacy vault, 2 for the current one. `None`
    /// when there is no vault file to inspect.
    pub format: Option<u32>,
    pub bytes: u64,
    /// Last write, as Unix milliseconds, so the UI can format it locally.
    pub modified_ms: Option<u64>,
    pub backup_count: usize,
}

impl Vault {
    pub fn exists() -> Result<bool> {
        paths::vault_exists()
    }

    /// Inspects the vault directory without unlocking anything.
    pub fn status() -> Result<VaultStatus> {
        Self::status_in(&paths::data_dir()?)
    }

    /// Same as [`Vault::status`] but rooted at an arbitrary directory.
    pub fn status_in(dir: &Path) -> Result<VaultStatus> {
        let blob_path = blob_file_path(dir);
        let file_name = blob_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let status = VaultStatus {
            dir: dir.to_string_lossy().into_owned(),
            file_name,
            exists: blob_path.exists(),
            format: None,
            bytes: 0,
            modified_ms: None,
            backup_count: count_backups(dir),
        };
        if !status.exists {
            return Ok(status);
        }

        let meta = fs::metadata(&blob_path)?;
        let modified_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);

        // Only the first bytes are needed to tell the formats apart, so a
        // large vault is never read into memory just to render this screen.
        let mut head = [0u8; 8];
        let format = match read_head(&blob_path, &mut head) {
            Ok(read) if read == MAGIC.len() && &head == MAGIC => Some(2),
            Ok(_) => Some(1),
            Err(_) => None,
        };

        Ok(VaultStatus {
            format,
            bytes: meta.len(),
            modified_ms,
            ..status
        })
    }

    /// Creates a brand-new vault protected by `password` at the standard
    /// per-OS application data directory. Fails if one already exists there.
    pub fn create(password: &str) -> Result<Self> {
        Self::create_in(&paths::data_dir()?, password)
    }

    /// Opens the existing vault at the standard location, deriving the key
    /// from `password`. Returns [`VaultError::WrongPassword`] if incorrect.
    pub fn open(password: &str) -> Result<Self> {
        Self::open_in(&paths::data_dir()?, password)
    }

    /// Opens using a raw data key (from the OS keychain) instead of a
    /// password, skipping the slow Argon2id derivation.
    pub fn open_with_key(key_hex: &str) -> Result<Self> {
        Self::open_with_key_in(&paths::data_dir()?, key_hex)
    }

    /// Opens using the recovery code from the user's recovery kit.
    pub fn open_with_recovery(code: &str) -> Result<Self> {
        Self::open_with_recovery_in(&paths::data_dir()?, code)
    }

    /// Same as [`Vault::create`] but rooted at an arbitrary directory —
    /// used directly by tests so they never touch the real OS data dir.
    pub fn create_in(dir: &Path, password: &str) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let blob_path = blob_file_path(dir);
        if blob_path.exists() {
            return Err(VaultError::AlreadyExists(blob_path));
        }
        remove_legacy_session_file(dir);

        let key = DerivedKey::random();
        let header = VaultHeader {
            version: FORMAT_VERSION,
            password: KeySlot::seal(password, &key)?,
            recovery: None,
        };

        let conn = Connection::open_in_memory()?;
        configure_connection(&conn)?;
        migrate(&conn)?;

        let vault = Vault {
            conn,
            key,
            format: Format::V2 { header },
            blob_path,
            dir: dir.to_path_buf(),
        };
        vault.persist()?;
        Ok(vault)
    }

    /// Opens the vault with a master password. A legacy v1 vault is decrypted
    /// with the old scheme and then immediately rewritten in the v2 format —
    /// this is the only moment we hold both the password and the plaintext, so
    /// it is the only moment the upgrade can happen.
    pub fn open_in(dir: &Path, password: &str) -> Result<Self> {
        let blob_path = blob_file_path(dir);
        if !blob_path.exists() {
            return Err(VaultError::NotFound(blob_path));
        }
        remove_legacy_session_file(dir);

        let full_bytes = fs::read(&blob_path)?;
        match parse_vault_bytes(&full_bytes)? {
            VaultFile::V2 { header, payload } => {
                let key = header.password.open(password)?;
                let mut plaintext = decrypt(&key, &payload)?;
                let vault = Self::from_plaintext(
                    key,
                    Format::V2 { header },
                    dir,
                    &plaintext,
                    Some(&full_bytes),
                );
                plaintext.zeroize();
                vault
            }
            VaultFile::V1 { blob } => {
                let meta_path = meta_file_path(dir);
                if !meta_path.exists() {
                    return Err(VaultError::Crypto(
                        "vault file is not recognized and its key metadata is missing".into(),
                    ));
                }
                let meta: VaultMetaFile = serde_json::from_slice(&fs::read(&meta_path)?)?;
                let legacy_key = derive_key(password, &meta)?;
                let mut plaintext = decrypt(&legacy_key, &blob)?;

                // Upgrade in place: a fresh random data key, wrapped under a
                // slot derived from the same password the user just proved. A
                // v1 vault has no backup support (see `rotate_backup`), so
                // there is nothing to preserve pre-migration here either.
                let key = DerivedKey::random();
                let header = VaultHeader {
                    version: FORMAT_VERSION,
                    password: KeySlot::seal(password, &key)?,
                    recovery: None,
                };
                let vault = Self::from_plaintext(key, Format::V2 { header }, dir, &plaintext, None);
                plaintext.zeroize();
                let vault = vault?;
                vault.persist()?;
                // Only once the v2 file is safely in place is the sidecar
                // redundant; a crash before this point simply retries.
                let _ = fs::remove_file(&meta_path);
                Ok(vault)
            }
        }
    }

    /// Opens using a data key remembered in the OS keychain. A v1 vault stays
    /// in v1 format here — without a password there is no way to build a key
    /// slot — and reports [`Vault::needs_migration`] so the UI can ask for one.
    pub fn open_with_key_in(dir: &Path, key_hex: &str) -> Result<Self> {
        let blob_path = blob_file_path(dir);
        if !blob_path.exists() {
            return Err(VaultError::NotFound(blob_path));
        }
        let key = DerivedKey::from_hex(key_hex)?;
        remove_legacy_session_file(dir);

        let full_bytes = fs::read(&blob_path)?;
        let (format, payload) = match parse_vault_bytes(&full_bytes)? {
            VaultFile::V2 { header, payload } => (Format::V2 { header }, payload),
            VaultFile::V1 { blob } => (Format::V1, blob),
        };
        let is_v2 = matches!(format, Format::V2 { .. });
        let mut plaintext = decrypt(&key, &payload)?;
        let vault = Self::from_plaintext(
            key,
            format,
            dir,
            &plaintext,
            is_v2.then_some(full_bytes.as_slice()),
        );
        plaintext.zeroize();
        vault
    }

    /// Builds an unlocked vault by loading a decrypted SQLite image into a
    /// fresh in-memory database. `plaintext` is never written to disk.
    ///
    /// `original_file_bytes` is the vault file exactly as it was on disk
    /// before this unlock, used to take a pre-migration backup if `migrate`
    /// ends up applying a non-trivial step. It is `None` for a v1-format
    /// vault, which has no backup support regardless (see
    /// [`Vault::rotate_backup`]) — by the time a `Vault` exists to back up
    /// from, the in-memory database has already been migrated, so a backup
    /// taken then would just be a copy of the post-migration state.
    fn from_plaintext(
        key: DerivedKey,
        format: Format,
        dir: &Path,
        plaintext: &[u8],
        original_file_bytes: Option<&[u8]>,
    ) -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        let data = owned_data_from(plaintext)?;
        conn.deserialize(DatabaseName::Main, data, false)?;
        configure_connection(&conn)?;
        let migrated_non_trivially = migrate(&conn)?;
        if migrated_non_trivially {
            if let (Format::V2 { .. }, Some(bytes)) = (&format, original_file_bytes) {
                crate::backup::backup_raw_bytes(dir, bytes)?;
            }
        }
        Ok(Vault {
            conn,
            key,
            format,
            blob_path: blob_file_path(dir),
            dir: dir.to_path_buf(),
        })
    }

    /// The data key as hex, for callers that want to stash it in the OS
    /// keychain for "remember on this device". Because this is the data key
    /// and not a password-derived key, a remembered entry survives a master
    /// password change.
    pub fn key_hex(&self) -> String {
        self.key.to_hex()
    }

    /// True for a vault still stored in the legacy v1 format, which cannot
    /// support a recovery kit or a password change until it is upgraded by
    /// unlocking once with the master password.
    pub fn needs_migration(&self) -> bool {
        matches!(self.format, Format::V1)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The raw, **unencrypted** SQLite image of the open vault. Holding an
    /// unlocked `Vault` already grants access to every secret in it, so this
    /// exposes nothing new — but the bytes must never be written to disk.
    pub fn serialize_image(&self) -> Result<Vec<u8>> {
        Ok(self.conn.serialize(DatabaseName::Main)?.to_vec())
    }

    /// Serializes the in-memory database, encrypts the image, and atomically
    /// replaces the on-disk vault file. Called after every mutating operation.
    pub(crate) fn persist(&self) -> Result<()> {
        let image = self.conn.serialize(DatabaseName::Main)?;
        let ciphertext = encrypt(&self.key, &image)?;
        let bytes = match &self.format {
            Format::V1 => ciphertext,
            Format::V2 { header } => encode_v2(header, &ciphertext)?,
        };
        let tmp_path = self.blob_path.with_extension("enc.tmp");
        fs::write(&tmp_path, &bytes)?;
        fs::rename(&tmp_path, &self.blob_path)?;
        Ok(())
    }

    /// Replaces the password key slot with one sealed under `new_password`,
    /// after proving `current_password` opens the existing one. The data key is
    /// untouched, so the database is not re-encrypted and any recovery kit
    /// stays valid.
    pub fn change_password(&mut self, current_password: &str, new_password: &str) -> Result<()> {
        if new_password.is_empty() {
            return Err(VaultError::InvalidInput(
                "new master password must not be empty".into(),
            ));
        }
        let Format::V2 { header } = &self.format else {
            return Err(VaultError::InvalidInput(
                "this vault must be unlocked with its master password once before \
                 the password can be changed"
                    .into(),
            ));
        };
        // Proves the current password and re-derives the same data key.
        let key = header.password.open(current_password)?;
        let new_header = VaultHeader {
            version: FORMAT_VERSION,
            password: KeySlot::seal(new_password, &key)?,
            recovery: header.recovery.clone(),
        };
        self.format = Format::V2 { header: new_header };
        self.persist()
    }

    /// Generates a new recovery code and seals the data key under it, replacing
    /// any previous recovery slot. The returned code is the only copy — it is
    /// never stored in recoverable form.
    pub fn generate_recovery_code(&mut self) -> Result<String> {
        let Format::V2 { header } = &self.format else {
            return Err(VaultError::InvalidInput(
                "this vault must be unlocked with its master password once before \
                 a recovery kit can be created"
                    .into(),
            ));
        };
        let code = crate::crypto::new_recovery_code();
        // Seal under the canonical form so the user may retype the code with or
        // without its grouping hyphens.
        let new_header = VaultHeader {
            version: FORMAT_VERSION,
            password: header.password.clone(),
            recovery: Some(KeySlot::seal(
                &crate::crypto::normalize_recovery_code(&code),
                &self.key,
            )?),
        };
        self.format = Format::V2 { header: new_header };
        self.persist()?;
        Ok(code)
    }

    pub fn has_recovery_code(&self) -> bool {
        matches!(&self.format, Format::V2 { header } if header.recovery.is_some())
    }

    /// Opens the vault with a recovery code instead of the master password.
    pub fn open_with_recovery_in(dir: &Path, code: &str) -> Result<Self> {
        let blob_path = blob_file_path(dir);
        if !blob_path.exists() {
            return Err(VaultError::NotFound(blob_path));
        }
        remove_legacy_session_file(dir);

        let full_bytes = fs::read(&blob_path)?;
        let VaultFile::V2 { header, payload } = parse_vault_bytes(&full_bytes)? else {
            return Err(VaultError::InvalidInput(
                "this vault has no recovery kit".into(),
            ));
        };
        let slot = header
            .recovery
            .as_ref()
            .ok_or_else(|| VaultError::InvalidInput("this vault has no recovery kit".into()))?;
        let key = slot.open(&crate::crypto::normalize_recovery_code(code))?;
        let mut plaintext = decrypt(&key, &payload)?;
        let vault = Self::from_plaintext(
            key,
            Format::V2 { header },
            dir,
            &plaintext,
            Some(&full_bytes),
        );
        plaintext.zeroize();
        vault
    }

    /// Sets the master password without knowing the old one. Only reachable
    /// once the caller has already unlocked the vault (via a recovery code),
    /// which is the proof of ownership.
    pub fn reset_password(&mut self, new_password: &str) -> Result<()> {
        if new_password.is_empty() {
            return Err(VaultError::InvalidInput(
                "new master password must not be empty".into(),
            ));
        }
        let Format::V2 { header } = &self.format else {
            return Err(VaultError::InvalidInput(
                "this vault must be upgraded before its password can be reset".into(),
            ));
        };
        let new_header = VaultHeader {
            version: FORMAT_VERSION,
            password: KeySlot::seal(new_password, &self.key)?,
            recovery: header.recovery.clone(),
        };
        self.format = Format::V2 { header: new_header };
        self.persist()
    }

    /// Drops the in-memory database and zeroizes the key. There is no plaintext
    /// working file to clean up. Kept for API symmetry with lock-on-exit
    /// callers; a plain drop is equivalent.
    pub fn lock(self) -> Result<()> {
        drop(self.conn);
        // `self.key` (DerivedKey) zeroizes itself on drop here.
        Ok(())
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn a_fresh_connection_reaches_the_current_schema_in_one_pass() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        // migration 2 (the projects table) must have applied too
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1);
    }

    #[test]
    fn migrating_twice_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        // second call: nothing pending, must not error or reapply anything
        let ran_non_trivial = migrate(&conn).unwrap();
        assert!(!ran_non_trivial);
        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn a_future_schema_version_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION + 1))
            .unwrap();
        let err = migrate(&conn).unwrap_err();
        assert!(matches!(
            err,
            VaultError::FutureSchema { found, supported }
                if found == SCHEMA_VERSION + 1 && supported == SCHEMA_VERSION
        ));
    }

    #[test]
    fn a_failing_migration_rolls_back_and_leaves_the_version_untouched() {
        let conn = Connection::open_in_memory().unwrap();
        let migrations = [
            Migration {
                version: 1,
                sql: "CREATE TABLE t (id INTEGER);",
                noop: true,
            },
            Migration {
                version: 2,
                sql: "THIS IS NOT VALID SQL;",
                noop: false,
            },
        ];
        assert!(apply_migrations(&conn, &migrations, 2).is_err());

        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 0);
        // the whole batch is one transaction: the earlier valid step must
        // have rolled back too, not just the failing one
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 't'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 0);
    }

    #[test]
    fn a_vault_with_unset_schema_version_migrates_and_keeps_its_data() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        vault.create_repo("api-gateway").unwrap();
        // simulate a vault written before this versioning scheme existed: at
        // user_version 0 none of the later migrations' artifacts exist yet
        vault
            .conn
            .execute_batch(
                "PRAGMA user_version = 0;
                 DROP TABLE projects;
                 ALTER TABLE variables DROP COLUMN description;
                 ALTER TABLE variables DROP COLUMN required;
                 ALTER TABLE variables DROP COLUMN rotate_after_days;",
            )
            .unwrap();
        vault.persist().unwrap();
        drop(vault);

        let reopened = Vault::open_in(dir.path(), "pw").unwrap();
        let repos = reopened.list_repos().unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "api-gateway");
        let version: i32 = reopened
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn a_non_trivial_migration_takes_a_pre_migration_backup() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        vault.create_repo("api-gateway").unwrap();
        // simulate a vault that predates the `projects` table and the
        // description/required columns (schema version 1)
        vault
            .conn
            .execute_batch(
                "PRAGMA user_version = 1;
                 DROP TABLE projects;
                 ALTER TABLE variables DROP COLUMN description;
                 ALTER TABLE variables DROP COLUMN required;
                 ALTER TABLE variables DROP COLUMN rotate_after_days;",
            )
            .unwrap();
        vault.persist().unwrap();
        drop(vault);

        let backups_dir = dir.path().join("backups");
        let before = if backups_dir.exists() {
            std::fs::read_dir(&backups_dir).unwrap().count()
        } else {
            0
        };

        let reopened = Vault::open_in(dir.path(), "pw").unwrap();
        assert_eq!(reopened.list_repos().unwrap()[0].name, "api-gateway");
        let version: i32 = reopened
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        let after = std::fs::read_dir(&backups_dir).unwrap().count();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn opening_a_vault_from_a_newer_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        vault
            .conn
            .execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION + 1))
            .unwrap();
        vault.persist().unwrap();
        drop(vault);

        let err = Vault::open_in(dir.path(), "pw").unwrap_err();
        assert!(matches!(err, VaultError::FutureSchema { .. }));
        // the future-schema vault was not touched by the refused open
        let bytes_before = std::fs::read(dir.path().join("vault.db.enc")).unwrap();
        assert!(Vault::open_in(dir.path(), "pw").is_err());
        let bytes_after = std::fs::read(dir.path().join("vault.db.enc")).unwrap();
        assert_eq!(bytes_before, bytes_after);
    }
}
