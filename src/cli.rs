use crate::{status_repository, uninstall_repository};
use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "model-routing", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Choose the next capability and its configured model. Reads JSON from stdin.
    Route,
    /// Prepare a capability-routed message for an existing Codex task. Reads JSON from stdin.
    Handoff,
    /// Inspect files managed by an existing Switchloom installation.
    Status(RepositoryArgs),
    /// Remove an existing installation, preserving user-modified files.
    Uninstall(RepositoryArgs),
}

#[derive(Args)]
struct RepositoryArgs {
    #[arg(long, default_value = ".")]
    repository: PathBuf,
}

pub fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Route => {
            let bytes = read_input(crate::decision::MAX_ROUTE_INPUT_BYTES)?;
            let input: crate::decision::RouteRequest = serde_json::from_slice(&bytes)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&crate::decision::route_task(&input)?)?
            );
        }
        Command::Handoff => {
            let bytes = read_input(crate::handoff::MAX_HANDOFF_INPUT_BYTES)?;
            let input = serde_json::from_slice(&bytes)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&crate::handoff::prepare_handoff(&input)?)?
            );
        }
        Command::Status(args) => println!(
            "{}",
            serde_json::to_string_pretty(&status_repository(&args.repository)?)?
        ),
        Command::Uninstall(args) => println!(
            "{}",
            serde_json::to_string_pretty(&uninstall_repository(&args.repository)?)?
        ),
    }
    Ok(())
}

fn read_input(limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    io::stdin().take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        bail!("input exceeds {limit} bytes");
    }
    Ok(bytes)
}
