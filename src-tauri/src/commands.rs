use crate::state::{self, AppState};
use tauri::State;
use tauri_plugin_clipboard_manager::ClipboardExt;
use vault_core::backup::BackupInfo;
use vault_core::gitguard::LeakReport;
use vault_core::health::HealthReport;
use vault_core::models::{
    DiffRow, Environment, GroupMember, Member, ProjectInfo, Repo, RepoSummary, SearchResult,
    Snapshot, SnapshotWithStats, UnlinkedMatch, Variable, VariableWithUsage,
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
            // Best-effort: a failed rotation must never block getting in.
            let _ = vault.rotate_backup();
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
    let _ = vault.rotate_backup();
    if remember {
        state::remember_key(&vault.key_hex())?;
    } else {
        state::forget_key();
    }
    *state.vault.lock().map_err(|_| "vault state poisoned")? = Some(vault);
    Ok(())
}

/// Unlocks with a recovery code from the user's recovery kit. The frontend
/// must follow this with [`vault_reset_password`] — a vault whose password is
/// unknown is not usable for long.
#[tauri::command]
pub fn vault_unlock_with_recovery(code: String, state: State<AppState>) -> Result<(), String> {
    let vault = Vault::open_with_recovery(&code).map_err(stringify)?;
    let _ = vault.rotate_backup();
    *state.vault.lock().map_err(|_| "vault state poisoned")? = Some(vault);
    Ok(())
}

#[tauri::command]
pub fn vault_lock(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    // Reclaim a secret still sitting on the clipboard from before the lock.
    if let Ok(mut last) = state.last_copied.lock() {
        if let Some(copied) = last.take() {
            if let Ok(current) = app.clipboard().read_text() {
                if current == copied {
                    let _ = app.clipboard().write_text(String::new());
                }
            }
        }
    }
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

/// Whether the open vault is still in the legacy on-disk format, which blocks
/// password changes, recovery kits and backups until it is upgraded.
#[tauri::command]
pub fn vault_needs_migration(state: State<AppState>) -> Result<bool, String> {
    state::with_vault(&state, |v| Ok(v.needs_migration()))
}

// ---------------------------------------------------------------------
// Master password and recovery kit
// ---------------------------------------------------------------------

#[tauri::command]
pub fn vault_change_password(
    current_password: String,
    new_password: String,
    remember: bool,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| v.rotate_backup().map(|_| ()))?;
    state::with_vault_mut(&state, |v| v.change_password(&current_password, &new_password))?;
    // The keychain holds the data key, which a password change does not alter,
    // so the stored entry stays valid — but honour the user's choice here.
    if remember {
        let key_hex = state::with_vault(&state, |v| Ok(v.key_hex()))?;
        state::remember_key(&key_hex)?;
    } else {
        state::forget_key();
    }
    Ok(())
}

/// Sets a new master password after a recovery unlock, where by definition the
/// old one is not available.
#[tauri::command]
pub fn vault_reset_password(new_password: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault_mut(&state, |v| v.reset_password(&new_password))
}

#[tauri::command]
pub fn vault_has_recovery_code(state: State<AppState>) -> Result<bool, String> {
    state::with_vault(&state, |v| Ok(v.has_recovery_code()))
}

/// Creates a recovery code and returns it. This is the only time the code is
/// ever available — it is stored only in a form that requires the code itself
/// to open — so the frontend must present it for the user to save.
#[tauri::command]
pub fn vault_generate_recovery_code(state: State<AppState>) -> Result<String, String> {
    state::with_vault(&state, |v| v.rotate_backup().map(|_| ()))?;
    state::with_vault_mut(&state, |v| v.generate_recovery_code())
}

// ---------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------

/// Writes the printable recovery kit to a path the user picked. The wording
/// lives here rather than in the frontend so the warnings cannot drift.
#[tauri::command]
pub fn save_recovery_kit(path: String, code: String) -> Result<(), String> {
    let body = format!(
        "Vault-R recovery kit\n\
         \n\
         Recovery code: {code}\n\
         \n\
         This code unlocks your vault without the master password.\n\
         Keep it somewhere safe and offline: anyone holding it can read every\n\
         secret in the vault. Generating a new recovery kit invalidates this code.\n"
    );
    std::fs::write(&path, body).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_backups(state: State<AppState>) -> Result<Vec<BackupInfo>, String> {
    state::with_vault(&state, |v| v.list_backups())
}

#[tauri::command]
pub fn export_backup(path: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.export_backup(std::path::Path::new(&path)))
}

/// Replaces the vault on disk with a backup file. Only valid while locked —
/// the caller then unlocks with whatever password protected that backup.
#[tauri::command]
pub fn restore_backup(path: String, state: State<AppState>) -> Result<(), String> {
    if state
        .vault
        .lock()
        .map_err(|_| "vault state poisoned".to_string())?
        .is_some()
    {
        return Err("lock the vault before restoring a backup".into());
    }
    vault_core::backup::restore_backup(std::path::Path::new(&path)).map_err(stringify)
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
pub fn rename_repo(id: String, new_name: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.rename_repo(&id, &new_name))
}

#[tauri::command]
pub fn delete_repo(id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.delete_repo(&id))
}

#[tauri::command]
pub fn create_environment(
    repo_id: String,
    name: String,
    state: State<AppState>,
) -> Result<Environment, String> {
    state::with_vault(&state, |v| v.create_environment(&repo_id, &name))
}

#[tauri::command]
pub fn rename_environment(
    id: String,
    new_name: String,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| v.rename_environment(&id, &new_name))
}

#[tauri::command]
pub fn delete_environment(id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.delete_environment(&id))
}

#[tauri::command]
pub fn duplicate_environment(
    env_id: String,
    new_name: String,
    copy_values: bool,
    state: State<AppState>,
) -> Result<Environment, String> {
    state::with_vault(&state, |v| v.duplicate_environment(&env_id, &new_name, copy_values))
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
pub fn rename_variable_key(
    var_id: String,
    new_key: String,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| v.rename_variable_key(&var_id, &new_key))
}

#[tauri::command]
pub fn delete_variable(var_id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.delete_variable(&var_id))
}

#[tauri::command]
pub fn delete_variables(var_ids: Vec<String>, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.delete_variables(&var_ids))
}

#[tauri::command]
pub fn move_variables(
    var_ids: Vec<String>,
    target_env_id: String,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| v.move_variables(&var_ids, &target_env_id))
}

#[tauri::command]
pub fn set_variable_metadata(
    var_id: String,
    description: Option<String>,
    required: bool,
    rotate_after_days: Option<i64>,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| {
        v.set_variable_metadata(&var_id, description.as_deref(), required, rotate_after_days)
    })
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
// Environment diff and sync
// ---------------------------------------------------------------------

#[tauri::command]
pub fn diff_environments(
    env_a: String,
    env_b: String,
    state: State<AppState>,
) -> Result<Vec<DiffRow>, String> {
    state::with_vault(&state, |v| v.diff_environments(&env_a, &env_b))
}

#[tauri::command]
pub fn copy_key_to_env(
    source_env_id: String,
    target_env_id: String,
    key: String,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| v.copy_key_to_env(&source_env_id, &target_env_id, &key))
}

#[tauri::command]
pub fn copy_missing_to_env(
    source_env_id: String,
    target_env_id: String,
    state: State<AppState>,
) -> Result<usize, String> {
    state::with_vault(&state, |v| v.copy_missing_to_env(&source_env_id, &target_env_id))
}

#[tauri::command]
pub fn unlinked_identical_pairs(
    env_a: String,
    env_b: String,
    state: State<AppState>,
) -> Result<Vec<UnlinkedMatch>, String> {
    state::with_vault(&state, |v| v.unlinked_identical_pairs(&env_a, &env_b))
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
pub fn list_snapshots_with_stats(
    env_id: String,
    state: State<AppState>,
) -> Result<Vec<SnapshotWithStats>, String> {
    state::with_vault(&state, |v| v.list_snapshots_with_stats(&env_id))
}

/// `against` is `"previous"` (what this snapshot changed) or `"current"`
/// (what restoring it would change).
#[tauri::command]
pub fn diff_snapshot(
    snapshot_id: String,
    against: String,
    state: State<AppState>,
) -> Result<Vec<DiffRow>, String> {
    state::with_vault(&state, |v| v.diff_snapshot(&snapshot_id, &against))
}

#[tauri::command]
pub fn restore_variable_from_snapshot(
    snapshot_id: String,
    key: String,
    state: State<AppState>,
) -> Result<(), String> {
    state::with_vault(&state, |v| v.restore_variable_from_snapshot(&snapshot_id, &key))
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
// Project auto-detection: directories linked to a repo/environment
// ---------------------------------------------------------------------

#[tauri::command]
pub fn link_project(path: String, env_id: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| {
        v.link_project(std::path::Path::new(&path), &env_id).map(|_| ())
    })
}

#[tauri::command]
pub fn unlink_project(path: String, state: State<AppState>) -> Result<(), String> {
    state::with_vault(&state, |v| v.unlink_project(std::path::Path::new(&path)))
}

#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<ProjectInfo>, String> {
    state::with_vault(&state, |v| v.list_projects())
}

// ---------------------------------------------------------------------
// Safety: git leak guard and secret health
// ---------------------------------------------------------------------

/// Scans every directory registered with `vault link`. Reports carry key
/// names and file locations but never secret values, so the panel that
/// renders them is safe to open in front of an audience.
#[tauri::command]
pub fn scan_linked_projects(state: State<AppState>) -> Result<Vec<LeakReport>, String> {
    state::with_vault(&state, |v| v.scan_linked_projects())
}

#[tauri::command]
pub fn scan_directory(path: String, state: State<AppState>) -> Result<LeakReport, String> {
    state::with_vault(&state, |v| v.scan_directory(std::path::Path::new(&path)))
}

/// Appends the given `.gitignore` patterns at `git_root`. This stops future
/// commits; it does not untrack anything already committed, which is why the
/// findings that need rotation say so themselves.
#[tauri::command]
pub fn apply_gitignore_patterns(git_root: String, patterns: Vec<String>) -> Result<usize, String> {
    vault_core::gitguard::apply_gitignore_patterns(std::path::Path::new(&git_root), &patterns)
        .map_err(stringify)
}

#[tauri::command]
pub fn health_report(state: State<AppState>) -> Result<HealthReport, String> {
    state::with_vault(&state, |v| v.health_report())
}

// ---------------------------------------------------------------------
// Secret generator
// ---------------------------------------------------------------------

#[tauri::command]
pub fn generate_secret(kind: String, length: usize) -> Result<String, String> {
    let kind = match kind.as_str() {
        "hex" => vault_core::crypto::SecretKind::Hex,
        "base64" => vault_core::crypto::SecretKind::Base64Url,
        "alnum" => vault_core::crypto::SecretKind::Alphanumeric,
        "words" => vault_core::crypto::SecretKind::Passphrase,
        other => return Err(format!("unknown secret kind '{other}'")),
    };
    vault_core::crypto::generate_secret(kind, length).map_err(stringify)
}

// ---------------------------------------------------------------------
// Clipboard hygiene: copied secrets are wiped after 30s unless overwritten
// ---------------------------------------------------------------------

#[tauri::command]
pub fn copy_secret_to_clipboard(
    app: tauri::AppHandle,
    text: String,
    state: State<AppState>,
) -> Result<(), String> {
    app.clipboard().write_text(text.clone()).map_err(|e| e.to_string())?;
    if let Ok(mut last) = state.last_copied.lock() {
        *last = Some(text.clone());
    }
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
