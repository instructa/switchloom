use super::{PiInput, ensure, write_json};
use anyhow::{Context, Result};
use model_routing::{
    ChildIdentityEvidence, DispatchEvidenceV1, ForkPolicy, GuaranteeLevel,
    RequestedDispatchEvidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs};

#[derive(Deserialize)]
struct Workflow {
    schema_version: u32,
    workflow: String,
    runner: String,
    runtime_class: String,
    arguments: Arguments,
    process: Process,
}
#[derive(Deserialize)]
struct Arguments {
    agent_type: String,
    provider_model: String,
    thinking: String,
    isolation: Isolation,
    task: Task,
}
#[derive(Clone, Deserialize, Serialize)]
struct Isolation {
    session: String,
    tools: String,
    extensions: String,
    skills: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_dir: Option<String>,
}
#[derive(Deserialize)]
struct Task {
    semantic_role: String,
    returns: String,
}
#[derive(Deserialize)]
struct Process {
    argv: Vec<String>,
}
#[derive(Deserialize)]
struct Invocation {
    nonce: String,
    argv: Vec<String>,
    env: BTreeMap<String, String>,
    prompt_sha256: String,
}

#[derive(Serialize)]
struct WorkflowReceipt<'a> {
    schema_version: u32,
    runner: &'static str,
    workflow: &'a str,
    runtime_class: &'static str,
    package_digest: &'a str,
    host_version: &'a str,
    invocation: ReceiptInvocation<'a>,
    requested: ReceiptRequested<'a>,
    observed: ReceiptObserved<'a>,
    verdict: &'static str,
}
#[derive(Serialize)]
struct ReceiptInvocation<'a> {
    argv: &'a [String],
    env: &'a BTreeMap<String, String>,
    prompt_sha256: &'a str,
}
#[derive(Serialize)]
struct ReceiptRequested<'a> {
    semantic_role: &'static str,
    profile: &'a str,
    agent_type: &'a str,
    provider_model: &'a str,
    thinking: &'a str,
    isolation: &'a Isolation,
}
#[derive(Serialize)]
struct ReceiptObserved<'a> {
    stdout_ref: &'static str,
    stderr_ref: &'static str,
    nonce: &'a str,
}

pub(crate) fn validate(input: PiInput) -> Result<()> {
    let workflow: Workflow = read_json(&input.workflow, "workflow")?;
    let invocation: Invocation = read_json(&input.invocation, "requested invocation")?;
    let stdout = fs::read_to_string(&input.stdout).context("failed to read host stdout")?;
    fs::read_to_string(&input.stderr).context("failed to read host stderr")?;
    ensure(
        !invocation.nonce.trim().is_empty(),
        "requested invocation must include nonce",
    )?;
    ensure(
        workflow.schema_version == 1
            && workflow.runner == "pi"
            && workflow.runtime_class == "external-runner",
        "workflow must be schema v1 Pi external-runner",
    )?;
    let args = &workflow.arguments;
    ensure(
        args.agent_type == input.agent_type,
        "workflow agent_type mismatch",
    )?;
    ensure(
        args.provider_model == input.model,
        "workflow provider_model mismatch",
    )?;
    ensure(
        args.thinking == input.thinking,
        "workflow thinking mismatch",
    )?;
    ensure(
        args.task.semantic_role == "worker" && args.task.returns == "nonce-only",
        "workflow task must require worker nonce-only return",
    )?;
    ensure(
        args.isolation.session == "none",
        "workflow isolation must disable session persistence",
    )?;
    ensure(
        args.isolation.tools == "none",
        "workflow isolation must disable tools",
    )?;
    ensure(
        args.isolation.extensions == "none",
        "workflow isolation must disable extensions",
    )?;
    ensure(
        args.isolation.skills == "none",
        "workflow isolation must disable skills",
    )?;
    for required in [
        "--print",
        "--no-session",
        "--no-tools",
        "--no-extensions",
        "--no-skills",
        "--provider",
        "--model",
        "--thinking",
    ] {
        ensure(
            workflow.process.argv.iter().any(|arg| arg == required),
            format_args!("workflow process argv must include {required}"),
        )?;
    }
    let expected_argv = [
        vec![
            "env".into(),
            "PI_CODING_AGENT_DIR=.pi-agent".into(),
            "PI_OFFLINE=1".into(),
        ],
        workflow.process.argv.clone(),
    ]
    .concat();
    ensure(
        invocation.argv == expected_argv,
        "requested invocation argv does not match workflow process argv with report-local env boundary",
    )?;
    ensure(
        invocation
            .env
            .get("PI_CODING_AGENT_DIR")
            .map(String::as_str)
            == Some(".pi-agent")
            && invocation.env.get("PI_OFFLINE").map(String::as_str) == Some("1"),
        "requested invocation must set report-local PI_CODING_AGENT_DIR and PI_OFFLINE",
    )?;
    let executed = &invocation.argv[3..];
    ensure(
        executed.first().map(String::as_str) == Some("pi"),
        "requested invocation must execute pi",
    )?;
    let provider = option_value(executed, "--provider")
        .context("requested invocation must include provider")?;
    let model =
        option_value(executed, "--model").context("requested invocation must include model")?;
    let thinking = option_value(executed, "--thinking")
        .context("requested invocation must include thinking")?;
    ensure(
        format!("{provider}/{model}") == input.model && thinking == input.thinking,
        "requested invocation model or thinking mismatch",
    )?;
    let expected_prompt = format!(
        "sha256:{:x}",
        Sha256::digest(format!(
            "Return only this nonce and no other text: {}",
            invocation.nonce
        ))
    );
    ensure(
        invocation.prompt_sha256 == expected_prompt,
        "requested invocation prompt hash does not match nonce task",
    )?;
    ensure(
        stdout.split_whitespace().collect::<Vec<_>>().join(" ") == invocation.nonce,
        format_args!(
            "Pi child output did not exactly return nonce {}",
            invocation.nonce
        ),
    )?;

    let workflow_receipt = WorkflowReceipt {
        schema_version: 1,
        runner: "pi",
        workflow: &workflow.workflow,
        runtime_class: "external-runner",
        package_digest: &input.package_digest,
        host_version: &input.host_version,
        invocation: ReceiptInvocation {
            argv: &invocation.argv,
            env: &invocation.env,
            prompt_sha256: &invocation.prompt_sha256,
        },
        requested: ReceiptRequested {
            semantic_role: "worker",
            profile: &input.profile,
            agent_type: &input.agent_type,
            provider_model: &input.model,
            thinking: &input.thinking,
            isolation: &args.isolation,
        },
        observed: ReceiptObserved {
            stdout_ref: "host-output:host-output.txt",
            stderr_ref: "host-stderr:host-output.stderr",
            nonce: &invocation.nonce,
        },
        verdict: "advisory",
    };
    write_json(&input.workflow_receipt, &workflow_receipt)?;
    let dispatch = DispatchEvidenceV1 {
        schema_version: 1,
        package_digest: input.package_digest.clone(),
        host_version: input.host_version.clone(),
        requested_dispatch: RequestedDispatchEvidence {
            semantic_role: "worker".into(),
            profile: input.profile.clone(),
            model: input.model.clone(),
            effort: Some(input.thinking.clone()),
            agent_type: Some(input.agent_type.clone()),
            fork_turns: Some(ForkPolicy {
                mode: "none".into(),
                turns: None,
            }),
        },
        child_identity: ChildIdentityEvidence {
            host: "pi".into(),
            role: "worker".into(),
            agent_role: input.agent_type.clone(),
            agent_type: Some(input.agent_type),
            task_name: Some("model-routing-preset-runner".into()),
        },
        effective_model: Some(input.model),
        effective_effort: Some(input.thinking),
        nonce: invocation.nonce,
        raw_evidence_refs: vec![
            "workflow:workflow.json#arguments".into(),
            "requested-invocation:requested-invocation.json#argv".into(),
            "workflow-receipt:workflow-receipt.json".into(),
            "host-output:host-output.txt".into(),
            "host-stderr:host-output.stderr".into(),
        ],
        verdict: GuaranteeLevel::Advisory,
    };
    write_json(&input.dispatch_receipt, &dispatch)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &std::path::Path, label: &str) -> Result<T> {
    serde_json::from_str(
        &fs::read_to_string(path).with_context(|| format!("failed to read {label}"))?,
    )
    .with_context(|| format!("{label} is not valid JSON"))
}
fn option_value<'a>(argv: &'a [String], option: &str) -> Option<&'a str> {
    let index = argv.iter().position(|arg| arg == option)?;
    argv.get(index + 1).map(String::as_str)
}
