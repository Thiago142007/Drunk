mod cli;
mod doctor;
mod redroid;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::parse().command {
        cli::Command::Redroid { command } => match redroid::run(command) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("redroid: {error}");
                ExitCode::FAILURE
            }
        },
        cli::Command::Doctor { json } => match doctor::run(json) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("doctor: {error}");
                ExitCode::FAILURE
            }
        },
    }
}
