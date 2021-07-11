use std::path::Path;
use std::time::Instant;

use once_cell::sync::OnceCell;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

use crate::db::block_list::{db_block_list, update_block_list};
use crate::dns::domain::Domain;
use crate::dns::filter::load_block;
use crate::{http_client, PiConfig, Timer, PI_CONFIG};

const WEEK: Duration = Duration::from_secs(7 * 24 * 60 * 60);
pub(crate) static UPDATE_LOCK: OnceCell<Mutex<()>> = OnceCell::new();

pub async fn refresh_block_list() -> anyhow::Result<()> {
    UPDATE_LOCK.set(Mutex::new(())).ok();
    loop {
        if let Err(e) = fetch_block_list(true).await {
            log::error!("Error while refreshing block list: {}", e);
        }
        time::sleep(Duration::from_secs(30 * 60)).await;
    }
}

pub async fn load_block_list(block_file: impl AsRef<Path>) -> anyhow::Result<Vec<Domain>> {
    let _lock = UPDATE_LOCK.get().unwrap().try_lock()?;
    let start = Instant::now();
    let mut list = Vec::new();
    let mut buff = String::with_capacity(100);
    let mut reader = BufReader::new(File::open(block_file).await?);
    loop {
        buff.clear();
        if reader.read_line(&mut buff).await? == 0 {
            break;
        }

        let line = buff.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(domain) = Domain::parse(line) {
            list.push(domain);
        }
        #[cfg(debug_assertions)]
        if list.len() >= 5000 {
            break;
        }
    }
    log::info!("Loaded {} blocked domains in {}", list.len(), start.t());
    Ok(list)
}

pub async fn fetch_block_list(check_existing: bool) -> anyhow::Result<()> {
    log::debug!("Acquiring lock for fetching block list...");
    let _lock = UPDATE_LOCK.get().unwrap().lock().await;
    let PiConfig { block_list, .. } = PI_CONFIG.get().unwrap();
    let block_file = Path::new(block_list);
    if check_existing && block_file.exists() {
        log::debug!("Block list file exists.");
        let mod_elapsed = block_file.metadata()?.modified()?.elapsed()?;
        if mod_elapsed < WEEK {
            let wait_time = WEEK - mod_elapsed;
            log::info!("Block list will be updated after {}", wait_time.t());
            return Ok(());
        }
        log::info!("Block list was last updated more than a week ago, updating now...");
    }

    let start = Instant::now();
    log::info!("Fetching AdBlock list...");

    let client = http_client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let mut block_list = db_block_list().await?;
    log::info!("Loaded {} block list from db", block_list.len());

    let mut total = 0;
    let mut writer = BufWriter::new(File::create(block_file).await?);
    for (i, bl) in block_list.iter_mut().enumerate() {
        let block_content = match fetch_target(&client, &bl.b_src).await {
            Ok(r) => r,
            Err(e) => {
                log::warn!("{}: {}", bl.b_src, e);
                bl.b_count = None;
                continue;
            }
        };
        let len = block_content.len();
        log::info!("{}. Fetched {} domains from {}", i + 1, len, &bl.b_src,);
        for bc in block_content {
            writer.write(bc.as_bytes()).await?;
            writer.write(b"\n").await?;
        }
        bl.b_count = Some(len as i64);
        total += len;
        #[cfg(debug_assertions)]
        if total >= 5000 {
            break;
        }
    }
    log::info!("Total domains fetched: {} in time: {}", total, start.t());

    update_block_list(block_list).await?;
    load_block().await?;

    Ok(())
}

async fn fetch_target(client: &Client, target: &str) -> anyhow::Result<Vec<String>> {
    let response = client.get(target).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch {}, status: {}",
            target,
            response.status()
        ));
    }

    let mut domains = Vec::new();
    for line in response.text().await?.split('\n') {
        let mut line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(idx) = line.find('#') {
            line = line[..idx].trim();
            if line.is_empty() {
                continue;
            }
        }
        line.split_ascii_whitespace()
            .map(str::trim)
            .filter(|&token| "localhost" != token)
            .filter_map(Domain::parse)
            .for_each(|name| {
                domains.push(name.name());
            });
    }
    Ok(domains)
}
