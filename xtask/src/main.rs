use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

mod release;

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Unpublished Switchloom maintainer tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Prepare and verify release artifacts without publishing them.
    Release(Box<ReleaseArgs>),
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
    match cli.command {
        Command::Release(args) => match args.command {
            ReleaseCommand::Prepare(args) => {
                release::prepare(release::PrepareOptions {
                    root: args.root,
                    version: args.version,
                    allow_dirty: args.allow_dirty,
                })?;
                Ok(())
            }
            ReleaseCommand::Verify(args) => {
                release::verify(release::VerifyOptions {
                    root: args.root,
                    inventory_only: args.inventory_only,
                    contract_only: args.contract_only,
                    require_provenance: args.require_provenance,
                    expected_tag: args.expected_tag,
                })?;
                Ok(())
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
                Ok(())
            }
        },
    }
}
