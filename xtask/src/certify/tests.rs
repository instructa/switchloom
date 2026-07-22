use super::{OpencodeInput, PiInput, validate_codex, validate_opencode, validate_pi};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);
impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "switchloom-{name}-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn codex_receipt() -> Value {
    let parent = "11111111-1111-4111-8111-111111111111";
    let maker = "22222222-2222-4222-8222-222222222222";
    let reviewer = "33333333-3333-4333-8333-333333333333";
    let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    json!({
        "schema_version": "switchloom.codex_runtime_evidence.v1",
        "run": { "status": "complete", "complete_marker": "SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE", "evidence_source": "codex_persisted_spawn_state", "parent_thread_id": parent, "parent_session": format!("{parent}.jsonl"), "workdir": "/tmp/work" },
        "children": [{
            "kind": "maker", "profile": "codex-terra-high", "agent_type": "model_routing_terra_high", "task_name": "maker", "canonical_task": "/root/maker", "parent_thread_id": parent, "child_thread_id": maker,
            "spawn": { "surface": "collaboration.spawn_agent", "agent_type": "model_routing_terra_high", "task_name": "maker", "fork_turns": "none", "call_id": "call-maker" },
            "input": { "message_sha256": hash, "message_bytes": 12, "max_message_bytes": 512, "message_encoding": "plaintext", "message_plaintext_verdict": "deterministic" },
            "spawn_output": { "task_name": "/root/maker" },
            "session": { "agent_role": "model_routing_terra_high", "agent_path": "/root/maker", "thread_source": "subagent", "parent_thread_id": parent, "session_file": format!("{maker}.jsonl") },
            "state": { "agent_role": "model_routing_terra_high", "agent_path": "/root/maker", "model": "gpt-5.6-terra", "reasoning_effort": "high", "thread_source": "subagent", "cwd": "/tmp/work" },
            "final_answer": { "message_type": "FINAL_ANSWER" }
        }, {
            "kind": "reviewer", "profile": "codex-sol-high", "agent_type": "model_routing_sol_high", "task_name": "reviewer", "canonical_task": "/root/reviewer", "parent_thread_id": parent, "child_thread_id": reviewer,
            "spawn": { "surface": "collaboration.spawn_agent", "agent_type": "model_routing_sol_high", "task_name": "reviewer", "fork_turns": "none", "call_id": "call-reviewer" },
            "input": { "message_sha256": hash, "message_bytes": 12, "max_message_bytes": 512, "message_encoding": "plaintext", "message_plaintext_verdict": "deterministic" },
            "spawn_output": { "task_name": "/root/reviewer" },
            "session": { "agent_role": "model_routing_sol_high", "agent_path": "/root/reviewer", "thread_source": "subagent", "parent_thread_id": parent, "session_file": format!("{reviewer}.jsonl") },
            "state": { "agent_role": "model_routing_sol_high", "agent_path": "/root/reviewer", "model": "gpt-5.6-sol", "reasoning_effort": "high", "thread_source": "subagent", "cwd": "/tmp/work" },
            "final_answer": { "message_type": "FINAL_ANSWER" }
        }],
        "dispatch_evidence": [{
            "schema_version": 1, "package_digest": format!("sha256:{hash}"), "host_version": "codex-cli 0.145.0",
            "requested_dispatch": { "semantic_role": "maker", "profile": "codex-terra-high", "model": "gpt-5.6-terra", "effort": "high", "agent_type": "model_routing_terra_high", "fork_turns": { "mode": "none" }, "message_sha256": hash, "message_encoding": "plaintext", "message_plaintext_verdict": "deterministic", "message_bytes": 12, "max_message_bytes": 512 },
            "child_identity": { "host": "codex", "role": "maker", "agent_role": "model_routing_terra_high", "agent_type": "model_routing_terra_high", "task_name": "maker" },
            "effective_model": "gpt-5.6-terra", "effective_effort": "high", "nonce": format!("{parent}:{maker}:call-maker"),
            "raw_evidence_refs": [format!("codex-session:{parent}.jsonl"), format!("codex-session:{maker}.jsonl"), format!("state_5.sqlite:thread_spawn_edges:{parent}:{maker}"), "spawn_call:call-maker"], "verdict": "deterministic"
        }, {
            "schema_version": 1, "package_digest": format!("sha256:{hash}"), "host_version": "codex-cli 0.145.0",
            "requested_dispatch": { "semantic_role": "reviewer", "profile": "codex-sol-high", "model": "gpt-5.6-sol", "effort": "high", "agent_type": "model_routing_sol_high", "fork_turns": { "mode": "none" }, "message_sha256": hash, "message_encoding": "plaintext", "message_plaintext_verdict": "deterministic", "message_bytes": 12, "max_message_bytes": 512 },
            "child_identity": { "host": "codex", "role": "reviewer", "agent_role": "model_routing_sol_high", "agent_type": "model_routing_sol_high", "task_name": "reviewer" },
            "effective_model": "gpt-5.6-sol", "effective_effort": "high", "nonce": format!("{parent}:{reviewer}:call-reviewer"),
            "raw_evidence_refs": [format!("codex-session:{parent}.jsonl"), format!("codex-session:{reviewer}.jsonl"), format!("state_5.sqlite:thread_spawn_edges:{parent}:{reviewer}"), "spawn_call:call-reviewer"], "verdict": "deterministic"
        }]
    })
}

fn validate_codex_value(value: &Value) -> anyhow::Result<()> {
    let dir = TempDir::new("codex");
    let path = dir.write("receipt.json", serde_json::to_vec(value).unwrap());
    validate_codex(&path, None)
}

#[test]
fn codex_accepts_correlated_typed_receipt() {
    validate_codex_value(&codex_receipt()).unwrap();
}

#[test]
fn codex_fails_closed_on_prose_missing_inherited_uncorrelated_and_tampered_evidence() {
    let dir = TempDir::new("codex-prose");
    let prose = dir.write("receipt.json", "worker used Terra High");
    assert!(validate_codex(&prose, None).is_err());
    for mutate in [
        |v: &mut Value| {
            v["run"].as_object_mut().unwrap().remove("complete_marker");
        },
        |v: &mut Value| {
            v["schema_version"] = json!("switchloom.codex_runtime_evidence.v0");
        },
        |v: &mut Value| {
            v["dispatch_evidence"][0]["host_version"] = json!("codex-cli 0.144.5");
        },
        |v: &mut Value| {
            v["children"][0]["spawn"]["surface"] = json!("nested_exec");
        },
        |v: &mut Value| {
            v["children"][0]["spawn"]["fork_turns"] = json!("all");
        },
        |v: &mut Value| {
            v["children"][0].as_object_mut().unwrap().remove("input");
        },
        |v: &mut Value| {
            v["children"][0]["session"]["parent_thread_id"] =
                json!("33333333-3333-4333-8333-333333333333");
        },
        |v: &mut Value| {
            v["children"][0]["state"]["model"] = json!("gpt-5.6-sol");
            v["children"][0]["state"]["reasoning_effort"] = json!("medium");
        },
        |v: &mut Value| {
            v["dispatch_evidence"][0]["nonce"] = json!("nonce-placeholder");
        },
        |v: &mut Value| {
            v["children"][0]["profile"] = json!("codex-luna-xhigh-experimental");
            v["children"][0]["agent_type"] = json!("model_routing_luna_xhigh");
            v["children"][0]["spawn"]["agent_type"] = json!("model_routing_luna_xhigh");
            v["children"][0]["session"]["agent_role"] = json!("model_routing_luna_xhigh");
            v["children"][0]["state"]["agent_role"] = json!("model_routing_luna_xhigh");
            v["children"][0]["state"]["model"] = json!("gpt-5.6-luna");
            v["children"][0]["state"]["reasoning_effort"] = json!("xhigh");
            v["dispatch_evidence"][0]["requested_dispatch"]["profile"] =
                json!("codex-luna-xhigh-experimental");
            v["dispatch_evidence"][0]["requested_dispatch"]["model"] = json!("gpt-5.6-luna");
            v["dispatch_evidence"][0]["requested_dispatch"]["effort"] = json!("xhigh");
            v["dispatch_evidence"][0]["requested_dispatch"]["agent_type"] =
                json!("model_routing_luna_xhigh");
            v["dispatch_evidence"][0]["child_identity"]["agent_role"] =
                json!("model_routing_luna_xhigh");
            v["dispatch_evidence"][0]["child_identity"]["agent_type"] =
                json!("model_routing_luna_xhigh");
            v["dispatch_evidence"][0]["effective_model"] = json!("gpt-5.6-luna");
            v["dispatch_evidence"][0]["effective_effort"] = json!("xhigh");
        },
    ] {
        let mut receipt = codex_receipt();
        mutate(&mut receipt);
        assert!(validate_codex_value(&receipt).is_err());
    }
}

fn opencode_input(dir: &TempDir, events: &[Value]) -> OpencodeInput {
    OpencodeInput {
        jsonl: dir.write(
            "host.jsonl",
            events
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        invocation: dir.write("invocation.json", r#"{"nonce":"nonce-123"}"#),
        receipt: dir.0.join("receipt.json"),
        package_digest: "sha256:abc".into(),
        host_version: "1.14.17".into(),
        profile: "opencode-worker".into(),
        model: "opencode/gpt-5-nano".into(),
        variant: "low".into(),
        worker: "model-routing-preset-worker".into(),
    }
}

#[test]
fn opencode_accepts_only_correlated_structured_task_results() {
    let dir = TempDir::new("opencode-valid");
    validate_opencode(opencode_input(&dir, &[json!({"type":"tool_call","tool":"Task","id":"call-1","agent":"model-routing-preset-worker","model":"opencode/gpt-5-nano","variant":"low"}), json!({"type":"tool_result","toolCallID":"call-1","agent":"model-routing-preset-worker","result":"nonce-123"})])).unwrap();
    let dir = TempDir::new("opencode-prose");
    assert!(validate_opencode(opencode_input(&dir, &[json!({"type":"message","agent":"driver","text":"model-routing-preset-worker returned nonce-123"})])).is_err());
    let dir = TempDir::new("opencode-tamper");
    assert!(validate_opencode(opencode_input(&dir, &[json!({"type":"tool_call","tool":"Task","id":"call-1","agent":"model-routing-preset-worker"}), json!({"type":"tool_result","toolCallID":"call-1","agent":"other-worker","result":"nonce-123"})])).is_err());
}

fn pi_fixture(dir: &TempDir) -> PiInput {
    let argv = [
        "pi",
        "--print",
        "--no-session",
        "--no-tools",
        "--no-extensions",
        "--no-skills",
        "--provider",
        "openai",
        "--model",
        "gpt-4o-mini",
        "--thinking",
        "low",
    ];
    let nonce = "nonce-123";
    let prompt = format!(
        "sha256:{:x}",
        Sha256::digest(format!("Return only this nonce and no other text: {nonce}"))
    );
    let workflow = json!({"schema_version":1,"workflow":"model-routing-preset-runner","runner":"pi","runtime_class":"external-runner","arguments":{"agent_type":"switchloom-pi-worker","provider_model":"openai/gpt-4o-mini","thinking":"low","isolation":{"session":"none","tools":"none","extensions":"none","skills":"none","agent_dir":"report-workdir/.pi-agent"},"task":{"semantic_role":"worker","returns":"nonce-only"}},"process":{"argv":argv}});
    let invocation_argv = ["env", "PI_CODING_AGENT_DIR=.pi-agent", "PI_OFFLINE=1"]
        .into_iter()
        .chain(argv)
        .collect::<Vec<_>>();
    let invocation = json!({"nonce":nonce,"argv":invocation_argv,"env":{"PI_CODING_AGENT_DIR":".pi-agent","PI_OFFLINE":"1"},"prompt_sha256":prompt});
    PiInput {
        workflow: dir.write("workflow.json", serde_json::to_vec(&workflow).unwrap()),
        invocation: dir.write("invocation.json", serde_json::to_vec(&invocation).unwrap()),
        stdout: dir.write("stdout", nonce),
        stderr: dir.write("stderr", ""),
        workflow_receipt: dir.0.join("workflow-receipt.json"),
        dispatch_receipt: dir.0.join("dispatch.json"),
        package_digest: format!("sha256:{}", "b".repeat(64)),
        host_version: "0.66.1".into(),
        profile: "pi-worker".into(),
        model: "openai/gpt-4o-mini".into(),
        thinking: "low".into(),
        agent_type: "switchloom-pi-worker".into(),
    }
}

#[test]
fn pi_accepts_nonce_only_and_rejects_prose_or_tampered_workflow() {
    let dir = TempDir::new("pi-valid");
    validate_pi(pi_fixture(&dir)).unwrap();
    let dir = TempDir::new("pi-prose");
    let mut input = pi_fixture(&dir);
    input.stdout = dir.write("stdout-bad", "nonce-123 plus prose");
    assert!(validate_pi(input).is_err());
    let dir = TempDir::new("pi-tamper");
    let input = pi_fixture(&dir);
    let mut workflow: Value =
        serde_json::from_str(&fs::read_to_string(&input.workflow).unwrap()).unwrap();
    workflow["arguments"]["isolation"]["tools"] = json!("default");
    fs::write(&input.workflow, serde_json::to_vec(&workflow).unwrap()).unwrap();
    assert!(validate_pi(input).is_err());
}
