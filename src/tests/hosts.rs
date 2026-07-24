use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn temp_host_repo(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("switchloom-hosts-{name}-{unique}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[cfg(unix)]
fn codex_version_stub(repository: &Path, output: &str) -> PathBuf {
    let script = repository.join("codex-version-stub.sh");
    fs::write(&script, format!("#!/bin/sh\nprintf '{}\\n'\n", output)).unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).unwrap();
    script
}

#[cfg(unix)]
fn write_semantic_codex_recipe(repository: &Path) {
    fs::create_dir_all(repository.join(".switchloom")).unwrap();
    fs::create_dir_all(repository.join(".codex/agents")).unwrap();
    fs::write(
        repository.join(".switchloom/config.toml"),
        r#"schema_version = 1
host = "codex-openai"
integration = "planr"
usage_policy = "balanced"

[selected_roles.implementer]
model = "gpt-5.6-terra"
effort = "medium"
[selected_roles.implementer.spawn]
agent_type = "switchloom_implementer"
task_name = "implementer"
[selected_roles.implementer.spawn.fork_turns]
mode = "none"

[selected_roles.reviewer]
model = "gpt-5.6-terra"
effort = "medium"
[selected_roles.reviewer.spawn]
agent_type = "switchloom_reviewer"
task_name = "reviewer"
[selected_roles.reviewer.spawn.fork_turns]
mode = "none"

[selected_roles.verifier]
model = "gpt-5.6-terra"
effort = "low"
[selected_roles.verifier.spawn]
agent_type = "switchloom_verifier"
task_name = "verifier"
[selected_roles.verifier.spawn.fork_turns]
mode = "none"

[[routes]]
work_type = "code"
role = "implementer"
fallbacks = []
"#,
    )
    .unwrap();
    fs::write(
        repository.join(".codex/config.toml"),
        r#"[agents.switchloom_implementer]
config_file = "./agents/switchloom_implementer.toml"
[agents.switchloom_reviewer]
config_file = "./agents/switchloom_reviewer.toml"
[agents.switchloom_verifier]
config_file = "./agents/switchloom_verifier.toml"

[features.multi_agent_v2]
enabled = true
hide_spawn_agent_metadata = true
"#,
    )
    .unwrap();
    for (agent_type, model, effort) in [
        ("switchloom_implementer", "gpt-5.6-terra", "medium"),
        ("switchloom_reviewer", "gpt-5.6-terra", "medium"),
        ("switchloom_verifier", "gpt-5.6-terra", "low"),
    ] {
        fs::write(
            repository.join(format!(".codex/agents/{agent_type}.toml")),
            format!(
                "name = \"{agent_type}\"\nmodel = \"{model}\"\nmodel_reasoning_effort = \"{effort}\"\n"
            ),
        )
        .unwrap();
    }
}

#[test]
fn adapter_contract_distinguishes_external_runner_runtime_class() {
    let binding = HostBinding {
        id: "pi-runner".to_string(),
        version: "1.0.0".to_string(),
        host: "pi".to_string(),
        runtime_class: RuntimeClass::ExternalRunner,
        default_role: Some("worker".to_string()),
        capability_evidence: vec!["pi-runner-contract".to_string()],
        known_limitations: vec!["process isolation is runner-owned".to_string()],
        capabilities: BindingCapabilities {
            model_override: true,
            effort_override: true,
            fork_none: true,
            fork_all: false,
        },
        profiles: BTreeMap::from([(
            "worker".to_string(),
            BindingProfile {
                profile: "pi-worker".to_string(),
                client: "pi".to_string(),
                model: "gpt-5.6-terra".to_string(),
                agent_type: None,
                effort: Some("high".to_string()),
                cost_tier: Some("standard".to_string()),
                fork_turns: Some(ForkPolicy {
                    mode: "none".to_string(),
                    turns: None,
                }),
            },
        )]),
        routes: Vec::new(),
        verification: BindingVerification {
            id: "pi-smoke-v1".to_string(),
            max_age_seconds: Some(60),
        },
        artifacts: Vec::new(),
    };
    let contract =
        adapter_contract_for_binding("balanced", &binding, Integration::Standalone).unwrap();
    assert_eq!(
        contract.capability.runtime_class,
        RuntimeClass::ExternalRunner
    );
    assert_eq!(contract.adapter.runtime_class, RuntimeClass::ExternalRunner);
    assert_eq!(
        contract.adapter.dispatch_recipe.invocation,
        "external-runner-process"
    );
}

#[cfg(unix)]
#[test]
fn codex_doctor_reports_exact_0145_v2_conflicts_and_reload_guidance() {
    let repository = temp_host_repo("codex-doctor");
    fs::create_dir_all(repository.join(".codex")).unwrap();
    fs::write(
        repository.join(".codex/config.toml"),
        r#"[agents.model_routing_terra_high]
config_file = "./agents/model-routing-terra-high.toml"

[agents.model_routing_sol_high]
config_file = "./agents/model-routing-sol-high.toml"

[features.multi_agent_v2]
enabled = false
hide_spawn_agent_metadata = false
"#,
    )
    .unwrap();
    let exact = codex_version_stub(&repository, "codex 0.145.0");
    let exact_report =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(exact_report.available);
    assert_eq!(exact_report.version.as_deref(), Some("codex 0.145.0"));
    assert!(exact_report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_exact_version_ready" && diagnostic.severity == "info"
    }));
    assert!(exact_report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_v2_activation_conflict"
            && diagnostic.repair.contains("enabled = true")
    }));
    assert!(exact_report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_v2_metadata_conflict"
            && diagnostic.message.contains("Codex 0.145")
            && diagnostic.message.contains("collaboration.spawn_agent")
            && diagnostic
                .repair
                .contains("hide_spawn_agent_metadata = true")
    }));
    assert!(exact_report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_trust_reload_required"
            && diagnostic.repair.contains("reload or restart")
    }));
    assert!(exact_report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_luna_experimental_unverified"
            && diagnostic.severity == "warning"
            && diagnostic.message.contains("experimental/unverified")
            && diagnostic.repair.contains("Terra")
    }));

    let drifted = codex_version_stub(&repository, "codex-cli 0.144.5");
    let drifted_report =
        probe_host_with_repository("codex", Some(drifted.to_str().unwrap()), &repository).unwrap();
    assert!(drifted_report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_exact_version_mismatch"
            && diagnostic.message.contains("0.144.5")
            && diagnostic.repair.contains("0.145.0")
    }));

    fs::write(
        repository.join(".codex/config.toml"),
        r#"[agents.model_routing_terra_high]
config_file = "./agents/model-routing-terra-high.toml"

[agents.model_routing_sol_high]
config_file = "./agents/model-routing-sol-high.toml"

[features.multi_agent_v2]
enabled = true
hide_spawn_agent_metadata = true
"#,
    )
    .unwrap();
    let exact = codex_version_stub(&repository, "codex 0.145.0");
    let ready_report =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(
        !ready_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error")
    );
    assert!(
        !ready_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "codex_v2_metadata_conflict")
    );
}

#[cfg(unix)]
#[test]
fn codex_doctor_uses_applied_semantic_roles_and_distinguishes_drift() {
    let repository = temp_host_repo("codex-semantic-doctor");
    write_semantic_codex_recipe(&repository);
    let exact = codex_version_stub(&repository, "codex-cli 0.145.0");

    let ready =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(!ready.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_role_registration_missing"
            || diagnostic.code == "codex_role_file_missing"
            || diagnostic.code == "codex_role_config_mismatch"
            || diagnostic.code == "codex_role_spawn_metadata_invalid"
    }));
    assert!(
        !ready
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.message.contains("model_routing_terra_high") })
    );
    assert!(ready.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_exact_version_ready"
            && diagnostic
                .repair
                .contains("Switchloom v0.3.3 certification")
    }));

    fs::write(
        repository.join(".codex/config.toml"),
        fs::read_to_string(repository.join(".codex/config.toml"))
            .unwrap()
            .replace(
                "[agents.switchloom_reviewer]\nconfig_file = \"./agents/switchloom_reviewer.toml\"\n",
                "",
            ),
    )
    .unwrap();
    let missing_registration =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(missing_registration.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_role_registration_missing"
            && diagnostic.message.contains("switchloom_reviewer")
    }));

    write_semantic_codex_recipe(&repository);
    fs::remove_file(repository.join(".codex/agents/switchloom_verifier.toml")).unwrap();
    let missing_file =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(missing_file.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_role_file_missing" && diagnostic.message.contains("verifier")
    }));

    write_semantic_codex_recipe(&repository);
    fs::write(
        repository.join(".codex/agents/switchloom_implementer.toml"),
        "name = \"switchloom_implementer\"\nmodel = \"gpt-5.6-sol\"\nmodel_reasoning_effort = \"medium\"\n",
    )
    .unwrap();
    let config_mismatch =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(config_mismatch.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_role_config_mismatch"
            && diagnostic.message.contains("implementer")
    }));

    write_semantic_codex_recipe(&repository);
    fs::write(
        repository.join(".switchloom/config.toml"),
        fs::read_to_string(repository.join(".switchloom/config.toml"))
            .unwrap()
            .replace("task_name = \"implementer\"", "task_name = \"wrong_task\""),
    )
    .unwrap();
    let invalid_spawn =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(invalid_spawn.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "codex_role_spawn_metadata_invalid"
            && diagnostic.message.contains("implementer")
    }));

    write_semantic_codex_recipe(&repository);
    fs::create_dir_all(repository.join(".model-routing")).unwrap();
    fs::write(
        repository.join(".model-routing/manifest.json"),
        r#"{"schema_version":1,"artifacts":[]}"#,
    )
    .unwrap();
    let manifest_mismatch =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(
        manifest_mismatch
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "codex_applied_manifest_mismatch" })
    );

    fs::write(
        repository.join(".model-routing/manifest.json"),
        "{ malformed manifest",
    )
    .unwrap();
    let invalid_manifest =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(
        invalid_manifest
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "codex_applied_manifest_invalid" })
    );

    fs::write(repository.join(".switchloom/config.toml"), "not = [valid").unwrap();
    let invalid_recipe =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(
        invalid_recipe
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "codex_applied_recipe_invalid" })
    );

    fs::remove_file(repository.join(".switchloom/config.toml")).unwrap();
    let missing_recipe =
        probe_host_with_repository("codex", Some(exact.to_str().unwrap()), &repository).unwrap();
    assert!(
        missing_recipe
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "codex_applied_recipe_missing" })
    );
}

#[test]
fn pi_external_adapter_declares_typed_runner_contract() {
    let bundle = compile_policy("balanced", "pi-external", Integration::Standalone).unwrap();
    let contract = bundle.adapter_contract.as_ref().unwrap();
    assert_eq!(
        contract.capability.runtime_class,
        RuntimeClass::ExternalRunner
    );
    assert_eq!(contract.adapter.runtime_class, RuntimeClass::ExternalRunner);
    assert_eq!(
        contract.adapter.dispatch_recipe.invocation,
        "external-runner-process"
    );
    for field in [
        "agent_type",
        "provider",
        "model",
        "effort",
        "fork_turns",
        "isolation",
        "task",
    ] {
        assert!(
            contract
                .adapter
                .dispatch_recipe
                .required_fields
                .contains(&field.to_string()),
            "Pi dispatch recipe should require {field}"
        );
    }
    assert_eq!(
        contract.capability.observability.effective_model,
        GuaranteeLevel::Advisory
    );
    assert!(
        contract
            .capability
            .known_limitations
            .iter()
            .any(|limitation| limitation.contains("process-isolated"))
    );

    let workflow = bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.path == ".pi/workflows/model-routing-preset-runner.json")
        .unwrap();
    assert!(
        workflow
            .content
            .contains("\"runtime_class\": \"external-runner\"")
    );
    assert!(
        workflow
            .content
            .contains("\"agent_type\": \"switchloom-pi-worker\"")
    );
    assert!(
        workflow
            .content
            .contains("\"provider_model\": \"openai/gpt-4o-mini\"")
    );
    assert!(workflow.content.contains("\"thinking\": \"low\""));
    assert!(workflow.content.contains("\"session\": \"none\""));
    assert!(workflow.content.contains("\"task\""));
}

#[test]
fn host_binding_runtime_class_is_required_and_explicit() {
    let missing_runtime_class = r#"
id = "pi-runner"
version = "1.0.0"
host = "pi"
default_role = "worker"

[capabilities]
model_override = true
effort_override = true
fork_none = true
fork_all = false

[profiles.worker]
profile = "pi-worker"
client = "pi"
model = "gpt-5.6-terra"

[verification]
id = "pi-smoke-v1"
"#;
    let error = toml::from_str::<HostBinding>(missing_runtime_class)
        .unwrap_err()
        .to_string();
    assert!(error.contains("runtime_class"));
}

#[test]
fn adapter_contract_rejects_unsupported_required_guarantees() {
    let mut contract = compile_policy("balanced", "cursor-openai", Integration::Standalone)
        .unwrap()
        .adapter_contract
        .unwrap();
    contract
        .routing_intent
        .required_guarantees
        .push("effort_selection".to_string());
    let error = validate_adapter_contract(&contract)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unsupported"));
}

#[test]
fn shared_adapter_validation_rejects_invalid_routes_before_rendering() {
    let binding = HostBinding {
        id: "cursor-test".to_string(),
        version: "1.0.0".to_string(),
        host: "cursor".to_string(),
        runtime_class: RuntimeClass::NativeSubagent,
        default_role: None,
        capability_evidence: Vec::new(),
        known_limitations: Vec::new(),
        capabilities: BindingCapabilities {
            model_override: true,
            effort_override: false,
            fork_none: true,
            fork_all: false,
        },
        profiles: BTreeMap::from([(
            "worker".to_string(),
            BindingProfile {
                profile: "cursor-worker".to_string(),
                client: "cursor".to_string(),
                model: "gpt-5.4-mini".to_string(),
                agent_type: None,
                effort: None,
                cost_tier: Some("standard".to_string()),
                fork_turns: Some(ForkPolicy {
                    mode: "none".to_string(),
                    turns: None,
                }),
            },
        )]),
        routes: vec![BindingRoute {
            work_type: "code".to_string(),
            role: "missing".to_string(),
            fallback_roles: Vec::new(),
        }],
        verification: BindingVerification {
            id: "cursor-test-v1".to_string(),
            max_age_seconds: Some(60),
        },
        artifacts: Vec::new(),
    };
    let error = compile_host_adapter("balanced", &binding, Integration::Standalone)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown role `missing`"));
}

#[test]
fn shared_adapter_validation_rejects_duplicate_profile_ids_before_rendering() {
    let binding = HostBinding {
        id: "cursor-test".to_string(),
        version: "1.0.0".to_string(),
        host: "cursor".to_string(),
        runtime_class: RuntimeClass::NativeSubagent,
        default_role: Some("first".to_string()),
        capability_evidence: Vec::new(),
        known_limitations: Vec::new(),
        capabilities: BindingCapabilities {
            model_override: true,
            effort_override: false,
            fork_none: true,
            fork_all: false,
        },
        profiles: BTreeMap::from([
            (
                "first".to_string(),
                BindingProfile {
                    profile: "cursor-worker".to_string(),
                    client: "cursor".to_string(),
                    model: "gpt-5.4-mini".to_string(),
                    agent_type: None,
                    effort: None,
                    cost_tier: Some("standard".to_string()),
                    fork_turns: Some(ForkPolicy {
                        mode: "none".to_string(),
                        turns: None,
                    }),
                },
            ),
            (
                "second".to_string(),
                BindingProfile {
                    profile: "cursor-worker".to_string(),
                    client: "cursor".to_string(),
                    model: "gpt-5.5".to_string(),
                    agent_type: None,
                    effort: None,
                    cost_tier: Some("premium".to_string()),
                    fork_turns: Some(ForkPolicy {
                        mode: "none".to_string(),
                        turns: None,
                    }),
                },
            ),
        ]),
        routes: vec![BindingRoute {
            work_type: "code".to_string(),
            role: "first".to_string(),
            fallback_roles: vec!["second".to_string()],
        }],
        verification: BindingVerification {
            id: "cursor-test-v1".to_string(),
            max_age_seconds: Some(60),
        },
        artifacts: Vec::new(),
    };
    let error = compile_host_adapter("balanced", &binding, Integration::Standalone)
        .unwrap_err()
        .to_string();
    assert!(error.contains("both normalize to profile `cursor-worker`"));
}

#[test]
fn dispatch_recipe_artifact_paths_match_final_bundle_artifacts() {
    let bundle = compile_policy("balanced", "mixed-host", Integration::Planr).unwrap();
    let contract_paths = bundle
        .adapter_contract
        .as_ref()
        .unwrap()
        .adapter
        .dispatch_recipe
        .artifact_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let artifact_paths = bundle
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(contract_paths, artifact_paths);
    assert!(
        !contract_paths
            .iter()
            .any(|path| path.contains("model-routing-native-routing"))
    );
}

#[test]
fn claude_and_cursor_native_adapters_emit_artifacts_with_advisory_effective_routing() {
    for (host, expected_path, requested_model) in [
        (
            "claude-native",
            ".claude/agents/model-routing-preset-worker.md",
            "sonnet",
        ),
        (
            "cursor-openai",
            ".cursor/agents/model-routing-preset-worker.md",
            "gpt-5.4-mini",
        ),
        (
            "cursor-fable-grok",
            ".cursor/agents/model-routing-preset-worker.md",
            "cursor-grok-4.5-medium",
        ),
        (
            "opencode-native",
            ".opencode/agents/model-routing-preset-worker.md",
            "opencode/gpt-5-nano",
        ),
    ] {
        let bundle = compile_policy("balanced", host, Integration::Standalone).unwrap();
        let contract = bundle.adapter_contract.as_ref().unwrap();
        assert_eq!(
            contract.capability.runtime_class,
            RuntimeClass::NativeSubagent
        );
        assert_eq!(contract.adapter.runtime_class, RuntimeClass::NativeSubagent);
        assert_eq!(
            contract.capability.observability.effective_model,
            GuaranteeLevel::Advisory
        );
        assert_eq!(
            contract
                .capability
                .guarantees
                .get("model_selection")
                .unwrap()
                .level,
            GuaranteeLevel::Advisory
        );
        assert!(
            contract
                .capability
                .known_limitations
                .iter()
                .any(|limitation| limitation.contains("override"))
                || contract
                    .capability
                    .known_limitations
                    .iter()
                    .any(|limitation| limitation.contains("preempt"))
                || contract
                    .capability
                    .known_limitations
                    .iter()
                    .any(|limitation| limitation.contains("provider"))
        );
        assert!(
            contract
                .adapter
                .dispatch_recipe
                .artifact_paths
                .contains(&expected_path.to_string())
        );

        let artifact = bundle
            .artifacts
            .iter()
            .find(|artifact| artifact.path == expected_path)
            .unwrap();
        assert!(artifact.content.contains(requested_model));
        assert!(artifact.content.contains("preserve routing evidence"));
    }
}

#[test]
fn codex_agent_types_match_registered_toml_names() {
    for host in ["codex-openai", "mixed-host"] {
        let source = show_policy("balanced", host).unwrap();
        assert!(
            source
                .artifacts
                .iter()
                .all(|artifact| !artifact.path.starts_with(".codex/skills/"))
        );
        assert!(
            source
                .artifacts
                .iter()
                .all(|artifact| !artifact.content.contains("model-routing-native-routing"))
        );
        let bundle = compile_policy("balanced", host, Integration::Standalone).unwrap();
        let config = bundle
            .artifacts
            .iter()
            .find(|artifact| artifact.path == ".codex/config.toml")
            .expect("Codex role config should be generated");
        let parsed_config: toml::Value = toml::from_str(&config.content).unwrap();
        let config_agents = parsed_config["agents"].as_table().unwrap();
        let registered_names = bundle
            .artifacts
            .iter()
            .filter(|artifact| artifact.path.starts_with(".codex/agents/"))
            .map(|artifact| {
                let agent_type = toml::from_str::<toml::Value>(&artifact.content).unwrap()["name"]
                    .as_str()
                    .unwrap()
                    .to_string();
                let relative_config_file =
                    artifact.path.strip_prefix(".codex/").unwrap().to_string();
                assert_eq!(
                    config_agents[&agent_type]["config_file"].as_str(),
                    Some(format!("./{relative_config_file}").as_str())
                );
                agent_type
            })
            .collect::<std::collections::BTreeSet<_>>();
        for profile in bundle
            .profiles
            .values()
            .filter(|profile| profile.client == "codex")
        {
            let agent_type = profile.agent_type.as_deref().unwrap();
            assert!(registered_names.contains(agent_type));
        }
    }
}
