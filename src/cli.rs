use crate::config::Config;
use crate::dns::spawn::start_dns_server;
use clap::builder::Styles;
use clap::{ColorChoice, Parser, Subcommand};
use eyre::{Error, Result};
use std::fmt::Write;

#[derive(Parser, Debug)]
#[command(author="terrastack", version, about="Terrastack Director is a lightweight, high-performance DNS forwarder and proxy built for modern cloud environments.", long_about = None, color = ColorChoice::Always, styles = Styles::styled())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Generate {
        #[arg(short, long, value_enum, default_value_t = ConfigFormat::Yaml)]
        format: ConfigFormat,
    },
    Run {
        #[arg(short, long)]
        config_file: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ConfigFormat {
    Env,
    Yaml,
    Toml,
}

pub async fn handle_commands(command: &Commands) -> Result<(), Error> {
    match command {
        Commands::Generate { format } => {
            let default_config = Config::default();
            match format {
                ConfigFormat::Env => {
                    let mut env_output = String::new();
                    writeln!(
                        &mut env_output,
                        "DIRECTOR_LISTEN_HTTP={}",
                        default_config.listen.http
                    )?;
                    writeln!(
                        &mut env_output,
                        "DIRECTOR_LISTEN_UDP={}",
                        default_config.listen.udp
                    )?;
                    writeln!(
                        &mut env_output,
                        "DIRECTOR_LISTEN_TCP={}",
                        default_config.listen.tcp
                    )?;
                    writeln!(
                        &mut env_output,
                        "DIRECTOR_LISTEN_TLS={}",
                        default_config.listen.tls
                    )?;
                    writeln!(
                        &mut env_output,
                        "DIRECTOR_UPSTREAMS={}",
                        default_config.upstreams.join(",")
                    )?;
                    writeln!(
                        &mut env_output,
                        "DIRECTOR_CACHE_ENABLED={}",
                        default_config.cache.enabled
                    )?;
                    writeln!(
                        &mut env_output,
                        "DIRECTOR_CACHE_TTL={}",
                        default_config.cache.ttl
                    )?;
                    print!("{}", env_output);
                }
                ConfigFormat::Yaml => {
                    let yaml_config = serde_yaml::to_string(&default_config)?;
                    println!("{}", yaml_config);
                }
                ConfigFormat::Toml => {
                    let toml_config = toml::to_string(&default_config)?;
                    println!("{}", toml_config);
                }
            }
        }
        Commands::Run { config_file } => {
            tracing::info!("Running director with config file: {:?}", config_file);
            let conf = crate::config::load_config(config_file.as_deref())?;
            tracing::info!("Configuration: {:?}", conf);
            let server_handle = start_dns_server(conf);
            server_handle
                .join()
                .map_err(|e| eyre::eyre!("DNS server thread panicked: {:?}", e))??;
        }
    }
    Ok(())
}
