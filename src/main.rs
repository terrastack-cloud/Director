mod cli;
mod config;

use clap::Parser;
use eyre::{Error, Result};

fn main() -> Result<(), Error> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let cli = cli::Cli::parse();
    cli::handle_commands(&cli.command)?;

    Ok(())
}
