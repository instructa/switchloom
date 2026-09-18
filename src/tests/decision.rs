use super::*;
use crate::catalog::ModelSettings;
use serde_json::json;

fn judgment(choice: &str, confidence: f64, missing: f64, enabled: &[&str]) -> Judgment {
    let other = if enabled.len() == 1 {
        0.0
    } else {
        0.1 / (enabled.len() - 1) as f64
    };
    let probabilities: BTreeMap<_, _> = enabled
        .iter()
        .map(|key| ((*key).to_string(), if *key == choice { 0.9 } else { other }))
        .collect();
    serde_json::from_value(json!({
        "model":"jev-test",
        "answers":{
            "route":{"type":"choice","choice":choice,"confidence":confidence,"probabilities":probabilities},
            "context_missing":{"type":"noul","noul":missing}
        },
        "usage":{"input_tokens":120,"output_tokens":30}
    }))
    .unwrap()
}

fn request(capability: Option<&str>) -> RouteRequest {
    RouteRequest {
        task: "Implement agreed change".into(),
        context: String::new(),
        routing: capability.map_or(Routing::Jev, |capability| Routing::Assigned {
            capability: capability.into(),
        }),
    }
}

#[test]
fn explicit_capabilities_are_offline_and_ownership_follows_configuration() {
    let catalog = load_catalog().unwrap();
    let mut profiles = catalog.default_profiles();
    profiles
        .get_mut("sol")
        .unwrap()
        .capabilities
        .retain(|cap| cap != "implementation");
    profiles
        .get_mut("luna")
        .unwrap()
        .capabilities
        .push("implementation".into());
    profiles.get_mut("luna").unwrap().model = "gpt-next".into();
    let result = route_task_for_profiles(&request(Some("implementation")), &profiles).unwrap();
    assert!(result.usage.is_none());
    let assigned = result.assignment.unwrap();
    assert_eq!(assigned.owner, "luna");
    assert_eq!(assigned.model, "gpt-next");
    assert_eq!(assigned.effort, "max");
}

#[test]
fn jev_selects_only_enabled_capabilities_and_preserves_model_settings() {
    let catalog = load_catalog().unwrap();
    let mut profiles = catalog.default_profiles();
    profiles.get_mut("sol").unwrap().model = "gpt-6-sol".into();
    let enabled = catalog.routable(&profiles);
    assert!(enabled.iter().any(|cap| cap.id == "browser"));
    assert!(!enabled.iter().any(|cap| cap.id == "spatial"));
    for capability in &enabled {
        let ids: Vec<_> = enabled.iter().map(|cap| cap.id.as_str()).collect();
        let result = decide(
            judgment(&capability.id, 0.9, 0.1, &ids),
            &enabled,
            &profiles,
            &catalog,
        )
        .unwrap();
        let actual = result.assignment.unwrap();
        let expected = assignment(&capability.id, &profiles, &catalog).unwrap();
        assert_eq!(actual.owner, expected.owner);
        assert_eq!(actual.model, expected.model);
        assert_eq!(actual.instructions, expected.instructions);
        assert!(matches!(result.reason, DecisionReason::Jev));
    }
}

#[test]
fn missing_context_and_suggestion_do_not_dispatch_a_guess() {
    let catalog = load_catalog().unwrap();
    let profiles = catalog.default_profiles();
    let enabled = catalog.routable(&profiles);
    let ids: Vec<_> = enabled.iter().map(|cap| cap.id.as_str()).collect();
    let missing = decide(
        judgment("planning", 0.95, 0.9, &ids),
        &enabled,
        &profiles,
        &catalog,
    )
    .unwrap();
    assert!(missing.assignment.is_none());
    assert!(matches!(missing.reason, DecisionReason::MissingContext));
    let split = |choice: &str, confidence| {
        let mut value = judgment(choice, confidence, 0.1, &ids);
        for probability in value.answers.route.probabilities.values_mut() {
            *probability = 0.0;
        }
        value.answers.route.probabilities.insert(choice.into(), 0.6);
        value
            .answers
            .route
            .probabilities
            .insert("implementation".into(), 0.4);
        value
    };
    let uncertain = decide(split("planning", 0.3), &enabled, &profiles, &catalog).unwrap();
    assert!(uncertain.assignment.is_none());
    let suggested = decide(split("planning", 0.6), &enabled, &profiles, &catalog).unwrap();
    assert_eq!(suggested.assignment.unwrap().capability, "planning");
    assert!(matches!(suggested.reason, DecisionReason::Suggested));
    let elevated = decide(split("review", 0.8), &enabled, &profiles, &catalog).unwrap();
    assert!(matches!(elevated.reason, DecisionReason::Suggested));
}

#[test]
fn sole_capability_requires_explicit_assignment_even_when_elevated() {
    for capability in ["implementation", "browser", "review"] {
        let profiles = BTreeMap::from([(
            "sol".into(),
            Profile {
                model: "gpt-5.6-sol".into(),
                effort: "medium".into(),
                capabilities: vec![capability.into()],
            },
        )]);
        let mut input = request(None);
        input.task = "Continue".into();
        let result = route_task_for_profiles(&input, &profiles).unwrap();
        assert!(result.assignment.is_none());
        assert!(matches!(
            result.reason,
            DecisionReason::ExplicitCapabilityRequired
        ));
        assert!(result.usage.is_none());
        input.routing = Routing::Assigned {
            capability: capability.into(),
        };
        assert_eq!(
            route_task_for_profiles(&input, &profiles)
                .unwrap()
                .assignment
                .unwrap()
                .capability,
            capability
        );
    }
}

#[test]
fn automatic_routing_uses_only_configured_work_capabilities() {
    let catalog = load_catalog().unwrap();
    let profiles = catalog.default_profiles();
    let prepared = prepare_route(&request(None), &profiles).unwrap();
    let PreparedRoute::Query { query } = prepared else {
        panic!("expected automatic judgment")
    };
    let criteria = query["questions"]["route"]["criteria"].as_object().unwrap();
    for capability in catalog
        .capabilities
        .iter()
        .filter(|cap| cap.default_owner.is_some())
    {
        assert_eq!(criteria.contains_key(&capability.id), capability.routable);
    }
}

#[test]
fn model_validation_evaluates_the_catalog_pattern() {
    let mut catalog = load_catalog().unwrap();
    let settings = |model: &str| ModelSettings {
        model: model.into(),
        effort: "medium".into(),
    };
    assert!(catalog.validate_settings(&settings("gpt-next")).is_ok());
    for id in ["gpt-next\n", "gpt bad", "", "gpt-NEXT"] {
        assert!(catalog.validate_settings(&settings(id)).is_err());
    }
    catalog.model_id_pattern = "^gpt-[A-Z]+$".into();
    assert!(catalog.validate_settings(&settings("gpt-NEXT")).is_ok());
    assert!(catalog.validate_settings(&settings("gpt-next")).is_err());
    catalog.model_id_pattern = "[".into();
    assert!(
        catalog
            .validate_settings(&settings("gpt-NEXT"))
            .unwrap_err()
            .to_string()
            .contains("invalid catalog")
    );
}

#[test]
fn invalid_distribution_cannot_trigger_assignment() {
    let catalog = load_catalog().unwrap();
    let profiles = catalog.default_profiles();
    let enabled = catalog.routable(&profiles);
    let ids: Vec<_> = enabled.iter().map(|cap| cap.id.as_str()).collect();
    for bad in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        let mut result = judgment("implementation", 0.9, 0.1, &ids);
        result.answers.route.confidence = bad;
        assert!(decide(result, &enabled, &profiles, &catalog).is_err());
    }
    let mut result = judgment("implementation", 0.9, 0.1, &ids);
    result.answers.route.choice = "planning".into();
    assert!(decide(result, &enabled, &profiles, &catalog).is_err());
    let mut result = judgment("implementation", 0.9, 0.1, &ids);
    result.answers.route.probabilities.remove("review");
    assert!(decide(result, &enabled, &profiles, &catalog).is_err());
    let mut result = judgment("implementation", 0.9, 0.1, &ids);
    result.answers.context_missing.noul = f64::NAN;
    assert!(decide(result, &enabled, &profiles, &catalog).is_err());
}

#[test]
fn invalid_input_and_duplicate_ownership_fail_before_network() {
    let catalog = load_catalog().unwrap();
    let mut profiles = catalog.default_profiles();
    profiles
        .get_mut("astra")
        .unwrap()
        .capabilities
        .push("implementation".into());
    let mut input = request(None);
    assert!(
        route_task_for_profiles(&input, &profiles)
            .unwrap_err()
            .to_string()
            .contains("one owner")
    );
    for task in [" ".into(), "x".repeat(MAX_ROUTE_INPUT_BYTES + 1)] {
        input.task = task;
        assert!(route_task(&input).is_err());
    }
}

#[test]
fn labeled_cases_name_catalog_capabilities_and_criteria_are_not_duty_text() {
    let catalog = load_catalog().unwrap();
    let cases: toml::Value =
        toml::from_str(include_str!("../../evaluations/capability-cases.toml")).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let expected = case["expected"].as_str().unwrap();
        assert!(
            expected == "clarify" || catalog.capabilities.iter().any(|cap| cap.id == expected),
            "{expected}"
        );
    }
    assert!(catalog.capabilities.iter().all(|cap| {
        cap.criteria.not_for != cap.instructions && !cap.criteria.examples.is_empty()
    }));
}

#[test]
fn shared_owner_resolves_capability_ambiguity_without_changing_jev_confidence() {
    let catalog = load_catalog().unwrap();
    let mut profiles = catalog.default_profiles();
    let owned_ids: Vec<_> = catalog
        .routable(&profiles)
        .iter()
        .map(|cap| cap.id.clone())
        .collect();
    let ids: Vec<_> = owned_ids.iter().map(String::as_str).collect();
    let make_judgment = || {
        let mut value = judgment("review", 0.03, 0.1, &ids);
        for probability in value.answers.route.probabilities.values_mut() {
            *probability = 0.0;
        }
        value
            .answers
            .route
            .probabilities
            .insert("review".into(), 0.53);
        value
            .answers
            .route
            .probabilities
            .insert("visual".into(), 0.47);
        value
    };
    let result = decide(
        make_judgment(),
        &catalog.routable(&profiles),
        &profiles,
        &catalog,
    )
    .unwrap();
    assert!(matches!(result.reason, DecisionReason::JevOwner));
    assert_eq!(result.confidence, Some(0.03));
    assert_eq!(result.owner_probability, Some(1.0));
    let assigned = result.assignment.unwrap();
    assert_eq!(assigned.owner, "astra");
    assert!(assigned.instructions.contains("Code review"));
    assert!(assigned.instructions.contains("Visual design & review"));
    assert!(!assigned.instructions.contains("Planning & architecture"));
    // The exact same distribution must not dispatch when the user splits ownership.
    profiles
        .get_mut("astra")
        .unwrap()
        .capabilities
        .retain(|cap| cap != "visual");
    profiles
        .get_mut("sol")
        .unwrap()
        .capabilities
        .push("visual".into());
    let result = decide(
        make_judgment(),
        &catalog.routable(&profiles),
        &profiles,
        &catalog,
    )
    .unwrap();
    assert!(matches!(result.reason, DecisionReason::Uncertain));
    assert!(result.assignment.is_none());
    let mut value = make_judgment();
    value.answers.context_missing.noul = 0.9;
    let profiles = catalog.default_profiles();
    let result = decide(value, &catalog.routable(&profiles), &profiles, &catalog).unwrap();
    assert!(matches!(result.reason, DecisionReason::MissingContext));
    assert!(result.assignment.is_none());
}
