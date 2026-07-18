mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::vault_exists,
            commands::vault_status,
            commands::vault_try_keychain,
            commands::vault_create,
            commands::vault_unlock,
            commands::vault_unlock_with_recovery,
            commands::vault_lock,
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
