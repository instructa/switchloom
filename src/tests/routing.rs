use crate::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn compiled_bundle_carries_typed_adapter_contract() {
    let bundle = compile_policy("balanced", "codex-openai", Integration::Planr).unwrap();
    let contract = bundle.adapter_contract.as_ref().unwrap();
    assert_eq!(contract.schema_version, 1);
    assert_eq!(
        contract.capability.runtime_class,
        RuntimeClass::NativeSubagent
    );
    assert_eq!(contract.adapter.runtime_class, RuntimeClass::NativeSubagent);
    assert_eq!(contract.routing_intent.integration, Integration::Planr);
    assert!(
        contract
            .routing_intent
            .semantic_roles
            .contains(&"worker".to_string())
    );
    assert!(
        !contract
            .routing_intent
            .semantic_roles
            .contains(&"codex-terra-high".to_string())
    );
    assert_eq!(
        contract
            .capability
            .guarantees
            .get("model_selection")
            .unwrap()
            .level,
        GuaranteeLevel::Deterministic
    );
    assert!(
        contract
            .capability
            .guarantees
            .values()
            .any(|guarantee| guarantee.level == GuaranteeLevel::Advisory)
    );
    assert!(
        contract
            .dispatch_evidence
            .required_verdicts
            .contains(&GuaranteeLevel::Unsupported)
    );
    assert_eq!(
        contract.dispatch_evidence.receipt_schema,
        "DispatchEvidenceV1"
    );
    assert!(
        contract
            .routing_intent
            .role_requests
            .iter()
            .any(|request| request.semantic_role == "worker"
                && request.requested_model == "gpt-5.6-terra"
                && request.requested_effort.as_deref() == Some("high"))
    );
    assert_eq!(
        contract.adapter.dispatch_recipe.invocation,
        "host-native-subagent"
    );
    assert_eq!(
        contract.capability.runtime_behavior.capability_version,
        codex_v2_runtime_evidence().unwrap().evidence_id
    );
    assert_eq!(
        contract
            .capability
            .runtime_behavior
            .installed_host_version_source,
        format!(
            "{} via {}",
            codex_v2_runtime_evidence()
                .unwrap()
                .installed_version
                .stdout,
            codex_v2_runtime_evidence()
                .unwrap()
                .installed_version
                .command
        )
    );
    assert_eq!(
        contract
            .capability
            .host_version_constraints
            .minimum
            .as_deref(),
        Some(codex_v2_host_version(&codex_v2_runtime_evidence().unwrap()).as_str())
    );
    assert_eq!(
        contract
            .capability
            .host_version_constraints
            .maximum
            .as_deref(),
        Some(codex_v2_host_version(&codex_v2_runtime_evidence().unwrap()).as_str())
    );
    assert!(
        contract
            .capability
            .runtime_behavior
            .backend_selection_source
            .contains("authenticated host account")
    );
    assert!(
        contract
            .capability
            .runtime_behavior
            .discovery_behavior
            .contains(".codex/config.toml")
    );
    assert!(
        contract
            .capability
            .runtime_behavior
            .role_precedence
            .iter()
            .any(|entry| entry.contains("agent file"))
    );
    assert!(contract.capability.runtime_behavior.shared_filesystem);
    assert_eq!(contract.capability.parallelism.max_parallel_children, 3);
    assert!(
        contract
            .capability
            .runtime_behavior
            .delegation_modes
            .explicit_agent_type_dispatch
    );
    assert!(
        contract
            .capability
            .runtime_behavior
            .delegation_modes
            .ultra_auto_delegation
    );
    assert!(
        contract
            .capability
            .runtime_behavior
            .source_references
            .contains(&codex_v2_runtime_evidence_reference())
    );
}

#[test]
fn codex_default_spec_can_be_partially_tuned_into_custom_standalone_bundle() {
    let mut spec = setup_spec_for_policy("balanced", "codex", Integration::Standalone).unwrap();
    let worker = spec.selected_roles.get_mut("worker").unwrap();
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

    let bundle = compile_setup_spec(&spec).unwrap();
    validate_bundle(&bundle).unwrap();
    assert_eq!(bundle.source.integration, Integration::Standalone);
    assert_eq!(bundle.evidence.status, "custom-unverified");
    assert_eq!(
        bundle.profiles.get("driver").unwrap().agent_type.as_deref(),
        Some("model_routing_sol_medium")
    );
    assert!(bundle.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/agents/model-routing-sol-medium.toml"
            && artifact
                .content
                .contains("name = \"model_routing_sol_medium\"")
    }));
    assert!(bundle.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/agents/switchloom_worker.toml"
            && artifact.content.contains("name = \"switchloom_worker\"")
    }));
    assert!(
        bundle
            .artifacts
            .iter()
            .all(|artifact| !artifact.path.starts_with(".planr/"))
    );
}

#[test]
fn codex_default_spec_can_be_partially_tuned_into_custom_planr_bundle() {
    let mut spec = setup_spec_for_policy("balanced", "codex", Integration::Planr).unwrap();
    let reviewer = spec.selected_roles.get_mut("reviewer").unwrap();
    reviewer.model = "gpt-5.6-terra".to_string();
    reviewer.effort = Some("high".to_string());
    reviewer.spawn = Some(SetupSpawnPolicy {
        agent_type: "switchloom_reviewer".to_string(),
        task_name: "reviewer".to_string(),
        fork_turns: ForkPolicy {
            mode: "none".to_string(),
            turns: None,
        },
    });

    let bundle = compile_setup_spec(&spec).unwrap();
    validate_bundle(&bundle).unwrap();
    assert_eq!(bundle.source.integration, Integration::Planr);
    assert_eq!(bundle.evidence.status, "custom-unverified");
    assert_eq!(
        bundle.profiles.get("driver").unwrap().agent_type.as_deref(),
        Some("model_routing_sol_medium")
    );
    assert!(bundle.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/agents/model-routing-sol-medium.toml"
            && artifact
                .content
                .contains("name = \"model_routing_sol_medium\"")
    }));
    assert!(bundle.artifacts.iter().any(|artifact| {
        artifact.path == ".codex/agents/switchloom_reviewer.toml"
            && artifact.content.contains("name = \"switchloom_reviewer\"")
    }));
    assert!(
        bundle
            .artifacts
            .iter()
            .any(|artifact| artifact.path == ".planr/agents.toml")
    );
}

#[test]
fn fully_custom_setup_compiles_as_unverified_host_native_bundle() {
    let spec = SetupSpecV1 {
        schema_version: 1,
        host: "codex".to_string(),
        integration: Integration::Standalone,
        usage_policy: "balanced".to_string(),
        selected_roles: BTreeMap::from([
            (
                "orchestrator".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-sol".to_string(),
                    effort: Some("medium".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_orchestrator".to_string(),
                        task_name: "orchestrator".to_string(),
                        fork_turns: ForkPolicy {
                            mode: "none".to_string(),
                            turns: None,
                        },
                    }),
                },
            ),
            (
                "implementer".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-terra".to_string(),
                    effort: Some("high".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_implementer".to_string(),
                        task_name: "implementer".to_string(),
                        fork_turns: ForkPolicy {
                            mode: "none".to_string(),
                            turns: None,
                        },
                    }),
                },
            ),
        ]),
        routes: vec![SetupRouteMapping {
            work_type: "code".to_string(),
            role: "implementer".to_string(),
            fallbacks: vec!["orchestrator".to_string()],
        }],
        route_default: Some(SetupDefaultRoute {
            role: "orchestrator".to_string(),
            fallbacks: Vec::new(),
        }),
    };
    let bundle = compile_setup_spec(&spec).unwrap();
    assert_eq!(bundle.source.integration, Integration::Standalone);
    assert_eq!(bundle.evidence.status, "custom-unverified");
    assert!(bundle.profiles.contains_key("implementer"));
    assert_eq!(
        bundle.profiles.get("implementer").unwrap().model,
        "gpt-5.6-terra"
    );
    assert!(bundle.artifacts.iter().any(|artifact| artifact.path
        == ".codex/agents/switchloom_implementer.toml"
        && artifact.content.contains("model = \"gpt-5.6-terra\"")));
    let adapter_contract = bundle.adapter_contract.as_ref().unwrap();
    assert!(adapter_contract.routing_intent.role_requests.iter().any(
        |request| request.semantic_role == "implementer"
            && request.requested_model == "gpt-5.6-terra"
            && request.requested_effort.as_deref() == Some("high")
    ));
    assert!(
        adapter_contract
            .adapter
            .dispatch_recipe
            .artifact_paths
            .contains(&".codex/agents/switchloom_implementer.toml".to_string())
    );
    assert!(bundle.artifacts.iter().any(|artifact| {
        artifact.content.contains("task_name `implementer`")
            && !artifact.content.contains("sandbox_mode")
    }));
    assert!(
        bundle
            .artifacts
            .iter()
            .all(|artifact| !artifact.path.starts_with(".planr/"))
    );
    validate_bundle(&bundle).unwrap();
}

#[test]
fn child_only_setup_does_not_reintroduce_parent_profiles_routes_or_artifacts() {
    let spec = SetupSpecV1 {
        schema_version: 1,
        host: "codex-openai".to_string(),
        integration: Integration::Planr,
        usage_policy: "balanced".to_string(),
        selected_roles: BTreeMap::from([
            (
                "implementer".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-terra".to_string(),
                    effort: Some("high".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_implementer".to_string(),
                        task_name: "implementer".to_string(),
                        fork_turns: ForkPolicy {
                            mode: "none".to_string(),
                            turns: None,
                        },
                    }),
                },
            ),
            (
                "verifier".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-terra".to_string(),
                    effort: Some("medium".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_verifier".to_string(),
                        task_name: "verifier".to_string(),
                        fork_turns: ForkPolicy {
                            mode: "none".to_string(),
                            turns: None,
                        },
                    }),
                },
            ),
        ]),
        routes: vec![
            SetupRouteMapping {
                work_type: "code".to_string(),
                role: "implementer".to_string(),
                fallbacks: Vec::new(),
            },
            SetupRouteMapping {
                work_type: "review".to_string(),
                role: "verifier".to_string(),
                fallbacks: Vec::new(),
            },
            SetupRouteMapping {
                work_type: "verification".to_string(),
                role: "verifier".to_string(),
                fallbacks: Vec::new(),
            },
        ],
        route_default: None,
    };

    let bundle = compile_setup_spec(&spec).unwrap();
    validate_bundle(&bundle).unwrap();
    assert!(!bundle.profiles.contains_key("orchestrator"));
    assert!(bundle.route_default.is_none());
    assert!(
        bundle
            .routes
            .iter()
            .all(|route| route.profile != "orchestrator")
    );
    assert!(
        bundle
            .artifacts
            .iter()
            .all(|artifact| !artifact.path.contains("orchestrator")
                && !artifact.content.contains("switchloom_orchestrator"))
    );

    let contract = bundle.adapter_contract.as_ref().unwrap();
    assert!(
        !contract
            .routing_intent
            .semantic_roles
            .contains(&"orchestrator".to_string())
    );
    assert!(
        contract
            .routing_intent
            .role_requests
            .iter()
            .all(|request| request.semantic_role != "orchestrator")
    );
}

#[test]
fn successful_custom_setups_validate_final_bundles_for_each_host_family() {
    for (host, role, model, effort) in [
        ("claude-code", "implementer", "sonnet", Some("medium")),
        ("cursor", "implementer", "composer-2.5", None),
        (
            "opencode",
            "implementer",
            "opencode/gpt-5-nano",
            Some("medium"),
        ),
        ("mixed-host", "implementer", "sonnet", Some("medium")),
    ] {
        let spec = SetupSpecV1 {
            schema_version: 1,
            host: host.to_string(),
            integration: Integration::Standalone,
            usage_policy: "balanced".to_string(),
            selected_roles: BTreeMap::from([(
                role.to_string(),
                SetupRoleSelection {
                    model: model.to_string(),
                    effort: effort.map(ToOwned::to_owned),
                    spawn: None,
                },
            )]),
            routes: vec![SetupRouteMapping {
                work_type: "code".to_string(),
                role: role.to_string(),
                fallbacks: Vec::new(),
            }],
            route_default: None,
        };
        let bundle = compile_setup_spec(&spec).unwrap();
        validate_bundle(&bundle).unwrap();
        assert_eq!(bundle.evidence.status, "custom-unverified");
        assert_eq!(bundle.profiles.get(role).unwrap().model, model);
        let artifact_path = match host {
            "claude-code" => ".claude/agents/switchloom-implementer.md",
            "cursor" => ".cursor/agents/switchloom-implementer.md",
            "opencode" => ".opencode/agents/switchloom-implementer.md",
            "mixed-host" => ".model-routing/roles/implementer.toml",
            _ => unreachable!(),
        };
        assert!(
            bundle
                .artifacts
                .iter()
                .any(|artifact| artifact.path == artifact_path)
        );
        let contract = bundle.adapter_contract.as_ref().unwrap();
        assert!(
            contract
                .routing_intent
                .role_requests
                .iter()
                .any(|request| request.semantic_role == role
                    && request.requested_model == model
                    && request.requested_effort.as_deref() == effort)
        );
        assert!(
            contract
                .adapter
                .dispatch_recipe
                .artifact_paths
                .contains(&artifact_path.to_string())
        );
    }
}

#[test]
fn standalone_policy_contract_changes_enforceable_output() {
    let balanced = compile_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    let read_only =
        compile_policy("read-only-audit", "codex-openai", Integration::Standalone).unwrap();

    assert_ne!(balanced.policy, read_only.policy);
    assert_eq!(read_only.policy.usage.max_parallel_writers, 0);
    assert_eq!(
        read_only
            .policy
            .execution
            .roles
            .get("worker")
            .unwrap()
            .filesystem
            .write_roots,
        Vec::<String>::new()
    );

    let mut balanced_value: Value = serde_json::from_str(
        &compile_json("balanced", "codex-openai", Integration::Standalone).unwrap(),
    )
    .unwrap();
    let mut read_only_value: Value = serde_json::from_str(
        &compile_json("read-only-audit", "codex-openai", Integration::Standalone).unwrap(),
    )
    .unwrap();
    for value in [&mut balanced_value, &mut read_only_value] {
        let object = value.as_object_mut().unwrap();
        object.remove("bundle_id");
        object.remove("policy_id");
    }
    assert_ne!(balanced_value, read_only_value);
}

#[test]
fn codex_and_mixed_bindings_keep_native_bounded_fork_topology() {
    let codex = compile_json("balanced", "codex-openai", Integration::Standalone).unwrap();
    let mixed = compile_json("balanced", "mixed-host", Integration::Standalone).unwrap();
    assert!(codex.contains("gpt-5.6-sol"));
    assert!(codex.contains("\"fork_turns\""));
    assert!(codex.contains("\"mode\": \"none\""));
    assert!(!codex.contains("fork_turns: \\\"all\\\""));
    assert!(mixed.contains("fable-5"));
    assert!(mixed.contains("gpt-5.6-terra"));
    assert_ne!(codex, mixed);
}

#[test]
fn built_in_codex_routed_agent_types_use_no_fork_dispatch() {
    for policy in ["low-usage", "balanced", "max-quality", "read-only-audit"] {
        let bundle = compile_policy(policy, "codex-openai", Integration::Standalone).unwrap();
        let mut routed_profiles = bundle
            .routes
            .iter()
            .map(|route| route.profile.as_str())
            .collect::<BTreeSet<_>>();
        routed_profiles.insert(bundle.route_default.as_ref().unwrap().profile.as_str());

        for profile_id in routed_profiles {
            let profile = bundle.profiles.get(profile_id).unwrap();
            if profile.client == "codex" && profile.agent_type.is_some() {
                let fork_turns = profile
                    .fork_turns
                    .as_ref()
                    .unwrap_or_else(|| panic!("{policy} profile {profile_id} omitted fork_turns"));
                assert_eq!(
                    fork_turns.mode, "none",
                    "{policy} profile {profile_id} must dispatch Codex role with fork_turns none"
                );
                assert_eq!(
                    fork_turns.turns, None,
                    "{policy} profile {profile_id} fork_turns none must not declare turns"
                );
            }
        }
    }
}

#[test]
fn built_in_codex_presets_do_not_route_to_ultra() {
    for policy in ["low-usage", "balanced", "max-quality"] {
        let bundle = compile_policy(policy, "codex-openai", Integration::Standalone).unwrap();
        assert_ne!(
            bundle.route_default.as_ref().unwrap().profile,
            "codex-sol-ultra",
            "{policy} must not default to Ultra"
        );
        assert!(
            bundle
                .routes
                .iter()
                .all(|route| route.profile != "codex-sol-ultra"),
            "{policy} must not route any default work type to Ultra"
        );
        assert!(
            bundle.profiles.contains_key("codex-sol-ultra"),
            "{policy} keeps Ultra available as an explicit manual profile"
        );
        assert!(
            bundle
                .artifacts
                .iter()
                .any(|artifact| artifact.path == ".codex/agents/model-routing-sol-ultra.toml"),
            "{policy} keeps the manual Ultra role artifact"
        );
    }
}

#[test]
fn codex_child_overrides_require_bounded_fork_policy() {
    let mut profile = Profile {
        client: "codex".to_string(),
        model: "gpt-5.6-terra".to_string(),
        agent_type: Some("model_routing_terra_high".to_string()),
        effort: Some("high".to_string()),
        cost_tier: None,
        capabilities: Vec::new(),
        skill: None,
        notes: None,
        fork_turns: None,
    };
    assert!(
        validate_profile_fork_policy(&profile)
            .unwrap_err()
            .to_string()
            .contains("must declare fork_turns")
    );

    profile.fork_turns = Some(ForkPolicy {
        mode: "all".to_string(),
        turns: None,
    });
    assert!(
        validate_profile_fork_policy(&profile)
            .unwrap_err()
            .to_string()
            .contains("must not use fork_turns all")
    );

    profile.fork_turns = Some(ForkPolicy {
        mode: "bounded".to_string(),
        turns: Some(2),
    });
    validate_profile_fork_policy(&profile).unwrap();
}

#[test]
fn generated_registry_is_derived_from_binding_profiles_and_routes() {
    for host in BINDINGS.map(|(host, _)| host) {
        let bundle = compile_policy("balanced", host, Integration::Planr).unwrap();
        let registry = bundle
            .artifacts
            .iter()
            .find(|artifact| artifact.path == ".planr/agents.toml")
            .unwrap();
        let parsed: toml::Value = toml::from_str(&registry.content).unwrap();
        assert_eq!(
            parsed["profiles"].as_table().unwrap().len(),
            bundle.profiles.len()
        );
        assert_eq!(
            parsed["routes"].as_array().unwrap().len(),
            bundle.routes.len()
        );
    }
}

#[test]
fn offline_evaluation_never_claims_live_verification_or_recommendation() {
    let report = evaluate_policy("balanced", "codex-openai").unwrap();
    assert!(report.offline_reproducible);
    assert!(report.scenario_count >= 7);
    assert_eq!(report.status, "experimental");
    assert!(!report.recommended);
}

#[test]
fn no_in_memory_claim_can_promote_offline_evaluation() {
    let report = evaluate_policy("balanced", "codex-openai").unwrap();
    assert!(report.live_evidence.is_none());
    assert_eq!(report.status, "experimental");
    assert!(!report.recommended);
}

#[test]
fn built_in_presets_compile_through_setup_spec_without_output_drift() {
    let spec = setup_spec_for_policy("balanced", "codex-openai", Integration::Planr).unwrap();
    assert_eq!(
        compile_setup_spec(&spec).unwrap(),
        compile_builtin_policy_direct("balanced", "codex-openai", Integration::Planr).unwrap()
    );
}

#[test]
fn public_host_aliases_preserve_built_in_preset_output() {
    for (alias, binding) in [
        ("codex", "codex-openai"),
        ("cursor", "cursor-openai"),
        ("claude-code", "claude-native"),
    ] {
        let spec = setup_spec_for_policy("balanced", alias, Integration::Standalone).unwrap();
        assert_eq!(spec.host, binding);
        assert_eq!(
            compile_setup_spec(&spec).unwrap(),
            compile_builtin_policy_direct("balanced", binding, Integration::Standalone).unwrap()
        );
    }
}
