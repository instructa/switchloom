use super::*;
use crate::decision::DecisionReason;
use serde_json::json;

fn request(capability: &str) -> HandoffRequest {
    serde_json::from_value(json!({
        "request": {"task": "Resolve the race before implementation", "context": "Relevant code: src/store.rs", "routing": {"mode": "assigned", "capability": capability}},
        "caller": {"thread_id": "controller", "host_id": "local"},
        "threads": {
            "sol": {"thread_id": "worker-task", "host_id": "local", "model": "gpt-5.6-sol", "effort": "medium", "capabilities": ["implementation"]},
            "astra": {"thread_id": "advisor-task", "host_id": "local", "model": "gpt-6-astra", "effort": "high", "capabilities": ["planning"]},
            "luna": {"thread_id": "coordinator-task", "host_id": "local", "model": "gpt-5.6-luna", "effort": "max", "capabilities": ["coordination", "mechanical"]}
        }
    })).unwrap()
}

#[test]
fn handoff_binds_the_selected_capability_to_an_existing_task_and_complete_message() {
    for (capability, owner) in [
        ("implementation", "sol"),
        ("planning", "astra"),
        ("mechanical", "luna"),
    ] {
        let input = request(capability);
        let result = prepare_handoff(&input).unwrap();
        assert_eq!(result.action, HandoffAction::Dispatch);
        let assignment = result.decision.assignment.unwrap();
        let dispatch = result.dispatch.unwrap();
        assert_eq!(dispatch.thread_id, input.threads[owner].thread_id);
        assert_eq!(dispatch.host_id, "local");
        assert_eq!(dispatch.model, assignment.model);
        assert_eq!(dispatch.thinking, assignment.effort);
        assert!(dispatch.prompt.contains(&assignment.instructions));
        assert!(dispatch.prompt.contains(&input.request.task));
        assert!(dispatch.prompt.contains(&input.request.context));
        let return_target = dispatch
            .prompt
            .lines()
            .find_map(|line| line.strip_prefix("Return target: "))
            .unwrap();
        let return_target: serde_json::Value = serde_json::from_str(return_target).unwrap();
        assert_eq!(
            return_target,
            json!({"threadId": "controller", "hostId": "local"})
        );
        assert!(dispatch.prompt.contains("Result for:"));
        assert!(dispatch.prompt.contains(&format!(
            "identify your thread as {} on host local",
            input.threads[owner].thread_id
        )));
        assert!(result.decision.usage.is_none());
        // This is the actual Codex send_message_to_thread argument shape.
        let payload = serde_json::to_value(dispatch).unwrap();
        assert_eq!(payload["threadId"], input.threads[owner].thread_id);
        assert_eq!(payload["hostId"], "local");
        assert_eq!(payload["thinking"], assignment.effort);
    }
}

#[test]
fn return_address_preserves_the_callers_host_and_escapes_ids() {
    let mut input = request("planning");
    input.caller.thread_id = "caller\"id".into();
    input.caller.host_id = "remote-host".into();
    let prompt = prepare_handoff(&input).unwrap().dispatch.unwrap().prompt;
    let target = prompt
        .lines()
        .find_map(|line| line.strip_prefix("Return target: "))
        .unwrap();
    let target: serde_json::Value = serde_json::from_str(target).unwrap();
    assert_eq!(
        target,
        json!({"threadId": input.caller.thread_id, "hostId": "remote-host"})
    );
}

#[test]
fn abstention_has_no_dispatch_payload() {
    let decision = RouteDecision {
        assignment: None,
        reason: DecisionReason::Uncertain,
        confidence: Some(0.4),
        probabilities: BTreeMap::new(),
        judge_model: Some("jev-test".into()),
        usage: None,
        act_threshold: None,
        context_missing: None,
        owner_probability: None,
    };
    let result = bind_decision(&request("implementation"), decision).unwrap();
    assert!(result.dispatch.is_none());
    assert_eq!(result.action, HandoffAction::Clarify);
    assert!(matches!(result.decision.reason, DecisionReason::Uncertain));
}

#[test]
fn caller_continues_locally_and_missing_owners_or_duplicate_targets_fail() {
    let mut input = request("implementation");
    input.caller.thread_id = input.threads["sol"].thread_id.clone();
    let local = prepare_handoff(&input).unwrap();
    assert_eq!(local.action, HandoffAction::ContinueHere);
    assert!(local.dispatch.is_none());
    assert_eq!(
        local.decision.assignment.unwrap().capability,
        "implementation"
    );

    let mut input = request("planning");
    input.threads.remove("astra");
    assert!(prepare_handoff(&input).is_err());

    let mut input = request("implementation");
    input
        .threads
        .insert("astra".into(), input.threads["sol"].clone());
    assert!(prepare_handoff(&input).is_err());
    input.threads.clear();
    assert!(prepare_handoff(&input).is_err());
}

#[test]
fn invalid_target_ids_fail_before_jev_is_called() {
    for id in ["", " ", "task\nother", &"x".repeat(257)] {
        let mut input = request("implementation");
        input.request.routing = crate::decision::Routing::Jev;
        input.caller.thread_id = id.into();
        let error = prepare_handoff(&input).unwrap_err().to_string();
        assert!(error.contains("IDs"));
    }
}
