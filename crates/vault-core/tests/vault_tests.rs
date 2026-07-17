use vault_core::error::VaultError;
use vault_core::Vault;

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("create temp dir")
}

#[test]
fn create_then_reopen_with_correct_password_succeeds() {
    let dir = temp_dir();
    {
        let vault = Vault::create_in(dir.path(), "correct horse battery staple").unwrap();
        vault.create_repo("api-gateway").unwrap();
    }
    let vault = Vault::open_in(dir.path(), "correct horse battery staple").unwrap();
    let repos = vault.list_repos().unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "api-gateway");
}

#[test]
fn open_with_wrong_password_fails() {
    let dir = temp_dir();
    {
        let vault = Vault::create_in(dir.path(), "right-password").unwrap();
        vault.create_repo("api-gateway").unwrap();
    }
    let err = Vault::open_in(dir.path(), "wrong-password").unwrap_err();
    assert!(matches!(err, VaultError::WrongPassword));
}

#[test]
fn creating_twice_in_the_same_dir_fails() {
    let dir = temp_dir();
    Vault::create_in(dir.path(), "pw").unwrap();
    let err = Vault::create_in(dir.path(), "pw").unwrap_err();
    assert!(matches!(err, VaultError::AlreadyExists(_)));
}

#[test]
fn repo_env_variable_crud() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();

    let repo = vault.create_repo("web-app").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    let var = vault.add_variable(&env.id, "PORT", "3000").unwrap();

    let vars = vault.list_variables(&env.id).unwrap();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].key, "PORT");
    assert_eq!(vars[0].value, "3000");

    vault.update_variable_value(&var.id, "4000").unwrap();
    let vars = vault.list_variables(&env.id).unwrap();
    assert_eq!(vars[0].value, "4000");

    vault.delete_variable(&var.id).unwrap();
    assert!(vault.list_variables(&env.id).unwrap().is_empty());
}

#[test]
fn duplicate_repo_name_is_rejected() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    vault.create_repo("dup").unwrap();
    let err = vault.create_repo("dup").unwrap_err();
    assert!(matches!(err, VaultError::Duplicate(_)));
}

#[test]
fn duplicate_key_in_same_env_is_rejected() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    vault.add_variable(&env.id, "KEY", "a").unwrap();
    let err = vault.add_variable(&env.id, "KEY", "b").unwrap_err();
    assert!(matches!(err, VaultError::Duplicate(_)));
}

#[test]
fn linking_propagates_value_updates_across_repos_and_envs() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();

    let repo_a = vault.create_repo("api-gateway").unwrap();
    let env_a = vault.create_environment(&repo_a.id, "local").unwrap();
    let var_a = vault.add_variable(&env_a.id, "DATABASE_URL", "postgres://a").unwrap();

    let repo_b = vault.create_repo("web-app").unwrap();
    let env_b = vault.create_environment(&repo_b.id, "local").unwrap();
    let var_b = vault.add_variable(&env_b.id, "DATABASE_URL", "postgres://b").unwrap();

    let group_id = vault
        .link_variables(&[var_a.id.clone(), var_b.id.clone()])
        .unwrap();

    // linking adopts the first variable's value across the whole group
    let vars_a = vault.list_variables(&env_a.id).unwrap();
    let vars_b = vault.list_variables(&env_b.id).unwrap();
    assert_eq!(vars_a[0].value, "postgres://a");
    assert_eq!(vars_b[0].value, "postgres://a");
    assert_eq!(vars_a[0].group_id.as_deref(), Some(group_id.as_str()));
    assert_eq!(vars_b[0].group_id.as_deref(), Some(group_id.as_str()));

    // editing either propagates to all group members
    vault.update_variable_value(&var_b.id, "postgres://updated").unwrap();
    let vars_a = vault.list_variables(&env_a.id).unwrap();
    let vars_b = vault.list_variables(&env_b.id).unwrap();
    assert_eq!(vars_a[0].value, "postgres://updated");
    assert_eq!(vars_b[0].value, "postgres://updated");

    let usage = vault.group_usage_counts().unwrap();
    assert_eq!(usage.get(&group_id), Some(&2));
    assert_eq!(vault.linked_group_count().unwrap(), 1);
}

#[test]
fn unlinking_the_second_to_last_member_dissolves_the_group() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env1 = vault.create_environment(&repo.id, "local").unwrap();
    let env2 = vault.create_environment(&repo.id, "staging").unwrap();
    let v1 = vault.add_variable(&env1.id, "K", "1").unwrap();
    let v2 = vault.add_variable(&env2.id, "K", "1").unwrap();
    vault.link_variables(&[v1.id.clone(), v2.id.clone()]).unwrap();

    vault.unlink_variable(&v1.id).unwrap();

    let vars1 = vault.list_variables(&env1.id).unwrap();
    let vars2 = vault.list_variables(&env2.id).unwrap();
    assert_eq!(vars1[0].group_id, None);
    // only one member remained after unlinking v1, so the group dissolves entirely
    assert_eq!(vars2[0].group_id, None);
    assert_eq!(vault.linked_group_count().unwrap(), 0);
}

#[test]
fn import_merges_existing_keys_and_appends_new_ones() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    vault.add_variable(&env.id, "EXISTING", "old").unwrap();

    let text = "# a comment\nEXISTING=new\nFRESH=\"hello world\"\n";
    let count = vault.import_env_text(&env.id, text).unwrap();
    assert_eq!(count, 2);

    let mut vars = vault.list_variables(&env.id).unwrap();
    vars.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].key, "EXISTING");
    assert_eq!(vars[0].value, "new");
    assert_eq!(vars[1].key, "FRESH");
    assert_eq!(vars[1].value, "hello world");
}

#[test]
fn export_then_import_round_trips() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    vault.add_variable(&env.id, "A", "1").unwrap();
    vault.add_variable(&env.id, "B", "hello world").unwrap();

    let exported = vault.export_env_text(&env.id).unwrap();

    let env2 = vault.create_environment(&repo.id, "staging").unwrap();
    vault.import_env_text(&env2.id, &exported).unwrap();

    let mut original = vault.list_variables(&env.id).unwrap();
    let mut copied = vault.list_variables(&env2.id).unwrap();
    original.sort_by(|a, b| a.key.cmp(&b.key));
    copied.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(original.len(), copied.len());
    for (o, c) in original.iter().zip(copied.iter()) {
        assert_eq!(o.key, c.key);
        assert_eq!(o.value, c.value);
    }
}

#[test]
fn restore_snapshot_brings_back_deleted_variable() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    let var = vault.add_variable(&env.id, "TO_DELETE", "value").unwrap();

    // snapshot exists right after add (summary "Added TO_DELETE")
    let snapshots_before_delete = vault.list_snapshots(&env.id).unwrap();
    let snapshot_with_var = snapshots_before_delete
        .iter()
        .find(|s| s.summary == "Added TO_DELETE")
        .expect("snapshot from add should exist")
        .clone();

    vault.delete_variable(&var.id).unwrap();
    assert!(vault.list_variables(&env.id).unwrap().is_empty());

    vault.restore_snapshot(&snapshot_with_var.id).unwrap();

    let vars = vault.list_variables(&env.id).unwrap();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].key, "TO_DELETE");
    assert_eq!(vars[0].value, "value");
}

#[test]
fn meta_kv_round_trips() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    assert_eq!(vault.get_meta("onboarding_done").unwrap(), None);
    vault.set_meta("onboarding_done", "true").unwrap();
    assert_eq!(vault.get_meta("onboarding_done").unwrap().as_deref(), Some("true"));
}

#[test]
fn open_with_key_hex_skips_password() {
    let dir = temp_dir();
    let key_hex = {
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        vault.create_repo("r").unwrap();
        vault.key_hex()
    };
    let vault = Vault::open_with_key_in(dir.path(), &key_hex).unwrap();
    assert_eq!(vault.list_repos().unwrap().len(), 1);
}
