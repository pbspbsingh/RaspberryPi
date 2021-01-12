use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use once_cell::sync::OnceCell;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use trust_dns_proto::rr::Name;

use crate::blocker::trie::Trie;
use crate::http_client;

pub mod trie;

static BLOCK_TRIE: OnceCell<RwLock<Trie>> = OnceCell::new();

const FIREBOG_URL: &str = "https://v.firebog.net/hosts/csv.txt";

pub async fn refresh_block_list(block_file: impl AsRef<Path>) -> anyhow::Result<()> {
    BLOCK_TRIE
        .set(RwLock::new(Trie::new()))
        .map_err(|_| anyhow::anyhow!("Failed to create BLOCK_TRIE"))?;

    let block_file = block_file.as_ref();
    update_trie(block_file).await?;

    loop {
        if let Err(e) = fetch_block_list(block_file).await {
            log::error!("Error while refreshing block list: {}", e);
        }
        time::sleep(Duration::from_secs(60 * 60)).await;
    }
}

pub async fn is_blocked(name: &Name) -> bool {
    if let Some(lock) = BLOCK_TRIE.get() {
        let lock = lock.read().await;
        lock.contains(name)
    } else {
        false
    }
}

async fn update_trie(block_file: &Path) -> anyhow::Result<()> {
    if !block_file.exists() {
        return Ok(());
    }

    let start = Instant::now();

    #[cfg(debug_assertions)]
    let mut count = 0;

    let mut trie = Trie::new();
    let mut buff = String::with_capacity(100);
    let mut reader = BufReader::new(File::open(block_file).await?);
    loop {
        buff.clear();
        if reader.read_line(&mut buff).await? <= 0 {
            break;
        }

        let mut line = buff.trim();
        if line.is_empty() || line.starts_with("#") {
            continue;
        }
        if let Some(idx) = line.find("#") {
            line = &line[..idx];
        }
        line = line.trim();
        if line.is_empty() {
            continue;
        }
        line.split_ascii_whitespace()
            .map(str::trim)
            .filter(|&token| "localhost" != token)
            .filter(|&token| is_valid_domain(token))
            .filter_map(|token| Name::from_str(token).ok())
            .for_each(|name| trie.push(&name));
        #[cfg(debug_assertions)]
        {
            count += 1;
            if count >= 50_000 {
                break;
            }
        }
    }
    trie.shrink();
    log::info!(
        "Domains to be blocked: {}, loaded in {}s",
        trie.len(),
        start.elapsed().as_secs()
    );

    let lock = BLOCK_TRIE
        .get()
        .ok_or_else(|| anyhow::anyhow!("BLOCK_TRIE is empty"))?;
    let mut lock = lock.write().await;
    let _ = std::mem::replace(&mut *lock, trie);
    Ok(())
}

async fn fetch_block_list(block_file: &Path) -> anyhow::Result<()> {
    if block_file.exists()
        && block_file.metadata()?.modified()?.elapsed()? < Duration::from_secs(24 * 60 * 60)
    {
        log::debug!("Block list already exists and seems to be up to date!");
        return Ok(());
    }

    let start = Instant::now();
    log::info!("Refreshing AdBlock list...");
    let client = http_client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let response = client.get(FIREBOG_URL).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Couldn't fetch {} error code: {}",
            FIREBOG_URL,
            status
        ));
    }
    let content = response.text().await?;
    let targets = find_targets(&content);
    log::info!(
        "Fetched {} target from block list: {}",
        targets.len(),
        status
    );

    let mut total = 0;
    let mut writer = BufWriter::new(File::create(block_file).await?);
    for (i, target) in targets.into_iter().enumerate() {
        if target.contains("facebook") {
            continue;
        }

        let target_content = match fetch_target(&client, &target).await {
            Err(e) => {
                log::warn!("{}: {}", target, e);
                continue;
            }
            Ok(r) => r,
        };
        log::info!(
            "{}. Fetched {} domains from {}",
            i + 1,
            target_content.len(),
            target
        );
        if target_content.len() > 0 {
            total += target_content.len();
            writer
                .write(format!("# {}. {}\n", i + 1, target).as_bytes())
                .await?;
            for tc in target_content {
                writer.write(tc.as_bytes()).await?;
                writer.write(b"\n").await?;
            }
            writer.write(b"\n\n").await?;
        }
    }
    log::info!(
        "Total domains fetched: {} in time: {}s",
        total,
        start.elapsed().as_secs()
    );
    update_trie(block_file).await
}

fn find_targets(content: &str) -> Vec<&str> {
    content
        .split('\n')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|row| {
            row.split(',')
                .filter(|s| s.len() > 2)
                .map(|s| &s[1..s.len() - 1])
                .collect::<Vec<_>>()
        })
        .filter(|row| row.len() >= 5)
        .filter(|row| row[1] != "cross")
        .map(|row| row[4])
        .collect::<Vec<_>>()
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
    let content = response.text().await?;
    Ok(content
        .split('\n')
        .map(str::trim)
        .filter(|d| !(d.is_empty() || d.starts_with("#")))
        .map(String::from)
        .collect())
}

pub(in crate::blocker) fn is_valid_domain(domain: &str) -> bool {
    domain
        .split(".")
        .map(|part| part.parse::<u32>())
        .all(|res| res.is_err())
}
