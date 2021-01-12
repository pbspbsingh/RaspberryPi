#[cfg(not(target_os = "windows"))]
use jemallocator::Jemalloc;

use pi_server::blocker::refresh_block_list;
use pi_server::dns::start_dns_server;
use pi_server::PiConfig;

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = PiConfig::read_config().await?;
    println!("Starting with {:#?}", config);

    init_logger(&config.log_config).await?;
    log::info!("Hello World!");

    let run = tokio::try_join!(
        refresh_block_list(&config.block_list),
        start_dns_server(&config),
    );
    run.map(|_| ())
}

async fn init_logger(config_file: impl AsRef<str>) -> anyhow::Result<()> {
    use std::path::Path;
    use tokio::fs;

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
    path: server_0.log
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
  trust_dns_proto:
    level: info
  trust_dns_server:
    level: info
  trust_dns_resolver:
    level: info
"##;
