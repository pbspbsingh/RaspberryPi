#[cfg(not(target_os = "windows"))]
use tikv_jemallocator::Jemalloc;

use pi_server::blocker::refresh_block_list;
use pi_server::cloudflared::init_cloudflare;
use pi_server::db::init_db;
use pi_server::dns::{start_dns_server, update_filters};
use pi_server::sysinfo::load_sys_info;
use pi_server::web::{start_web_server, ws_sender};
use pi_server::{PiConfig, PI_CONFIG};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    PiConfig::read_config().await?;

    init_logger().await?;
    init_db().await?;

    let cloudflared = init_cloudflare().await?;

    if let Err(e) = tokio::try_join!(
        cloudflared.start_daemon(),
        start_dns_server(),
        start_web_server(),
        load_sys_info(),
        update_filters(),
        refresh_block_list(),
        ws_sender(),
    ) {
        println!("Something went wrong: {e:?}");
        log::error!("Failed to start the app: {e:?}");
    }
    Ok(())
}

async fn init_logger() -> anyhow::Result<()> {
    use std::path::Path;
    use tokio::fs;

    let config_file = &PI_CONFIG.get().unwrap().log_config;
    if !Path::new(config_file).exists() {
        fs::write(config_file, DEFAULT_LOG_CONFIG).await?;
    }
    log4rs::init_file(config_file, Default::default())
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
"##;
