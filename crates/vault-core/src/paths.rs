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
    restrict_dir(&dir);
    Ok(dir)
}

/// Tightens a freshly written secret file to owner-only (`0600`) on Unix
/// (Linux/macOS). A no-op on Windows, where the per-user profile ACL on
/// `%APPDATA%` already restricts access to the owner.
#[cfg(unix)]
pub fn restrict_file(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
pub fn restrict_file(_path: &std::path::Path) {}

/// Tightens a vault directory to owner-only (`0700`) on Unix.
#[cfg(unix)]
fn restrict_dir(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn restrict_dir(_path: &std::path::Path) {}

/// Plaintext sidecar holding the Argon2 salt/params of a legacy v1 vault.
/// Current (v2) vaults carry their key metadata in the vault file itself and
/// leave no sidecar behind.
pub fn meta_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("vault.meta.json"))
}

/// The single encrypted-at-rest artifact containing the whole SQLite database.
/// While unlocked, the database lives only in memory; this file is the sole
/// plaintext-free representation on disk.
pub fn db_blob_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("vault.db.enc"))
}

/// Where rotating local backups are kept.
pub fn backups_dir() -> Result<PathBuf> {
    let dir = data_dir()?.join("backups");
    fs::create_dir_all(&dir)?;
    restrict_dir(&dir);
    Ok(dir)
}

pub fn vault_exists() -> Result<bool> {
    Ok(db_blob_path()?.exists())
}
