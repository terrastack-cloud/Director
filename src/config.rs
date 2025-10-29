use eyre::Report;
use figment::{
    Figment,
    providers::{Env, Format, Toml, Yaml},
};
use std::path::Path;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub listen: String,
    pub upstreams: Vec<String>,
    pub cache: Cache,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Cache {
    pub enabled: bool,
    pub ttl: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen: "https://127.0.0.1:443".to_string(),
            upstreams: vec![
                "tcp://1.1.1.1".to_string(),
                "udp://8.8.8.8".to_string(),
            ],
            cache: Cache::default(),
        }
    }
}

impl Default for Cache {
    fn default() -> Self {
        Cache {
            enabled: false,
            ttl: 300,
        }
    }
}

pub fn load_config(config_file: Option<&str>) -> Result<Config, Report> {
    let figment_builder = if let Some(path) = config_file {
        let path_obj = Path::new(path);
        let extension = path_obj.extension().and_then(|s| s.to_str());
        let fig = Figment::new();
        match extension {
            Some("toml") => fig.merge(Toml::file(path)),
            Some("yaml") | Some("yml") => fig.merge(Yaml::file(path)),
            _ => fig.merge(Toml::file(path)),
        }
    } else {
        Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Yaml::file("config.yaml"))
            .merge(Yaml::file("config.yml"))
            .merge(Env::prefixed("DIRECTOR_"))
    };

    let config: Config = figment_builder.extract()?;
    Ok(config)
}
