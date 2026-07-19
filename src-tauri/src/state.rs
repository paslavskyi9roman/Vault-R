use std::sync::Mutex;
use std::time::SystemTime;
use vault_core::Vault;
use zeroize::Zeroizing;

pub const KEYCHAIN_SERVICE: &str = "vault-r";
pub const KEYCHAIN_ACCOUNT: &str = "master-key";

/// Vault meta key holding the idle auto-lock period in minutes (`0` disables).
pub const AUTO_LOCK_META_KEY: &str = "auto_lock_minutes";
pub const DEFAULT_AUTO_LOCK_MINUTES: u64 = 15;

pub struct AppState {
    pub vault: Mutex<Option<Vault>>,
    /// The most recent secret written to the clipboard by the app, so locking
    /// can take it back rather than leaving it sitting there until the 30 s
    /// timer fires.
    pub last_copied: Mutex<Option<String>>,
    /// Wall-clock, not `Instant` — time spent asleep/suspended still counts.
    pub last_activity: Mutex<SystemTime>,
    /// Cached from the vault's meta so the enforcer never touches the store.
    pub auto_lock_minutes: Mutex<u64>,
    /// Held only until `save_recovery_kit` writes it, so the code never
    /// round-trips through the webview.
    pub pending_recovery_code: Mutex<Option<Zeroizing<String>>>,
    /// Failed-unlock streak, for backing off brute-force against a live instance.
    pub failed_attempts: Mutex<u32>,
    pub last_attempt: Mutex<Option<SystemTime>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: Mutex::new(None),
            last_copied: Mutex::new(None),
            last_activity: Mutex::new(SystemTime::now()),
            auto_lock_minutes: Mutex::new(DEFAULT_AUTO_LOCK_MINUTES),
            pending_recovery_code: Mutex::new(None),
            failed_attempts: Mutex::new(0),
            last_attempt: Mutex::new(None),
        }
    }
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

pub fn touch_activity(state: &AppState) {
    if let Ok(mut t) = state.last_activity.lock() {
        *t = SystemTime::now();
    }
}

pub fn refresh_auto_lock(state: &AppState, vault: &Vault) {
    let minutes = vault
        .get_meta(AUTO_LOCK_META_KEY)
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_AUTO_LOCK_MINUTES);
    if let Ok(mut m) = state.auto_lock_minutes.lock() {
        *m = minutes;
    }
    touch_activity(state);
}
