use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing_subscriber::{EnvFilter, fmt};

mod orchestrator;
mod providers;
mod vcs;
mod workspace;

use orchestrator::{Orchestrator, OrchestratorConfig};

#[derive(Parser, Debug)]
#[command(
    name = "red-green-refactor",
    version,
    about = "Orchestrate TDD with LLM roles: tester, implementor, refactorer."
)]
struct Cli {
    /// Path to the kata project (a cargo project recommended)
    #[arg(long, default_value = ".")]
    project: PathBuf,

    /// Path to YAML config with provider settings
    #[arg(long)]
    config: Option<PathBuf>,

    /// Increase verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the Red-Green-Refactor loop once (tester -> implementor -> refactorer)
    RunOnce,
    /// Run continuously until stopped (Ctrl-C)
    Run,
    /// Initialize a sample config file
    InitConfig {
        #[arg(long, default_value = "red-green-refactor.yaml")]
        out: PathBuf,
    },
}

fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = fmt().with_env_filter(filter).without_time().try_init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command.unwrap_or(Commands::RunOnce) {
        Commands::InitConfig { out } => {
            let path = if out.is_dir() {
                out.join("red-green-refactor.yaml")
            } else {
                out
            };
            let cfg = OrchestratorConfig::example();
            let s = serde_yaml::to_string(&cfg)?;
            std::fs::write(&path, s)?;
            println!("Wrote sample config to {}", path.display());
            Ok(())
        }
        Commands::RunOnce => run(&cli.project, &cli.config, false).await,
        Commands::Run => run(&cli.project, &cli.config, true).await,
    }
}

async fn run(project: &Path, config_path: &Option<PathBuf>, continuous: bool) -> Result<()> {
    let cfg = orchestrator::load_orchestrator_config(config_path.as_ref())?;
    let mut orch = Orchestrator::new(project.to_path_buf(), cfg).await?;

    if continuous {
        loop {
            orch.red_green_refactor_cycle().await?;
        }
    } else {
        orch.red_green_refactor_cycle().await
    }
}
