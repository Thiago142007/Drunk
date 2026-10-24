use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "drunk", version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Control the experimental external Android runtime.
    Redroid {
        #[command(subcommand)]
        command: RedroidCommand,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum RedroidCommand {
    /// Start a privileged redroid container with localhost-only ADB.
    Start {
        /// Explicitly allow Docker's privileged mode for this invocation.
        #[arg(long)]
        allow_privileged: bool,
    },
    /// Show the container state, or report that it is absent.
    Status,
    /// Stop the container while preserving its Android data volume.
    Stop,
    /// Open Android Settings through ADB (requires a running runtime).
    Demo,
}

pub fn parse() -> Cli {
    Cli::parse()
}
