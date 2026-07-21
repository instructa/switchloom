use crate::*;
use serde_json::Value;

#[test]
fn planr_integration_is_explicit_and_adds_planr_declarations() {
    let standalone = compile_json("balanced", "codex-openai", Integration::Standalone).unwrap();
    let planr = compile_json("balanced", "codex-openai", Integration::Planr).unwrap();
    assert!(!standalone.contains(".planr/agents.toml"));
    assert!(planr.contains(".planr/agents.toml"));
    assert!(planr.contains(".planr/policy.toml"));
}

#[test]
fn adapter_contract_handoff_names_planr_consumer_boundaries() {
    let binding = binding_for_selector("codex-openai").unwrap();
    let contract = adapter_contract_for_binding("balanced", &binding, Integration::Planr).unwrap();
    let handoff = &contract.planr_handoff;
    let package_json: Value = serde_json::from_str(NPM_PACKAGE_JSON).unwrap();
    let expected_package = format!(
        "{}@{}",
        package_json["name"].as_str().unwrap(),
        package_json["version"].as_str().unwrap()
    );
    assert_eq!(handoff.switchloom_package, expected_package);
    assert_ne!(
        handoff.switchloom_package,
        format!("{}@{PACKAGE_VERSION}", env!("CARGO_PKG_NAME"))
    );
    assert!(
        handoff
            .semantic_role_contract
            .contains("usage policy `balanced`")
    );
    assert!(
        handoff
            .required_consumer_behavior
            .iter()
            .any(|behavior| behavior.contains("RoutingIntentV1"))
    );
    assert!(
        handoff
            .required_consumer_behavior
            .iter()
            .any(|behavior| behavior.contains("package digest"))
    );
    assert!(
        handoff
            .forbidden_duplicate_ownership
            .iter()
            .any(|behavior| behavior.contains("model catalog"))
    );
    assert!(
        handoff
            .certification_report_reference
            .contains("reports/native-host-certification")
    );
}

#[test]
fn planr_integration_is_skill_free_and_preloads_existing_protocols() {
    for host in BINDINGS.map(|(host, _)| host) {
        let bundle = compile_policy("balanced", host, Integration::Planr).unwrap();
        assert!(
            bundle
                .artifacts
                .iter()
                .any(|artifact| artifact.path == ".planr/agents.toml")
        );
        assert!(
            bundle
                .artifacts
                .iter()
                .any(|artifact| artifact.path == ".planr/policy.toml")
        );
        assert!(
            bundle
                .artifacts
                .iter()
                .all(|artifact| !artifact.path.contains("/skills/"))
        );
        let content = serde_json::to_string(&bundle).unwrap();
        assert!(!content.contains("model-routing-native-routing"));
        assert!(!content.contains("planr-native-routing"));
        assert!(!content.contains("Protocol preload: $planr-goal"));
        assert!(!content.contains("Protocol preload: $planr-loop"));

        let worker_protocol = bundle.artifacts.iter().any(|artifact| {
            (artifact.path.contains("terra-high")
                || artifact.path.contains("luna-xhigh")
                || artifact.path.contains("preset-worker")
                || artifact.path.starts_with(".pi/workflows/"))
                && artifact.content.contains("Protocol preload: $planr-work")
        });
        assert!(worker_protocol, "missing Planr worker preload for {host}");

        if host == "codex-openai" || host == "mixed-host" {
            assert!(
                bundle
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.content.contains("Protocol preload: $planr-review")),
                "missing Planr review preload for {host}"
            );
        }

        if host == "codex-openai" {
            let registry = bundle
                .artifacts
                .iter()
                .find(|artifact| artifact.path == ".planr/agents.toml")
                .expect("missing Planr registry");
            assert!(!registry.content.contains("agent_type ="));
            assert!(!registry.content.contains("fork_turns"));
            assert!(
                registry
                    .content
                    .contains("[profiles.model_routing_terra_high]")
            );
            assert!(
                registry
                    .content
                    .contains("[profiles.model_routing_sol_high]")
            );
            assert!(
                registry
                    .content
                    .contains("profile = \"model_routing_terra_high\"")
            );
            assert!(
                registry
                    .content
                    .contains("profile = \"model_routing_sol_high\"")
            );
        }
    }
}
