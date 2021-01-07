use std::env;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::fs;

use pi_server::dns::start_dns_server;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::read_config().await?;
    println!("Starting with {:#?}", config);

    init_logger(&config.log_config).await?;
    log::info!("Hello World!");

    let run = tokio::try_join!(start_dns_server(config.port));
    run.map(|_| ())
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    port: u64,
    log_config: String,
}

impl Config {
    fn default() -> Self {
        Config {
            port: 53,
            log_config: String::from("log4rs.yml"),
        }
    }

    async fn read_config() -> anyhow::Result<Config> {
        let config_file = env::args()
            .nth(1)
            .unwrap_or_else(|| String::from("config.json"));
        println!("Using config from file '{}'", config_file);

        let config = if Path::new(&config_file).exists() {
            serde_json::from_str(&fs::read_to_string(&config_file).await?)?
        } else {
            Config::default()
        };

        fs::write(config_file, &serde_json::to_string_pretty(&config)?).await?;
        Ok(config)
    }
}

async fn init_logger(config_file: impl AsRef<str>) -> anyhow::Result<()> {
    let config_file = config_file.as_ref();
    if !Path::new(config_file).exists() {
        fs::write(config_file, DEFAULT_LOG_CONFIG).await?;
    }
    Ok(log4rs::init_file(config_file, Default::default())?)
}

const DEFAULT_LOG_CONFIG: &str = r##"
# Scan this file for changes every 30 secs
refresh_rate: 30 seconds

appenders:
  # An appender named "main" that writes to a file with a custom pattern encoder
  main:
    kind: rolling_file
    path: server.log
    policy:
      kind: compound
      trigger:
        kind: size
        limit: 5 mb
      roller:
        kind: fixed_window
        pattern: 'server_{}.log'
        base: 0
        count: 6

# Set the default logging level to "warn" and attach the "stdout" appender to the root
root:
  level: warn
  appenders:
    - main

loggers:
  # Raise the maximum log level for events sent to the "pi_server" logger to "debug"
  pi_server:
    level: debug

  # Add other crates log config below
"##;
