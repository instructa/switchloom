mod codex;
mod codex_spawn;
mod live;
mod opencode;
mod pi;
mod runner;

#[cfg(test)]
mod tests;

use crate::{OpencodeArgs, PiArgs};
use anyhow::{Context, Result, bail};
use model_routing::validate_dispatch_evidence_json_for_bundle;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) use codex::validate as validate_codex;
pub(crate) use codex_spawn::{CodexRawInput, extract as extract_codex};
pub(crate) use live::{
    LiveRunArgs, PlanrRunArgs, run_codex as run_live_codex,
    run_codex_negative_fixture as run_live_codex_negative_fixture, run_native as run_live_native,
    run_opencode as run_live_opencode, run_pi as run_live_pi, run_planr,
};
pub(crate) use opencode::validate as validate_opencode;
pub(crate) use pi::validate as validate_pi;

pub(crate) fn validate_cursor(receipt: &Path, bundle: &Path) -> Result<()> {
    let receipt = fs::read_to_string(receipt).context("failed to read Cursor evidence receipt")?;
    let bundle = fs::read_to_string(bundle).context("failed to read Cursor compiled bundle")?;
    validate_dispatch_evidence_json_for_bundle(&receipt, &bundle)
        .map_err(|error| anyhow::anyhow!(error))
}

fn required<T>(value: Option<T>, name: &str) -> Result<T> {
    value.ok_or_else(|| anyhow::anyhow!("--{name} is required when validating evidence"))
}

pub(crate) struct OpencodeInput {
    pub jsonl: PathBuf,
    pub invocation: PathBuf,
    pub receipt: PathBuf,
    pub package_digest: String,
    pub host_version: String,
    pub profile: String,
    pub model: String,
    pub variant: String,
    pub worker: String,
}

impl TryFrom<OpencodeArgs> for OpencodeInput {
    type Error = anyhow::Error;

    fn try_from(value: OpencodeArgs) -> Result<Self> {
        Ok(Self {
            jsonl: required(value.jsonl, "jsonl")?,
            invocation: required(value.invocation, "invocation")?,
            receipt: required(value.receipt, "receipt")?,
            package_digest: required(value.package_digest, "package-digest")?,
            host_version: required(value.host_version, "host-version")?,
            profile: required(value.profile, "profile")?,
            model: required(value.model, "model")?,
            variant: required(value.variant, "variant")?,
            worker: required(value.worker, "worker")?,
        })
    }
}

pub(crate) struct PiInput {
    pub workflow: PathBuf,
    pub invocation: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub workflow_receipt: PathBuf,
    pub dispatch_receipt: PathBuf,
    pub package_digest: String,
    pub host_version: String,
    pub profile: String,
    pub model: String,
    pub thinking: String,
    pub agent_type: String,
}

impl TryFrom<PiArgs> for PiInput {
    type Error = anyhow::Error;

    fn try_from(value: PiArgs) -> Result<Self> {
        Ok(Self {
            workflow: required(value.workflow, "workflow")?,
            invocation: required(value.invocation, "invocation")?,
            stdout: required(value.stdout, "stdout")?,
            stderr: required(value.stderr, "stderr")?,
            workflow_receipt: required(value.workflow_receipt, "workflow-receipt")?,
            dispatch_receipt: required(value.dispatch_receipt, "dispatch-receipt")?,
            package_digest: required(value.package_digest, "package-digest")?,
            host_version: required(value.host_version, "host-version")?,
            profile: required(value.profile, "profile")?,
            model: required(value.model, "model")?,
            thinking: required(value.thinking, "thinking")?,
            agent_type: required(value.agent_type, "agent-type")?,
        })
    }
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes =
        serde_json::to_vec_pretty(value).context("failed to serialize evidence receipt")?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}

fn ensure(condition: bool, message: impl std::fmt::Display) -> Result<()> {
    if condition {
        Ok(())
    } else {
        bail!("{message}")
    }
}
