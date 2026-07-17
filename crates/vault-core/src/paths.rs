use crate::error::{Result, VaultError};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("dev", "vaultr", "vault-r").ok_or_else(|| {
        VaultError::Crypto("could not determine application data directory".into())
    })
}

/// Directory where vault files live, e.g. `%APPDATA%\vault-r\` on Windows.
pub fn data_dir() -> Result<PathBuf> {
    let dirs = project_dirs()?;
    let dir = dirs.data_dir().to_path_buf();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Plaintext sidecar holding the Argon2 salt/params, readable before unlock.
pub fn meta_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("vault.meta.json"))
}

/// The single encrypted-at-rest artifact containing the whole SQLite database.
pub fn db_blob_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("vault.db.enc"))
}

/// Live plaintext working copy used only while the vault is unlocked in this
/// process. Deleted on lock/clean exit; overwritten fresh on every unlock.
pub fn session_db_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("vault.session.db"))
}

pub fn vault_exists() -> Result<bool> {
    Ok(meta_path()?.exists() && db_blob_path()?.exists())
}
