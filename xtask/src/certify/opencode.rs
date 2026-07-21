use super::{OpencodeInput, ensure, write_json};
use anyhow::{Context, Result};
use model_routing::{
    ChildIdentityEvidence, DispatchEvidenceV1, ForkPolicy, GuaranteeLevel,
    RequestedDispatchEvidence,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;

#[derive(Deserialize)]
struct Invocation {
    nonce: String,
}

pub(crate) fn validate(input: OpencodeInput) -> Result<()> {
    let invocation: Invocation = serde_json::from_str(
        &fs::read_to_string(&input.invocation).context("failed to read requested invocation")?,
    )
    .context("requested invocation is not valid JSON")?;
    ensure(
        !invocation.nonce.trim().is_empty(),
        "requested invocation must include nonce",
    )?;
    let events =
        parse_jsonl(&fs::read_to_string(&input.jsonl).context("failed to read host output")?)?;
    ensure(!events.is_empty(), "host output has no JSON events")?;

    let task_invocations = events
        .iter()
        .filter_map(|event| {
            let id = string_for_keys(event, ID_KEYS)?;
            (contains(event, &input.worker) && mentions_task(event)).then_some((event, id))
        })
        .collect::<Vec<_>>();
    ensure(
        !task_invocations.is_empty(),
        format_args!(
            "no structured Task invocation with non-null call ID targeted {}",
            input.worker
        ),
    )?;
    let task_ids = task_invocations
        .iter()
        .map(|(_, id)| id.as_str())
        .collect::<Vec<_>>();

    for event in &events {
        if contains(event, &invocation.nonce) && is_result(event) {
            if let (Some(id), Some(agent)) = (
                string_for_keys(event, ID_KEYS),
                string_for_keys(event, AGENT_KEYS),
            ) {
                if task_ids.contains(&id.as_str()) && agent != input.worker {
                    anyhow::bail!("worker result came from {agent}, expected {}", input.worker);
                }
            }
        }
    }
    let worker_event = events
        .iter()
        .find(|event| {
            contains(event, &invocation.nonce)
                && is_result(event)
                && string_for_keys(event, ID_KEYS).is_some_and(|id| task_ids.contains(&id.as_str()))
                && string_for_keys(event, AGENT_KEYS).as_deref() == Some(input.worker.as_str())
        })
        .with_context(|| {
            format!(
                "nonce {} was not returned by an explicit {} Task result with matching call ID",
                invocation.nonce, input.worker
            )
        })?;
    let observed_agent = string_for_keys(worker_event, AGENT_KEYS)
        .context("worker result is missing explicit child identity")?;
    let effective_model = string_for_keys(worker_event, MODEL_KEYS).or_else(|| {
        task_invocations
            .iter()
            .find_map(|(event, _)| string_for_keys(event, MODEL_KEYS))
    });
    let effective_effort = string_for_keys(worker_event, VARIANT_KEYS).or_else(|| {
        task_invocations
            .iter()
            .find_map(|(event, _)| string_for_keys(event, VARIANT_KEYS))
    });

    let receipt = DispatchEvidenceV1 {
        schema_version: 1,
        package_digest: input.package_digest,
        host_version: input.host_version,
        requested_dispatch: RequestedDispatchEvidence {
            semantic_role: "worker".into(),
            profile: input.profile,
            model: input.model,
            effort: Some(input.variant),
            agent_type: Some(input.worker.clone()),
            fork_turns: Some(ForkPolicy {
                mode: "none".into(),
                turns: None,
            }),
        },
        child_identity: ChildIdentityEvidence {
            host: "opencode".into(),
            role: "worker".into(),
            agent_role: observed_agent.clone(),
            agent_type: Some(observed_agent.clone()),
            task_name: Some(observed_agent),
        },
        effective_model,
        effective_effort,
        nonce: invocation.nonce,
        raw_evidence_refs: vec![
            "requested-invocation:requested-invocation.json#argv".into(),
            "host-output:host-output.jsonl#task".into(),
            "host-stderr:host-output.stderr".into(),
        ],
        verdict: GuaranteeLevel::Advisory,
    };
    write_json(&input.receipt, &receipt)
}

const ID_KEYS: &[&str] = &[
    "id",
    "toolCallID",
    "toolCallId",
    "call_id",
    "callId",
    "taskID",
    "taskId",
];
const AGENT_KEYS: &[&str] = &[
    "agent",
    "agentName",
    "agent_name",
    "subagent",
    "subagentName",
    "taskAgent",
    "task_agent",
];
const MODEL_KEYS: &[&str] = &[
    "model",
    "modelID",
    "modelId",
    "providerModel",
    "provider_model",
];
const VARIANT_KEYS: &[&str] = &["variant", "effort", "reasoningEffort", "reasoning_effort"];

fn parse_jsonl(text: &str) -> Result<Vec<Value>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("host output line {} is not JSON", index + 1))
        })
        .collect()
}

fn visit(value: &Value, callback: &mut impl FnMut(&str, &Value)) {
    match value {
        Value::Array(values) => values.iter().for_each(|value| visit(value, callback)),
        Value::Object(values) => {
            for (key, value) in values {
                callback(key, value);
                visit(value, callback);
            }
        }
        _ => {}
    }
}

fn string_for_keys(value: &Value, keys: &[&str]) -> Option<String> {
    let mut found = None;
    visit(value, &mut |key, value| {
        if found.is_none() && keys.contains(&key) {
            found = value.as_str().map(str::to_owned);
        }
    });
    found
}

fn contains(value: &Value, needle: &str) -> bool {
    value.to_string().contains(needle)
}

fn mentions_task(value: &Value) -> bool {
    let mut found = false;
    visit(value, &mut |key, value| {
        if value.as_str().is_some_and(|text| {
            (key.to_ascii_lowercase().contains("tool")
                || key.to_ascii_lowercase().contains("type")
                || key.to_ascii_lowercase().contains("name"))
                && text.to_ascii_lowercase().contains("task")
        }) {
            found = true;
        }
    });
    found
}

fn is_result(value: &Value) -> bool {
    let mut found = false;
    visit(value, &mut |key, value| {
        if value.as_str().is_some_and(|text| {
            let key = key.to_ascii_lowercase();
            let text = text.to_ascii_lowercase();
            ((key.contains("type") || key.contains("event") || key.contains("kind"))
                && text.contains("result"))
                || ((key.contains("tool") || key.contains("name")) && text.contains("result"))
        }) {
            found = true;
        }
    });
    found
}
