mod cli;
mod doctor;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::parse().command {
        cli::Command::Doctor { json } => match doctor::run(json) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("doctor: {error}");
                ExitCode::FAILURE
            }
        },
    }
}
