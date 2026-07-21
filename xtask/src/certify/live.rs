use super::{
    CodexRawInput, OpencodeInput, PiInput,
    runner::{
        FileSnapshot, OwnedReportRepo, ProcessReceipt, ProcessSpec, RestorationOutcome,
        RestorationTracker, run_process,
    },
    validate_opencode, validate_pi,
};
use anyhow::{Context, Result, bail};
use model_routing::{
    ChildIdentityEvidence, DispatchEvidenceV1, ForkPolicy, GuaranteeLevel,
    RequestedDispatchEvidence, validate_dispatch_evidence_json_for_bundle,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) struct LiveRunArgs {
    pub routing_bin: PathBuf,
    pub report_root: PathBuf,
    pub timeout: Duration,
}
impl LiveRunArgs {
    pub fn new(routing_bin: PathBuf, report_root: PathBuf, timeout_seconds: u64) -> Self {
        Self {
            routing_bin,
            report_root,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
}
pub(crate) struct PlanrRunArgs {
    pub live: LiveRunArgs,
    pub protected_planr_root: PathBuf,
}

#[derive(Serialize)]
struct CertificationReport {
    schema_version: u32,
    host: String,
    success: bool,
    live_verified: bool,
    limitation: Option<String>,
    restoration: RestorationOutcome,
    workdir: PathBuf,
    commands: Vec<ProcessReceipt>,
}

struct CertificationSession {
    report_dir: PathBuf,
    report: CertificationReport,
    restoration: Option<RestorationTracker>,
}

impl CertificationSession {
    fn new(owned: &OwnedReportRepo, host: &str) -> Self {
        Self {
            report_dir: owned.report_dir.clone(),
            report: CertificationReport {
                schema_version: 1,
                host: host.into(),
                success: false,
                live_verified: false,
                limitation: Some("certification did not complete".into()),
                restoration: RestorationOutcome::NotRequired,
                workdir: owned.workdir.clone(),
                commands: Vec::new(),
            },
            restoration: None,
        }
    }

    fn track_restoration(&mut self, tracker: RestorationTracker) {
        self.restoration = Some(tracker);
        self.persist_best_effort();
    }

    fn run_checked(
        &mut self,
        spec: ProcessSpec,
        owned: &OwnedReportRepo,
    ) -> Result<ProcessReceipt> {
        let receipt = run_process(&spec, &owned.report_dir)?;
        self.report.commands.push(receipt.clone());
        if !receipt.success() {
            self.report.limitation = Some(format!(
                "{} failed with status {:?}{}",
                receipt.label,
                receipt.status,
                if receipt.timed_out {
                    " after timeout"
                } else {
                    ""
                }
            ));
            self.persist()?;
            bail!(self.report.limitation.clone().unwrap_or_default());
        }
        self.persist()?;
        Ok(receipt)
    }

    fn record(&mut self, receipt: ProcessReceipt) -> Result<()> {
        self.report.commands.push(receipt);
        self.persist()
    }

    fn fail(&mut self, message: impl Into<String>) -> Result<()> {
        self.report.limitation = Some(message.into());
        self.persist()
    }

    fn complete(&mut self, live_verified: bool, limitation: Option<String>) -> Result<()> {
        self.report.success = true;
        self.report.live_verified = live_verified;
        self.report.limitation = limitation;
        self.persist()
    }

    fn persist(&mut self) -> Result<()> {
        if let Some(tracker) = &self.restoration {
            self.report.restoration = tracker.borrow().clone();
        }
        write_json(
            &self.report_dir.join("certification-report.json"),
            &self.report,
        )
    }

    fn persist_best_effort(&mut self) {
        let _ = self.persist();
    }
}

impl Drop for CertificationSession {
    fn drop(&mut self) {
        self.persist_best_effort();
    }
}

pub(crate) fn run_native(host: &str, args: LiveRunArgs) -> Result<()> {
    let owned = OwnedReportRepo::create(&args.report_root, host)?;
    let mut session = CertificationSession::new(&owned, host);
    if host == "claude-native" {
        session.complete(
            false,
            Some("Claude effective model/effort telemetry is not treated as live verified".into()),
        )?;
        println!(
            "Claude certification recorded as explicitly not live verified: {}",
            owned.report_dir.display()
        );
        return Ok(());
    }
    let (model, profile) = match host {
        "cursor-openai" => ("gpt-5.4-mini", "cursor-openai-worker"),
        "cursor-fable-grok" => ("cursor-grok-4.5-medium", "cursor-grok-worker"),
        other => bail!("unsupported native certification host {other}"),
    };
    let routing_bin = absolute_binary(&args.routing_bin)?;
    session.run_checked(
        command(
            "compile",
            &routing_bin,
            [
                "compile",
                "balanced",
                "--host",
                host,
                "--output",
                "bundle.json",
            ],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    session.run_checked(
        command(
            "apply",
            &routing_bin,
            ["apply", "bundle.json", "--repository", "."],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    ensure_file(
        &owned
            .workdir
            .join(".cursor/agents/model-routing-preset-worker.md"),
    )?;
    let version = session.run_checked(
        command(
            "cursor-version",
            Path::new("cursor-agent"),
            ["--version"],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    let host_version = last_line(&version.stdout, &version.stderr);
    let nonce = nonce("cursor");
    let prompt = format!("Return only this nonce and do not edit files: {nonce}");
    let invocation = json!({"host":"cursor","mode":"live","nonce":nonce,"argv":["cursor-agent","--print","--output-format","json","--trust","--model",model],"prompt":prompt,"artifact_path":".cursor/agents/model-routing-preset-worker.md"});
    write_json(
        &owned.workdir.join("requested-invocation.json"),
        &invocation,
    )?;
    let host_run = session.run_checked(
        command(
            "cursor-host",
            Path::new("cursor-agent"),
            [
                "--print",
                "--output-format",
                "json",
                "--trust",
                "--model",
                model,
                &prompt,
            ],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    ensure_contains_nonce(&host_run, &nonce)?;
    fs::write(owned.workdir.join("host-output.json"), &host_run.stdout)?;
    fs::write(owned.workdir.join("host-output.stderr"), &host_run.stderr)?;
    let output: Value =
        serde_json::from_str(&host_run.stdout).context("Cursor output must be structured JSON")?;
    let effective_model = output
        .pointer("/effective_model")
        .or_else(|| output.pointer("/model"))
        .or_else(|| output.pointer("/result/model"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let deterministic = effective_model.as_deref() == Some(model);
    let receipt = DispatchEvidenceV1 {
        schema_version: 1,
        package_digest: sha256_file(&routing_bin)?,
        host_version,
        requested_dispatch: RequestedDispatchEvidence {
            semantic_role: "worker".into(),
            profile: profile.into(),
            model: model.into(),
            effort: None,
            agent_type: Some("model-routing-preset-worker".into()),
            fork_turns: Some(ForkPolicy {
                mode: "none".into(),
                turns: None,
            }),
        },
        child_identity: ChildIdentityEvidence {
            host: "cursor".into(),
            role: "worker".into(),
            agent_role: "model-routing-preset-worker".into(),
            agent_type: Some("model-routing-preset-worker".into()),
            task_name: Some("model-routing-preset-worker".into()),
        },
        effective_model,
        effective_effort: None,
        nonce,
        raw_evidence_refs: vec![
            "requested-invocation:requested-invocation.json#argv".into(),
            "host-output:host-output.json".into(),
            "host-stderr:host-output.stderr".into(),
        ],
        verdict: if deterministic {
            GuaranteeLevel::Deterministic
        } else {
            GuaranteeLevel::Advisory
        },
    };
    write_json(&owned.workdir.join("dispatch-evidence.json"), &receipt)?;
    validate_bundle_receipt(&owned)?;
    session.complete(
        true,
        (!deterministic).then(|| {
            "Cursor ran live but did not expose deterministic effective-model telemetry".into()
        }),
    )?;
    println!(
        "Cursor live certification passed: {}",
        owned.report_dir.display()
    );
    Ok(())
}

pub(crate) fn run_opencode(args: LiveRunArgs) -> Result<()> {
    let host = "opencode-native";
    let owned = OwnedReportRepo::create(&args.report_root, host)?;
    let mut session = CertificationSession::new(&owned, host);
    let routing_bin = absolute_binary(&args.routing_bin)?;
    session.run_checked(
        command(
            "compile",
            &routing_bin,
            [
                "compile",
                "balanced",
                "--host",
                host,
                "--output",
                "bundle.json",
            ],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    session.run_checked(
        command(
            "apply",
            &routing_bin,
            ["apply", "bundle.json", "--repository", "."],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    let version = session.run_checked(
        command(
            "opencode-version",
            Path::new("opencode"),
            ["--version"],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    let host_version = last_line(&version.stdout, &version.stderr);
    let nonce = nonce("opencode");
    let worker = "model-routing-preset-worker";
    let model = "opencode/gpt-5-nano";
    let variant = "low";
    let prompt = format!(
        "Use the Task tool to invoke {worker}. The worker must return only this nonce and must not edit files: {nonce}. After the worker returns, return only the same nonce."
    );
    let argv = vec![
        "env",
        "XDG_DATA_HOME=.opencode-xdg/data",
        "XDG_STATE_HOME=.opencode-xdg/state",
        "XDG_CACHE_HOME=.opencode-xdg/cache",
        "opencode",
        "run",
        "--dir",
        ".",
        "--agent",
        "model-routing-preset-driver",
        "--model",
        model,
        "--variant",
        variant,
        "--format",
        "json",
    ];
    write_json(
        &owned.workdir.join("requested-invocation.json"),
        &json!({"host":"opencode","mode":"live","nonce":nonce,"argv":argv,"prompt":prompt,"artifact_path":".opencode/agents/model-routing-preset-worker.md"}),
    )?;
    for dir in [
        ".opencode-xdg/data",
        ".opencode-xdg/state",
        ".opencode-xdg/cache",
    ] {
        fs::create_dir_all(owned.workdir.join(dir))?;
    }
    let mut spec = command(
        "opencode-host",
        Path::new("opencode"),
        [
            "run",
            "--dir",
            ".",
            "--agent",
            "model-routing-preset-driver",
            "--model",
            model,
            "--variant",
            variant,
            "--format",
            "json",
            &prompt,
        ],
        &owned,
        args.timeout,
    );
    spec.env = BTreeMap::from([
        (
            "XDG_DATA_HOME".into(),
            owned
                .workdir
                .join(".opencode-xdg/data")
                .display()
                .to_string(),
        ),
        (
            "XDG_STATE_HOME".into(),
            owned
                .workdir
                .join(".opencode-xdg/state")
                .display()
                .to_string(),
        ),
        (
            "XDG_CACHE_HOME".into(),
            owned
                .workdir
                .join(".opencode-xdg/cache")
                .display()
                .to_string(),
        ),
    ]);
    let host_run = session.run_checked(spec, &owned)?;
    fs::write(owned.workdir.join("host-output.jsonl"), &host_run.stdout)?;
    fs::write(owned.workdir.join("host-output.stderr"), &host_run.stderr)?;
    validate_opencode(OpencodeInput {
        jsonl: owned.workdir.join("host-output.jsonl"),
        invocation: owned.workdir.join("requested-invocation.json"),
        receipt: owned.workdir.join("dispatch-evidence.json"),
        package_digest: sha256_file(&routing_bin)?,
        host_version,
        profile: "opencode-worker".into(),
        model: model.into(),
        variant: variant.into(),
        worker: worker.into(),
    })?;
    validate_bundle_receipt(&owned)?;
    session.complete(
        true,
        Some(
            "OpenCode evidence remains advisory unless host telemetry proves effective routing"
                .into(),
        ),
    )?;
    println!(
        "OpenCode live certification passed: {}",
        owned.report_dir.display()
    );
    Ok(())
}

pub(crate) fn run_pi(args: LiveRunArgs) -> Result<()> {
    let host = "pi-external";
    let owned = OwnedReportRepo::create(&args.report_root, host)?;
    let mut session = CertificationSession::new(&owned, host);
    let routing_bin = absolute_binary(&args.routing_bin)?;
    session.run_checked(
        command(
            "compile",
            &routing_bin,
            [
                "compile",
                "balanced",
                "--host",
                host,
                "--output",
                "bundle.json",
            ],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    session.run_checked(
        command(
            "apply",
            &routing_bin,
            ["apply", "bundle.json", "--repository", "."],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    fs::copy(
        owned
            .workdir
            .join(".pi/workflows/model-routing-preset-runner.json"),
        owned.workdir.join("workflow.json"),
    )?;
    let version = session.run_checked(
        command(
            "pi-version",
            Path::new("pi"),
            ["--version"],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    let host_version = last_line(&version.stdout, &version.stderr);
    let nonce = nonce("pi");
    let prompt = format!("Return only this nonce and no other text: {nonce}");
    let prompt_sha = format!("sha256:{:x}", Sha256::digest(prompt.as_bytes()));
    let argv = vec![
        "env",
        "PI_CODING_AGENT_DIR=.pi-agent",
        "PI_OFFLINE=1",
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
    write_json(
        &owned.workdir.join("requested-invocation.json"),
        &json!({"host":"pi","nonce":nonce,"argv":argv,"env":{"PI_CODING_AGENT_DIR":".pi-agent","PI_OFFLINE":"1"},"requested":{"profile":"pi-worker","agent_type":"switchloom-pi-worker","provider_model":"openai/gpt-4o-mini","thinking":"low","isolation":{"session":"none","tools":"none","extensions":"none","skills":"none"}},"prompt_sha256":prompt_sha,"artifact_path":".pi/workflows/model-routing-preset-runner.json"}),
    )?;
    fs::create_dir_all(owned.workdir.join(".pi-agent"))?;
    let mut spec = command(
        "pi-host",
        Path::new("pi"),
        [
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
            &prompt,
        ],
        &owned,
        args.timeout,
    );
    spec.env = BTreeMap::from([
        (
            "PI_CODING_AGENT_DIR".into(),
            owned.workdir.join(".pi-agent").display().to_string(),
        ),
        ("PI_OFFLINE".into(), "1".into()),
    ]);
    let host_run = session.run_checked(spec, &owned)?;
    fs::write(owned.workdir.join("host-output.txt"), &host_run.stdout)?;
    fs::write(owned.workdir.join("host-output.stderr"), &host_run.stderr)?;
    validate_pi(PiInput {
        workflow: owned.workdir.join("workflow.json"),
        invocation: owned.workdir.join("requested-invocation.json"),
        stdout: owned.workdir.join("host-output.txt"),
        stderr: owned.workdir.join("host-output.stderr"),
        workflow_receipt: owned.workdir.join("workflow-receipt.json"),
        dispatch_receipt: owned.workdir.join("dispatch-evidence.json"),
        package_digest: sha256_file(&routing_bin)?,
        host_version,
        profile: "pi-worker".into(),
        model: "openai/gpt-4o-mini".into(),
        thinking: "low".into(),
        agent_type: "switchloom-pi-worker".into(),
    })?;
    validate_bundle_receipt(&owned)?;
    session.complete(
        true,
        Some("Pi is an isolated external runner and reports advisory dispatch evidence".into()),
    )?;
    println!(
        "Pi live certification passed: {}",
        owned.report_dir.display()
    );
    Ok(())
}

pub(crate) fn run_codex(args: LiveRunArgs) -> Result<()> {
    let host = "codex-openai";
    let owned = OwnedReportRepo::create(&args.report_root, host)?;
    let mut session = CertificationSession::new(&owned, host);
    let routing_bin = absolute_binary(&args.routing_bin)?;
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .context("CODEX_HOME/HOME is unavailable")?;
    let mut external_config = FileSnapshot::capture(codex_home.join("config.toml"))?;
    session.track_restoration(external_config.outcome_tracker());
    session.run_checked(
        command(
            "compile",
            &routing_bin,
            [
                "compile",
                "balanced",
                "--host",
                host,
                "--output",
                "bundle.json",
            ],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    session.run_checked(
        command(
            "apply",
            &routing_bin,
            ["apply", "bundle.json", "--repository", "."],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    let version = session.run_checked(
        command(
            "codex-version",
            Path::new("codex"),
            ["--version"],
            &owned,
            args.timeout,
        ),
        &owned,
    )?;
    let host_version = last_line(&version.stdout, &version.stderr);
    let worker_message = "Inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_IMPLEMENTER_DONE.";
    let reviewer_message = "Independently inspect the generated repository without editing files. End your final answer with SWITCHLOOM_STANDALONE_REVIEWER_DONE.";
    let expected = json!({"package_digest":sha256_file(&routing_bin)?,"host_version":host_version,"children":[{"semantic_role":"implementer","profile":"codex-terra-high","agent_type":"model_routing_terra_high","task_name":"standalone_implementer","canonical_task":"/root/standalone_implementer","model":"gpt-5.6-terra","effort":"high","message_sha256":format!("{:x}",Sha256::digest(worker_message)),"max_message_bytes":512,"completion_contains":"SWITCHLOOM_STANDALONE_IMPLEMENTER_DONE","allow_encrypted_message":true},{"semantic_role":"reviewer","profile":"codex-sol-high","agent_type":"model_routing_sol_high","task_name":"standalone_reviewer","canonical_task":"/root/standalone_reviewer","model":"gpt-5.6-sol","effort":"high","message_sha256":format!("{:x}",Sha256::digest(reviewer_message)),"max_message_bytes":512,"completion_contains":"SWITCHLOOM_STANDALONE_REVIEWER_DONE","allow_encrypted_message":true}]});
    write_json(&owned.workdir.join("expected.json"), &expected)?;
    let prompt = format!(
        "Use the native collaboration spawn_agent tool exactly twice, then wait for both child agents to finish.\n\n\
Your first tool call must be spawn_agent with exactly these fields:\n\
- agent_type: model_routing_terra_high\n\
- task_name: standalone_implementer\n\
- fork_turns: none\n\
- message: {worker_message}\n\n\
Your second tool call must be spawn_agent with exactly these fields:\n\
- agent_type: model_routing_sol_high\n\
- task_name: standalone_reviewer\n\
- fork_turns: none\n\
- message: {reviewer_message}\n\n\
Do not omit agent_type. Do not change either message. Do not pass model or reasoning_effort in either spawn call. Do not call wait_agent or answer before both spawn_agent calls have succeeded.\n\n\
After both children finish, return a short final answer containing:\n\
SWITCHLOOM_CODEX_RUNTIME_EVIDENCE_COMPLETE"
    );
    let trust_override = format!(
        "projects.\"{}\".trust_level=\"trusted\"",
        owned.workdir.display()
    );
    let terra_override = format!(
        "agents.model_routing_terra_high.config_file=\"{}/.codex/agents/model-routing-terra-high.toml\"",
        owned.workdir.display()
    );
    let sol_override = format!(
        "agents.model_routing_sol_high.config_file=\"{}/.codex/agents/model-routing-sol-high.toml\"",
        owned.workdir.display()
    );
    let host_run = run_process(
        &command(
            "codex-host",
            Path::new("codex"),
            [
                "exec",
                "--json",
                "--ignore-user-config",
                "-C",
                owned
                    .workdir
                    .to_str()
                    .context("workdir path is not UTF-8")?,
                "-s",
                "workspace-write",
                "-c",
                "approval_policy=\"never\"",
                "-c",
                &trust_override,
                "-c",
                &terra_override,
                "-c",
                &sol_override,
                "-c",
                "multi_agent_v2.hide_spawn_agent_metadata=false",
                "-c",
                "cli_auth_credentials_store=\"auto\"",
                "-c",
                "mcp_oauth_credentials_store=\"auto\"",
                &prompt,
            ],
            &owned,
            args.timeout,
        ),
        &owned.report_dir,
    )?;
    session.record(host_run.clone())?;
    if let Err(error) = external_config.restore() {
        session.fail(format!("failed to restore external Codex config: {error}"))?;
        return Err(error);
    }
    if !host_run.success() {
        session.fail(format!(
            "Codex host failed with status {:?}{}; external state restored",
            host_run.status,
            if host_run.timed_out {
                " after timeout"
            } else {
                ""
            }
        ))?;
        bail!(
            "Codex host failed with status {:?}{}; external state restored",
            host_run.status,
            if host_run.timed_out {
                " after timeout"
            } else {
                ""
            }
        );
    }
    fs::write(owned.workdir.join("codex-events.jsonl"), &host_run.stdout)?;
    let receipt = super::extract_codex(CodexRawInput {
        events: owned.workdir.join("codex-events.jsonl"),
        workdir: owned.workdir.clone(),
        expected: owned.workdir.join("expected.json"),
        state_db: Some(codex_home.join("state_5.sqlite")),
        sessions_dir: Some(codex_home.join("sessions")),
        archived_sessions_dir: Some(codex_home.join("archived_sessions")),
    })?;
    fs::write(owned.workdir.join("codex-runtime-evidence.json"), receipt)?;
    session.complete(true, None)?;
    println!(
        "Codex live certification passed: {}",
        owned.report_dir.display()
    );
    Ok(())
}

pub(crate) fn run_planr(args: PlanrRunArgs) -> Result<()> {
    let before = repo_identity(&args.protected_planr_root)?;
    let owned = OwnedReportRepo::create(&args.live.report_root, "planr")?;
    let mut session = CertificationSession::new(&owned, "planr");
    let routing_bin = absolute_binary(&args.live.routing_bin)?;
    let db = owned.workdir.join(".planr/planr.sqlite");
    session.run_checked(
        command(
            "planr-init",
            Path::new("planr"),
            [
                "--db",
                db.to_str().context("db path is not UTF-8")?,
                "project",
                "init",
                "Switchloom Planr Certification",
                "--json",
            ],
            &owned,
            args.live.timeout,
        ),
        &owned,
    )?;
    session.run_checked(
        command(
            "compile",
            &routing_bin,
            [
                "compile",
                "balanced",
                "--host",
                "codex-openai",
                "--integration",
                "planr",
                "--output",
                "bundle.json",
            ],
            &owned,
            args.live.timeout,
        ),
        &owned,
    )?;
    session.run_checked(
        command(
            "apply",
            &routing_bin,
            ["apply", "bundle.json", "--repository", "."],
            &owned,
            args.live.timeout,
        ),
        &owned,
    )?;
    for path in [
        ".codex/config.toml",
        ".codex/agents/model-routing-terra-high.toml",
        ".codex/agents/model-routing-sol-high.toml",
        ".planr/agents.toml",
        ".planr/policy.toml",
    ] {
        ensure_file(&owned.workdir.join(path))?;
    }
    session.run_checked(
        command(
            "planr-agents",
            Path::new("planr"),
            ["--db", db.to_str().unwrap(), "agents", "check", "--json"],
            &owned,
            args.live.timeout,
        ),
        &owned,
    )?;
    session.run_checked(
        command(
            "planr-routing",
            Path::new("planr"),
            [
                "--db",
                db.to_str().unwrap(),
                "prompt",
                "routing",
                "--client",
                "codex",
                "--json",
            ],
            &owned,
            args.live.timeout,
        ),
        &owned,
    )?;
    let after = repo_identity(&args.protected_planr_root)?;
    if before != after {
        bail!("protected Planr repository changed during certification");
    }
    session.complete(
        true,
        Some("Planr certification validates generated declarations and routing consumption without mutating protected Planr source/state".into()),
    )?;
    println!(
        "Planr certification passed with protected repository unchanged: {}",
        owned.report_dir.display()
    );
    Ok(())
}

fn command<'a>(
    label: &str,
    program: &Path,
    args: impl IntoIterator<Item = &'a str>,
    owned: &OwnedReportRepo,
    timeout: Duration,
) -> ProcessSpec {
    ProcessSpec {
        label: label.into(),
        program: program.display().to_string(),
        args: args.into_iter().map(str::to_owned).collect(),
        env: BTreeMap::new(),
        cwd: owned.workdir.clone(),
        timeout,
    }
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}
fn absolute_binary(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
    .and_then(|path| {
        if path.is_file() {
            Ok(path)
        } else {
            bail!("routing binary not found at {}", path.display())
        }
    })
}
fn sha256_file(path: &Path) -> Result<String> {
    Ok(format!("sha256:{:x}", Sha256::digest(fs::read(path)?)))
}
fn nonce(prefix: &str) -> String {
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{prefix}-{:x}",
        Sha256::digest(format!("{}-{epoch}", std::process::id()))
    )
}
fn last_line(stdout: &str, stderr: &str) -> String {
    stdout
        .lines()
        .chain(stderr.lines())
        .filter(|line| !line.trim().is_empty())
        .last()
        .unwrap_or("unknown")
        .trim()
        .to_owned()
}
fn ensure_contains_nonce(receipt: &ProcessReceipt, nonce: &str) -> Result<()> {
    if receipt.stdout.contains(nonce) || receipt.stderr.contains(nonce) {
        Ok(())
    } else {
        bail!("live host output did not return correlated nonce")
    }
}
fn ensure_file(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        bail!(
            "expected certification artifact missing: {}",
            path.display()
        )
    }
}
fn validate_bundle_receipt(owned: &OwnedReportRepo) -> Result<()> {
    let receipt = fs::read_to_string(owned.workdir.join("dispatch-evidence.json"))?;
    let bundle = fs::read_to_string(owned.workdir.join("bundle.json"))?;
    validate_dispatch_evidence_json_for_bundle(&receipt, &bundle)
        .map_err(|error| anyhow::anyhow!(error))
}
fn repo_identity(path: &Path) -> Result<String> {
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(path)
        .output()?;
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .current_dir(path)
        .output()?;
    if !head.status.success() || !status.status.success() {
        bail!("protected Planr path is not a readable Git repository");
    }
    Ok(format!(
        "{:x}",
        Sha256::digest([head.stdout, status.stdout].concat())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "switchloom-certification-session-{name}-{}-{}",
            std::process::id(),
            nonce(name)
        ))
    }

    #[test]
    fn exit_seven_report_preserves_status_and_explicit_restoration() {
        let root = report_root("exit-seven");
        let owned = OwnedReportRepo::create(&root, "test").unwrap();
        let state = root.join("external-config.toml");
        fs::write(&state, "original").unwrap();
        let mut session = CertificationSession::new(&owned, "test");
        {
            let mut snapshot = FileSnapshot::capture(&state).unwrap();
            session.track_restoration(snapshot.outcome_tracker());
            fs::write(&state, "temporary").unwrap();
            let result = session.run_checked(
                ProcessSpec {
                    label: "exit-seven".into(),
                    program: "sh".into(),
                    args: vec!["-c".into(), "exit 7".into()],
                    env: BTreeMap::new(),
                    cwd: owned.workdir.clone(),
                    timeout: Duration::from_secs(1),
                },
                &owned,
            );
            assert!(result.is_err());
            snapshot.restore().unwrap();
        }
        drop(session);
        let report: Value = serde_json::from_slice(
            &fs::read(owned.report_dir.join("certification-report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(report.pointer("/commands/0/status"), Some(&json!(7)));
        assert_eq!(
            report.pointer("/restoration/status"),
            Some(&json!("restored_explicitly"))
        );
        assert_eq!(fs::read_to_string(state).unwrap(), "original");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn timeout_report_records_timeout_and_drop_restoration() {
        let root = report_root("timeout");
        let owned = OwnedReportRepo::create(&root, "test").unwrap();
        let state = root.join("external-config.toml");
        fs::write(&state, "original").unwrap();
        let mut session = CertificationSession::new(&owned, "test");
        {
            let snapshot = FileSnapshot::capture(&state).unwrap();
            session.track_restoration(snapshot.outcome_tracker());
            fs::write(&state, "temporary").unwrap();
            let result = session.run_checked(
                ProcessSpec {
                    label: "timeout".into(),
                    program: "sh".into(),
                    args: vec!["-c".into(), "sleep 2".into()],
                    env: BTreeMap::new(),
                    cwd: owned.workdir.clone(),
                    timeout: Duration::from_millis(50),
                },
                &owned,
            );
            assert!(result.is_err());
        }
        drop(session);
        let report: Value = serde_json::from_slice(
            &fs::read(owned.report_dir.join("certification-report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(report.pointer("/commands/0/timed_out"), Some(&json!(true)));
        assert_eq!(report.pointer("/commands/0/status"), Some(&Value::Null));
        assert_eq!(
            report.pointer("/restoration/status"),
            Some(&json!("restored_by_drop_fallback"))
        );
        assert_eq!(fs::read_to_string(state).unwrap(), "original");
        fs::remove_dir_all(root).unwrap();
    }
}
