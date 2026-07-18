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

fn read_vault_file(path: &Path) -> Result<VaultFile> {
    let bytes = fs::read(path)?;
    if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
        return Ok(VaultFile::V1 { blob: bytes });
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

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
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
    )?;
    Ok(())
}

impl Vault {
    pub fn exists() -> Result<bool> {
        paths::vault_exists()
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

        match read_vault_file(&blob_path)? {
            VaultFile::V2 { header, payload } => {
                let key = header.password.open(password)?;
                let mut plaintext = decrypt(&key, &payload)?;
                let vault = Self::from_plaintext(key, Format::V2 { header }, dir, &plaintext);
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
                // slot derived from the same password the user just proved.
                let key = DerivedKey::random();
                let header = VaultHeader {
                    version: FORMAT_VERSION,
                    password: KeySlot::seal(password, &key)?,
                    recovery: None,
                };
                let vault = Self::from_plaintext(key, Format::V2 { header }, dir, &plaintext);
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

        let (format, payload) = match read_vault_file(&blob_path)? {
            VaultFile::V2 { header, payload } => (Format::V2 { header }, payload),
            VaultFile::V1 { blob } => (Format::V1, blob),
        };
        let mut plaintext = decrypt(&key, &payload)?;
        let vault = Self::from_plaintext(key, format, dir, &plaintext);
        plaintext.zeroize();
        vault
    }

    /// Builds an unlocked vault by loading a decrypted SQLite image into a
    /// fresh in-memory database. `plaintext` is never written to disk.
    fn from_plaintext(
        key: DerivedKey,
        format: Format,
        dir: &Path,
        plaintext: &[u8],
    ) -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        let data = owned_data_from(plaintext)?;
        conn.deserialize(DatabaseName::Main, data, false)?;
        configure_connection(&conn)?;
        migrate(&conn)?;
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

        let VaultFile::V2 { header, payload } = read_vault_file(&blob_path)? else {
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
        let vault = Self::from_plaintext(key, Format::V2 { header }, dir, &plaintext);
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
