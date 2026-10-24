mod cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::parse().command {
        cli::Command::Doctor { json: _ } => {
            eprintln!("doctor implementation pending");
            ExitCode::from(2)
        }
    }
}
