mod cli;
mod config;
mod dns;

use clap::Parser;
use eyre::{Error, Result};

#[tokio::main]
async fn main() -> Result<(), Error> {
    const LOGO: &str = include_str!("director.ascii");
    println!();
    for line in LOGO.lines() {
        println!(" {line}");
    }
    println!();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    rustls::crypto::ring::default_provider().install_default().map_err(|e| eyre::eyre!("Failed to install rustls crypto provider: {:?}", e))?;

    let cli = cli::Cli::parse();
    cli::handle_commands(&cli.command).await?;

    Ok(())
}
