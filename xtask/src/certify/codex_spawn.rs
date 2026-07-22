use super::{codex, ensure};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) struct CodexRawInput {
    pub events: PathBuf,
    pub workdir: PathBuf,
    pub expected: PathBuf,
    pub state_db: Option<PathBuf>,
    pub sessions_dir: Option<PathBuf>,
    pub archived_sessions_dir: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Expected {
    package_digest: String,
    host_version: String,
    children: Vec<ExpectedChild>,
}

#[derive(Deserialize)]
struct ExpectedChild {
    semantic_role: String,
    profile: String,
    agent_type: String,
    task_name: String,
    canonical_task: String,
    model: String,
    effort: String,
    #[serde(default)]
    message_sha256: Option<String>,
    #[serde(default)]
    message_ciphertext_sha256: Option<String>,
    max_message_bytes: u64,
    #[serde(default)]
    completion_contains: Option<String>,
    #[serde(default)]
    allow_encrypted_message: bool,
}

#[derive(Deserialize)]
struct Edge {
    parent_thread_id: String,
    child_thread_id: String,
    status: String,
    agent_path: Option<String>,
    agent_role: Option<String>,
    model: String,
    reasoning_effort: String,
    thread_source: String,
    cwd: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpawnArgs {
    agent_type: String,
    task_name: String,
    fork_turns: String,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct SpawnOutput {
    task_name: String,
    #[serde(default)]
    agent_id: Option<String>,
}

pub(crate) fn extract(input: CodexRawInput) -> Result<String> {
    let canonical_workdir =
        fs::canonicalize(&input.workdir).context("failed to canonicalize Codex oracle workdir")?;
    let expected: Expected = read_json(&input.expected, "expected Codex dispatch")?;
    ensure(
        !expected.children.is_empty(),
        "expected children list is empty",
    )?;
    ensure(
        is_sha256_digest(&expected.package_digest),
        "expected package_digest must be sha256:<64 lowercase hex>",
    )?;
    ensure(
        super::codex::is_codex_version(&expected.host_version),
        "expected host_version must be observed codex --version output",
    )?;

    let outer = read_jsonl(&input.events)?;
    let parent_thread_id = outer
        .iter()
        .find(|record| record["type"] == "thread.started")
        .and_then(|record| record["thread_id"].as_str())
        .context("Codex exec JSONL did not contain thread.started.thread_id")?
        .to_owned();
    ensure(
        super::codex::is_uuid(&parent_thread_id),
        "invalid parent thread id",
    )?;

    let sessions_dir = input
        .sessions_dir
        .unwrap_or_else(|| default_codex_path("sessions"));
    let archived_dir = input
        .archived_sessions_dir
        .unwrap_or_else(|| default_codex_path("archived_sessions"));
    let parent_session = find_session(&parent_thread_id, [&sessions_dir, &archived_dir])?;
    let parent_records = read_jsonl(&parent_session)?;
    let parent_meta =
        find_payload(&parent_records, "session_meta").context("parent session_meta missing")?;
    ensure(
        parent_meta["id"] == parent_thread_id,
        "parent session_meta id does not match thread.started",
    )?;
    ensure(
        parent_meta["thread_source"] == "user",
        "parent session is not a user thread",
    )?;
    ensure(
        persisted_cwd_matches(parent_meta["cwd"].as_str(), &canonical_workdir),
        "parent session cwd does not match oracle workdir",
    )?;

    let state_db = input
        .state_db
        .unwrap_or_else(|| default_codex_path("state_5.sqlite"));
    let edges = query_edges(&state_db, &parent_thread_id)?;
    validate_expected(&expected.children)?;
    let spawn_calls = parent_records
        .iter()
        .filter(|record| is_spawn_call(record))
        .collect::<Vec<_>>();
    ensure(
        spawn_calls.len() == expected.children.len(),
        format_args!(
            "parent must contain exactly {} V2 spawn_agent calls",
            expected.children.len()
        ),
    )?;
    let started = parent_records
        .iter()
        .filter(|record| {
            record["type"] == "event_msg"
                && record["payload"]["type"] == "sub_agent_activity"
                && record["payload"]["kind"] == "started"
        })
        .collect::<Vec<_>>();
    ensure(
        started.len() == expected.children.len(),
        format_args!(
            "parent must contain exactly {} sub_agent_activity started events",
            expected.children.len()
        ),
    )?;
    ensure(
        edges.len() == expected.children.len(),
        format_args!(
            "parent must contain exactly {} persisted child edges",
            expected.children.len()
        ),
    )?;

    for record in &spawn_calls {
        let args: SpawnArgs =
            parse_embedded(&record["payload"]["arguments"], "spawn_agent arguments")?;
        ensure(
            expected.children.iter().any(|child| {
                child.agent_type == args.agent_type && child.task_name == args.task_name
            }),
            "unexpected spawn_agent call",
        )?;
    }
    for record in &started {
        let path = record["payload"]["agent_path"].as_str();
        ensure(
            expected
                .children
                .iter()
                .any(|child| Some(child.canonical_task.as_str()) == path),
            "unexpected started child path",
        )?;
    }

    let mut observed = Vec::new();
    for child in &expected.children {
        let matches = spawn_calls
            .iter()
            .filter_map(|record| {
                let args: SpawnArgs =
                    parse_embedded(&record["payload"]["arguments"], "spawn_agent arguments")
                        .ok()?;
                (args.agent_type == child.agent_type && args.task_name == child.task_name)
                    .then_some((*record, args))
            })
            .collect::<Vec<_>>();
        ensure(
            matches.len() == 1,
            format_args!(
                "{} must have exactly one raw spawn_agent call",
                child.agent_type
            ),
        )?;
        let (spawn_record, spawn_args) = &matches[0];
        ensure(
            spawn_args.fork_turns == "none",
            format_args!("{} spawn did not use fork_turns=none", child.agent_type),
        )?;
        let message = spawn_args.message.as_deref().unwrap_or_default();
        ensure(
            !message.is_empty(),
            format_args!("{} spawn message missing", child.agent_type),
        )?;
        let message_bytes = message.len() as u64;
        ensure(
            message_bytes <= child.max_message_bytes,
            format_args!(
                "{} spawn message exceeds max_message_bytes",
                child.agent_type
            ),
        )?;
        let message_hash = sha256_hex(message.as_bytes());
        let (encoding, plaintext_verdict, intent_hash) =
            if child.message_sha256.as_deref() == Some(&message_hash) {
                ("plaintext", "deterministic", None)
            } else {
                ensure(
                    is_codex_ciphertext(message),
                    format_args!("{} spawn message_sha256 mismatch", child.agent_type),
                )?;
                ensure(
                    child.allow_encrypted_message || child.message_ciphertext_sha256.is_some(),
                    format_args!(
                        "{} encrypted spawn message cannot certify expected plaintext",
                        child.agent_type
                    ),
                )?;
                if let Some(ciphertext_hash) = &child.message_ciphertext_sha256 {
                    ensure(
                        *ciphertext_hash == message_hash,
                        format_args!(
                            "{} encrypted spawn message_ciphertext_sha256 mismatch",
                            child.agent_type
                        ),
                    )?;
                }
                (
                    "codex-encrypted",
                    "unsupported",
                    child.message_sha256.clone(),
                )
            };
        let call_id = spawn_record["payload"]["call_id"]
            .as_str()
            .context("spawn call_id missing")?;
        let output: SpawnOutput = parent_records
            .iter()
            .find(|record| {
                record["type"] == "response_item"
                    && record["payload"]["type"] == "function_call_output"
                    && record["payload"]["call_id"] == call_id
            })
            .map(|record| parse_embedded(&record["payload"]["output"], "spawn_agent output"))
            .transpose()?
            .context("spawn_agent output missing")?;
        ensure(
            output.task_name == child.canonical_task,
            format_args!("{} spawn output task_name mismatch", child.agent_type),
        )?;
        let activity = started
            .iter()
            .find(|record| record["payload"]["event_id"] == call_id)
            .context("missing sub_agent_activity started event")?;
        let child_thread_id = activity["payload"]["agent_thread_id"]
            .as_str()
            .context("started event missing child thread id")?;
        ensure(
            super::codex::is_uuid(child_thread_id),
            "started event missing child thread id",
        )?;
        ensure(
            activity["payload"]["agent_path"] == child.canonical_task,
            "started event agent_path mismatch",
        )?;
        ensure(
            has_final_answer(&parent_records, child, child_thread_id),
            format_args!(
                "{} missing child FINAL_ANSWER payload in parent session",
                child.agent_type
            ),
        )?;
        let edge = edges
            .iter()
            .find(|edge| edge.child_thread_id == child_thread_id)
            .context("missing thread_spawn_edges row")?;
        validate_edge(edge, child, &parent_thread_id, &canonical_workdir)?;
        let child_session = find_session(child_thread_id, [&sessions_dir, &archived_dir])?;
        let child_records = read_jsonl(&child_session)?;
        let meta =
            find_payload(&child_records, "session_meta").context("child session_meta missing")?;
        validate_child_meta(meta, child, &parent_thread_id, child_thread_id)?;
        let context =
            find_payload(&child_records, "turn_context").context("child turn_context missing")?;
        ensure(
            context["model"] == child.model && context["effort"] == child.effort,
            "child turn_context model/effort mismatch",
        )?;
        ensure(
            context["collaboration_mode"]["settings"]["model"] == child.model
                && context["collaboration_mode"]["settings"]["reasoning_effort"] == child.effort,
            "child collaboration model/effort mismatch",
        )?;

        observed.push(json!({
            "kind": child.semantic_role, "profile": child.profile, "agent_type": child.agent_type, "task_name": child.task_name,
            "canonical_task": child.canonical_task, "parent_thread_id": parent_thread_id, "child_thread_id": child_thread_id,
            "spawn": {"surface":"collaboration.spawn_agent","agent_type":spawn_args.agent_type,"task_name":spawn_args.task_name,"fork_turns":spawn_args.fork_turns,"call_id":call_id},
            "input": {"message_sha256":message_hash,"message_bytes":message_bytes,"max_message_bytes":child.max_message_bytes,"message_encoding":encoding,"message_plaintext_verdict":plaintext_verdict,"message_plaintext_intent_sha256":intent_hash},
            "spawn_output": {"task_name":output.task_name,"agent_id":output.agent_id},
            "session": {"agent_role":meta["agent_role"],"agent_path":meta.get("agent_path").cloned().unwrap_or_else(|| json!(child.canonical_task)),"thread_source":meta["thread_source"],"parent_thread_id":meta["parent_thread_id"],"session_file":file_name(&child_session)},
            "state": {"agent_role":edge.agent_role,"agent_path":edge.agent_path,"model":edge.model,"reasoning_effort":edge.reasoning_effort,"thread_source":edge.thread_source,"cwd":&canonical_workdir},
            "final_answer":{"message_type":"FINAL_ANSWER"}
        }));
    }

    let dispatch = observed.iter().map(|child| json!({
        "schema_version":1,"package_digest":expected.package_digest,"host_version":expected.host_version,
        "requested_dispatch":{"semantic_role":child["kind"],"profile":child["profile"],"model":child["state"]["model"],"effort":child["state"]["reasoning_effort"],"agent_type":child["agent_type"],"fork_turns":{"mode":child["spawn"]["fork_turns"]},"message_sha256":child["input"]["message_sha256"],"message_encoding":child["input"]["message_encoding"],"message_plaintext_verdict":child["input"]["message_plaintext_verdict"],"message_plaintext_intent_sha256":child["input"]["message_plaintext_intent_sha256"],"message_bytes":child["input"]["message_bytes"],"max_message_bytes":child["input"]["max_message_bytes"]},
        "child_identity":{"host":"codex","role":child["kind"],"agent_role":child["state"]["agent_role"],"agent_type":child["agent_type"],"task_name":child["task_name"]},
        "effective_model":child["state"]["model"],"effective_effort":child["state"]["reasoning_effort"],
        "nonce":format!("{}:{}:{}", parent_thread_id, child["child_thread_id"].as_str().unwrap(), child["spawn"]["call_id"].as_str().unwrap()),
        "raw_evidence_refs":[format!("codex-session:{}",file_name(&parent_session)),format!("codex-session:{}",child["session"]["session_file"].as_str().unwrap()),format!("state_5.sqlite:thread_spawn_edges:{}:{}",parent_thread_id,child["child_thread_id"].as_str().unwrap()),format!("spawn_call:{}",child["spawn"]["call_id"].as_str().unwrap())],"verdict":"deterministic"
    })).collect::<Vec<_>>();
    let receipt = json!({"schema_version":"switchloom.codex_runtime_evidence.v1","run":{"status":"complete","complete_marker":"SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE","evidence_source":"codex_persisted_spawn_state","parent_thread_id":parent_thread_id,"parent_session":file_name(&parent_session),"workdir":canonical_workdir},"children":observed,"dispatch_evidence":dispatch});
    let text = serde_json::to_string_pretty(&receipt)?;
    codex::validate_json(&text, Some(&input.expected))?;
    Ok(text)
}

fn validate_expected(children: &[ExpectedChild]) -> Result<()> {
    for child in children {
        ensure(
            !child.semantic_role.is_empty()
                && !child.profile.is_empty()
                && !child.agent_type.is_empty()
                && !child.task_name.is_empty(),
            "expected child identity fields must not be blank",
        )?;
        ensure(
            child.canonical_task == format!("/root/{}", child.task_name),
            "expected child has invalid canonical_task",
        )?;
        ensure(
            child.max_message_bytes > 0,
            "expected max_message_bytes must be positive",
        )?;
        if let Some(hash) = &child.message_sha256 {
            ensure(
                super::codex::is_sha256_hex(hash),
                "expected message_sha256 must be lowercase sha256 hex",
            )?;
        }
        if let Some(hash) = &child.message_ciphertext_sha256 {
            ensure(
                super::codex::is_sha256_hex(hash),
                "expected message_ciphertext_sha256 must be lowercase sha256 hex",
            )?;
        }
    }
    Ok(())
}

fn validate_edge(edge: &Edge, child: &ExpectedChild, parent: &str, workdir: &Path) -> Result<()> {
    ensure(
        edge.parent_thread_id == parent && !edge.status.is_empty() && edge.status != "unknown",
        "persisted child edge mismatch",
    )?;
    ensure(
        edge.agent_path
            .as_deref()
            .is_none_or(|path| path == child.canonical_task),
        "state agent_path mismatch",
    )?;
    ensure(
        edge.agent_role.as_deref() == Some(&child.agent_type)
            && edge.model == child.model
            && edge.reasoning_effort == child.effort,
        "state identity/model/effort mismatch",
    )?;
    ensure(
        edge.thread_source == "subagent" && persisted_cwd_matches(Some(&edge.cwd), workdir),
        "state source/cwd mismatch",
    )
}

fn persisted_cwd_matches(persisted: Option<&str>, expected: &Path) -> bool {
    let Some(persisted) = persisted else {
        return false;
    };
    match (fs::canonicalize(persisted), fs::canonicalize(expected)) {
        (Ok(persisted), Ok(expected)) => persisted == expected,
        _ => false,
    }
}

fn validate_child_meta(
    meta: &Value,
    child: &ExpectedChild,
    parent: &str,
    thread: &str,
) -> Result<()> {
    ensure(
        meta["id"] == thread
            && meta["parent_thread_id"] == parent
            && meta["thread_source"] == "subagent",
        "child session correlation mismatch",
    )?;
    ensure(
        meta["agent_role"] == child.agent_type,
        "child session agent_role mismatch",
    )?;
    ensure(
        meta.get("agent_path")
            .is_none_or(|path| path.is_null() || path.as_str() == Some(&child.canonical_task)),
        "child session agent_path mismatch",
    )?;
    let spawn = &meta["source"]["subagent"]["thread_spawn"];
    ensure(
        spawn["parent_thread_id"] == parent && spawn["agent_role"] == child.agent_type,
        "child source correlation mismatch",
    )?;
    ensure(
        spawn
            .get("agent_path")
            .is_none_or(|path| path.is_null() || path.as_str() == Some(&child.canonical_task)),
        "child source agent_path mismatch",
    )
}

fn has_final_answer(records: &[Value], child: &ExpectedChild, thread: &str) -> bool {
    records.iter().any(|record| {
        let payload = &record["payload"];
        let text = content_text(payload);
        (record["type"] == "response_item"
            && payload["type"] == "agent_message"
            && payload["author"] == child.canonical_task
            && payload["recipient"] == "/root"
            && text.contains("Message Type: FINAL_ANSWER")
            && child
                .completion_contains
                .as_ref()
                .is_none_or(|needle| text.contains(needle)))
            || (record["type"] == "response_item"
                && payload["type"] == "message"
                && payload["role"] == "user"
                && text.contains("<subagent_notification>")
                && text.contains(thread)
                && child
                    .completion_contains
                    .as_ref()
                    .is_none_or(|needle| text.contains(needle)))
    })
}

fn content_text(payload: &Value) -> String {
    payload["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| part["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn is_spawn_call(record: &Value) -> bool {
    record["type"] == "response_item"
        && record["payload"]["type"] == "function_call"
        && record["payload"]["namespace"] == "collaboration"
        && record["payload"]["name"] == "spawn_agent"
}
fn find_payload<'a>(records: &'a [Value], kind: &str) -> Option<&'a Value> {
    records
        .iter()
        .find(|record| record["type"] == kind)
        .map(|record| &record["payload"])
}
fn parse_embedded<T: serde::de::DeserializeOwned>(value: &Value, label: &str) -> Result<T> {
    serde_json::from_str(
        value
            .as_str()
            .with_context(|| format!("{label} is not a JSON string"))?,
    )
    .with_context(|| format!("{label} is not valid JSON"))
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Result<T> {
    serde_json::from_str(
        &fs::read_to_string(path).with_context(|| format!("failed to read {label}"))?,
    )
    .with_context(|| format!("{label} is not valid JSON"))
}
fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("{}:{} is not JSON", path.display(), index + 1))
        })
        .collect()
}
fn query_edges(db: &Path, parent: &str) -> Result<Vec<Edge>> {
    ensure(
        db.is_file(),
        format_args!("Codex state DB not found at {}", db.display()),
    )?;
    let query = format!(
        "select e.parent_thread_id,e.child_thread_id,e.status,t.agent_path,t.agent_role,t.model,t.reasoning_effort,t.thread_source,t.cwd from thread_spawn_edges e join threads t on t.id=e.child_thread_id where e.parent_thread_id='{parent}'"
    );
    let output = Command::new("sqlite3")
        .args([
            "-json",
            db.to_str().context("state DB path is not UTF-8")?,
            &query,
        ])
        .output()
        .context("failed to run sqlite3")?;
    ensure(
        output.status.success(),
        String::from_utf8_lossy(&output.stderr),
    )?;
    parse_sqlite_edges(&output.stdout)
}

fn parse_sqlite_edges(output: &[u8]) -> Result<Vec<Edge>> {
    if output.iter().all(u8::is_ascii_whitespace) {
        return Ok(Vec::new());
    }
    serde_json::from_slice(output).context("sqlite3 returned invalid JSON")
}
fn find_session(thread: &str, roots: [&PathBuf; 2]) -> Result<PathBuf> {
    let suffix = format!("{thread}.jsonl");
    let mut hits = Vec::new();
    for root in roots {
        walk(root, &suffix, &mut hits)?;
    }
    ensure(
        hits.len() == 1,
        format_args!(
            "expected exactly one persisted Codex session for {thread}, found {}",
            hits.len()
        ),
    )?;
    Ok(hits.remove(0))
}
fn walk(root: &Path, suffix: &str, hits: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, suffix, hits)?;
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(suffix))
        {
            hits.push(path);
        }
    }
    Ok(())
}
fn default_codex_path(name: &str) -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
        .join(name)
}
fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(super::codex::is_sha256_hex)
}
fn is_codex_ciphertext(value: &str) -> bool {
    value.starts_with("gAAAA")
        && value[5..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'='))
}

#[cfg(test)]
mod tests {
    use super::{parse_sqlite_edges, persisted_cwd_matches};
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn sqlite_json_no_rows_is_an_empty_edge_set() {
        assert!(parse_sqlite_edges(b"").unwrap().is_empty());
        assert!(parse_sqlite_edges(b"\n").unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn persisted_cwd_accepts_symlink_alias_and_rejects_a_different_directory() {
        let root =
            std::env::temp_dir().join(format!("switchloom-codex-cwd-{}", std::process::id()));
        let real = root.join("real");
        let alias = root.join("alias");
        let different = root.join("different");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&different).unwrap();
        symlink(&real, &alias).unwrap();

        assert!(persisted_cwd_matches(alias.to_str(), &real));
        assert!(!persisted_cwd_matches(different.to_str(), &real));

        fs::remove_dir_all(root).unwrap();
    }
}
