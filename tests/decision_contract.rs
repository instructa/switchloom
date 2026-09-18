use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn route(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_switchloom"))
        .arg("route")
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
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn structured_capability_selection_is_offline_and_does_not_echo_context() {
    let result = route(&json!({
        "task": "Implement the agreed fix", "context": "private-context-marker", "routing": {"mode": "assigned", "capability": "implementation"}
    }).to_string());
    assert!(result.status.success(), "{:?}", result);
    let decision: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(decision["assignment"]["model"], "gpt-5.6-sol");
    assert_eq!(decision["reason"], "explicit_capability");
    assert!(decision["usage"].is_null());
    assert!(
        !String::from_utf8(result.stdout)
            .unwrap()
            .contains("private-context-marker")
    );
}

#[test]
fn missing_key_and_invalid_input_fail_without_an_assignment() {
    for input in [
        r#"{"task":"Implement the fix","routing":{"mode":"jev"}}"#,
        r#"{"task":"x","routing":{"mode":"assigned","capability":"unknown"}}"#,
        "{}",
        "not-json",
    ] {
        let result = route(input);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
    let result = route(r#"{"task":"Implement the fix","routing":{"mode":"jev"}}"#);
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("TYPESAFE_API_KEY")
    );
}
