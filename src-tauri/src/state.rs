use std::sync::Mutex;
use vault_core::Vault;

pub const KEYCHAIN_SERVICE: &str = "vault-r";
pub const KEYCHAIN_ACCOUNT: &str = "master-key";

#[derive(Default)]
pub struct AppState {
    pub vault: Mutex<Option<Vault>>,
    /// The most recent secret written to the clipboard by the app, so locking
    /// can take it back rather than leaving it sitting there until the 30 s
    /// timer fires.
    pub last_copied: Mutex<Option<String>>,
}

/// Runs `f` against the unlocked vault, or returns a "locked" error string
/// suitable for surfacing directly to the frontend.
pub fn with_vault<T>(
    state: &tauri::State<AppState>,
    f: impl FnOnce(&Vault) -> vault_core::Result<T>,
) -> Result<T, String> {
    let guard = state.vault.lock().map_err(|_| "vault state poisoned".to_string())?;
    let vault = guard.as_ref().ok_or_else(|| "vault is locked".to_string())?;
    f(vault).map_err(|e| e.to_string())
}

/// As [`with_vault`], for the operations that rewrite the vault's key slots.
pub fn with_vault_mut<T>(
    state: &tauri::State<AppState>,
    f: impl FnOnce(&mut Vault) -> vault_core::Result<T>,
) -> Result<T, String> {
    let mut guard = state.vault.lock().map_err(|_| "vault state poisoned".to_string())?;
    let vault = guard.as_mut().ok_or_else(|| "vault is locked".to_string())?;
    f(vault).map_err(|e| e.to_string())
}

pub fn remember_key(key_hex: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).map_err(|e| e.to_string())?;
    entry.set_password(key_hex).map_err(|e| e.to_string())
}

pub fn forget_key() {
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        let _ = entry.delete_password();
    }
}
