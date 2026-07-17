use crate::state::{self, AppState};
use tauri::State;
use tauri_plugin_clipboard_manager::ClipboardExt;
use vault_core::models::{
    Environment, GroupMember, Member, Repo, RepoSummary, SearchResult, Snapshot, Variable,
    VariableWithUsage,
};
use vault_core::Vault;

fn stringify(e: vault_core::VaultError) -> String {
    e.to_string()
}

// ---------------------------------------------------------------------
// Lifecycle: create / unlock / lock
// ---------------------------------------------------------------------

#[tauri::command]
pub fn vault_exists() -> Result<bool, String> {
    Vault::exists().map_err(stringify)
}

/// Tries to unlock silently using a key remembered in the OS keychain.
/// Returns `true` if it succeeded, `false` if there's nothing remembered
/// (never an error for the "not remembered" case).
#[tauri::command]
pub fn vault_try_keychain(state: State<AppState>) -> Result<bool, String> {
    let entry = match keyring::Entry::new(state::KEYCHAIN_SERVICE, state::KEYCHAIN_ACCOUNT) {
        Ok(e) => e,
        Err(_) => return Ok(false),
    };
    let key_hex = match entry.get_password() {
        Ok(k) => k,
        Err(_) => return Ok(false),
    };
    match Vault::open_with_key(&key_hex) {
        Ok(vault) => {
            *state.vault.lock().map_err(|_| "vault state poisoned")? = Some(vault);
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub fn vault_create(password: String, remember: bool, state: State<AppState>) -> Result<(), String> {
    let vault = Vault::create(&password).map_err(stringify)?;
    if remember {
        state::remember_key(&vault.key_hex())?;
    }
    *state.vault.lock().map_err(|_| "vault state poisoned")? = Some(vault);
    Ok(())
}

#[tauri::command]
pub fn vault_unlock(password: String, remember: bool, state: State<AppState>) -> Result<(), String> {
    let vault = Vault::open(&password).map_err(stringify)?;
    if remember {
        state::remember_key(&vault.key_hex())?;
    } else {
        state::forget_key();
    }
    *state.vault.lock().map_err(|_| "vault state poisoned")? = Some(vault);
    Ok(())
}

#[tauri::command]
pub fn vault_lock(state: State<AppState>) -> Result<(), String> {
    let taken = state
        .vault
        .lock()
        .map_err(|_| "vault state poisoned".to_string())?
        .take();
    if let Some(vault) = taken {
        vault.lock().map_err(stringify)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Repos / environments
// ---------------------------------------------------------------------

#[tauri::command]
pub fn list_repo_summaries(state: State<AppState>) -> Result<Vec<RepoSummary>, String> {
    state::with_vault(&state, |v| v.list_repo_summaries())
}

#[tauri::command]
pub fn create_repo(name: String, state: State<AppState>) -> Result<Repo, String> {
    state::with_vault(&state, |v| v.create_repo(&name))
}

#[tauri::command]
pub fn create_environment(
    repo_id: String,
    name: String,
    state: State<AppState>,
) -> Result<Environment, String> {
    state::with_vault(&state, |v| v.create_environment(&repo_id, &name))
}

// ---------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------

#[tauri::command]
pub fn list_variables_with_usage(
    env_id: String,
    state: State<AppState>,
) -> Result<Vec<VariableWithUsage>, String> {
    state::with_vault(&state, |v| v.list_variables_with_usage(&env_id))
}

#[tauri::command]
pub fn add_variable(
    env_id: String,
    key: String,
    value: String,
    state: State<AppState>,
) -> Result<Variable, String> {
    state::with_vault(&state, |v| v.add_variable(&env_id, &key, &value))
}

#[tauri::command]
pub fn update_variable_value(
    var_id: String,
    new_value: String,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| v.update_variable_value(&var_id, &new_value))
}

#[tauri::command]
pub fn delete_variable(var_id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.delete_variable(&var_id))
}

// ---------------------------------------------------------------------
// Linked groups
// ---------------------------------------------------------------------

#[tauri::command]
pub fn link_candidates(var_id: String, state: State<AppState>) -> Result<Vec<GroupMember>, String> {
    state::with_vault(&state, |v| v.link_candidates(&var_id))
}

#[tauri::command]
pub fn link_variables(var_ids: Vec<String>, state: State<AppState>) -> Result<String, String> {
    state::with_vault(&state, |v| v.link_variables(&var_ids))
}

#[tauri::command]
pub fn unlink_variable(var_id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.unlink_variable(&var_id))
}

#[tauri::command]
pub fn group_members(group_id: String, state: State<AppState>) -> Result<Vec<GroupMember>, String> {
    state::with_vault(&state, |v| v.group_members(&group_id))
}

#[tauri::command]
pub fn linked_group_count(state: State<AppState>) -> Result<i64, String> {
    state::with_vault(&state, |v| v.linked_group_count())
}

#[tauri::command]
pub fn search(query: String, state: State<AppState>) -> Result<Vec<SearchResult>, String> {
    state::with_vault(&state, |v| v.search(&query))
}

// ---------------------------------------------------------------------
// Import / export
// ---------------------------------------------------------------------

#[tauri::command]
pub fn import_env_text(env_id: String, text: String, state: State<AppState>) -> Result<usize, String> {
    state::with_vault(&state, |v| v.import_env_text(&env_id, &text))
}

#[tauri::command]
pub fn export_env_text(env_id: String, state: State<AppState>) -> Result<String, String> {
    state::with_vault(&state, |v| v.export_env_text(&env_id))
}

/// Writes the environment's `.env` text to a path already chosen by the
/// frontend via the native save dialog.
#[tauri::command]
pub fn export_env_to_file(env_id: String, path: String, state: State<AppState>) -> Result<(), String> {
    let text = state::with_vault(&state, |v| v.export_env_text(&env_id))?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// History
// ---------------------------------------------------------------------

#[tauri::command]
pub fn list_snapshots(env_id: String, state: State<AppState>) -> Result<Vec<Snapshot>, String> {
    state::with_vault(&state, |v| v.list_snapshots(&env_id))
}

#[tauri::command]
pub fn restore_snapshot(snapshot_id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.restore_snapshot(&snapshot_id))
}

// ---------------------------------------------------------------------
// Members (local-only mock of the Share modal)
// ---------------------------------------------------------------------

#[tauri::command]
pub fn list_members(state: State<AppState>) -> Result<Vec<Member>, String> {
    state::with_vault(&state, |v| v.list_members())
}

#[tauri::command]
pub fn add_member(email: String, role: String, state: State<AppState>) -> Result<Member, String> {
    state::with_vault(&state, |v| v.add_member(&email, &role))
}

#[tauri::command]
pub fn remove_member(id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.remove_member(&id))
}

// ---------------------------------------------------------------------
// Misc metadata (onboarding flag, etc.)
// ---------------------------------------------------------------------

#[tauri::command]
pub fn get_meta(key: String, state: State<AppState>) -> Result<Option<String>, String> {
    state::with_vault(&state, |v| v.get_meta(&key))
}

#[tauri::command]
pub fn set_meta(key: String, value: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.set_meta(&key, &value))
}

// ---------------------------------------------------------------------
// Clipboard hygiene: copied secrets are wiped after 30s unless overwritten
// ---------------------------------------------------------------------

#[tauri::command]
pub fn copy_secret_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text.clone()).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        if let Ok(current) = app.clipboard().read_text() {
            if current == text {
                let _ = app.clipboard().write_text(String::new());
            }
        }
    });
    Ok(())
}
