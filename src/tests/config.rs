use crate::*;
use std::collections::BTreeMap;

#[test]
fn setup_spec_roundtrips_through_canonical_toml_json_and_recipe() {
    let spec = setup_spec_for_policy("balanced", "codex-openai", Integration::Standalone).unwrap();
    let json = setup_spec_to_canonical_json(&spec).unwrap();
    let toml = setup_spec_to_canonical_toml(&spec).unwrap();
    let recipe = setup_spec_to_recipe(&spec).unwrap();
    assert!(json.contains("\"schema_version\": 1"));
    assert!(toml.contains("schema_version = 1"));
    assert!(recipe.starts_with(SETUP_RECIPE_PREFIX));
    assert_eq!(setup_spec_from_json(&json).unwrap(), spec);
    assert_eq!(setup_spec_from_toml(&toml).unwrap(), spec);
    assert_eq!(setup_spec_from_recipe(&recipe).unwrap(), spec);
}

#[test]
fn setup_recipe_enforces_exact_pre_decode_size_boundaries() {
    assert_eq!(MAX_SETUP_RECIPE_BYTES % 3, 1);
    assert_eq!(
        MAX_SETUP_RECIPE_ENCODED_BYTES,
        (MAX_SETUP_RECIPE_BYTES / 3) * 4 + 2
    );

    let boundary_payload = encode_base64url(&vec![0_u8; MAX_SETUP_RECIPE_BYTES]);
    assert_eq!(boundary_payload.len(), MAX_SETUP_RECIPE_ENCODED_BYTES);
    assert_eq!(
        decode_base64url(&boundary_payload).unwrap().len(),
        MAX_SETUP_RECIPE_BYTES
    );
    let boundary_error =
        setup_spec_from_recipe(&format!("{SETUP_RECIPE_PREFIX}{boundary_payload}"))
            .unwrap_err()
            .to_string();
    assert!(!boundary_error.contains("exceeds"));

    let first_oversized_payload = encode_base64url(&vec![0_u8; MAX_SETUP_RECIPE_BYTES + 1]);
    assert_eq!(
        first_oversized_payload.len(),
        MAX_SETUP_RECIPE_ENCODED_BYTES + 1
    );
    assert!(
        setup_spec_from_recipe(&format!("{SETUP_RECIPE_PREFIX}{first_oversized_payload}"))
            .unwrap_err()
            .to_string()
            .contains("exceeds")
    );

    let very_large_payload = "A".repeat(MAX_SETUP_RECIPE_ENCODED_BYTES * 4);
    assert!(
        setup_spec_from_recipe(&format!("{SETUP_RECIPE_PREFIX}{very_large_payload}"))
            .unwrap_err()
            .to_string()
            .contains("base64url characters")
    );
}

#[test]
fn custom_setup_rejects_duplicate_codex_spawn_identities() {
    let duplicate_agent_type = SetupSpecV1 {
        schema_version: 1,
        host: "codex".to_string(),
        integration: Integration::Standalone,
        usage_policy: "balanced".to_string(),
        selected_roles: BTreeMap::from([
            (
                "implementer".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-terra".to_string(),
                    effort: Some("high".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_shared".to_string(),
                        task_name: "implementer".to_string(),
                        fork_turns: ForkPolicy {
                            mode: "none".to_string(),
                            turns: None,
                        },
                    }),
                },
            ),
            (
                "reviewer".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-sol".to_string(),
                    effort: Some("high".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_shared".to_string(),
                        task_name: "reviewer".to_string(),
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
            fallbacks: vec!["reviewer".to_string()],
        }],
        route_default: Some(SetupDefaultRoute {
            role: "implementer".to_string(),
            fallbacks: Vec::new(),
        }),
    };
    assert!(
        compile_setup_spec(&duplicate_agent_type)
            .unwrap_err()
            .to_string()
            .contains("both declare Codex agent_type `switchloom_shared`")
    );

    let duplicate_task_name = SetupSpecV1 {
        schema_version: 1,
        host: "codex".to_string(),
        integration: Integration::Standalone,
        usage_policy: "balanced".to_string(),
        selected_roles: BTreeMap::from([
            (
                "foo-bar".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-terra".to_string(),
                    effort: Some("high".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_foo_bar".to_string(),
                        task_name: "foo_bar".to_string(),
                        fork_turns: ForkPolicy {
                            mode: "none".to_string(),
                            turns: None,
                        },
                    }),
                },
            ),
            (
                "foo_bar".to_string(),
                SetupRoleSelection {
                    model: "gpt-5.6-sol".to_string(),
                    effort: Some("high".to_string()),
                    spawn: Some(SetupSpawnPolicy {
                        agent_type: "switchloom_foo_bar_alt".to_string(),
                        task_name: "foo_bar".to_string(),
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
            role: "foo-bar".to_string(),
            fallbacks: vec!["foo_bar".to_string()],
        }],
        route_default: None,
    };
    assert!(
        compile_setup_spec(&duplicate_task_name)
            .unwrap_err()
            .to_string()
            .contains("both normalize to `foo_bar`")
    );
}

#[test]
fn custom_setup_rejects_normalized_artifact_path_collisions() {
    for (host, model, effort, expected_path) in [
        (
            "claude-code",
            "sonnet",
            Some("medium"),
            ".claude/agents/switchloom-foo_bar.md",
        ),
        (
            "cursor",
            "composer-2.5",
            None,
            ".cursor/agents/switchloom-foo_bar.md",
        ),
        (
            "mixed-host",
            "sonnet",
            Some("medium"),
            ".model-routing/roles/foo_bar.toml",
        ),
    ] {
        let spec = SetupSpecV1 {
            schema_version: 1,
            host: host.to_string(),
            integration: Integration::Standalone,
            usage_policy: "balanced".to_string(),
            selected_roles: BTreeMap::from([
                (
                    "foo-bar".to_string(),
                    SetupRoleSelection {
                        model: model.to_string(),
                        effort: effort.map(ToOwned::to_owned),
                        spawn: None,
                    },
                ),
                (
                    "foo_bar".to_string(),
                    SetupRoleSelection {
                        model: model.to_string(),
                        effort: effort.map(ToOwned::to_owned),
                        spawn: None,
                    },
                ),
            ]),
            routes: vec![SetupRouteMapping {
                work_type: "code".to_string(),
                role: "foo-bar".to_string(),
                fallbacks: vec!["foo_bar".to_string()],
            }],
            route_default: None,
        };
        let error = compile_setup_spec(&spec).unwrap_err().to_string();
        assert!(
            error.contains("both normalize to `foo_bar`") || error.contains(expected_path),
            "expected normalized collision for {host}, got {error}"
        );
    }
}

#[test]
fn setup_spec_rejects_unknown_fields_and_invalid_combinations() {
    let unknown = r#"{
  "schema_version": 1,
  "host": "codex",
  "integration": "standalone",
  "usage_policy": "balanced",
  "selected_roles": {},
  "routes": [],
  "unexpected": true
}"#;
    assert!(format!("{:#}", setup_spec_from_json(unknown).unwrap_err()).contains("unknown field"));

    let invalid_effort = SetupSpecV1 {
        schema_version: 1,
        host: "codex".to_string(),
        integration: Integration::Standalone,
        usage_policy: "balanced".to_string(),
        selected_roles: BTreeMap::from([(
            "implementer".to_string(),
            SetupRoleSelection {
                model: "gpt-5.6-luna".to_string(),
                effort: Some("ultra".to_string()),
                spawn: Some(SetupSpawnPolicy {
                    agent_type: "switchloom_implementer".to_string(),
                    task_name: "implementer".to_string(),
                    fork_turns: ForkPolicy {
                        mode: "none".to_string(),
                        turns: None,
                    },
                }),
            },
        )]),
        routes: vec![SetupRouteMapping {
            work_type: "code".to_string(),
            role: "implementer".to_string(),
            fallbacks: Vec::new(),
        }],
        route_default: None,
    };
    assert!(
        validate_setup_spec(&invalid_effort)
            .unwrap_err()
            .to_string()
            .contains("is not supported")
    );

    let mut invalid_fork = invalid_effort;
    invalid_fork
        .selected_roles
        .get_mut("implementer")
        .unwrap()
        .model = "gpt-5.6-terra".to_string();
    invalid_fork
        .selected_roles
        .get_mut("implementer")
        .unwrap()
        .effort = Some("high".to_string());
    invalid_fork
        .selected_roles
        .get_mut("implementer")
        .unwrap()
        .spawn
        .as_mut()
        .unwrap()
        .fork_turns = ForkPolicy {
        mode: "all".to_string(),
        turns: None,
    };
    assert!(
        validate_setup_spec(&invalid_fork)
            .unwrap_err()
            .to_string()
            .contains("must not use fork_turns all")
    );

    let mut missing_spawn = invalid_fork.clone();
    missing_spawn
        .selected_roles
        .get_mut("implementer")
        .unwrap()
        .spawn = None;
    assert!(
        validate_setup_spec(&missing_spawn)
            .unwrap_err()
            .to_string()
            .contains("must declare Codex spawn policy")
    );

    let mut name_mismatch = invalid_fork.clone();
    let spawn = name_mismatch
        .selected_roles
        .get_mut("implementer")
        .unwrap()
        .spawn
        .as_mut()
        .unwrap();
    spawn.fork_turns = ForkPolicy {
        mode: "none".to_string(),
        turns: None,
    };
    spawn.task_name = "wrong_name".to_string();
    assert!(
        validate_setup_spec(&name_mismatch)
            .unwrap_err()
            .to_string()
            .contains("must match `implementer`")
    );

    let mut task_path = name_mismatch;
    task_path
        .selected_roles
        .get_mut("implementer")
        .unwrap()
        .spawn
        .as_mut()
        .unwrap()
        .task_name = "/root/task".to_string();
    assert!(
        validate_setup_spec(&task_path)
            .unwrap_err()
            .to_string()
            .contains("not a canonical task path")
    );

    let legacy_fork_context = r#"{
  "schema_version": 1,
  "host": "codex",
  "integration": "standalone",
  "usage_policy": "balanced",
  "selected_roles": {
    "implementer": {
      "model": "gpt-5.6-terra",
      "effort": "high",
      "fork_context": "none"
    }
  },
  "routes": [{"work_type": "code", "role": "implementer"}]
}"#;
    assert!(
        format!(
            "{:#}",
            setup_spec_from_json(legacy_fork_context).unwrap_err()
        )
        .contains("fork_context")
    );
}

#[test]
fn setup_contract_catalog_exposes_transport_and_host_options() {
    let catalog = setup_contract_catalog_value().unwrap();
    assert_eq!(catalog["configPath"], SETUP_CONFIG_PATH);
    assert_eq!(catalog["recipePrefix"], SETUP_RECIPE_PREFIX);
    assert!(catalog["hosts"].as_array().unwrap().iter().any(|host| {
        host["id"] == "codex"
            && host["models"]
                .as_array()
                .unwrap()
                .iter()
                .any(|model| model["id"] == "gpt-5.6-sol")
    }));
    assert!(catalog["hosts"].as_array().unwrap().iter().any(|host| {
        host["id"] == "opencode"
            && host["binding"] == "opencode-native"
            && host["models"]
                .as_array()
                .unwrap()
                .iter()
                .any(|model| model["id"] == "opencode/gpt-5-nano")
    }));
}
