mod linux;
mod report;

use std::io::{self, Write};

pub use report::{Capability, DoctorReport};

#[derive(Debug, thiserror::Error)]
pub enum DoctorError {
    #[error("could not serialize diagnostics: {0}")]
    Json(#[from] serde_json::Error),
    #[error("could not write diagnostics: {0}")]
    Output(#[from] io::Error),
}

pub fn run(json: bool) -> Result<(), DoctorError> {
    let report = linux::collect();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    if json {
        serde_json::to_writer(&mut output, &report)?;
        writeln!(output)?;
    } else {
        writeln!(output, "Drunk read-only Linux diagnostics")?;
        writeln!(
            output,
            "Comparison baseline: Sober public requirements (Linux x86-64, kernel >= 5.11)."
        )?;
        writeln!(
            output,
            "Statuses describe individual hints only; Drunk runtime feasibility remains provisional no-go."
        )?;
        writeln!(
            output,
            "These results do not establish Roblox or Android runtime support."
        )?;
        for (name, capability) in report.rows() {
            let (status, detail) = capability.status_and_detail();
            writeln!(output, "{name:<18} {status:<11} {detail}")?;
        }
    }
    Ok(())
}
