use crate::{
    Integration, apply_bundle_file, apply_saved_setup, apply_setup_config_file, apply_setup_recipe,
    compile_json, inspect_bundle_json, list_policies, prepare_saved_setup,
    prepare_setup_config_file, prepare_setup_recipe, preview_bundle_file, preview_prepared_setup,
    preview_saved_setup, preview_setup_config_file, preview_setup_recipe,
    probe_host_with_repository, rollback_repository, show_policy, status_repository,
    uninstall_repository, update_bundle_file, update_saved_setup, update_setup_config_file,
    update_setup_recipe,
};
use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "model-routing", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Policy(PolicyArgs),
    Compile(CompileArgs),
    Inspect(FileArgs),
    Preview(LifecycleSourceArgs),
    Apply(LifecycleApplyArgs),
    Update(LifecycleSourceArgs),
    Status(RepositoryArgs),
    Uninstall(RepositoryArgs),
    Rollback(RepositoryArgs),
    /// Check host availability and version before previewing or applying setup.
    Doctor(ProbeArgs),
}

#[derive(Args)]
struct PolicyArgs {
    #[command(subcommand)]
    command: PolicyCommand,
}

#[derive(Subcommand)]
enum PolicyCommand {
    List,
    Show(PolicySelector),
}

#[derive(Args)]
struct PolicySelector {
    policy: String,
    #[arg(long)]
    host: String,
}

#[derive(Args)]
struct CompileArgs {
    policy: String,
    #[arg(long)]
    host: String,
    #[arg(long, value_enum, default_value_t = CliIntegration::Standalone)]
    integration: CliIntegration,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliIntegration {
    Standalone,
    Planr,
}

impl From<CliIntegration> for Integration {
    fn from(value: CliIntegration) -> Self {
        match value {
            CliIntegration::Standalone => Self::Standalone,
            CliIntegration::Planr => Self::Planr,
        }
    }
}

#[derive(Args)]
struct ProbeArgs {
    host: String,
    #[arg(long)]
    command: Option<String>,
    #[arg(long, default_value = ".")]
    repository: PathBuf,
}

#[derive(Args)]
struct FileArgs {
    file: PathBuf,
}

#[derive(Args)]
struct LifecycleSourceArgs {
    bundle: Option<PathBuf>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    recipe: Option<String>,
    #[arg(long, default_value = ".")]
    repository: PathBuf,
}

#[derive(Args)]
struct LifecycleApplyArgs {
    #[command(flatten)]
    source: LifecycleSourceArgs,
    #[arg(long)]
    yes: bool,
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
        Command::Policy(args) => match args.command {
            PolicyCommand::List => println!("{}", serde_json::to_string_pretty(&list_policies()?)?),
            PolicyCommand::Show(selector) => println!(
                "{}",
                serde_json::to_string_pretty(&show_policy(&selector.policy, &selector.host)?)?
            ),
        },
        Command::Compile(args) => {
            let output = compile_json(&args.policy, &args.host, args.integration.into())?;
            if let Some(path) = args.output {
                fs::write(path, output)?;
            } else {
                print!("{output}");
            }
        }
        Command::Inspect(args) => {
            let current = fs::read_to_string(args.file)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&inspect_bundle_json(&current)?)?
            );
        }
        Command::Preview(args) => println!(
            "{}",
            serde_json::to_string_pretty(&preview_lifecycle_source(&args)?)?
        ),
        Command::Apply(args) => {
            if lifecycle_source_kind(&args.source)? == LifecycleSourceKind::Bundle {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&apply_lifecycle_source(&args.source)?)?
                );
            } else {
                let prepared = prepare_lifecycle_source(&args.source)?;
                let preview = preview_prepared_setup(&args.source.repository, &prepared)?;
                eprintln!("{}", serde_json::to_string_pretty(&preview)?);
                confirm_setup_apply(args.yes)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&crate::apply_prepared_setup(
                        &args.source.repository,
                        &prepared,
                        &preview,
                    )?)?
                );
            }
        }
        Command::Update(args) => println!(
            "{}",
            serde_json::to_string_pretty(&update_lifecycle_source(&args)?)?
        ),
        Command::Status(args) => println!(
            "{}",
            serde_json::to_string_pretty(&status_repository(&args.repository)?)?
        ),
        Command::Uninstall(args) => println!(
            "{}",
            serde_json::to_string_pretty(&uninstall_repository(&args.repository)?)?
        ),
        Command::Rollback(args) => println!(
            "{}",
            serde_json::to_string_pretty(&rollback_repository(&args.repository)?)?
        ),
        Command::Doctor(args) => println!(
            "{}",
            serde_json::to_string_pretty(&probe_host_with_repository(
                &args.host,
                args.command.as_deref(),
                &args.repository,
            )?)?
        ),
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleSourceKind {
    Bundle,
    Config,
    Recipe,
    SavedConfig,
}

fn lifecycle_source_kind(args: &LifecycleSourceArgs) -> Result<LifecycleSourceKind> {
    let selected = [
        args.bundle.is_some(),
        args.config.is_some(),
        args.recipe.is_some(),
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count();
    if selected > 1 {
        bail!("choose only one of bundle, --config, or --recipe");
    }
    Ok(if args.bundle.is_some() {
        LifecycleSourceKind::Bundle
    } else if args.config.is_some() {
        LifecycleSourceKind::Config
    } else if args.recipe.is_some() {
        LifecycleSourceKind::Recipe
    } else {
        LifecycleSourceKind::SavedConfig
    })
}

fn preview_lifecycle_source(args: &LifecycleSourceArgs) -> Result<crate::LifecycleReport> {
    Ok(match lifecycle_source_kind(args)? {
        LifecycleSourceKind::Bundle => preview_bundle_file(
            &args.repository,
            args.bundle.as_ref().expect("bundle source checked"),
        ),
        LifecycleSourceKind::Config => preview_setup_config_file(
            &args.repository,
            args.config.as_ref().expect("config source checked"),
        ),
        LifecycleSourceKind::Recipe => preview_setup_recipe(
            &args.repository,
            args.recipe.as_deref().expect("recipe checked"),
        ),
        LifecycleSourceKind::SavedConfig => preview_saved_setup(&args.repository),
    }?)
}

fn apply_lifecycle_source(args: &LifecycleSourceArgs) -> Result<crate::LifecycleReport> {
    Ok(match lifecycle_source_kind(args)? {
        LifecycleSourceKind::Bundle => apply_bundle_file(
            &args.repository,
            args.bundle.as_ref().expect("bundle source checked"),
        ),
        LifecycleSourceKind::Config => apply_setup_config_file(
            &args.repository,
            args.config.as_ref().expect("config source checked"),
        ),
        LifecycleSourceKind::Recipe => apply_setup_recipe(
            &args.repository,
            args.recipe.as_deref().expect("recipe checked"),
        ),
        LifecycleSourceKind::SavedConfig => apply_saved_setup(&args.repository),
    }?)
}

fn prepare_lifecycle_source(args: &LifecycleSourceArgs) -> Result<crate::PreparedSetupLifecycle> {
    Ok(match lifecycle_source_kind(args)? {
        LifecycleSourceKind::Bundle => bail!("bundle lifecycle source cannot be prepared as setup"),
        LifecycleSourceKind::Config => {
            prepare_setup_config_file(args.config.as_ref().expect("config source checked"))
        }
        LifecycleSourceKind::Recipe => {
            prepare_setup_recipe(args.recipe.as_deref().expect("recipe checked"))
        }
        LifecycleSourceKind::SavedConfig => prepare_saved_setup(&args.repository),
    }?)
}

fn update_lifecycle_source(args: &LifecycleSourceArgs) -> Result<crate::LifecycleReport> {
    Ok(match lifecycle_source_kind(args)? {
        LifecycleSourceKind::Bundle => update_bundle_file(
            &args.repository,
            args.bundle.as_ref().expect("bundle source checked"),
        ),
        LifecycleSourceKind::Config => update_setup_config_file(
            &args.repository,
            args.config.as_ref().expect("config source checked"),
        ),
        LifecycleSourceKind::Recipe => update_setup_recipe(
            &args.repository,
            args.recipe.as_deref().expect("recipe checked"),
        ),
        LifecycleSourceKind::SavedConfig => update_saved_setup(&args.repository),
    }?)
}

fn confirm_setup_apply(yes: bool) -> Result<()> {
    if yes {
        return Ok(());
    }
    if !atty_stdin() {
        bail!("setup apply requires --yes when stdin is not interactive");
    }
    eprint!("Apply these repository-local setup changes? Type yes to continue: ");
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim() != "yes" {
        bail!("setup apply cancelled");
    }
    Ok(())
}

fn atty_stdin() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn public_command_surface_is_the_v0_3_0_contract() {
        let command = Cli::command();
        let actual = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name())
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            [
                "policy",
                "compile",
                "inspect",
                "preview",
                "apply",
                "update",
                "status",
                "uninstall",
                "rollback",
                "doctor",
            ]
        );
    }

    #[test]
    fn public_help_does_not_expose_internal_command_roots() {
        let mut help = Vec::new();
        Cli::command().write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();
        for internal in [
            "certify", "evidence", "probe", "evaluate", "catalog", "registry",
        ] {
            assert!(!help.contains(internal), "public help exposed {internal}");
        }
    }
}
