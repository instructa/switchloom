use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

mod certify;
mod release;

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Unpublished Switchloom maintainer tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run bounded host or integration certification.
    Certify(Box<CertifyArgs>),
    /// Run deterministic offline verification.
    Verify(VerifyArgs),
    /// Prepare and verify release artifacts without publishing them.
    Release(Box<ReleaseArgs>),
}

#[derive(Debug, Args)]
struct CertifyArgs {
    #[command(subcommand)]
    command: CertifyCommand,
}

#[derive(Debug, Subcommand)]
enum CertifyCommand {
    Codex(CodexArgs),
    Cursor(CursorArgs),
    Opencode(OpencodeArgs),
    Pi(PiArgs),
    Planr(PlanrArgs),
}

#[derive(Debug, Args)]
struct CodexArgs {
    /// Validate an extracted Codex persisted-runtime receipt.
    #[arg(long)]
    receipt: Option<PathBuf>,
    /// Bind the receipt to an independently recorded expected dispatch.
    #[arg(long)]
    expect: Option<PathBuf>,
    /// Extract and validate evidence directly from raw Codex runtime state.
    #[arg(long, conflicts_with = "receipt")]
    events: Option<PathBuf>,
    #[arg(long, requires = "events")]
    workdir: Option<PathBuf>,
    #[arg(long, requires = "events")]
    state_db: Option<PathBuf>,
    #[arg(long, requires = "events")]
    sessions_dir: Option<PathBuf>,
    #[arg(long, requires = "events")]
    archived_sessions_dir: Option<PathBuf>,
    #[arg(long, default_value = "target/debug/model-routing")]
    routing_bin: PathBuf,
    #[arg(long, default_value = "reports/native-host-certification")]
    report_root: PathBuf,
    #[arg(long, default_value_t = 180)]
    timeout_seconds: u64,
    /// Run the Codex exact-version fixture that must fail closed on missing child evidence.
    #[arg(long, conflicts_with_all = ["receipt", "events"])]
    negative_fixture: bool,
}

#[derive(Debug, Args)]
struct CursorArgs {
    /// Validate a Cursor dispatch receipt against its compiled bundle.
    #[arg(long)]
    receipt: Option<PathBuf>,
    #[arg(long, requires = "receipt")]
    bundle: Option<PathBuf>,
    #[arg(long, default_value = "cursor-openai")]
    host: String,
    #[arg(long, default_value = "target/debug/model-routing")]
    routing_bin: PathBuf,
    #[arg(long, default_value = "reports/native-host-certification")]
    report_root: PathBuf,
    #[arg(long, default_value_t = 180)]
    timeout_seconds: u64,
}

#[derive(Debug, Args)]
struct OpencodeArgs {
    #[arg(long)]
    jsonl: Option<PathBuf>,
    #[arg(long)]
    invocation: Option<PathBuf>,
    #[arg(long)]
    receipt: Option<PathBuf>,
    #[arg(long)]
    package_digest: Option<String>,
    #[arg(long)]
    host_version: Option<String>,
    #[arg(long)]
    profile: Option<String>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    variant: Option<String>,
    #[arg(long)]
    worker: Option<String>,
    #[arg(long, default_value = "target/debug/model-routing")]
    routing_bin: PathBuf,
    #[arg(long, default_value = "reports/native-host-certification")]
    report_root: PathBuf,
    #[arg(long, default_value_t = 180)]
    timeout_seconds: u64,
}

#[derive(Debug, Args)]
struct PiArgs {
    #[arg(long)]
    workflow: Option<PathBuf>,
    #[arg(long)]
    invocation: Option<PathBuf>,
    #[arg(long)]
    stdout: Option<PathBuf>,
    #[arg(long)]
    stderr: Option<PathBuf>,
    #[arg(long)]
    workflow_receipt: Option<PathBuf>,
    #[arg(long)]
    dispatch_receipt: Option<PathBuf>,
    #[arg(long)]
    package_digest: Option<String>,
    #[arg(long)]
    host_version: Option<String>,
    #[arg(long)]
    profile: Option<String>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    thinking: Option<String>,
    #[arg(long)]
    agent_type: Option<String>,
    #[arg(long, default_value = "target/debug/model-routing")]
    routing_bin: PathBuf,
    #[arg(long, default_value = "reports/native-host-certification")]
    report_root: PathBuf,
    #[arg(long, default_value_t = 180)]
    timeout_seconds: u64,
}

#[derive(Debug, Args)]
struct PlanrArgs {
    #[arg(long, default_value = "target/debug/model-routing")]
    routing_bin: PathBuf,
    #[arg(long, default_value = "reports/native-host-certification")]
    report_root: PathBuf,
    #[arg(long, default_value_t = 180)]
    timeout_seconds: u64,
    #[arg(long, default_value = "../planr")]
    protected_planr_root: PathBuf,
}

#[derive(Debug, Args)]
struct VerifyArgs {
    #[command(subcommand)]
    command: VerifyCommand,
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum VerifyCommand {
    Offline,
}

#[derive(Debug, Args)]
struct ReleaseArgs {
    #[command(subcommand)]
    command: ReleaseCommand,
}

#[derive(Debug, Subcommand)]
enum ReleaseCommand {
    Prepare(ReleasePrepareArgs),
    Verify(ReleaseVerifyArgs),
    Package(ReleasePackageArgs),
}

#[derive(Debug, Args)]
struct ReleasePrepareArgs {
    #[arg(long)]
    version: Option<String>,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    allow_dirty: bool,
}

#[derive(Debug, Args)]
struct ReleaseVerifyArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    inventory_only: bool,
    #[arg(long)]
    contract_only: bool,
    #[arg(long)]
    require_provenance: bool,
    #[arg(long)]
    expected_tag: Option<String>,
}

#[derive(Debug, Args)]
struct ReleasePackageArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    cargo_target: Option<String>,
    #[arg(long)]
    stage_npm: bool,
    #[arg(long)]
    assemble_provenance: bool,
    #[arg(long)]
    aggregate_checksums_dir: Option<PathBuf>,
    #[arg(long)]
    provenance_dir: Option<PathBuf>,
    #[arg(long, default_value = "local")]
    runner: String,
    #[arg(long)]
    git_sha: Option<String>,
    #[arg(long, default_value = "local-reproducible")]
    built_at: String,
    #[arg(long, default_value = "xtask-release")]
    generated_by: String,
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let command = match cli.command {
        Command::Certify(args) => match args.command {
            CertifyCommand::Codex(args) => {
                if let Some(receipt) = args.receipt {
                    certify::validate_codex(&receipt, args.expect.as_deref())?;
                    println!("codex runtime evidence validation passed");
                    return Ok(());
                }
                if let Some(events) = args.events {
                    let receipt = certify::extract_codex(certify::CodexRawInput {
                        events,
                        workdir: args.workdir.ok_or_else(|| {
                            anyhow::anyhow!("--workdir is required with --events")
                        })?,
                        expected: args
                            .expect
                            .ok_or_else(|| anyhow::anyhow!("--expect is required with --events"))?,
                        state_db: args.state_db,
                        sessions_dir: args.sessions_dir,
                        archived_sessions_dir: args.archived_sessions_dir,
                    })?;
                    println!("{receipt}");
                    return Ok(());
                }
                let live_args = certify::LiveRunArgs::new(
                    args.routing_bin,
                    args.report_root,
                    args.timeout_seconds,
                );
                if args.negative_fixture {
                    certify::run_live_codex_negative_fixture(live_args)?;
                } else {
                    certify::run_live_codex(live_args)?;
                }
                return Ok(());
            }
            CertifyCommand::Cursor(args) => {
                if let (Some(receipt), Some(bundle)) = (args.receipt, args.bundle) {
                    certify::validate_cursor(&receipt, &bundle)?;
                    println!("cursor runtime evidence validation passed");
                    return Ok(());
                }
                certify::run_live_native(
                    &args.host,
                    certify::LiveRunArgs::new(
                        args.routing_bin,
                        args.report_root,
                        args.timeout_seconds,
                    ),
                )?;
                return Ok(());
            }
            CertifyCommand::Opencode(args) => {
                if args.jsonl.is_some() {
                    certify::validate_opencode(args.try_into()?)?;
                    println!("opencode runtime evidence validated");
                    return Ok(());
                }
                certify::run_live_opencode(certify::LiveRunArgs::new(
                    args.routing_bin,
                    args.report_root,
                    args.timeout_seconds,
                ))?;
                return Ok(());
            }
            CertifyCommand::Pi(args) => {
                if args.workflow.is_some() {
                    certify::validate_pi(args.try_into()?)?;
                    println!("pi runtime evidence validated");
                    return Ok(());
                }
                certify::run_live_pi(certify::LiveRunArgs::new(
                    args.routing_bin,
                    args.report_root,
                    args.timeout_seconds,
                ))?;
                return Ok(());
            }
            CertifyCommand::Planr(args) => {
                certify::run_planr(certify::PlanrRunArgs {
                    live: certify::LiveRunArgs::new(
                        args.routing_bin,
                        args.report_root,
                        args.timeout_seconds,
                    ),
                    protected_planr_root: args.protected_planr_root,
                })?;
                return Ok(());
            }
        },
        Command::Verify(args) => match args.command {
            VerifyCommand::Offline => "verify offline",
        },
        Command::Release(args) => match args.command {
            ReleaseCommand::Prepare(args) => {
                release::prepare(release::PrepareOptions {
                    root: args.root,
                    version: args.version,
                    allow_dirty: args.allow_dirty,
                })?;
                return Ok(());
            }
            ReleaseCommand::Verify(args) => {
                release::verify(release::VerifyOptions {
                    root: args.root,
                    inventory_only: args.inventory_only,
                    contract_only: args.contract_only,
                    require_provenance: args.require_provenance,
                    expected_tag: args.expected_tag,
                })?;
                return Ok(());
            }
            ReleaseCommand::Package(args) => {
                release::package(release::PackageOptions {
                    root: args.root,
                    target: args.target,
                    cargo_target: args.cargo_target,
                    stage_npm: args.stage_npm,
                    assemble_provenance: args.assemble_provenance,
                    aggregate_checksums_dir: args.aggregate_checksums_dir,
                    provenance_dir: args.provenance_dir,
                    runner: args.runner,
                    git_sha: args.git_sha,
                    built_at: args.built_at,
                    generated_by: args.generated_by,
                })?;
                return Ok(());
            }
        },
    };

    bail!(
        "internal command root `{command}` is reserved but not implemented; later migration slices must transfer one canonical owner before enabling it"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn internal_command_tree_matches_the_ownership_contract() {
        let command = Cli::command();
        let roots = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name())
            .collect::<Vec<_>>();
        assert_eq!(roots, ["certify", "verify", "release"]);
    }

    #[test]
    fn ownership_contract_classifies_every_public_internal_and_transfer_command() {
        let ownership: toml::Value = toml::from_str(include_str!("../command-ownership.toml"))
            .expect("ownership contract must be valid TOML");

        let commands = |section: &str| {
            ownership[section]["commands"]
                .as_array()
                .expect("command section must be an array")
                .iter()
                .map(|value| value.as_str().expect("commands must be strings"))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            commands("public_cli"),
            [
                "policy",
                "compile",
                "inspect",
                "preview",
                "apply",
                "update",
                "status",
                "rollback",
                "uninstall",
                "doctor",
            ]
        );
        assert_eq!(
            commands("internal_cli"),
            [
                "certify codex",
                "certify cursor",
                "certify opencode",
                "certify pi",
                "certify planr",
                "verify offline",
                "release prepare",
                "release verify",
                "release package",
            ]
        );

        let mut transfers = ownership["transfers"]
            .as_table()
            .expect("transfers must be a table")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        transfers.sort_unstable();
        assert_eq!(
            transfers,
            [
                "catalog",
                "certify",
                "evaluate",
                "evidence validate",
                "probe",
                "registry",
            ]
        );
    }
}
