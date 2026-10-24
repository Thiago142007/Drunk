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
    /// Show a low-rate Android screen preview in a desktop window with basic input.
    View,
    /// Measure raw H.264 output from Android's screenrecord command.
    StreamProbe {
        /// Number of seconds to capture (1-30).
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=30))]
        seconds: u8,
    },
    /// Open an experimental continuous Rust-decoded Android H.264 preview.
    Stream {
        /// Maximum stream duration in seconds (Android screenrecord is time-limited).
        #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u16).range(1..=180))]
        seconds: u16,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}
