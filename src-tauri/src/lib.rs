mod commands;
mod state;

use state::AppState;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first (plugin requirement). Without it, a second
        // launch would race the same vault file instead of focusing this one.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState::default())
        .setup(|app| {
            // No-op on Linux, which has no screen-capture-exclusion API.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_content_protected(true);
            }

            // Enforced here, not in the webview, so a frozen or compromised
            // renderer can't hold the vault open past the idle period.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(10));
                let st = handle.state::<AppState>();
                let minutes = st
                    .auto_lock_minutes
                    .lock()
                    .map(|m| *m)
                    .unwrap_or(state::DEFAULT_AUTO_LOCK_MINUTES);
                if minutes == 0 {
                    continue;
                }
                let unlocked = st.vault.lock().map(|v| v.is_some()).unwrap_or(false);
                if !unlocked {
                    continue;
                }
                let idle = st
                    .last_activity
                    .lock()
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .unwrap_or_default();
                if idle >= std::time::Duration::from_secs(minutes * 60)
                    && commands::perform_lock(&handle).is_ok()
                {
                    let _ = handle.emit("vault-locked", ());
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let _ = commands::perform_lock(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_exists,
            commands::vault_status,
            commands::vault_try_keychain,
            commands::vault_create,
            commands::vault_unlock,
            commands::vault_unlock_with_recovery,
            commands::vault_lock,
            commands::notify_activity,
            commands::vault_needs_migration,
            commands::vault_change_password,
            commands::vault_reset_password,
            commands::vault_has_recovery_code,
            commands::vault_generate_recovery_code,
            commands::save_recovery_kit,
            commands::list_backups,
            commands::export_backup,
            commands::restore_backup,
            commands::list_repo_summaries,
            commands::create_repo,
            commands::rename_repo,
            commands::delete_repo,
            commands::create_environment,
            commands::rename_environment,
            commands::delete_environment,
            commands::duplicate_environment,
            commands::list_variables_with_usage,
            commands::add_variable,
            commands::update_variable_value,
            commands::rename_variable_key,
            commands::delete_variable,
            commands::set_variable_metadata,
            commands::delete_variables,
            commands::move_variables,
            commands::link_candidates,
            commands::link_variables,
            commands::unlink_variable,
            commands::group_members,
            commands::linked_group_count,
            commands::search,
            commands::diff_environments,
            commands::copy_key_to_env,
            commands::copy_missing_to_env,
            commands::unlinked_identical_pairs,
            commands::link_project,
            commands::unlink_project,
            commands::list_projects,
            commands::import_env_text,
            commands::export_env_text,
            commands::export_env_to_file,
            commands::read_dropped_file,
            commands::list_snapshots,
            commands::list_snapshots_with_stats,
            commands::diff_snapshot,
            commands::restore_variable_from_snapshot,
            commands::restore_snapshot,
            commands::list_members,
            commands::add_member,
            commands::remove_member,
            commands::get_meta,
            commands::set_meta,
            commands::copy_secret_to_clipboard,
            commands::generate_secret,
            commands::scan_linked_projects,
            commands::scan_directory,
            commands::apply_gitignore_patterns,
            commands::health_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
