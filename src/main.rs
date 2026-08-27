mod doctor;
mod info;
mod install;
mod util;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "lf",
    version,
    about = "Lightweight system info + install cross-platform shells (nushell/brush) for a unified AI-agent environment"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show system info (fastfetch-like) as text or machine-readable JSON
    Info {
        #[arg(long, help = "output JSON (agent-readable)")]
        json: bool,
    },
    /// Install shells. Default: nushell + brush
    Install {
        #[arg(value_name = "SHELL", default_values = ["nu", "brush"])]
        targets: Vec<String>,
        #[arg(long, help = "reinstall/upgrade even if already present")]
        force: bool,
        #[arg(long, help = "skip editing the OS PATH (default: add ~/.lf/bin)")]
        no_path: bool,
    },
    /// Check nushell + brush + common tools are ready
    Doctor {
        #[arg(long, help = "output JSON (agent-readable)")]
        json: bool,
    },
    /// One-shot: install both shells, then run the doctor check
    Setup {
        #[arg(long, help = "force reinstall")]
        force: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Info { json } => info::run(json),
        Cmd::Install {
            targets,
            force,
            no_path,
        } => install::run(&targets, force, !no_path),
        Cmd::Doctor { json } => doctor::run(json),
        Cmd::Setup { force } => {
            install::run(&[], force, true)?;
            doctor::run(false)
        }
    }
}
