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
    /// Start a Rust H.264 preview and forward Android 13 playback audio together.
    Session {
        /// Maximum session duration in seconds (screenrecord is time-limited).
        #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u16).range(1..=180))]
        seconds: u16,
    },
    /// Forward Android playback audio to the host using the optional scrcpy helper.
    Audio,
}

pub fn parse() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command, RedroidCommand};
    use clap::Parser;

    #[test]
    fn redroid_session_defaults_to_a_thirty_second_preview() {
        let cli = Cli::try_parse_from(["drunk", "redroid", "session"]).unwrap();

        assert!(matches!(
            cli.command,
            Command::Redroid {
                command: RedroidCommand::Session { seconds: 30 }
            }
        ));

        let cli = Cli::try_parse_from(["drunk", "redroid", "session", "--seconds", "60"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Redroid {
                command: RedroidCommand::Session { seconds: 60 }
            }
        ));
    }
}
