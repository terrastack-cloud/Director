mod cli;
mod config;
mod dns;

use clap::Parser;
use eyre::{Error, Result};

#[tokio::main]
async fn main() -> Result<(), Error> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let cli = cli::Cli::parse();
    cli::handle_commands(&cli.command).await?;

    Ok(())
}
