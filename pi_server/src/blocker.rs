use std::path::Path;
use std::time::Instant;

use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::time::{self, Duration};

use crate::dns::domain::Domain;
use crate::{http_client, Timer};

const WEEK: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const FIREBOG_URL: &str = "https://v.firebog.net/hosts/csv.txt";

pub async fn refresh_block_list(block_file: impl AsRef<Path>) -> anyhow::Result<()> {
    let block_file = block_file.as_ref();
    loop {
        if let Err(e) = fetch_block_list(block_file).await {
            log::error!("Error while refreshing block list: {}", e);
        }
        time::sleep(Duration::from_secs(30 * 60)).await;
    }
}

pub async fn load_block_list(block_file: impl AsRef<Path>) -> anyhow::Result<Vec<Domain>> {
    let start = Instant::now();
    let mut list = Vec::new();
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
                list.push(name);
            });
        #[cfg(debug_assertions)]
        if list.len() >= 5000 {
            break;
        }
    }
    log::info!("Loaded {} blocked domains in {}", list.len(), start.t());
    Ok(list)
}

async fn fetch_block_list(block_file: &Path) -> anyhow::Result<()> {
    if block_file.exists() {
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
    log::info!("Total domains fetched: {} in time: {}", total, start.t());
    Ok(())
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
