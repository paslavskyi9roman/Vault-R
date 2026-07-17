use crate::crypto::{decrypt, derive_key, encrypt, DerivedKey, VaultMetaFile};
use crate::error::{Result, VaultError};
use crate::paths;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

/// An unlocked vault: a live SQLite connection to a plaintext working copy on
/// disk, backed by an AES-256-GCM encrypted blob that is the only artifact
/// meant to persist across app restarts.
///
/// The working copy (`session_db_path`) is deleted on [`Vault::lock`] / clean
/// process exit and is always overwritten fresh from the encrypted blob on
/// the next open, so a leftover file from an abnormal crash is never
/// trusted — it is simply discarded.
#[derive(Debug)]
pub struct Vault {
    pub(crate) conn: Connection,
    key: DerivedKey,
    session_db_path: PathBuf,
    blob_path: PathBuf,
}

fn meta_file_path(dir: &Path) -> PathBuf {
    dir.join("vault.meta.json")
}
fn blob_file_path(dir: &Path) -> PathBuf {
    dir.join("vault.db.enc")
}
fn session_file_path(dir: &Path) -> PathBuf {
    dir.join("vault.session.db")
}

fn configure_connection(conn: &Connection) -> Result<()> {
    // DELETE journal mode (not WAL) keeps the entire database state in the
    // single working file, since persist() re-encrypts that file wholesale.
    conn.execute_batch(
        "PRAGMA journal_mode = DELETE; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;",
    )?;
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

        let session_db_path = session_file_path(dir);
        let _ = fs::remove_file(&session_db_path);
        let conn = Connection::open(&session_db_path)?;
        configure_connection(&conn)?;
        migrate(&conn)?;

        let vault = Vault {
            conn,
            key,
            session_db_path,
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
        let plaintext = decrypt(&key, &blob)?;

        let session_db_path = session_file_path(dir);
        let _ = fs::remove_file(&session_db_path);
        fs::write(&session_db_path, &plaintext)?;

        let conn = Connection::open(&session_db_path)?;
        configure_connection(&conn)?;
        migrate(&conn)?;

        Ok(Vault {
            conn,
            key,
            session_db_path,
            blob_path,
        })
    }

    pub fn open_with_key_in(dir: &Path, key_hex: &str) -> Result<Self> {
        let blob_path = blob_file_path(dir);
        if !blob_path.exists() {
            return Err(VaultError::NotFound(blob_path));
        }
        let key_bytes = hex::decode(key_hex).map_err(|e| VaultError::Crypto(e.to_string()))?;
        if key_bytes.len() != 32 {
            return Err(VaultError::Crypto("stored key has wrong length".into()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&key_bytes);
        let key = DerivedKey(arr);

        let blob = fs::read(&blob_path)?;
        let plaintext = decrypt(&key, &blob)?;

        let session_db_path = session_file_path(dir);
        let _ = fs::remove_file(&session_db_path);
        fs::write(&session_db_path, &plaintext)?;

        let conn = Connection::open(&session_db_path)?;
        configure_connection(&conn)?;
        migrate(&conn)?;

        Ok(Vault {
            conn,
            key,
            session_db_path,
            blob_path,
        })
    }

    /// The raw derived key as hex, for callers that want to stash it in the
    /// OS keychain for "remember on this device".
    pub fn key_hex(&self) -> String {
        hex::encode(self.key.0)
    }

    /// Re-encrypts the current working copy and atomically replaces the
    /// on-disk vault blob. Called after every mutating operation.
    pub(crate) fn persist(&self) -> Result<()> {
        let plaintext = fs::read(&self.session_db_path)?;
        let ciphertext = encrypt(&self.key, &plaintext)?;
        let tmp_path = self.blob_path.with_extension("enc.tmp");
        fs::write(&tmp_path, &ciphertext)?;
        fs::rename(&tmp_path, &self.blob_path)?;
        Ok(())
    }

    /// Deletes the plaintext working copy. Call on app exit or explicit lock.
    pub fn lock(self) -> Result<()> {
        let session_db_path = self.session_db_path.clone();
        drop(self.conn);
        let _ = fs::remove_file(&session_db_path);
        Ok(())
    }
}
