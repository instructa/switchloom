use model_routing::*;
use serde_json::Value;

#[test]
fn complete_policy_binding_pool_compiles_deterministically() {
    let summaries = list_policies().unwrap();
    assert_eq!(summaries.len(), 28);
    for summary in summaries {
        let first =
            compile_json(&summary.policy_id, &summary.host, Integration::Standalone).unwrap();
        let second =
            compile_json(&summary.policy_id, &summary.host, Integration::Standalone).unwrap();
        assert_eq!(first, second);
        assert!(!first.contains(".planr/policy.toml"));
        assert!(first.contains("\"package\": \"model-routing\""));
    }
}

#[test]
fn valid_generated_bundles_pass_strict_inspection() {
    for integration in [Integration::Standalone, Integration::Planr] {
        let bundle = compile_json("balanced", "codex-openai", integration).unwrap();
        let inspection = inspect_bundle_json(&bundle).unwrap();
        assert!(inspection.valid);
        assert_eq!(inspection.integration, integration);
        assert_eq!(inspection.policy_id, "balanced");
    }
}

#[test]
fn invalid_contract_fixtures_fail_for_named_reasons() {
    for (fixture, expected) in [
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-unsupported-version.json"),
            "unsupported schema_version 2",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-dual-artifact-payload.json"),
            "cannot define both content and content_ref",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-artifact-hash.json"),
            "sha256 mismatch",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-unknown-source-field.json"),
            "unknown field `unexpected`",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-unknown-policy-usage-field.json"),
            "unknown field `unexpected`",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-unknown-profile-field.json"),
            "unknown field `unexpected`",
        ),
        (
            include_str!(
                "../fixtures/routing-bundle-v1/invalid-runtime-missing-source-reference.json"
            ),
            "runtime behavior must declare source references",
        ),
        (
            include_str!(
                "../fixtures/routing-bundle-v1/invalid-runtime-bogus-source-reference.json"
            ),
            "source reference must match the digest-bound evidence artifact",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-runtime-capability-mismatch.json"),
            "capability_version must match parsed evidence_id",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-runtime-slot-count.json"),
            "exactly the parsed evidence child slots",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-runtime-version-drift.json"),
            "installed host version must match parsed evidence command output",
        ),
        (
            include_str!("../fixtures/routing-bundle-v1/invalid-runtime-ultra-delegation.json"),
            "delegation modes must match parsed evidence",
        ),
    ] {
        let error = inspect_bundle_json(fixture).unwrap_err().to_string();
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
fn checked_in_planr_contract_fixtures_are_generated_outputs() {
    for (host, fixture) in [
        (
            "codex-openai",
            include_str!("../fixtures/routing-bundle-v1/valid-balanced-codex.json"),
        ),
        (
            "mixed-host",
            include_str!("../fixtures/routing-bundle-v1/valid-balanced-mixed.json"),
        ),
    ] {
        let generated: serde_json::Value =
            serde_json::from_str(&compile_json("balanced", host, Integration::Planr).unwrap())
                .unwrap();
        let checked_in: serde_json::Value = serde_json::from_str(fixture).unwrap();
        assert_eq!(generated, checked_in, "regenerate fixture for {host}");
    }
}

#[test]
fn catalog_is_reproducible_and_contains_the_full_pool() {
    let first = catalog_json().unwrap();
    let second = catalog_json().unwrap();
    assert_eq!(first, second);
    let value: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(value["compositions"].as_array().unwrap().len(), 28);
    assert!(
        value["compositions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["recommended"] == false)
    );
}
