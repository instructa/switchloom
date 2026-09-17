use super::*;
use serde_json::json;
fn judgment(choice: &str, confidence: f64, options: &BTreeMap<&str, &str>) -> Judgment {
    let other = 0.1 / (options.len() - 1) as f64;
    let probabilities: BTreeMap<_, _> = options
        .keys()
        .map(|key| ((*key).to_string(), if *key == choice { 0.9 } else { other }))
        .collect();
    serde_json::from_value(json!({ "model":"jev-test", "answers":{"route":{"type":"choice","choice":choice,"confidence":confidence,"probabilities":probabilities}}, "usage":{"input_tokens":120,"output_tokens":30} })).unwrap()
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
    let input = RouteRequest {
        task: "Implement agreed change".into(),
        context: String::new(),
        capability: Some("implementation".into()),
        jev: true,
    };
    let result = route_task_for_profiles(&input, &profiles).unwrap();
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
    let options = available_criteria(&catalog, &profiles);
    assert!(!options.contains_key("browser"));
    for cap in options.keys().filter(|key| **key != "clarify") {
        let result = decide(judgment(cap, 0.9, &options), &options, &profiles, &catalog).unwrap();
        let actual = result.assignment.unwrap();
        let expected = assignment(cap, &profiles, &catalog).unwrap();
        assert_eq!(actual.owner, expected.owner);
        assert_eq!(actual.model, expected.model);
        assert_eq!(actual.instructions, expected.instructions);
    }
}
#[test]
fn abstention_and_vanilla_never_guess_an_owner() {
    let catalog = load_catalog().unwrap();
    let profiles = catalog.default_profiles();
    let options = available_criteria(&catalog, &profiles);
    for (cap, confidence) in [("planning", 0.3), ("clarify", 0.9)] {
        assert!(
            decide(
                judgment(cap, confidence, &options),
                &options,
                &profiles,
                &catalog
            )
            .unwrap()
            .assignment
            .is_none()
        );
    }
    let result = route_task(&RouteRequest {
        task: "Do the next step".into(),
        context: String::new(),
        capability: None,
        jev: false,
    })
    .unwrap();
    assert!(matches!(
        result.reason,
        DecisionReason::ExplicitCapabilityRequired
    ));
    assert!(result.judge_model.is_none());
}
#[test]
fn invalid_distribution_cannot_trigger_assignment() {
    let catalog = load_catalog().unwrap();
    let profiles = catalog.default_profiles();
    let options = available_criteria(&catalog, &profiles);
    for bad in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        let mut result = judgment("implementation", 0.9, &options);
        result.answers.route.confidence = bad;
        assert!(decide(result, &options, &profiles, &catalog).is_err());
    }
    let mut result = judgment("implementation", 0.9, &options);
    result.answers.route.choice = "planning".into();
    assert!(decide(result, &options, &profiles, &catalog).is_err());
    let mut result = judgment("implementation", 0.9, &options);
    result.answers.route.probabilities.remove("review");
    assert!(decide(result, &options, &profiles, &catalog).is_err());
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
    let mut input = RouteRequest {
        task: "Implement change".into(),
        context: String::new(),
        capability: None,
        jev: true,
    };
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
