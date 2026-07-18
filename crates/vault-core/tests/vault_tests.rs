use vault_core::error::VaultError;
use vault_core::Vault;

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("create temp dir")
}

/// Rewrites `dir` as a legacy v1 vault (database encrypted directly under
/// `Argon2id(password)`, salt in a plaintext sidecar) so the migration path can
/// be exercised against the real thing rather than a mock.
fn write_legacy_v1_vault(dir: &std::path::Path, password: &str, seed: impl FnOnce(&Vault)) {
    use vault_core::crypto::{derive_key, encrypt, VaultMetaFile};

    let staging = tempfile::tempdir().expect("staging dir");
    let vault = Vault::create_in(staging.path(), password).unwrap();
    seed(&vault);
    let image = vault.serialize_image().unwrap();
    drop(vault);

    let meta = VaultMetaFile::new_random();
    let key = derive_key(password, &meta).unwrap();
    let blob = encrypt(&key, &image).unwrap();
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("vault.meta.json"), serde_json::to_vec(&meta).unwrap()).unwrap();
    std::fs::write(dir.join("vault.db.enc"), blob).unwrap();
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
fn renaming_a_repo_and_environment_updates_the_tree() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("old-repo").unwrap();
    let env = vault.create_environment(&repo.id, "old-env").unwrap();

    vault.rename_repo(&repo.id, "new-repo").unwrap();
    vault.rename_environment(&env.id, "new-env").unwrap();

    let summaries = vault.list_repo_summaries().unwrap();
    assert_eq!(summaries[0].name, "new-repo");
    assert_eq!(summaries[0].envs[0].name, "new-env");
}

#[test]
fn renaming_onto_an_existing_name_is_rejected() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let a = vault.create_repo("a").unwrap();
    vault.create_repo("b").unwrap();
    let err = vault.rename_repo(&a.id, "b").unwrap_err();
    assert!(matches!(err, VaultError::Duplicate(_)));

    let env1 = vault.create_environment(&a.id, "local").unwrap();
    vault.create_environment(&a.id, "staging").unwrap();
    let err = vault.rename_environment(&env1.id, "staging").unwrap_err();
    assert!(matches!(err, VaultError::Duplicate(_)));
}

#[test]
fn renaming_a_variable_key_keeps_its_link_group() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env1 = vault.create_environment(&repo.id, "local").unwrap();
    let env2 = vault.create_environment(&repo.id, "staging").unwrap();
    let v1 = vault.add_variable(&env1.id, "OLD_KEY", "shared").unwrap();
    let v2 = vault.add_variable(&env2.id, "OLD_KEY", "shared").unwrap();
    let group_id = vault.link_variables(&[v1.id.clone(), v2.id.clone()]).unwrap();

    vault.rename_variable_key(&v1.id, "NEW_KEY").unwrap();

    let vars1 = vault.list_variables(&env1.id).unwrap();
    assert_eq!(vars1[0].key, "NEW_KEY");
    // the rename must not disturb the group: values still propagate both ways
    assert_eq!(vars1[0].group_id.as_deref(), Some(group_id.as_str()));
    vault.update_variable_value(&v1.id, "rotated").unwrap();
    assert_eq!(vault.list_variables(&env2.id).unwrap()[0].value, "rotated");

    // and the rename is recorded in history
    let summaries: Vec<String> = vault
        .list_snapshots(&env1.id)
        .unwrap()
        .into_iter()
        .map(|s| s.summary)
        .collect();
    assert!(summaries.iter().any(|s| s == "Renamed OLD_KEY to NEW_KEY"));
}

#[test]
fn renaming_a_variable_key_onto_a_sibling_key_is_rejected() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    let a = vault.add_variable(&env.id, "A", "1").unwrap();
    vault.add_variable(&env.id, "B", "2").unwrap();

    let err = vault.rename_variable_key(&a.id, "B").unwrap_err();
    assert!(matches!(err, VaultError::Duplicate(_)));
    // the failed rename left the original untouched
    assert_eq!(vault.list_variables(&env.id).unwrap()[0].key, "A");
}

#[test]
fn deleting_an_environment_dissolves_link_groups_it_leaves_behind() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo_a = vault.create_repo("a").unwrap();
    let env_a = vault.create_environment(&repo_a.id, "local").unwrap();
    let repo_b = vault.create_repo("b").unwrap();
    let env_b = vault.create_environment(&repo_b.id, "local").unwrap();
    let v_a = vault.add_variable(&env_a.id, "SHARED", "v").unwrap();
    let v_b = vault.add_variable(&env_b.id, "SHARED", "v").unwrap();
    vault.link_variables(&[v_a.id.clone(), v_b.id.clone()]).unwrap();
    assert_eq!(vault.linked_group_count().unwrap(), 1);

    vault.delete_environment(&env_a.id).unwrap();

    // the surviving partner is left unlinked rather than showing a stale "linked x1"
    let survivors = vault.list_variables(&env_b.id).unwrap();
    assert_eq!(survivors.len(), 1);
    assert_eq!(survivors[0].group_id, None);
    assert_eq!(vault.linked_group_count().unwrap(), 0);
    assert!(vault.group_usage_counts().unwrap().is_empty());
}

#[test]
fn deleting_a_repo_cascades_and_prunes_groups_across_repos() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo_a = vault.create_repo("a").unwrap();
    let env_a1 = vault.create_environment(&repo_a.id, "local").unwrap();
    let env_a2 = vault.create_environment(&repo_a.id, "staging").unwrap();
    let repo_b = vault.create_repo("b").unwrap();
    let env_b = vault.create_environment(&repo_b.id, "local").unwrap();

    // group 1 spans repo a and repo b; group 2 lives entirely inside repo a
    let a1 = vault.add_variable(&env_a1.id, "SHARED", "v").unwrap();
    let b1 = vault.add_variable(&env_b.id, "SHARED", "v").unwrap();
    vault.link_variables(&[a1.id.clone(), b1.id.clone()]).unwrap();
    let a2 = vault.add_variable(&env_a1.id, "INTERNAL", "w").unwrap();
    let a3 = vault.add_variable(&env_a2.id, "INTERNAL", "w").unwrap();
    vault.link_variables(&[a2.id.clone(), a3.id.clone()]).unwrap();
    assert_eq!(vault.linked_group_count().unwrap(), 2);

    vault.delete_repo(&repo_a.id).unwrap();

    assert_eq!(vault.list_repos().unwrap().len(), 1);
    let survivors = vault.list_variables(&env_b.id).unwrap();
    assert_eq!(survivors[0].group_id, None);
    assert_eq!(vault.linked_group_count().unwrap(), 0);
}

#[test]
fn deleting_a_missing_repo_or_environment_reports_it() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    assert!(matches!(
        vault.delete_repo("nope").unwrap_err(),
        VaultError::Missing(_)
    ));
    assert!(matches!(
        vault.delete_environment("nope").unwrap_err(),
        VaultError::Missing(_)
    ));
}

#[test]
fn deleting_one_member_of_a_three_way_group_keeps_the_group() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let e1 = vault.create_environment(&repo.id, "local").unwrap();
    let e2 = vault.create_environment(&repo.id, "staging").unwrap();
    let e3 = vault.create_environment(&repo.id, "production").unwrap();
    let v1 = vault.add_variable(&e1.id, "K", "v").unwrap();
    let v2 = vault.add_variable(&e2.id, "K", "v").unwrap();
    let v3 = vault.add_variable(&e3.id, "K", "v").unwrap();
    let group_id = vault
        .link_variables(&[v1.id.clone(), v2.id.clone(), v3.id.clone()])
        .unwrap();

    vault.delete_environment(&e1.id).unwrap();

    // two members remain, so the group survives with a corrected count
    assert_eq!(vault.linked_group_count().unwrap(), 1);
    assert_eq!(vault.group_usage_counts().unwrap().get(&group_id), Some(&2));
    assert_eq!(
        vault.list_variables(&e2.id).unwrap()[0].group_id.as_deref(),
        Some(group_id.as_str())
    );
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

// ---------------------------------------------------------------------
// Vault file format v2: key slots, migration, password change, recovery
// ---------------------------------------------------------------------

#[test]
fn a_new_vault_is_written_in_the_v2_single_file_format() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    assert!(!vault.needs_migration());

    let bytes = std::fs::read(dir.path().join("vault.db.enc")).unwrap();
    assert_eq!(&bytes[..8], b"VAULT-R2");
    // the v1 plaintext sidecar is not created at all any more
    assert!(!dir.path().join("vault.meta.json").exists());
}

#[test]
fn a_legacy_v1_vault_is_upgraded_on_first_password_unlock() {
    let dir = temp_dir();
    write_legacy_v1_vault(dir.path(), "legacy-pw", |v| {
        let repo = v.create_repo("api-gateway").unwrap();
        let env = v.create_environment(&repo.id, "local").unwrap();
        v.add_variable(&env.id, "DATABASE_URL", "postgres://legacy").unwrap();
    });
    assert!(dir.path().join("vault.meta.json").exists());

    let vault = Vault::open_in(dir.path(), "legacy-pw").unwrap();
    assert!(!vault.needs_migration());
    let repos = vault.list_repo_summaries().unwrap();
    assert_eq!(repos[0].name, "api-gateway");
    let vars = vault.list_variables(&repos[0].envs[0].id).unwrap();
    assert_eq!(vars[0].value, "postgres://legacy");
    drop(vault);

    // the upgrade is durable and the sidecar is gone
    let bytes = std::fs::read(dir.path().join("vault.db.enc")).unwrap();
    assert_eq!(&bytes[..8], b"VAULT-R2");
    assert!(!dir.path().join("vault.meta.json").exists());

    let reopened = Vault::open_in(dir.path(), "legacy-pw").unwrap();
    assert_eq!(reopened.list_repos().unwrap()[0].name, "api-gateway");
}

#[test]
fn a_legacy_v1_vault_opened_by_key_stays_v1_and_reports_it() {
    let dir = temp_dir();
    write_legacy_v1_vault(dir.path(), "legacy-pw", |v| {
        v.create_repo("r").unwrap();
    });

    // the v1 "remembered key" is Argon2id(password) itself
    let key_hex = {
        use vault_core::crypto::{derive_key, VaultMetaFile};
        let meta: VaultMetaFile =
            serde_json::from_slice(&std::fs::read(dir.path().join("vault.meta.json")).unwrap())
                .unwrap();
        derive_key("legacy-pw", &meta).unwrap().to_hex()
    };

    let mut vault = Vault::open_with_key_in(dir.path(), &key_hex).unwrap();
    assert!(vault.needs_migration());
    assert_eq!(vault.list_repos().unwrap().len(), 1);

    // it stays fully usable and keeps writing v1, so the password still opens it
    vault.create_repo("second").unwrap();
    assert!(matches!(
        vault.change_password("legacy-pw", "new-pw").unwrap_err(),
        VaultError::InvalidInput(_)
    ));
    drop(vault);

    let reopened = Vault::open_in(dir.path(), "legacy-pw").unwrap();
    assert_eq!(reopened.list_repos().unwrap().len(), 2);
}

#[test]
fn changing_the_password_swaps_which_secret_opens_the_vault() {
    let dir = temp_dir();
    let key_hex = {
        let mut vault = Vault::create_in(dir.path(), "old-pw").unwrap();
        vault.create_repo("r").unwrap();
        let key_hex = vault.key_hex();
        vault.change_password("old-pw", "new-pw").unwrap();
        // the data key is untouched by a password change
        assert_eq!(vault.key_hex(), key_hex);
        key_hex
    };

    assert!(matches!(
        Vault::open_in(dir.path(), "old-pw").unwrap_err(),
        VaultError::WrongPassword
    ));
    let vault = Vault::open_in(dir.path(), "new-pw").unwrap();
    assert_eq!(vault.list_repos().unwrap().len(), 1);

    // a key remembered in the OS keychain survives the change
    let by_key = Vault::open_with_key_in(dir.path(), &key_hex).unwrap();
    assert_eq!(by_key.list_repos().unwrap().len(), 1);
}

#[test]
fn changing_the_password_requires_the_current_one() {
    let dir = temp_dir();
    let mut vault = Vault::create_in(dir.path(), "right").unwrap();
    assert!(matches!(
        vault.change_password("wrong", "new").unwrap_err(),
        VaultError::WrongPassword
    ));
    assert!(matches!(
        vault.change_password("right", "").unwrap_err(),
        VaultError::InvalidInput(_)
    ));
    drop(vault);
    // the rejected attempts left the original password working
    Vault::open_in(dir.path(), "right").unwrap();
}

#[test]
fn a_recovery_code_opens_the_vault_and_survives_a_password_change() {
    let dir = temp_dir();
    let code = {
        let mut vault = Vault::create_in(dir.path(), "pw").unwrap();
        assert!(!vault.has_recovery_code());
        vault.create_repo("r").unwrap();
        let code = vault.generate_recovery_code().unwrap();
        assert!(vault.has_recovery_code());
        vault.change_password("pw", "pw2").unwrap();
        code
    };

    // hyphens and case are cosmetic
    let vault = Vault::open_with_recovery_in(dir.path(), &code.to_lowercase()).unwrap();
    assert_eq!(vault.list_repos().unwrap().len(), 1);
    drop(vault);
    let vault = Vault::open_with_recovery_in(dir.path(), &code.replace('-', "")).unwrap();
    assert!(vault.has_recovery_code());
}

#[test]
fn recovery_unlock_lets_the_user_set_a_new_password() {
    let dir = temp_dir();
    let code = {
        let mut vault = Vault::create_in(dir.path(), "forgotten").unwrap();
        vault.create_repo("r").unwrap();
        vault.generate_recovery_code().unwrap()
    };

    let mut vault = Vault::open_with_recovery_in(dir.path(), &code).unwrap();
    vault.reset_password("brand-new").unwrap();
    drop(vault);

    Vault::open_in(dir.path(), "brand-new").unwrap();
    assert!(matches!(
        Vault::open_in(dir.path(), "forgotten").unwrap_err(),
        VaultError::WrongPassword
    ));
    // the kit still works after the reset
    Vault::open_with_recovery_in(dir.path(), &code).unwrap();
}

#[test]
fn regenerating_a_recovery_kit_invalidates_the_previous_code() {
    let dir = temp_dir();
    let mut vault = Vault::create_in(dir.path(), "pw").unwrap();
    let first = vault.generate_recovery_code().unwrap();
    let second = vault.generate_recovery_code().unwrap();
    assert_ne!(first, second);
    drop(vault);

    assert!(matches!(
        Vault::open_with_recovery_in(dir.path(), &first).unwrap_err(),
        VaultError::WrongPassword
    ));
    Vault::open_with_recovery_in(dir.path(), &second).unwrap();
}

#[test]
fn a_vault_without_a_kit_rejects_recovery_unlock() {
    let dir = temp_dir();
    Vault::create_in(dir.path(), "pw").unwrap();
    assert!(matches!(
        Vault::open_with_recovery_in(dir.path(), "AAAAA-AAAAA-AAAAA-AAAAA-AAAAA").unwrap_err(),
        VaultError::InvalidInput(_)
    ));
}

#[test]
fn a_wrong_recovery_code_is_rejected() {
    let dir = temp_dir();
    let mut vault = Vault::create_in(dir.path(), "pw").unwrap();
    vault.generate_recovery_code().unwrap();
    drop(vault);
    assert!(matches!(
        Vault::open_with_recovery_in(dir.path(), "ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ").unwrap_err(),
        VaultError::WrongPassword
    ));
}

// ---------------------------------------------------------------------
// History diffs
// ---------------------------------------------------------------------

#[test]
fn a_snapshot_diff_reports_additions_removals_and_changes() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    let keep = vault.add_variable(&env.id, "KEEP", "same").unwrap();
    let change = vault.add_variable(&env.id, "CHANGE", "before").unwrap();
    let remove = vault.add_variable(&env.id, "REMOVE", "gone").unwrap();

    let baseline = vault
        .list_snapshots(&env.id)
        .unwrap()
        .into_iter()
        .find(|s| s.summary == "Added REMOVE")
        .unwrap();

    vault.update_variable_value(&change.id, "after").unwrap();
    vault.delete_variable(&remove.id).unwrap();
    vault.add_variable(&env.id, "ADDED", "new").unwrap();
    let _ = keep;

    // "what would restoring this snapshot do to the environment as it stands?"
    let rows = vault.diff_snapshot(&baseline.id, "current").unwrap();
    let kinds: Vec<(&str, &str)> = rows.iter().map(|r| (r.key.as_str(), r.kind.as_str())).collect();
    assert_eq!(
        kinds,
        vec![("ADDED", "removed"), ("CHANGE", "changed"), ("REMOVE", "added")]
    );
    let changed = rows.iter().find(|r| r.key == "CHANGE").unwrap();
    assert_eq!(changed.old_value.as_deref(), Some("after"));
    assert_eq!(changed.new_value.as_deref(), Some("before"));
    // an untouched key never appears in a diff
    assert!(!rows.iter().any(|r| r.key == "KEEP"));
}

#[test]
fn the_first_snapshot_of_an_environment_diffs_against_nothing() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    vault.add_variable(&env.id, "FIRST", "v").unwrap();

    let stats = vault.list_snapshots_with_stats(&env.id).unwrap();
    let oldest = stats.last().unwrap();
    assert_eq!(oldest.added, 1);
    assert_eq!(oldest.removed, 0);
    assert_eq!(oldest.changed, 0);

    let rows = vault.diff_snapshot(&oldest.snapshot.id, "previous").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "added");
    assert_eq!(rows[0].key, "FIRST");
}

#[test]
fn snapshot_stats_describe_each_step_of_an_environments_history() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    vault.add_variable(&env.id, "A", "1").unwrap();
    vault.import_env_text(&env.id, "B=2\nC=3\nA=changed\n").unwrap();

    let stats = vault.list_snapshots_with_stats(&env.id).unwrap();
    let newest = &stats[0];
    assert_eq!(newest.snapshot.summary, "Imported 3 variables");
    assert_eq!(newest.added, 2);
    assert_eq!(newest.changed, 1);
    assert_eq!(newest.removed, 0);
}

#[test]
fn restoring_a_single_variable_leaves_its_siblings_alone_and_still_propagates() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    let other_env = vault.create_environment(&repo.id, "staging").unwrap();

    let secret = vault.add_variable(&env.id, "SECRET", "original").unwrap();
    let partner = vault.add_variable(&other_env.id, "SECRET", "original").unwrap();
    vault.link_variables(&[secret.id.clone(), partner.id.clone()]).unwrap();
    let sibling = vault.add_variable(&env.id, "SIBLING", "untouched").unwrap();

    let good = vault
        .list_snapshots(&env.id)
        .unwrap()
        .into_iter()
        .find(|s| s.summary == "Added SIBLING")
        .unwrap();

    vault.update_variable_value(&secret.id, "rotated").unwrap();
    vault.update_variable_value(&sibling.id, "also-changed").unwrap();

    vault.restore_variable_from_snapshot(&good.id, "SECRET").unwrap();

    let vars = vault.list_variables(&env.id).unwrap();
    let restored = vars.iter().find(|v| v.key == "SECRET").unwrap();
    let untouched = vars.iter().find(|v| v.key == "SIBLING").unwrap();
    assert_eq!(restored.value, "original");
    // the single-key restore did not roll back the rest of the environment
    assert_eq!(untouched.value, "also-changed");
    // and the link group followed the restored value
    assert_eq!(vault.list_variables(&other_env.id).unwrap()[0].value, "original");
}

#[test]
fn restoring_a_variable_the_snapshot_never_had_is_reported() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    vault.add_variable(&env.id, "A", "1").unwrap();
    let snap = vault.list_snapshots(&env.id).unwrap().remove(0);

    assert!(matches!(
        vault.restore_variable_from_snapshot(&snap.id, "NEVER_EXISTED").unwrap_err(),
        VaultError::Missing(_)
    ));
    assert!(matches!(
        vault.diff_snapshot(&snap.id, "sideways").unwrap_err(),
        VaultError::InvalidInput(_)
    ));
}

#[test]
fn restoring_a_variable_that_was_since_deleted_adds_it_back() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    let repo = vault.create_repo("r").unwrap();
    let env = vault.create_environment(&repo.id, "local").unwrap();
    let var = vault.add_variable(&env.id, "GONE", "value").unwrap();
    let snap = vault
        .list_snapshots(&env.id)
        .unwrap()
        .into_iter()
        .find(|s| s.summary == "Added GONE")
        .unwrap();
    vault.delete_variable(&var.id).unwrap();

    vault.restore_variable_from_snapshot(&snap.id, "GONE").unwrap();

    let vars = vault.list_variables(&env.id).unwrap();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].value, "value");
}

// ---------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------

#[test]
fn an_exported_backup_restores_the_vault_it_was_taken_from() {
    let dir = temp_dir();
    let elsewhere = temp_dir();
    let backup_path = elsewhere.path().join("my-backup.vrbackup");

    {
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        let repo = vault.create_repo("api-gateway").unwrap();
        let env = vault.create_environment(&repo.id, "local").unwrap();
        vault.add_variable(&env.id, "TOKEN", "backed-up").unwrap();
        vault.export_backup(&backup_path).unwrap();

        // carry on working after the backup was taken
        vault.add_variable(&env.id, "ADDED_LATER", "x").unwrap();
    }

    vault_core::backup::restore_backup_in(dir.path(), &backup_path).unwrap();

    let vault = Vault::open_in(dir.path(), "pw").unwrap();
    let repos = vault.list_repo_summaries().unwrap();
    let vars = vault.list_variables(&repos[0].envs[0].id).unwrap();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].key, "TOKEN");
    assert_eq!(vars[0].value, "backed-up");
}

#[test]
fn a_restore_keeps_a_copy_of_the_vault_it_replaced() {
    let dir = temp_dir();
    let elsewhere = temp_dir();
    let backup_path = elsewhere.path().join("old.vrbackup");
    {
        let vault = Vault::create_in(dir.path(), "pw").unwrap();
        vault.create_repo("original").unwrap();
        vault.export_backup(&backup_path).unwrap();
        vault.create_repo("added-after-backup").unwrap();
    }

    vault_core::backup::restore_backup_in(dir.path(), &backup_path).unwrap();

    let replaced = vault_core::backup::list_backups_in(&dir.path().join("backups")).unwrap();
    assert_eq!(replaced.len(), 1);
    // the superseded vault is recoverable: it still has the repo the backup lacks
    let recovered = temp_dir();
    vault_core::backup::restore_backup_in(
        recovered.path(),
        std::path::Path::new(&replaced[0].path),
    )
    .unwrap();
    let vault = Vault::open_in(recovered.path(), "pw").unwrap();
    assert_eq!(vault.list_repos().unwrap().len(), 2);
}

#[test]
fn restoring_a_file_that_is_not_a_vault_is_refused() {
    let dir = temp_dir();
    Vault::create_in(dir.path(), "pw").unwrap();
    let junk = dir.path().join("notes.txt");
    std::fs::write(&junk, b"this is not a vault at all").unwrap();

    assert!(matches!(
        vault_core::backup::restore_backup_in(dir.path(), &junk).unwrap_err(),
        VaultError::InvalidInput(_)
    ));
    // the live vault was left untouched
    Vault::open_in(dir.path(), "pw").unwrap();
}

#[test]
fn automatic_backups_rotate_and_keep_only_the_newest_ten() {
    let dir = temp_dir();
    let vault = Vault::create_in(dir.path(), "pw").unwrap();
    for i in 0..14 {
        vault.create_repo(&format!("repo-{i}")).unwrap();
        vault.rotate_backup().unwrap().expect("v2 vault backs up");
    }

    let backups = vault.list_backups().unwrap();
    assert_eq!(backups.len(), vault_core::backup::MAX_AUTOMATIC_BACKUPS);

    // newest first, and the newest holds all 14 repos
    let newest = temp_dir();
    vault_core::backup::restore_backup_in(
        newest.path(),
        std::path::Path::new(&backups[0].path),
    )
    .unwrap();
    let restored = Vault::open_in(newest.path(), "pw").unwrap();
    assert_eq!(restored.list_repos().unwrap().len(), 14);
}

#[test]
fn a_legacy_v1_vault_is_not_backed_up_until_it_is_upgraded() {
    let dir = temp_dir();
    write_legacy_v1_vault(dir.path(), "pw", |v| {
        v.create_repo("r").unwrap();
    });
    let key_hex = {
        use vault_core::crypto::{derive_key, VaultMetaFile};
        let meta: VaultMetaFile =
            serde_json::from_slice(&std::fs::read(dir.path().join("vault.meta.json")).unwrap())
                .unwrap();
        derive_key("pw", &meta).unwrap().to_hex()
    };

    let vault = Vault::open_with_key_in(dir.path(), &key_hex).unwrap();
    assert!(vault.rotate_backup().unwrap().is_none());
    assert!(matches!(
        vault.export_backup(&dir.path().join("x.vrbackup")).unwrap_err(),
        VaultError::InvalidInput(_)
    ));
    drop(vault);

    // once upgraded, backups work normally
    let vault = Vault::open_in(dir.path(), "pw").unwrap();
    assert!(vault.rotate_backup().unwrap().is_some());
}

#[test]
fn a_truncated_or_tampered_vault_file_fails_cleanly() {
    let dir = temp_dir();
    Vault::create_in(dir.path(), "pw").unwrap();
    let path = dir.path().join("vault.db.enc");
    let original = std::fs::read(&path).unwrap();

    // truncated inside the header
    std::fs::write(&path, &original[..10]).unwrap();
    assert!(Vault::open_in(dir.path(), "pw").is_err());

    // flipped bit in the encrypted payload is caught by the GCM tag
    let mut tampered = original.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xff;
    std::fs::write(&path, &tampered).unwrap();
    assert!(matches!(
        Vault::open_in(dir.path(), "pw").unwrap_err(),
        VaultError::WrongPassword
    ));

    // and the untouched original still opens
    std::fs::write(&path, &original).unwrap();
    Vault::open_in(dir.path(), "pw").unwrap();
}
