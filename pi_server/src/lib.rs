use std::env;
use std::path::Path;

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::fs;

pub use timer::Timer;

pub mod blocker;
pub mod cloudflared;
pub mod db;
pub mod dns;
pub mod http_client;
pub mod sysinfo;
mod timer;
pub mod web;

pub static PI_CONFIG: OnceCell<PiConfig> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct PiConfig {
    pub db_path: String,
    pub db_opt: String,
    pub db_pool: u32,
    pub dns_port: u32,
    pub web_port: u32,
    pub cloudflared_path: String,
    pub cloudflared_port: u32,
    pub log_config: String,
    pub block_list: String,
    pub dht22_pin: Option<u32>,
}

impl PiConfig {
    fn default() -> Self {
        PiConfig {
            db_path: "server.db".into(),
            db_opt: "PRAGMA synchronous=OFF;".into(),
            db_pool: 1,
            dns_port: 53,
            web_port: 8080,
            cloudflared_path: "cloudflared".into(),
            cloudflared_port: 5053,
            log_config: "log4rs.yml".into(),
            block_list: "block_list.txt".into(),
            dht22_pin: None,
        }
    }

    pub async fn read_config() -> anyhow::Result<()> {
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
        PI_CONFIG
            .set(config)
            .map_err(|_| anyhow::anyhow!("Failed to read PiConfig"))
    }
}

pub fn next_maintainence() -> NaiveDateTime {
    let now = Local::now().naive_local();
    let next_slot = NaiveDate::from_ymd(now.year(), now.month(), now.day()).and_hms(2, 0, 0);
    next_slot + Duration::days(1)
}
