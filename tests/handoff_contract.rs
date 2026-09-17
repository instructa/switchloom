use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn handoff(input: &Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_switchloom"))
        .arg("handoff")
        .env_remove("TYPESAFE_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn input() -> Value {
    json!({
        "request": {"task": "Review the race in src/store.rs", "context": "Do not edit files", "capability": "review", "jev": true},
        "caller": {"thread_id": "existing-worker", "host_id": "local"},
        "threads": {"astra": {"thread_id": "existing-advisor", "host_id": "local", "model": "gpt-6-astra", "effort": "high", "capabilities": ["review"]}}
    })
}

#[test]
fn cli_prepares_a_complete_codex_desktop_dispatch_without_calling_a_model() {
    let result = handoff(&input());
    assert!(result.status.success(), "{:?}", result);
    let output: Value = serde_json::from_slice(&result.stdout).unwrap();
    let dispatch = &output["dispatch"];
    assert_eq!(output["action"], "dispatch");
    assert_eq!(dispatch["threadId"], "existing-advisor");
    assert_eq!(dispatch["hostId"], "local");
    assert_eq!(dispatch["model"], "gpt-6-astra");
    assert_eq!(dispatch["thinking"], "high");
    assert!(
        dispatch["prompt"]
            .as_str()
            .unwrap()
            .contains("Review the race in src/store.rs")
    );
    assert!(
        dispatch["prompt"]
            .as_str()
            .unwrap()
            .contains("Do not edit files")
    );
    assert_eq!(output["decision"]["reason"], "explicit_capability");
    let prompt = dispatch["prompt"].as_str().unwrap();
    let return_target = prompt
        .lines()
        .find_map(|line| line.strip_prefix("Return target: "))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(return_target).unwrap(),
        json!({"threadId": "existing-worker", "hostId": "local"})
    );
    assert!(output["decision"]["usage"].is_null());
}

#[test]
fn failures_leave_no_dispatch_on_stdout() {
    let mut missing_key = input();
    missing_key["request"]
        .as_object_mut()
        .unwrap()
        .remove("capability");
    let mut missing_owner = input();
    missing_owner["request"]["capability"] = json!("implementation");
    for input in [missing_key, missing_owner] {
        let result = handoff(&input);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn selected_caller_keeps_work_in_its_current_turn() {
    let mut input = input();
    input["caller"]["thread_id"] = json!("existing-advisor");
    let result = handoff(&input);
    assert!(result.status.success());
    let output: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(output["action"], "continue_here");
    assert_eq!(output["decision"]["assignment"]["capability"], "review");
    assert!(output["dispatch"].is_null());
}

#[test]
fn handoffs_keep_selected_models_including_ids_added_after_this_release() {
    for (model, effort) in [("gpt-5.6-sol", "xhigh"), ("gpt-next-advisor", "high")] {
        let mut input = input();
        input["threads"]["astra"]["model"] = json!(model);
        input["threads"]["astra"]["effort"] = json!(effort);
        let result = handoff(&input);
        assert!(result.status.success(), "{:?}", result);
        let output: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(output["decision"]["assignment"]["model"], model);
        assert_eq!(output["decision"]["assignment"]["effort"], effort);
        assert_eq!(output["dispatch"]["model"], model);
        assert_eq!(output["dispatch"]["thinking"], effort);
    }
}

#[test]
fn invalid_model_settings_fail_before_routing_or_dispatch() {
    for (model, effort) in [
        ("", "medium"),
        ("gpt bad", "medium"),
        ("claude-test", "high"),
        ("gpt-5.6-luna", "ultra"),
        ("gpt-next-test", "bogus"),
    ] {
        let mut input = input();
        input["request"]
            .as_object_mut()
            .unwrap()
            .remove("capability");
        input["threads"]["astra"]["model"] = json!(model);
        input["threads"]["astra"]["effort"] = json!(effort);
        let result = handoff(&input);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("TYPESAFE_API_KEY"));
    }
}
