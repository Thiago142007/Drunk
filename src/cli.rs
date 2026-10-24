use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "drunk", version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Doctor {
        #[arg(long)]
        json: bool,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}
