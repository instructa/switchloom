use model_routing::*;

#[test]
fn probe_does_not_infer_authentication_from_version_availability() {
    let report = probe_host(
        "codex-openai",
        Some("definitely-not-a-model-routing-host-command"),
    )
    .unwrap();
    assert!(!report.available);
    assert_eq!(report.authentication, "not_tested");
}
