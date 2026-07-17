use crate::crypto::{decrypt, derive_key, encrypt, DerivedKey, VaultMetaFile};
use crate::error::{Result, VaultError};
use crate::paths;
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};
use std::fs;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use zeroize::Zeroize;

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
    key: DerivedKey,
    blob_path: PathBuf,
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

    /// Opens using a raw derived key (from the OS keychain) instead of a
    /// password, skipping the slow Argon2id derivation.
    pub fn open_with_key(key_hex: &str) -> Result<Self> {
        Self::open_with_key_in(&paths::data_dir()?, key_hex)
    }

    /// Same as [`Vault::create`] but rooted at an arbitrary directory —
    /// used directly by tests so they never touch the real OS data dir.
    pub fn create_in(dir: &Path, password: &str) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let blob_path = blob_file_path(dir);
        let meta_path = meta_file_path(dir);
        if blob_path.exists() {
            return Err(VaultError::AlreadyExists(blob_path));
        }

        let meta = VaultMetaFile::new_random();
        let key = derive_key(password, &meta)?;
        fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?)?;
        remove_legacy_session_file(dir);

        let conn = Connection::open_in_memory()?;
        configure_connection(&conn)?;
        migrate(&conn)?;

        let vault = Vault {
            conn,
            key,
            blob_path,
        };
        vault.persist()?;
        Ok(vault)
    }

    pub fn open_in(dir: &Path, password: &str) -> Result<Self> {
        let meta_path = meta_file_path(dir);
        let blob_path = blob_file_path(dir);
        if !meta_path.exists() || !blob_path.exists() {
            return Err(VaultError::NotFound(blob_path));
        }

        let meta: VaultMetaFile = serde_json::from_slice(&fs::read(&meta_path)?)?;
        let key = derive_key(password, &meta)?;
        let blob = fs::read(&blob_path)?;
        let mut plaintext = decrypt(&key, &blob)?;
        remove_legacy_session_file(dir);

        let vault = Self::from_plaintext(key, blob_path, &plaintext);
        plaintext.zeroize();
        vault
    }

    pub fn open_with_key_in(dir: &Path, key_hex: &str) -> Result<Self> {
        let blob_path = blob_file_path(dir);
        if !blob_path.exists() {
            return Err(VaultError::NotFound(blob_path));
        }
        let mut key_bytes = hex::decode(key_hex).map_err(|e| VaultError::Crypto(e.to_string()))?;
        if key_bytes.len() != 32 {
            key_bytes.zeroize();
            return Err(VaultError::Crypto("stored key has wrong length".into()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&key_bytes);
        key_bytes.zeroize();
        let key = DerivedKey(arr);

        let blob = fs::read(&blob_path)?;
        let mut plaintext = decrypt(&key, &blob)?;
        remove_legacy_session_file(dir);

        let vault = Self::from_plaintext(key, blob_path, &plaintext);
        plaintext.zeroize();
        vault
    }

    /// Builds an unlocked vault by loading a decrypted SQLite image into a
    /// fresh in-memory database. `plaintext` is never written to disk.
    fn from_plaintext(key: DerivedKey, blob_path: PathBuf, plaintext: &[u8]) -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        let data = owned_data_from(plaintext)?;
        conn.deserialize(DatabaseName::Main, data, false)?;
        configure_connection(&conn)?;
        migrate(&conn)?;
        Ok(Vault {
            conn,
            key,
            blob_path,
        })
    }

    /// The raw derived key as hex, for callers that want to stash it in the
    /// OS keychain for "remember on this device".
    pub fn key_hex(&self) -> String {
        hex::encode(self.key.0)
    }

    /// Serializes the in-memory database, encrypts the image, and atomically
    /// replaces the on-disk vault blob. Called after every mutating operation.
    pub(crate) fn persist(&self) -> Result<()> {
        let image = self.conn.serialize(DatabaseName::Main)?;
        let ciphertext = encrypt(&self.key, &image)?;
        let tmp_path = self.blob_path.with_extension("enc.tmp");
        fs::write(&tmp_path, &ciphertext)?;
        fs::rename(&tmp_path, &self.blob_path)?;
        Ok(())
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
