use crate::*;

#[test]
fn codex_runtime_evidence_fixtures_fail_for_named_provenance_reasons() {
    for (fixture, expected) in [
        (
            include_str!(
                "../../fixtures/codex-v2-runtime-evidence/invalid-prose-only-provenance.json"
            ),
            "must include raw output",
        ),
        (
            include_str!(
                "../../fixtures/codex-v2-runtime-evidence/invalid-arbitrary-prose-provenance.json"
            ),
            "raw capture does not support",
        ),
        (
            include_str!(
                "../../fixtures/codex-v2-runtime-evidence/invalid-tampered-raw-digest.json"
            ),
            "raw output digest mismatch",
        ),
        (
            include_str!(
                "../../fixtures/codex-v2-runtime-evidence/invalid-unsupported-provenance-kind.json"
            ),
            "unsupported provenance kind",
        ),
    ] {
        let evidence: CodexV2RuntimeEvidence = serde_json::from_str(fixture).unwrap();
        let error = validate_codex_v2_runtime_evidence(&evidence)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
fn codex_runtime_evidence_rejects_retained_source_without_claimed_raw_output() {
    let mut evidence = codex_v2_runtime_evidence().unwrap();
    let record = evidence
        .claim_provenance
        .get_mut("installed_version")
        .unwrap()
        .first_mut()
        .unwrap();
    record.source_path = Some(
        "fixtures/codex-v2-runtime-evidence/codex-0.145-after-without-version.txt".to_string(),
    );

    let error = validate_codex_v2_runtime_evidence(&evidence)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("does not contain claimed raw output"),
        "unexpected error: {error}"
    );
}

#[test]
fn dispatch_evidence_requires_persisted_requested_and_effective_receipt_fields() {
    let mut valid = DispatchEvidenceV1 {
        schema_version: 1,
        package_digest: "sha256:abc".to_string(),
        host_version: "codex 0.145.0".to_string(),
        requested_dispatch: RequestedDispatchEvidence {
            semantic_role: "worker".to_string(),
            profile: "codex-terra-high".to_string(),
            model: "gpt-5.6-terra".to_string(),
            effort: Some("high".to_string()),
            agent_type: Some("model_routing_terra_high".to_string()),
            fork_turns: Some(ForkPolicy {
                mode: "none".to_string(),
                turns: None,
            }),
        },
        child_identity: ChildIdentityEvidence {
            host: "codex".to_string(),
            role: "worker".to_string(),
            agent_role: "model_routing_terra_high".to_string(),
            agent_type: Some("model_routing_terra_high".to_string()),
            task_name: Some("worker".to_string()),
        },
        effective_model: Some("gpt-5.6-terra".to_string()),
        effective_effort: Some("high".to_string()),
        nonce: "nonce-123".to_string(),
        raw_evidence_refs: vec!["receipt.json".to_string()],
        verdict: GuaranteeLevel::Deterministic,
    };
    let encoded = serde_json::to_string(&valid).unwrap();
    let decoded: DispatchEvidenceV1 = serde_json::from_str(&encoded).unwrap();
    validate_dispatch_evidence(&decoded).unwrap();

    let missing_nonce = r#"{
  "schema_version": 1,
  "package_digest": "sha256:abc",
  "host_version": "codex 0.145.0",
  "requested_dispatch": {
    "semantic_role": "worker",
    "profile": "codex-terra-high",
    "model": "gpt-5.6-terra"
  },
  "child_identity": {
    "host": "codex",
    "role": "worker",
    "agent_role": "model_routing_terra_high"
  },
  "raw_evidence_refs": ["receipt.json"],
  "verdict": "deterministic"
}"#;
    let error = serde_json::from_str::<DispatchEvidenceV1>(missing_nonce)
        .unwrap_err()
        .to_string();
    assert!(error.contains("nonce"));

    valid.effective_model = None;
    let error = validate_dispatch_evidence(&valid).unwrap_err().to_string();
    assert!(error.contains("effective_model"));

    valid.effective_model = Some("gpt-5.6-sol".to_string());
    let error = validate_dispatch_evidence(&valid).unwrap_err().to_string();
    assert!(error.contains("does not match requested model"));

    valid.effective_model = Some("gpt-5.6-terra".to_string());
    valid.effective_effort = Some("medium".to_string());
    let error = validate_dispatch_evidence(&valid).unwrap_err().to_string();
    assert!(error.contains("does not match requested effort"));

    valid.effective_effort = None;
    valid.verdict = GuaranteeLevel::Advisory;
    validate_dispatch_evidence(&valid).unwrap();
}

#[test]
fn adapter_validation_blocks_unproven_deterministic_claude_and_cursor_evidence() {
    for (host, role, profile, model, effort) in [
        (
            "claude-native",
            "worker",
            "claude-native-worker",
            "sonnet",
            Some("medium"),
        ),
        (
            "cursor-openai",
            "worker",
            "cursor-openai-worker",
            "gpt-5.4-mini",
            None,
        ),
    ] {
        let contract = compile_policy("balanced", host, Integration::Standalone)
            .unwrap()
            .adapter_contract
            .unwrap();
        let mut evidence = DispatchEvidenceV1 {
            schema_version: 1,
            package_digest: "sha256:abc".to_string(),
            host_version: format!("{} cli 1.0.0", contract.capability.host),
            requested_dispatch: RequestedDispatchEvidence {
                semantic_role: role.to_string(),
                profile: profile.to_string(),
                model: model.to_string(),
                effort: effort.map(str::to_string),
                agent_type: None,
                fork_turns: Some(ForkPolicy {
                    mode: "none".to_string(),
                    turns: None,
                }),
            },
            child_identity: ChildIdentityEvidence {
                host: contract.capability.host.clone(),
                role: role.to_string(),
                agent_role: "model-routing-preset-worker".to_string(),
                agent_type: None,
                task_name: Some("model-routing-preset-worker".to_string()),
            },
            effective_model: Some(model.to_string()),
            effective_effort: effort.map(str::to_string),
            nonce: "nonce-456".to_string(),
            raw_evidence_refs: vec!["host-output.json".to_string()],
            verdict: GuaranteeLevel::Deterministic,
        };
        let error = validate_dispatch_evidence_for_adapter(&evidence, &contract)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("effective model observability is Advisory"),
            "{host}: {error}"
        );

        evidence.verdict = GuaranteeLevel::Advisory;
        validate_dispatch_evidence_for_adapter(&evidence, &contract).unwrap();

        evidence.verdict = GuaranteeLevel::Deterministic;
        evidence.raw_evidence_refs.push(format!(
            "host-authenticated-effective-model:{}:host-output.json#model",
            contract.capability.host
        ));
        if effort.is_some() {
            evidence.raw_evidence_refs.push(format!(
                "host-authenticated-effective-effort:{}:host-output.json#effort",
                contract.capability.host
            ));
        }
        let error = validate_dispatch_evidence_for_adapter(&evidence, &contract)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("effective model observability is Advisory"),
            "forged refs should not upgrade {host}: {error}"
        );
    }
}
