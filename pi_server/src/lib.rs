use std::env;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::fs;

pub mod dns;

#[derive(Debug, Serialize, Deserialize)]
pub struct PiConfig {
    pub port: u64,
    forward_server: String,
    pub forward_port: u64,
    pub log_config: String,
}

impl PiConfig {
    fn default() -> Self {
        PiConfig {
            port: 53,
            forward_server: "127.0.0.1".into(),
            forward_port: 5053,
            log_config: "log4rs.yml".into(),
        }
    }

    pub async fn read_config() -> anyhow::Result<PiConfig> {
        let config_file = env::args()
            .nth(1)
            .unwrap_or_else(|| String::from("config.json"));
        println!("Using config from file '{}'", config_file);

        let config = if Path::new(&config_file).exists() {
            serde_json::from_str(&fs::read_to_string(&config_file).await?)?
        } else {
            PiConfig::default()
        };

        fs::write(config_file, &serde_json::to_string_pretty(&config)?).await?;
        Ok(config)
    }
}
