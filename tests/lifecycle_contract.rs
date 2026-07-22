use model_routing::*;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_repo(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("model-routing-{name}-{unique}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn test_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
        output
    })
}

fn apply_bundle_file_with_bundle(
    repository: &Path,
    bundle: &RoutingBundleV1,
) -> Result<LifecycleReport> {
    let bundle_file = write_bundle_file(repository, "bundle.json", bundle);
    apply_bundle_file(repository, &bundle_file)
}

fn preview_bundle_with_bundle(
    repository: &Path,
    bundle: &RoutingBundleV1,
) -> Result<LifecycleReport> {
    let bundle_file = write_bundle_file(repository, "preview-bundle.json", bundle);
    preview_bundle_file(repository, &bundle_file)
}

fn write_bundle_file(repository: &Path, name: &str, bundle: &RoutingBundleV1) -> PathBuf {
    let bundle_file = repository.join(name);
    fs::write(&bundle_file, serde_json::to_vec_pretty(bundle).unwrap()).unwrap();
    bundle_file
}

fn assert_codex_config_entry(content: &str, agent_type: &str, config_file: &str) {
    let parsed: toml::Value = toml::from_str(content).unwrap();
    assert_eq!(
        parsed["agents"][agent_type]["config_file"].as_str(),
        Some(config_file)
    );
}

fn assert_no_codex_config_entry(content: &str, agent_type: &str) {
    let parsed: toml::Value = toml::from_str(content).unwrap();
    assert!(
        parsed
            .get("agents")
            .and_then(toml::Value::as_table)
            .and_then(|agents| agents.get(agent_type))
            .is_none()
    );
}

fn assert_codex_v2_activation(content: &str) {
    let parsed: toml::Value = toml::from_str(content).unwrap();
    assert_eq!(
        parsed["features"]["multi_agent_v2"]["enabled"].as_bool(),
        Some(true)
    );
    assert_eq!(
        parsed["features"]["multi_agent_v2"]["hide_spawn_agent_metadata"].as_bool(),
        Some(false)
    );
}

fn assert_codex_v2_disabled(content: &str) {
    let parsed: toml::Value = toml::from_str(content).unwrap();
    assert_eq!(
        parsed["features"]["multi_agent_v2"]["enabled"].as_bool(),
        Some(false)
    );
    assert_eq!(
        parsed["features"]["multi_agent_v2"]["hide_spawn_agent_metadata"].as_bool(),
        Some(false)
    );
}

#[test]
fn setup_config_lifecycle_persists_normalized_config_and_reuses_manifest_flow() {
    let repository = temp_repo("setup-config-lifecycle");
    let config_file = repository.join("input.setup.toml");
    let original = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
    let original_toml = setup_spec_to_canonical_toml(&original).unwrap();
    fs::write(&config_file, &original_toml).unwrap();

    let preview = preview_setup_config_file(&repository, &config_file).unwrap();
    assert_eq!(preview.action, "preview");
    assert!(
        preview
            .artifacts
            .iter()
            .any(|artifact| { artifact.path == SETUP_CONFIG_PATH && artifact.status == "create" })
    );

    let applied = apply_setup_config_file(&repository, &config_file).unwrap();
    assert_eq!(applied.action, "apply");
    assert_eq!(
        fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
        original_toml
    );
    assert!(
        !repository.join(".planr").exists(),
        "standalone setup must not create .planr"
    );
    let status = status_repository(&repository).unwrap();
    assert!(
        status
            .artifacts
            .iter()
            .any(|artifact| { artifact.path == SETUP_CONFIG_PATH && artifact.status == "managed" })
    );
    let saved_preview = preview_saved_setup(&repository).unwrap();
    assert!(
        saved_preview.artifacts.iter().any(|artifact| {
            artifact.path == SETUP_CONFIG_PATH && artifact.status == "unchanged"
        })
    );

    let mut updated = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
    let worker = updated.selected_roles.get_mut("worker").unwrap();
    worker.model = "gpt-5.6-sol".to_string();
    worker.effort = Some("medium".to_string());
    worker.spawn = Some(SetupSpawnPolicy {
        agent_type: "switchloom_worker".to_string(),
        task_name: "worker".to_string(),
        fork_turns: ForkPolicy {
            mode: "none".to_string(),
            turns: None,
        },
    });
    let updated_file = repository.join("updated.setup.toml");
    let updated_toml = setup_spec_to_canonical_toml(&updated).unwrap();
    fs::write(&updated_file, &updated_toml).unwrap();
    let update = update_setup_config_file(&repository, &updated_file).unwrap();
    assert_eq!(update.action, "update");
    assert_eq!(
        fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
        updated_toml
    );
    assert!(
        repository
            .join(".codex/agents/switchloom_worker.toml")
            .exists()
    );

    let rollback = rollback_repository(&repository).unwrap();
    assert_eq!(rollback.action, "rollback");
    assert_eq!(
        fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
        original_toml
    );
    assert!(
        !repository
            .join(".codex/agents/switchloom_worker.toml")
            .exists()
    );

    let uninstall = uninstall_repository(&repository).unwrap();
    assert_eq!(uninstall.action, "uninstall");
    assert!(!repository.join(SETUP_CONFIG_PATH).exists());
    assert!(!repository.join(".model-routing/manifest.json").exists());
}

#[test]
fn setup_recipe_apply_persists_config_and_rejects_existing_conflicts() {
    let repository = temp_repo("setup-recipe-lifecycle");
    let spec = setup_spec_for_policy("balanced", "codex", Integration::Planr).unwrap();
    let recipe = setup_spec_to_recipe(&spec).unwrap();

    let preview = preview_setup_recipe(&repository, &recipe).unwrap();
    assert_eq!(preview.action, "preview");
    assert!(
        preview.artifacts.iter().any(|artifact| {
            artifact.path == ".planr/agents.toml" && artifact.status == "create"
        })
    );
    apply_setup_recipe(&repository, &recipe).unwrap();
    assert_eq!(
        fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
        setup_spec_to_canonical_toml(&spec).unwrap()
    );
    assert!(repository.join(".planr/agents.toml").exists());

    let conflict_repo = temp_repo("setup-recipe-conflict");
    fs::create_dir_all(conflict_repo.join(".switchloom")).unwrap();
    fs::write(conflict_repo.join(SETUP_CONFIG_PATH), "not managed\n").unwrap();
    let error = apply_setup_recipe(&conflict_repo, &recipe)
        .unwrap_err()
        .to_string();
    assert!(error.contains(SETUP_CONFIG_PATH));
}

#[test]
fn opencode_native_lifecycle_manages_project_agents() {
    let repository = temp_repo("opencode-lifecycle");
    let bundle = compile_policy("balanced", "opencode-native", Integration::Standalone).unwrap();
    let preview = preview_bundle_with_bundle(&repository, &bundle).unwrap();
    assert!(preview.artifacts.iter().any(|artifact| {
        artifact.path == ".opencode/agents/model-routing-preset-worker.md"
            && artifact.status == "create"
    }));

    apply_bundle_file_with_bundle(&repository, &bundle).unwrap();
    let worker = repository.join(".opencode/agents/model-routing-preset-worker.md");
    let driver = repository.join(".opencode/agents/model-routing-preset-driver.md");
    assert!(worker.exists());
    assert!(driver.exists());
    let driver_content = fs::read_to_string(&driver).unwrap();
    assert!(driver_content.contains("permission:"));
    assert!(driver_content.contains("model-routing-preset-worker: allow"));

    let status = status_repository(&repository).unwrap();
    assert!(status.artifacts.iter().any(|artifact| {
        artifact.path == ".opencode/agents/model-routing-preset-worker.md"
            && artifact.status == "managed"
    }));
    uninstall_repository(&repository).unwrap();
    assert!(!worker.exists());
    assert!(!driver.exists());
    assert!(!repository.join(".model-routing/manifest.json").exists());
}

#[test]
fn pi_external_lifecycle_manages_workflow_artifacts() {
    let repository = temp_repo("pi-external-lifecycle");
    let bundle = compile_policy("balanced", "pi-external", Integration::Standalone).unwrap();
    let preview = preview_bundle_with_bundle(&repository, &bundle).unwrap();
    assert!(preview.artifacts.iter().any(|artifact| {
        artifact.path == ".pi/workflows/model-routing-preset-runner.json"
            && artifact.status == "create"
    }));

    apply_bundle_file_with_bundle(&repository, &bundle).unwrap();
    let workflow = repository.join(".pi/workflows/model-routing-preset-runner.json");
    assert!(workflow.exists());
    let workflow_content = fs::read_to_string(&workflow).unwrap();
    assert!(workflow_content.contains("\"external-runner\""));
    assert!(workflow_content.contains("\"--no-tools\""));
    assert!(workflow_content.contains("\"PI_CODING_AGENT_DIR"));

    let status = status_repository(&repository).unwrap();
    assert!(status.artifacts.iter().any(|artifact| {
        artifact.path == ".pi/workflows/model-routing-preset-runner.json"
            && artifact.status == "managed"
    }));
    uninstall_repository(&repository).unwrap();
    assert!(!workflow.exists());
    assert!(!repository.join(".model-routing/manifest.json").exists());
}

#[test]
fn lifecycle_update_and_rollback_are_manifest_aware() {
    let repository = temp_repo("update-rollback");
    let original = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &original).unwrap();

    let mut updated = original.clone();
    updated.bundle_id = "balanced-codex-openai@updated".to_string();
    updated.artifacts[0].content.push_str("\n# updated\n");
    updated.artifacts[0].sha256 = test_sha256(updated.artifacts[0].content.as_bytes());
    let bundle_file = write_bundle_file(&repository, "updated-bundle.json", &updated);

    let update = update_bundle_file(&repository, &bundle_file).unwrap();
    assert_eq!(update.action, "update");
    assert!(
        update
            .artifacts
            .iter()
            .any(|artifact| artifact.status == "update")
    );
    assert_eq!(
        test_sha256(&fs::read(repository.join(&updated.artifacts[0].path)).unwrap()),
        updated.artifacts[0].sha256
    );

    let rollback = rollback_repository(&repository).unwrap();
    assert_eq!(rollback.action, "rollback");
    assert!(
        rollback
            .artifacts
            .iter()
            .any(|artifact| artifact.status == "rollback")
    );
    assert_eq!(
        test_sha256(&fs::read(repository.join(&original.artifacts[0].path)).unwrap()),
        original.artifacts[0].sha256
    );
}

#[test]
fn lifecycle_codex_config_merges_unrelated_entries_update_rollback_and_uninstall() {
    let repository = temp_repo("codex-config-ownership");
    fs::create_dir_all(repository.join(".codex/agents")).unwrap();
    fs::write(
            repository.join(".codex/config.toml"),
            "[agents.local_reviewer]\nconfig_file = \"./agents/local-reviewer.toml\"\n\n[features]\nlocal = true\n",
        )
        .unwrap();
    fs::write(
        repository.join(".codex/agents/local-reviewer.toml"),
        "name = \"local_reviewer\"\n",
    )
    .unwrap();
    let global_codex_home = temp_repo("global-codex-home");
    fs::write(
        global_codex_home.join("config.toml"),
        "[agents.global_reviewer]\nconfig_file = \"./agents/global-reviewer.toml\"\n",
    )
    .unwrap();
    let global_config_before = fs::read(global_codex_home.join("config.toml")).unwrap();

    let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    let applied = apply_bundle_file_with_bundle(&repository, &codex).unwrap();
    assert!(
        applied.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/config.toml" && artifact.status == "update"
        })
    );
    let config_after_apply = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_config_entry(
        &config_after_apply,
        "local_reviewer",
        "./agents/local-reviewer.toml",
    );
    assert_codex_config_entry(
        &config_after_apply,
        "model_routing_terra_high",
        "./agents/model-routing-terra-high.toml",
    );
    assert_codex_config_entry(
        &config_after_apply,
        "model_routing_sol_high",
        "./agents/model-routing-sol-high.toml",
    );
    assert_codex_v2_activation(&config_after_apply);
    assert!(config_after_apply.contains("[features]"));

    let mixed = compile_policy("balanced", "mixed-host", Integration::Standalone).unwrap();
    let mixed_file = write_bundle_file(&repository, "mixed.json", &mixed);
    let update = update_bundle_file(&repository, &mixed_file).unwrap();
    assert!(
        update.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/config.toml" && artifact.status == "update"
        })
    );
    let config_after_update = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_config_entry(
        &config_after_update,
        "local_reviewer",
        "./agents/local-reviewer.toml",
    );
    assert_codex_config_entry(
        &config_after_update,
        "model_routing_terra_high",
        "./agents/model-routing-terra-high.toml",
    );
    assert_codex_v2_activation(&config_after_update);
    assert_no_codex_config_entry(&config_after_update, "model_routing_sol_medium");
    assert_no_codex_config_entry(&config_after_update, "model_routing_sol_ultra");

    let rollback = rollback_repository(&repository).unwrap();
    assert!(rollback.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/config.toml" && artifact.status == "rollback"
    }));
    let config_after_rollback = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_config_entry(
        &config_after_rollback,
        "local_reviewer",
        "./agents/local-reviewer.toml",
    );
    assert_codex_config_entry(
        &config_after_rollback,
        "model_routing_sol_medium",
        "./agents/model-routing-sol-medium.toml",
    );
    assert_codex_config_entry(
        &config_after_rollback,
        "model_routing_sol_ultra",
        "./agents/model-routing-sol-ultra.toml",
    );
    assert_codex_v2_activation(&config_after_rollback);

    let uninstall = uninstall_repository(&repository).unwrap();
    assert!(
        uninstall.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/config.toml" && artifact.status == "removed"
        })
    );
    let config_after_uninstall = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_config_entry(
        &config_after_uninstall,
        "local_reviewer",
        "./agents/local-reviewer.toml",
    );
    assert_no_codex_config_entry(&config_after_uninstall, "model_routing_terra_high");
    assert_no_codex_config_entry(&config_after_uninstall, "model_routing_sol_high");
    assert!(
        toml::from_str::<toml::Value>(&config_after_uninstall).unwrap()["features"]
            .get("multi_agent_v2")
            .is_none(),
        "Switchloom-managed V2 activation should be removed when Switchloom created it"
    );
    assert!(config_after_uninstall.contains("[features]"));
    assert_eq!(
        fs::read_to_string(repository.join(".codex/agents/local-reviewer.toml")).unwrap(),
        "name = \"local_reviewer\"\n"
    );
    assert_eq!(
        fs::read(global_codex_home.join("config.toml")).unwrap(),
        global_config_before
    );
    assert!(!repository.join(".model-routing/manifest.json").exists());
}

#[test]
fn lifecycle_preserves_compatible_user_owned_codex_v2_activation_and_rejects_false() {
    let repository = temp_repo("codex-v2-compatible");
    fs::create_dir_all(repository.join(".codex")).unwrap();
    fs::write(
        repository.join(".codex/config.toml"),
        "[features.multi_agent_v2]\nenabled = true\nhide_spawn_agent_metadata = false\n",
    )
    .unwrap();

    let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &codex).unwrap();
    let after_apply = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_v2_activation(&after_apply);
    assert_codex_config_entry(
        &after_apply,
        "model_routing_terra_high",
        "./agents/model-routing-terra-high.toml",
    );

    let mixed = compile_policy("balanced", "mixed-host", Integration::Standalone).unwrap();
    let mixed_file = write_bundle_file(&repository, "mixed.json", &mixed);
    update_bundle_file(&repository, &mixed_file).unwrap();
    let after_update = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_v2_activation(&after_update);
    let status_after_update = status_repository(&repository).unwrap();
    assert!(
        status_after_update.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/config.toml" && artifact.status == "managed"
        })
    );
    rollback_repository(&repository).unwrap();
    let after_rollback = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_v2_activation(&after_rollback);
    assert_codex_config_entry(
        &after_rollback,
        "model_routing_terra_high",
        "./agents/model-routing-terra-high.toml",
    );

    uninstall_repository(&repository).unwrap();
    let after_uninstall = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert_codex_v2_activation(&after_uninstall);
    assert_no_codex_config_entry(&after_uninstall, "model_routing_terra_high");

    let conflict_repo = temp_repo("codex-v2-post-update-conflict");
    fs::create_dir_all(conflict_repo.join(".codex")).unwrap();
    fs::write(
        conflict_repo.join(".codex/config.toml"),
        "[features.multi_agent_v2]\nenabled = true\nhide_spawn_agent_metadata = false\n",
    )
    .unwrap();
    apply_bundle_file_with_bundle(&conflict_repo, &codex).unwrap();
    let mixed_file = write_bundle_file(&conflict_repo, "mixed.json", &mixed);
    update_bundle_file(&conflict_repo, &mixed_file).unwrap();
    let config_path = conflict_repo.join(".codex/config.toml");
    let drifted = fs::read_to_string(&config_path)
        .unwrap()
        .replace("enabled = true", "enabled = false");
    fs::write(&config_path, drifted).unwrap();
    let status_after_false = status_repository(&conflict_repo).unwrap();
    assert!(status_after_false.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/config.toml"
            && artifact.status == "modified"
            && artifact.repair.is_some()
    }));
    let error = rollback_repository(&conflict_repo).unwrap_err().to_string();
    assert!(error.contains(".codex/config.toml"));
    assert!(error.contains("unmanaged content"));
    let after_failed_rollback = fs::read_to_string(&config_path).unwrap();
    assert_codex_v2_disabled(&after_failed_rollback);
    assert_codex_config_entry(
        &after_failed_rollback,
        "model_routing_terra_high",
        "./agents/model-routing-terra-high.toml",
    );

    let uninstall = uninstall_repository(&conflict_repo).unwrap();
    assert!(
        uninstall.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/config.toml" && artifact.status == "removed"
        })
    );
    let after_conflict_uninstall = fs::read_to_string(&config_path).unwrap();
    assert_codex_v2_disabled(&after_conflict_uninstall);
    assert_no_codex_config_entry(&after_conflict_uninstall, "model_routing_terra_high");
    assert_no_codex_config_entry(&after_conflict_uninstall, "model_routing_sol_high");
    assert_no_codex_config_entry(&after_conflict_uninstall, "model_routing_luna_xhigh");
    assert!(
        !conflict_repo
            .join(".codex/agents/model-routing-terra-high.toml")
            .exists()
    );
    assert!(!conflict_repo.join(".model-routing/manifest.json").exists());

    let cross_host_repo = temp_repo("codex-v2-cross-host-false");
    fs::create_dir_all(cross_host_repo.join(".codex")).unwrap();
    fs::write(
        cross_host_repo.join(".codex/config.toml"),
        "[features.multi_agent_v2]\nenabled = true\nhide_spawn_agent_metadata = false\n",
    )
    .unwrap();
    apply_bundle_file_with_bundle(&cross_host_repo, &codex).unwrap();
    let cross_host_config = cross_host_repo.join(".codex/config.toml");
    let drifted = fs::read_to_string(&cross_host_config)
        .unwrap()
        .replace("enabled = true", "enabled = false");
    fs::write(&cross_host_config, drifted).unwrap();

    let cursor = compile_policy("balanced", "cursor-openai", Integration::Standalone).unwrap();
    let cursor_file = write_bundle_file(&cross_host_repo, "cursor.json", &cursor);
    let update = update_bundle_file(&cross_host_repo, &cursor_file).unwrap();
    assert!(
        update.artifacts.iter().any(|artifact| {
            artifact.path == ".codex/config.toml" && artifact.status == "removed"
        })
    );
    let after_cross_host_update = fs::read_to_string(&cross_host_config).unwrap();
    assert_codex_v2_disabled(&after_cross_host_update);
    assert_no_codex_config_entry(&after_cross_host_update, "model_routing_terra_high");
    assert_no_codex_config_entry(&after_cross_host_update, "model_routing_sol_high");
    assert_no_codex_config_entry(&after_cross_host_update, "model_routing_luna_xhigh");
    assert!(
        cross_host_repo
            .join(".model-routing/manifest.json")
            .exists()
    );
    let cross_host_status = status_repository(&cross_host_repo).unwrap();
    assert!(
        !cross_host_status
            .artifacts
            .iter()
            .any(|artifact| artifact.path == ".codex/config.toml")
    );
}

#[test]
fn lifecycle_codex_config_modified_managed_entry_is_preserved_with_repair() {
    let repository = temp_repo("codex-config-modified-entry");
    let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &codex).unwrap();
    fs::write(
        repository.join(".codex/config.toml"),
        "[agents.model_routing_terra_high]\nconfig_file = \"./agents/hacked.toml\"\n",
    )
    .unwrap();

    let status = status_repository(&repository).unwrap();
    assert!(status.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/config.toml"
            && artifact.status == "modified"
            && artifact.repair.is_some()
    }));

    let uninstall = uninstall_repository(&repository).unwrap();
    assert!(uninstall.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/config.toml"
            && artifact.status == "preserved-modified"
            && artifact.repair.is_some()
    }));
    assert!(repository.join(".codex/config.toml").exists());
    assert!(repository.join(".model-routing/manifest.json").exists());
}

#[test]
fn lifecycle_preserves_modified_files_and_residual_manifest() {
    let repository = temp_repo("preserve-residual");
    let mut bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    bundle.artifacts.truncate(1);
    apply_bundle_file_with_bundle(&repository, &bundle).unwrap();

    let target = repository.join(&bundle.artifacts[0].path);
    fs::write(&target, "user modified").unwrap();
    let uninstall = uninstall_repository(&repository).unwrap();
    assert_eq!(uninstall.artifacts[0].status, "preserved-modified");
    assert!(uninstall.artifacts[0].repair.is_some());
    assert!(target.exists());
    assert!(repository.join(".model-routing/manifest.json").exists());

    let status = status_repository(&repository).unwrap();
    assert_eq!(status.artifacts[0].status, "modified");
    assert!(status.artifacts[0].repair.is_some());
}

#[test]
fn lifecycle_cross_host_update_and_rollback_remove_old_managed_artifacts() {
    let repository = temp_repo("cross-host");
    let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    let claude = compile_policy("balanced", "claude-native", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &codex).unwrap();
    let codex_artifact = repository.join(".codex/agents/model-routing-sol-medium.toml");
    assert!(codex_artifact.exists());

    let claude_file = write_bundle_file(&repository, "claude.json", &claude);
    let update = update_bundle_file(&repository, &claude_file).unwrap();
    assert!(
        update
            .artifacts
            .iter()
            .any(|artifact| artifact.mode == "delete" && artifact.status == "removed")
    );
    assert!(!codex_artifact.exists());
    let status = status_repository(&repository).unwrap();
    assert!(
        status
            .artifacts
            .iter()
            .all(|artifact| artifact.path.starts_with(".claude/"))
    );

    let claude_artifact = repository.join(".claude/agents/model-routing-preset-worker.md");
    assert!(claude_artifact.exists());
    let rollback = rollback_repository(&repository).unwrap();
    assert!(
        rollback
            .artifacts
            .iter()
            .any(|artifact| artifact.mode == "delete" && artifact.status == "removed")
    );
    assert!(!claude_artifact.exists());
    assert!(codex_artifact.exists());
    let status = status_repository(&repository).unwrap();
    assert!(
        status
            .artifacts
            .iter()
            .all(|artifact| artifact.path.starts_with(".codex/"))
    );

    uninstall_repository(&repository).unwrap();
    assert!(!repository.join(".model-routing/manifest.json").exists());
    assert!(!codex_artifact.exists());
}

#[test]
fn lifecycle_cross_host_update_preserves_modified_removed_paths() {
    let repository = temp_repo("cross-host-preserve");
    let codex = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    let claude = compile_policy("balanced", "claude-native", Integration::Standalone).unwrap();
    apply_bundle_file_with_bundle(&repository, &codex).unwrap();
    let codex_artifact = repository.join(".codex/agents/model-routing-sol-medium.toml");
    fs::write(&codex_artifact, "user modified codex artifact").unwrap();

    let claude_file = write_bundle_file(&repository, "claude.json", &claude);
    let update = update_bundle_file(&repository, &claude_file).unwrap();
    let preserved = update
        .artifacts
        .iter()
        .find(|artifact| artifact.path == ".codex/agents/model-routing-sol-medium.toml")
        .unwrap();
    assert_eq!(preserved.mode, "delete");
    assert_eq!(preserved.status, "preserved-modified");
    assert!(preserved.repair.is_some());
    assert!(codex_artifact.exists());

    let status = status_repository(&repository).unwrap();
    assert!(status.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/agents/model-routing-sol-medium.toml"
            && artifact.status == "modified"
            && artifact.repair.is_some()
    }));
    assert!(
        status
            .artifacts
            .iter()
            .any(|artifact| artifact.path.starts_with(".claude/"))
    );
}

#[test]
fn fresh_repository_registers_codex_native_role_discovery_config() {
    let repository = temp_repo("codex-native-discovery-config");
    let bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    assert!(bundle.artifacts.iter().all(|artifact| {
        artifact.path == ".codex/config.toml" || artifact.path.starts_with(".codex/agents/")
    }));
    assert!(
        bundle
            .artifacts
            .iter()
            .all(|artifact| !artifact.path.starts_with(".codex/skills/"))
    );
    apply_bundle_file_with_bundle(&repository, &bundle).unwrap();

    for role in ["model-routing-terra-high", "model-routing-sol-high"] {
        assert!(
            repository
                .join(format!(".codex/agents/{role}.toml"))
                .exists(),
            "generated native Codex role file {role} should exist"
        );
    }

    let config = bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.path == ".codex/config.toml")
        .expect("repository-local Codex role discovery config should be generated");
    let parsed: toml::Value = toml::from_str(&config.content).unwrap();
    assert_eq!(
        parsed["agents"]["model_routing_terra_high"]["config_file"].as_str(),
        Some("./agents/model-routing-terra-high.toml")
    );
    assert_eq!(
        parsed["agents"]["model_routing_sol_high"]["config_file"].as_str(),
        Some("./agents/model-routing-sol-high.toml")
    );
    assert_codex_v2_activation(&config.content);
    assert_eq!(
        fs::read_to_string(repository.join(".codex/config.toml")).unwrap(),
        config.content
    );
}

#[test]
fn lifecycle_preview_apply_status_and_uninstall_are_repository_safe() {
    let repository = temp_repo("lifecycle");
    let bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    assert!(bundle.artifacts.iter().all(|artifact| {
        artifact.path == ".codex/config.toml" || artifact.path.starts_with(".codex/agents/")
    }));
    assert!(
        bundle
            .artifacts
            .iter()
            .all(|artifact| !artifact.content.contains("codex exec"))
    );
    let preview = preview_bundle_with_bundle(&repository, &bundle).unwrap();
    assert_eq!(preview.action, "preview");
    assert_eq!(preview.artifacts.len(), 8);
    assert!(
        preview
            .artifacts
            .iter()
            .all(|artifact| artifact.status == "create")
    );

    let bundle_file = repository.join("bundle.json");
    fs::write(
        &bundle_file,
        compile_json("balanced", "codex-openai", Integration::Standalone).unwrap(),
    )
    .unwrap();
    let applied = apply_bundle_file(&repository, &bundle_file).unwrap();
    assert_eq!(applied.action, "apply");
    assert!(repository.join(".model-routing/manifest.json").exists());
    assert!(
        repository
            .join(".codex/agents/model-routing-sol-medium.toml")
            .exists()
    );
    let codex_config = fs::read_to_string(repository.join(".codex/config.toml")).unwrap();
    assert!(codex_config.contains("[agents.model_routing_terra_high]"));
    assert!(codex_config.contains("config_file = \"./agents/model-routing-terra-high.toml\""));
    assert!(codex_config.contains("[agents.model_routing_terra_mechanical]"));
    assert!(
        codex_config.contains("config_file = \"./agents/model-routing-terra-mechanical.toml\"")
    );
    assert!(codex_config.contains("[agents.model_routing_sol_high]"));
    assert!(codex_config.contains("config_file = \"./agents/model-routing-sol-high.toml\""));
    assert_codex_v2_activation(&codex_config);
    assert!(
        repository
            .join(".codex/agents/model-routing-terra-high.toml")
            .exists()
    );
    assert!(
        repository
            .join(".codex/agents/model-routing-sol-high.toml")
            .exists()
    );

    let status = status_repository(&repository).unwrap();
    assert_eq!(status.action, "status");
    assert!(
        status
            .artifacts
            .iter()
            .all(|artifact| artifact.status == "managed")
    );

    let uninstalled = uninstall_repository(&repository).unwrap();
    assert_eq!(uninstalled.action, "uninstall");
    assert!(
        uninstalled
            .artifacts
            .iter()
            .all(|artifact| artifact.status == "removed")
    );
    assert!(!repository.join(".model-routing/manifest.json").exists());
    assert!(
        !repository
            .join(".codex/agents/model-routing-sol-medium.toml")
            .exists()
    );
}

#[test]
fn lifecycle_rejects_unsafe_paths_and_conflicts() {
    let repository = temp_repo("unsafe");
    let mut bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();

    bundle.artifacts[0].path = ".model-routing/unsafe.toml".to_string();
    assert!(
        preview_bundle_with_bundle(&repository, &bundle)
            .unwrap_err()
            .to_string()
            .contains("reserved path")
    );

    let mut bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    bundle.artifacts[0].path = "../escape.toml".to_string();
    assert!(
        preview_bundle_with_bundle(&repository, &bundle)
            .unwrap_err()
            .to_string()
            .contains("must not traverse")
    );

    let bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    let target = repository.join(&bundle.artifacts[0].path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "user edit").unwrap();
    let error = apply_bundle_file_with_bundle(&repository, &bundle)
        .unwrap_err()
        .to_string();
    assert!(error.contains("already exists with different content"));
}

#[test]
fn lifecycle_rejects_parent_child_targets_without_partial_apply() {
    let repository = temp_repo("parent-child");
    let mut bundle = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    bundle.artifacts.truncate(2);
    bundle.artifacts[0].path = ".codex/agents/collision".to_string();
    bundle.artifacts[0].content = "parent".to_string();
    bundle.artifacts[0].sha256 = test_sha256(bundle.artifacts[0].content.as_bytes());
    bundle.artifacts[1].path = ".codex/agents/collision/child.toml".to_string();
    bundle.artifacts[1].content = "child".to_string();
    bundle.artifacts[1].sha256 = test_sha256(bundle.artifacts[1].content.as_bytes());

    let error = apply_bundle_file_with_bundle(&repository, &bundle)
        .unwrap_err()
        .to_string();
    assert!(error.contains("parent-child collision"));
    assert!(!repository.join(".codex/agents/collision").exists());
    assert!(!repository.join(".model-routing/manifest.json").exists());
}

#[test]
fn prepared_setup_apply_aborts_when_repository_plan_changes_after_preview() {
    let repository = temp_repo("prepared-setup-toctou");
    let spec = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
    let prepared = prepare_setup_recipe(&setup_spec_to_recipe(&spec).unwrap()).unwrap();
    let preview = preview_prepared_setup(&repository, &prepared).unwrap();
    fs::create_dir_all(repository.join(".switchloom")).unwrap();
    fs::write(repository.join(SETUP_CONFIG_PATH), "external change\n").unwrap();
    let error = apply_prepared_setup(&repository, &prepared, &preview)
        .unwrap_err()
        .to_string();
    assert!(error.contains("repository state changed after preview"));
    assert_eq!(
        fs::read_to_string(repository.join(SETUP_CONFIG_PATH)).unwrap(),
        "external change\n"
    );
    assert!(!repository.join(".model-routing/manifest.json").exists());
}

#[cfg(unix)]
#[test]
fn prepared_setup_apply_aborts_when_repository_symlink_retargets_after_preview() {
    use std::os::unix::fs::symlink;

    let root = temp_repo("prepared-setup-symlink");
    let repo_a = root.join("repo-a");
    let repo_b = root.join("repo-b");
    let link = root.join("repo-link");
    fs::create_dir_all(&repo_a).unwrap();
    fs::create_dir_all(&repo_b).unwrap();
    symlink(&repo_a, &link).unwrap();

    let spec = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
    let prepared = prepare_setup_recipe(&setup_spec_to_recipe(&spec).unwrap()).unwrap();
    let preview = preview_prepared_setup(&link, &prepared).unwrap();
    assert_eq!(
        preview.repository,
        repo_a.canonicalize().unwrap().display().to_string()
    );

    fs::remove_file(&link).unwrap();
    symlink(&repo_b, &link).unwrap();
    let error = apply_prepared_setup(&link, &prepared, &preview)
        .unwrap_err()
        .to_string();
    assert!(error.contains("repository state changed after preview"));
    assert!(!repo_a.join(SETUP_CONFIG_PATH).exists());
    assert!(!repo_b.join(SETUP_CONFIG_PATH).exists());
    assert!(!repo_a.join(".model-routing/manifest.json").exists());
    assert!(!repo_b.join(".model-routing/manifest.json").exists());
}
