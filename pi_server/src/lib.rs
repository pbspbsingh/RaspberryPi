use std::env;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::fs;

pub mod blocker;
pub mod db;
pub mod dns;
pub mod http_client;
pub mod sysinfo;

#[derive(Debug, Serialize, Deserialize)]
pub struct PiConfig {
    pub db_path: String,
    pub db_opt: String,
    pub db_pool: u32,
    pub dns_port: u64,
    pub forward_server: String,
    pub log_config: String,
    pub block_list: String,
}

impl PiConfig {
    fn default() -> Self {
        PiConfig {
            db_path: "server.db".into(),
            db_opt: "PRAGMA synchronous=OFF;".into(),
            db_pool: 1,
            dns_port: 53,
            forward_server: "127.0.0.1:5053".into(),
            log_config: "log4rs.yml".into(),
            block_list: "block_list.txt".into(),
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
