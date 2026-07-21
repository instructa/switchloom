use super::ensure;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const SCHEMA: &str = "switchloom.codex_runtime_evidence.v1";

#[derive(Deserialize)]
struct Receipt {
    schema_version: String,
    run: Run,
    children: Vec<Child>,
    dispatch_evidence: Vec<DispatchEvidence>,
}

#[derive(Deserialize)]
struct Run {
    status: String,
    #[serde(default)]
    complete_marker: Option<String>,
    evidence_source: String,
    parent_thread_id: String,
    parent_session: String,
    workdir: String,
}

#[derive(Deserialize)]
struct Child {
    kind: String,
    profile: String,
    agent_type: String,
    task_name: String,
    canonical_task: String,
    parent_thread_id: String,
    child_thread_id: String,
    spawn: Spawn,
    #[serde(default = "missing_input")]
    input: MessageInput,
    spawn_output: SpawnOutput,
    session: Session,
    state: State,
    final_answer: FinalAnswer,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spawn {
    surface: String,
    agent_type: String,
    task_name: String,
    fork_turns: String,
    call_id: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    reasoning_effort: Option<String>,
}

#[derive(Deserialize)]
struct MessageInput {
    message_sha256: String,
    message_bytes: u64,
    max_message_bytes: u64,
    #[serde(default = "plaintext")]
    message_encoding: String,
    message_plaintext_verdict: String,
    #[serde(default)]
    message_plaintext_intent_sha256: Option<String>,
}

fn plaintext() -> String {
    "plaintext".into()
}

fn missing_input() -> MessageInput {
    MessageInput {
        message_sha256: String::new(),
        message_bytes: 0,
        max_message_bytes: 0,
        message_encoding: plaintext(),
        message_plaintext_verdict: String::new(),
        message_plaintext_intent_sha256: None,
    }
}

#[derive(Deserialize)]
struct SpawnOutput {
    #[serde(default)]
    task_name: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Deserialize)]
struct Session {
    agent_role: String,
    #[serde(default)]
    agent_path: Option<String>,
    thread_source: String,
    parent_thread_id: String,
    session_file: String,
}

#[derive(Deserialize)]
struct State {
    agent_role: String,
    #[serde(default)]
    agent_path: Option<String>,
    model: String,
    reasoning_effort: String,
    thread_source: String,
    cwd: String,
}

#[derive(Deserialize)]
struct FinalAnswer {
    message_type: String,
}

#[derive(Deserialize)]
struct DispatchEvidence {
    schema_version: u32,
    package_digest: String,
    host_version: String,
    requested_dispatch: RequestedDispatch,
    child_identity: ChildIdentity,
    effective_model: String,
    effective_effort: String,
    nonce: String,
    raw_evidence_refs: Vec<String>,
    verdict: String,
}

#[derive(Deserialize)]
struct RequestedDispatch {
    semantic_role: String,
    profile: String,
    model: String,
    effort: String,
    agent_type: String,
    fork_turns: ForkTurns,
    message_sha256: String,
    message_encoding: String,
    message_plaintext_verdict: String,
    #[serde(default)]
    message_plaintext_intent_sha256: Option<String>,
    message_bytes: u64,
    max_message_bytes: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForkTurns {
    mode: String,
}

#[derive(Deserialize)]
struct ChildIdentity {
    host: String,
    role: String,
    agent_role: String,
    agent_type: String,
    task_name: String,
}

#[derive(Deserialize)]
struct ExpectedReceipt {
    #[serde(default)]
    package_digest: Option<String>,
    #[serde(default)]
    host_version: Option<String>,
    children: Vec<ExpectedChild>,
}

#[derive(Clone, Deserialize)]
struct ExpectedChild {
    #[serde(default)]
    semantic_role: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    agent_type: String,
    profile: String,
    task_name: String,
    #[serde(default)]
    message_sha256: Option<String>,
    #[serde(default)]
    message_ciphertext_sha256: Option<String>,
    #[serde(default)]
    max_message_bytes: Option<u64>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    effort: Option<String>,
    #[serde(default)]
    state: Option<ExpectedState>,
    #[serde(default)]
    input: Option<ExpectedInput>,
}

#[derive(Clone, Deserialize)]
struct ExpectedState {
    model: String,
    reasoning_effort: String,
}

#[derive(Clone, Deserialize)]
struct ExpectedInput {
    message_sha256: String,
    max_message_bytes: u64,
}

pub(crate) fn validate(receipt_path: &Path, expected_path: Option<&Path>) -> Result<()> {
    let receipt_json =
        fs::read_to_string(receipt_path).context("failed to read Codex runtime evidence")?;
    validate_json(&receipt_json, expected_path)
}

pub(super) fn validate_json(receipt_json: &str, expected_path: Option<&Path>) -> Result<()> {
    let receipt: Receipt =
        serde_json::from_str(receipt_json).context("Codex runtime evidence is not valid JSON")?;
    let expected: Option<ExpectedReceipt> = expected_path
        .map(|path| read_json(path, "expected Codex dispatch"))
        .transpose()?;

    ensure(
        receipt.schema_version == SCHEMA,
        format_args!("schema_version must be {SCHEMA}"),
    )?;
    ensure(
        receipt.run.status == "complete",
        "run.status must be complete",
    )?;
    ensure(
        receipt.run.complete_marker.as_deref()
            == Some("SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE"),
        "run complete marker missing",
    )?;
    ensure(
        receipt.run.evidence_source == "codex_persisted_spawn_state",
        "run.evidence_source must be codex_persisted_spawn_state",
    )?;
    ensure(
        is_uuid(&receipt.run.parent_thread_id),
        "run.parent_thread_id must be a UUID",
    )?;
    ensure(
        receipt.run.parent_session.ends_with(".jsonl"),
        "run.parent_session must name a persisted session jsonl",
    )?;
    ensure(
        receipt.run.workdir.starts_with('/'),
        "run.workdir must be absolute",
    )?;

    let expected_children = expected_children(&receipt, expected.as_ref())?;
    ensure(
        receipt.children.len() == expected_children.len(),
        "children must contain exactly the expected roles",
    )?;
    ensure(
        receipt.dispatch_evidence.len() == expected_children.len(),
        "dispatch_evidence must contain exactly the expected receipts",
    )?;

    let mut seen = BTreeSet::new();
    for child in &receipt.children {
        let expected_child = expected_children
            .get(&child.kind)
            .with_context(|| format!("unexpected child kind {}", child.kind))?;
        ensure(
            seen.insert(child.kind.as_str()),
            format_args!("duplicate child kind {}", child.kind),
        )?;
        validate_child(&receipt, child, expected_child, expected.as_ref())?;
    }
    Ok(())
}

fn validate_child(
    receipt: &Receipt,
    child: &Child,
    expected: &Expected,
    expected_receipt: Option<&ExpectedReceipt>,
) -> Result<()> {
    let kind = &child.kind;
    ensure(
        child.agent_type == expected.agent_type,
        format_args!("{kind} agent_type mismatch"),
    )?;
    ensure(
        child.profile == expected.profile,
        format_args!("{kind} profile mismatch"),
    )?;
    ensure(
        child.task_name == expected.task_name,
        format_args!("{kind} task_name mismatch"),
    )?;
    ensure(
        valid_task_name(&child.task_name),
        format_args!("{kind} task_name is invalid"),
    )?;
    ensure(
        child.canonical_task == format!("/root/{}", child.task_name),
        format_args!("{kind} canonical_task mismatch"),
    )?;
    ensure(
        child.parent_thread_id == receipt.run.parent_thread_id,
        format_args!("{kind} parent_thread_id mismatch"),
    )?;
    ensure(
        is_uuid(&child.child_thread_id) && child.child_thread_id != receipt.run.parent_thread_id,
        format_args!("{kind} child_thread_id must be a distinct UUID"),
    )?;
    ensure(
        child.spawn.surface == "collaboration.spawn_agent",
        format_args!("{kind} spawn surface must be Codex V2 collaboration.spawn_agent"),
    )?;
    ensure(
        child.spawn.agent_type == child.agent_type,
        format_args!("{kind} spawn agent_type mismatch"),
    )?;
    ensure(
        child.spawn.task_name == child.task_name,
        format_args!("{kind} spawn task_name mismatch"),
    )?;
    ensure(
        child.spawn.fork_turns == "none",
        format_args!("{kind} fork_turns must be none"),
    )?;
    ensure(
        !child.spawn.call_id.trim().is_empty(),
        format_args!("{kind} spawn call_id must not be blank"),
    )?;
    ensure(
        child.spawn.model.is_none(),
        format_args!("{kind} spawn must not manually override model"),
    )?;
    ensure(
        child.spawn.reasoning_effort.is_none(),
        format_args!("{kind} spawn must not manually override effort"),
    )?;
    ensure(
        is_sha256_hex(&child.input.message_sha256),
        format_args!("{kind} input message_sha256 must be lowercase sha256 hex"),
    )?;

    let expected_hash = match child.input.message_encoding.as_str() {
        "plaintext" => {
            ensure(
                child.input.message_plaintext_verdict == "deterministic",
                format_args!("{kind} input message_plaintext_verdict mismatch"),
            )?;
            expected
                .message_sha256
                .as_ref()
                .context("plaintext input requires expected message_sha256")?
        }
        "codex-encrypted" => {
            ensure(
                child.input.message_plaintext_verdict == "unsupported",
                format_args!("{kind} encrypted input cannot claim deterministic plaintext"),
            )?;
            if let Some(intent) = &child.input.message_plaintext_intent_sha256 {
                ensure(
                    Some(intent) == expected.message_sha256.as_ref(),
                    format_args!("{kind} input message_plaintext_intent_sha256 mismatch"),
                )?;
            }
            expected
                .message_ciphertext_sha256
                .as_ref()
                .unwrap_or(&child.input.message_sha256)
        }
        encoding => anyhow::bail!("{kind} unsupported input message_encoding {encoding}"),
    };
    ensure(
        is_sha256_hex(expected_hash) && child.input.message_sha256 == *expected_hash,
        format_args!("{kind} input message_sha256 mismatch"),
    )?;
    ensure(
        child.input.message_bytes > 0 && child.input.message_bytes <= expected.max_message_bytes,
        format_args!("{kind} input message_bytes exceeds max_message_bytes"),
    )?;
    ensure(
        child.input.max_message_bytes == expected.max_message_bytes,
        format_args!("{kind} input max_message_bytes mismatch"),
    )?;
    ensure(
        child.spawn_output.task_name.as_deref() == Some(&child.canonical_task)
            || child.spawn_output.agent_id.as_deref() == Some(&child.child_thread_id),
        format_args!("{kind} spawn output task mismatch"),
    )?;
    ensure(
        child.session.agent_role == child.agent_type && child.state.agent_role == child.agent_type,
        format_args!("{kind} agent_role mismatch"),
    )?;
    ensure(
        path_matches(&child.session.agent_path, &child.canonical_task)
            && path_matches(&child.state.agent_path, &child.canonical_task),
        format_args!("{kind} agent_path mismatch"),
    )?;
    ensure(
        child.session.thread_source == "subagent" && child.state.thread_source == "subagent",
        format_args!("{kind} thread_source mismatch"),
    )?;
    ensure(
        child.session.parent_thread_id == receipt.run.parent_thread_id,
        format_args!("{kind} session parent mismatch"),
    )?;
    ensure(
        child.session.session_file.ends_with(".jsonl"),
        format_args!("{kind} session file missing"),
    )?;
    ensure(
        child.state.model == expected.model,
        format_args!("{kind} effective model mismatch"),
    )?;
    ensure(
        child.state.reasoning_effort == expected.effort,
        format_args!("{kind} effective effort mismatch"),
    )?;
    ensure(
        child.state.cwd == receipt.run.workdir,
        format_args!("{kind} state cwd mismatch"),
    )?;
    ensure(
        !(child.state.model == "gpt-5.6-sol" && child.state.reasoning_effort == "medium"),
        format_args!("{kind} inherited Sol Medium evidence is forbidden"),
    )?;
    ensure(
        child.final_answer.message_type == "FINAL_ANSWER",
        format_args!("{kind} final answer marker missing"),
    )?;

    let dispatch = receipt
        .dispatch_evidence
        .iter()
        .find(|e| {
            e.requested_dispatch.semantic_role == *kind
                && e.requested_dispatch.agent_type == child.agent_type
                && e.child_identity.task_name == child.task_name
        })
        .with_context(|| format!("{kind} dispatch_evidence receipt missing"))?;
    validate_dispatch(receipt, child, expected, dispatch, expected_receipt)
}

fn validate_dispatch(
    receipt: &Receipt,
    child: &Child,
    expected: &Expected,
    evidence: &DispatchEvidence,
    expected_receipt: Option<&ExpectedReceipt>,
) -> Result<()> {
    let kind = &child.kind;
    ensure(
        evidence.schema_version == 1,
        format_args!("{kind} dispatch_evidence schema_version mismatch"),
    )?;
    ensure(
        is_sha256_digest(&evidence.package_digest),
        format_args!("{kind} dispatch_evidence package_digest must be sha256:<64 lowercase hex>"),
    )?;
    ensure(
        is_codex_version(&evidence.host_version),
        format_args!("{kind} dispatch_evidence host_version must come from codex --version"),
    )?;
    if let Some(expected_receipt) = expected_receipt {
        if let Some(digest) = &expected_receipt.package_digest {
            ensure(
                evidence.package_digest == *digest,
                format_args!("{kind} package_digest mismatch"),
            )?;
        }
        if let Some(version) = &expected_receipt.host_version {
            ensure(
                evidence.host_version == *version,
                format_args!("{kind} host_version mismatch"),
            )?;
        }
    }
    let requested = &evidence.requested_dispatch;
    ensure(
        requested.profile == expected.profile
            && requested.model == expected.model
            && requested.effort == expected.effort,
        format_args!("{kind} requested dispatch mismatch"),
    )?;
    ensure(
        requested.agent_type == child.agent_type && requested.fork_turns.mode == "none",
        format_args!("{kind} requested child identity mismatch"),
    )?;
    ensure(
        requested.message_sha256 == child.input.message_sha256,
        format_args!("{kind} requested message_sha256 mismatch"),
    )?;
    ensure(
        requested.message_encoding == child.input.message_encoding
            && requested.message_plaintext_verdict == child.input.message_plaintext_verdict
            && requested.message_plaintext_intent_sha256
                == child.input.message_plaintext_intent_sha256
            && requested.message_bytes == child.input.message_bytes
            && requested.max_message_bytes == expected.max_message_bytes,
        format_args!("{kind} requested message evidence mismatch"),
    )?;
    ensure(
        evidence.child_identity.host == "codex",
        format_args!("{kind} child host mismatch"),
    )?;
    ensure(
        evidence.child_identity.role == *kind,
        format_args!("{kind} child role mismatch"),
    )?;
    ensure(
        evidence.child_identity.agent_role == child.agent_type,
        format_args!("{kind} child agent_role mismatch"),
    )?;
    ensure(
        evidence.child_identity.agent_type == child.agent_type,
        format_args!("{kind} child agent_type mismatch"),
    )?;
    ensure(
        evidence.effective_model == child.state.model
            && evidence.effective_effort == child.state.reasoning_effort,
        format_args!("{kind} effective receipt mismatch"),
    )?;
    let nonce = format!(
        "{}:{}:{}",
        receipt.run.parent_thread_id, child.child_thread_id, child.spawn.call_id
    );
    ensure(
        evidence.nonce == nonce
            && !evidence.nonce.contains("nonce-")
            && !evidence.nonce.contains("placeholder"),
        format_args!(
            "{kind} dispatch_evidence nonce must bind parent thread, child thread, and spawn call"
        ),
    )?;
    for reference in [
        format!("codex-session:{}", receipt.run.parent_session),
        format!("codex-session:{}", child.session.session_file),
        format!(
            "state_5.sqlite:thread_spawn_edges:{}:{}",
            receipt.run.parent_thread_id, child.child_thread_id
        ),
        format!("spawn_call:{}", child.spawn.call_id),
    ] {
        ensure(
            evidence.raw_evidence_refs.contains(&reference),
            format_args!(
                "{kind} raw evidence refs must bind parent session, child session, spawn edge, and spawn call"
            ),
        )?;
    }
    ensure(
        evidence.verdict == "deterministic",
        format_args!("{kind} dispatch_evidence verdict mismatch"),
    )
}

struct Expected {
    agent_type: String,
    profile: String,
    task_name: String,
    message_sha256: Option<String>,
    message_ciphertext_sha256: Option<String>,
    max_message_bytes: u64,
    model: String,
    effort: String,
}

fn expected_children(
    receipt: &Receipt,
    explicit: Option<&ExpectedReceipt>,
) -> Result<BTreeMap<String, Expected>> {
    let mut result = BTreeMap::new();
    if let Some(explicit) = explicit {
        for child in &explicit.children {
            let role = child
                .semantic_role
                .as_ref()
                .or(child.kind.as_ref())
                .context("expected child semantic_role must not be blank")?;
            let model = child
                .model
                .clone()
                .or_else(|| child.state.as_ref().map(|s| s.model.clone()))
                .context("expected child model must not be blank")?;
            let effort = child
                .effort
                .clone()
                .or_else(|| child.state.as_ref().map(|s| s.reasoning_effort.clone()))
                .context("expected child effort must not be blank")?;
            let message_sha256 = child
                .message_sha256
                .clone()
                .or_else(|| child.input.as_ref().map(|i| i.message_sha256.clone()));
            let max_message_bytes = child
                .max_message_bytes
                .or_else(|| child.input.as_ref().map(|i| i.max_message_bytes))
                .context("expected max_message_bytes must be positive")?;
            result.insert(
                role.clone(),
                Expected {
                    agent_type: child.agent_type.clone(),
                    profile: child.profile.clone(),
                    task_name: child.task_name.clone(),
                    message_sha256,
                    message_ciphertext_sha256: child.message_ciphertext_sha256.clone(),
                    max_message_bytes,
                    model,
                    effort,
                },
            );
        }
    } else {
        for child in &receipt.children {
            ensure(
                is_sha256_hex(&child.input.message_sha256),
                format_args!(
                    "{} input message_sha256 must be lowercase sha256 hex",
                    child.kind
                ),
            )?;
            result.insert(
                child.kind.clone(),
                Expected {
                    agent_type: child.agent_type.clone(),
                    profile: child.profile.clone(),
                    task_name: child.task_name.clone(),
                    message_sha256: Some(child.input.message_sha256.clone()),
                    message_ciphertext_sha256: None,
                    max_message_bytes: child.input.max_message_bytes,
                    model: child.state.model.clone(),
                    effort: child.state.reasoning_effort.clone(),
                },
            );
        }
    }
    ensure(
        result.values().all(|e| {
            !e.agent_type.trim().is_empty()
                && !e.profile.trim().is_empty()
                && !e.task_name.trim().is_empty()
                && e.max_message_bytes > 0
                && !e.model.trim().is_empty()
                && !e.effort.trim().is_empty()
        }),
        "expected child fields must not be blank",
    )?;
    Ok(result)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("failed to read {label}"))?;
    serde_json::from_str(&text).with_context(|| format!("{label} is not valid JSON"))
}

pub(super) fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && [8, 13, 18, 23]
            .iter()
            .all(|&i| value.as_bytes().get(i) == Some(&b'-'))
        && value.chars().enumerate().all(|(i, c)| {
            [8, 13, 18, 23].contains(&i) || c.is_ascii_hexdigit() && !c.is_ascii_uppercase()
        })
}
pub(super) fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn is_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(is_sha256_hex)
}
fn valid_task_name(value: &str) -> bool {
    value.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn path_matches(value: &Option<String>, canonical: &str) -> bool {
    value.as_deref().is_none_or(|value| value == canonical)
}
pub(super) fn is_codex_version(value: &str) -> bool {
    let Some(rest) = value
        .strip_prefix("codex ")
        .or_else(|| value.strip_prefix("codex-cli "))
    else {
        return false;
    };
    rest.split(['-', '+']).next().is_some_and(|v| {
        v.split('.').count() == 3
            && v.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    })
}
