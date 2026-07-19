//! Backups of the vault file.
//!
//! A backup is a byte-for-byte copy of the vault file, which is already
//! encrypted — there is no separate backup crypto and no moment where secrets
//! exist in the clear. It follows that a backup can only be opened with the
//! master password (or recovery code) that vault had *at the time the copy was
//! taken*, which the UI has to say out loud.

use crate::db::Vault;
use crate::error::{Result, VaultError};
use crate::paths;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// How many automatic backups to keep before the oldest is pruned.
pub const MAX_AUTOMATIC_BACKUPS: usize = 10;

pub const BACKUP_EXTENSION: &str = "vrbackup";

/// Leading bytes every backup we accept must have. Restoring a file we cannot
/// recognize would overwrite a working vault with rubbish, so the check is a
/// hard gate rather than a warning.
const MAGIC: &[u8; 8] = b"VAULT-R2";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub created_at: String,
    pub bytes: u64,
}

/// Rejects anything that is not a v2 vault file before it can replace a live
/// vault. Returns the file's bytes so callers do not have to read it twice.
fn validate_backup(path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(path)?;
    if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
        return Err(VaultError::InvalidInput(format!(
            "{} is not a Vault-R backup file",
            path.display()
        )));
    }
    Ok(bytes)
}

fn timestamp_name() -> String {
    format!(
        "vault-{}.{}",
        Utc::now().format("%Y%m%d-%H%M%S%.3f"),
        BACKUP_EXTENSION
    )
}

impl Vault {
    /// Copies the current vault file to `dest` for the user to store wherever
    /// they like. The copy stays encrypted.
    pub fn export_backup(&self, dest: &Path) -> Result<()> {
        // Flush any in-memory state first so the copy is current, not stale.
        self.persist()?;
        if self.needs_migration() {
            return Err(VaultError::InvalidInput(
                "unlock this vault with its master password once to upgrade it \
                 before taking a backup"
                    .into(),
            ));
        }
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::copy(self.blob_path(), dest)?;
        paths::restrict_file(dest);
        Ok(())
    }

    /// Writes a timestamped copy into the vault's own `backups/` directory and
    /// prunes the oldest beyond [`MAX_AUTOMATIC_BACKUPS`]. Called before
    /// anything destructive; a failure here is reported rather than swallowed,
    /// because "we took a backup" is a promise the UI makes to the user.
    pub fn rotate_backup(&self) -> Result<Option<PathBuf>> {
        // A legacy v1 vault has nothing we can restore later; it is upgraded on
        // the next password unlock, and backups begin from there.
        if self.needs_migration() {
            return Ok(None);
        }
        self.persist()?;
        let dir = self.backups_dir()?;
        let dest = dir.join(timestamp_name());
        fs::copy(self.blob_path(), &dest)?;
        paths::restrict_file(&dest);
        prune_backups(&dir, MAX_AUTOMATIC_BACKUPS)?;
        Ok(Some(dest))
    }

    /// The automatic backups for this vault, newest first.
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        list_backups_in(&self.backups_dir()?)
    }

    fn backups_dir(&self) -> Result<PathBuf> {
        let dir = self.dir().join("backups");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn blob_path(&self) -> PathBuf {
        self.dir().join("vault.db.enc")
    }
}

/// Writes `bytes` (a raw vault file image) into `vault_dir`'s `backups/`
/// directory and prunes down to [`MAX_AUTOMATIC_BACKUPS`]. Used by the schema
/// migration path, which must preserve the file exactly as it was *before* a
/// migration ran: by the time a [`Vault`] exists to call
/// [`Vault::rotate_backup`] on, the in-memory database has already been
/// migrated, so persisting it first (as `rotate_backup` does) would overwrite
/// the pre-migration file this is meant to save.
pub(crate) fn backup_raw_bytes(vault_dir: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let dir = vault_dir.join("backups");
    fs::create_dir_all(&dir)?;
    let dest = dir.join(timestamp_name());
    fs::write(&dest, bytes)?;
    paths::restrict_file(&dest);
    prune_backups(&dir, MAX_AUTOMATIC_BACKUPS)?;
    Ok(dest)
}

pub fn list_backups_in(dir: &Path) -> Result<Vec<BackupInfo>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(BACKUP_EXTENSION) {
            continue;
        }
        let meta = entry.metadata()?;
        let created: chrono::DateTime<Utc> = meta
            .modified()
            .map(chrono::DateTime::from)
            .unwrap_or_else(|_| Utc::now());
        out.push(BackupInfo {
            path: path.to_string_lossy().into_owned(),
            created_at: created.to_rfc3339(),
            bytes: meta.len(),
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

fn prune_backups(dir: &Path, keep: usize) -> Result<()> {
    let backups = list_backups_in(dir)?;
    for stale in backups.into_iter().skip(keep) {
        let _ = fs::remove_file(&stale.path);
    }
    Ok(())
}

/// Replaces the vault at the standard location with `src`.
///
/// The vault must be locked: this swaps the file underneath, and the caller
/// then unlocks with whatever password protected the backup. The vault being
/// replaced is itself copied into `backups/` first, so an accidental restore is
/// not the end of the story.
pub fn restore_backup(src: &Path) -> Result<()> {
    restore_backup_in(&paths::data_dir()?, src)
}

pub fn restore_backup_in(dir: &Path, src: &Path) -> Result<()> {
    let bytes = validate_backup(src)?;

    fs::create_dir_all(dir)?;
    let blob_path = dir.join("vault.db.enc");

    if blob_path.exists() {
        let backups = dir.join("backups");
        fs::create_dir_all(&backups)?;
        let superseded = backups.join(format!("replaced-{}", timestamp_name()));
        fs::copy(&blob_path, &superseded)?;
        paths::restrict_file(&superseded);
        prune_backups(&backups, MAX_AUTOMATIC_BACKUPS)?;
    }

    // Land the new bytes atomically so an interrupted restore cannot leave a
    // half-written vault behind.
    let tmp = blob_path.with_extension("enc.restoring");
    fs::write(&tmp, &bytes)?;
    paths::restrict_file(&tmp);
    fs::rename(&tmp, &blob_path)?;

    // A restored v2 file carries its own key metadata; a leftover v1 sidecar
    // would only confuse the next unlock.
    let _ = fs::remove_file(dir.join("vault.meta.json"));
    Ok(())
}
