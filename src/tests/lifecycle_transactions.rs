use super::*;

#[test]
fn lifecycle_recovers_interrupted_transaction_before_next_entrypoint() {
    let repository = temp_repo("interrupted-transaction");
    let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &original).unwrap();

    let mut updated = original.clone();
    updated.bundle_id = "balanced-codex-openai@interrupted".to_string();
    updated.artifacts[0]
        .content
        .push_str("\n# interrupted update\n");
    updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
    let updated_file = write_bundle_file(&repository, "interrupted.json", &updated);

    TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(1));
    let interrupted = std::panic::catch_unwind(|| {
        update_bundle_file(&repository, &updated_file).unwrap();
    });
    TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(0));
    assert!(interrupted.is_err());
    assert!(has_transaction_directory(&repository));

    let status = status_repository(&repository).unwrap();
    assert_eq!(
        status.bundle_id.as_deref(),
        Some(original.bundle_id.as_str())
    );
    assert_eq!(
        sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
        original.artifacts[0].sha256
    );
    assert!(
        status
            .artifacts
            .iter()
            .all(|artifact| artifact.status == "managed")
    );
    assert!(!has_transaction_directory(&repository));

    let update = update_bundle_file(&repository, &updated_file).unwrap();
    assert!(
        update
            .artifacts
            .iter()
            .any(|artifact| artifact.status == "update")
    );
}

#[test]
fn lifecycle_recovers_interrupted_atomic_journal_replacement() {
    let repository = temp_repo("journal-replace");
    let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &original).unwrap();

    let mut updated = original.clone();
    updated.bundle_id = "balanced-codex-openai@journal-replace".to_string();
    updated.artifacts[0]
        .content
        .push_str("\n# journal replace interruption\n");
    updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
    let updated_file = write_bundle_file(&repository, "journal-replace.json", &updated);

    TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER.with(|fail_after| fail_after.set(2));
    let interrupted = std::panic::catch_unwind(|| {
        update_bundle_file(&repository, &updated_file).unwrap();
    });
    TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER.with(|fail_after| fail_after.set(0));
    assert!(interrupted.is_err());
    assert!(has_transaction_directory(&repository));

    let status = status_repository(&repository).unwrap();
    assert_eq!(
        status.bundle_id.as_deref(),
        Some(original.bundle_id.as_str())
    );
    assert_eq!(
        sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
        original.artifacts[0].sha256
    );
    assert!(!has_transaction_directory(&repository));
}

#[test]
fn lifecycle_restore_failure_preserves_recoverable_transaction_data() {
    let repository = temp_repo("restore-failure");
    let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &original).unwrap();

    let mut updated = original.clone();
    updated.bundle_id = "balanced-codex-openai@restore-failure".to_string();
    updated.artifacts[0]
        .content
        .push_str("\n# restore failure\n");
    updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
    let updated_file = write_bundle_file(&repository, "restore-failure.json", &updated);

    TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(1));
    let interrupted = std::panic::catch_unwind(|| {
        update_bundle_file(&repository, &updated_file).unwrap();
    });
    TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| fail_after.set(0));
    assert!(interrupted.is_err());
    assert!(has_transaction_directory(&repository));

    TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(true));
    let recovery_error = status_repository(&repository).unwrap_err().to_string();
    TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(false));
    assert!(recovery_error.contains("failed to recover transaction write"));
    assert!(has_transaction_directory(&repository));

    let status = status_repository(&repository).unwrap();
    assert_eq!(
        status.bundle_id.as_deref(),
        Some(original.bundle_id.as_str())
    );
    assert_eq!(
        sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
        original.artifacts[0].sha256
    );
    assert!(!has_transaction_directory(&repository));
}

#[test]
fn lifecycle_returned_journal_error_retains_backup_when_immediate_rollback_fails() {
    let repository = temp_repo("rollback-retains-backup");
    let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &original).unwrap();

    let mut updated = original.clone();
    updated.bundle_id = "balanced-codex-openai@rollback-retains-backup".to_string();
    updated.artifacts[0]
        .content
        .push_str("\n# returned journal error\n");
    updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
    let updated_file = write_bundle_file(&repository, "rollback-retains-backup.json", &updated);

    TRANSACTION_RETURN_JOURNAL_ERROR_AFTER.with(|fail_after| fail_after.set(2));
    TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(true));
    let error = update_bundle_file(&repository, &updated_file)
        .unwrap_err()
        .to_string();
    TRANSACTION_RETURN_JOURNAL_ERROR_AFTER.with(|fail_after| fail_after.set(0));
    TRANSACTION_FAIL_RESTORE.with(|fail| fail.set(false));
    assert!(error.contains("transaction rollback incomplete"));
    assert!(has_transaction_directory(&repository));
    assert!(has_transaction_backup(&repository));

    let status = status_repository(&repository).unwrap();
    assert_eq!(
        status.bundle_id.as_deref(),
        Some(original.bundle_id.as_str())
    );
    assert_eq!(
        sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
        original.artifacts[0].sha256
    );
    assert!(!has_transaction_directory(&repository));
}

#[test]
fn lifecycle_staged_rename_error_restores_backup_before_commit_mark() {
    let repository = temp_repo("staged-rename");
    let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &original).unwrap();

    let mut updated = original.clone();
    updated.bundle_id = "balanced-codex-openai@staged-rename".to_string();
    updated.artifacts[0]
        .content
        .push_str("\n# staged rename failure\n");
    updated.artifacts[0].sha256 = sha256(updated.artifacts[0].content.as_bytes());
    let updated_file = write_bundle_file(&repository, "staged-rename.json", &updated);

    TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER.with(|fail_after| fail_after.set(1));
    let error = update_bundle_file(&repository, &updated_file)
        .unwrap_err()
        .to_string();
    TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER.with(|fail_after| fail_after.set(0));
    assert!(error.contains("injected staged rename error after backup"));
    assert!(!has_transaction_directory(&repository));

    let status = status_repository(&repository).unwrap();
    assert_eq!(
        status.bundle_id.as_deref(),
        Some(original.bundle_id.as_str())
    );
    assert_eq!(
        sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
        original.artifacts[0].sha256
    );
}
