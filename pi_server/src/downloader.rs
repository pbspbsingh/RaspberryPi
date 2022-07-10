use futures_util::{stream, Stream, StreamExt};
use log::*;
use once_cell::sync::OnceCell;
use reqwest::{header, Client};
use tokio::sync::mpsc::{self, UnboundedSender};

use trust_dns_proto::rr::Name;

use domain::block_list::update_blocked_domains;

const USER_AGENT_VAL: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.5 Safari/605.1.15";

static SENDER: OnceCell<UnboundedSender<()>> = OnceCell::new();

pub async fn start_download_loop() -> anyhow::Result<()> {
    let (sender, receiver) = mpsc::unbounded_channel();
    SENDER.set(sender).unwrap();

    update_blocked_domains(receiver, download).await
}

pub fn signal_blocked_domain_refresh() {
    if let Some(sender) = SENDER.get() {
        info!("Sending a refresh signal to bocked domains.");
        sender
            .send(())
            .map_err(|_| warn!("Failed to send the signal"))
            .ok();
    } else {
        warn!("Can't send the signal is not initialized yet");
    }
}

fn client() -> Client {
    let headers = [(header::USER_AGENT, USER_AGENT_VAL)]
        .into_iter()
        .map(|(k, v)| (k, v.parse().unwrap()))
        .collect();
    Client::builder()
        .cookie_store(true)
        .referer(true)
        .default_headers(headers)
        .build()
        .expect("Failed to create http client")
}

async fn download(url: String) -> anyhow::Result<impl Stream<Item = String> + Unpin> {
    let response = client().get(&url).send().await?;
    let status = response.status();
    let cl = response.content_length();
    info!("Downloading {url}({status}) with content-length: {cl:?}");
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download {url}, status code: {status}"
        ));
    }

    Ok(stream::unfold(
        (response, Vec::new(), 0),
        |(mut response, mut buff, mut size)| async move {
            let lines = if let Ok(Some(chunk)) = response.chunk().await {
                buff.extend(chunk);
                find_valid_domain(string_lines(&mut buff, false))
            } else if !buff.is_empty() {
                find_valid_domain(string_lines(&mut buff, true))
            } else {
                info!("Downloaded {size} domains!");
                return None;
            };
            size += lines.len();
            Some((stream::iter(lines), (response, buff, size)))
        },
    )
    .flatten()
    .boxed())
}

fn find_valid_domain(lines: impl Iterator<Item = String>) -> Vec<String> {
    // Can't use filter/map as it causes borrow issue
    let mut result = Vec::with_capacity(lines.size_hint().0);
    for line in lines {
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
        result.extend(
            line.split_whitespace()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter(|line| !["localhost", "0.0.0.0"].contains(line))
                .filter_map(|name| Name::from_utf8(name).ok().map(|name| name.to_utf8())),
        );
    }
    result
}

fn string_lines(buff: &mut Vec<u8>, is_last: bool) -> impl Iterator<Item = String> + '_ {
    itertools::unfold(buff, move |buff| {
        if let Some(idx) = buff.iter().position(|&c| c == b'\n') {
            let range = ..(idx + 1);
            let line = String::from_utf8_lossy(&buff[range]).trim().to_lowercase();
            buff.drain(range);
            return Some(line);
        }
        if is_last && !buff.is_empty() {
            let line = String::from_utf8_lossy(buff).trim().to_lowercase();
            buff.clear();
            return Some(line);
        }
        None
    })
}

#[cfg(test)]
mod test {
    use crate::downloader::find_valid_domain;

    use super::string_lines;

    #[test]
    fn test_r() {
        let mut arr = b"\rhello\n\rworld\n\n\nhehe".to_vec();
        dbg!(string_lines(&mut arr, false).collect::<Vec<_>>());
        dbg!(string_lines(&mut arr, false).collect::<Vec<_>>());
        dbg!(string_lines(&mut arr, true).collect::<Vec<_>>());
        dbg!(arr);
    }

    #[test]
    fn test_name() {
        let mut arr = include_bytes!("../block_list.txt").to_vec();
        let lines = find_valid_domain(string_lines(&mut arr, true));
        println!("Lines: {}", lines.len());
        println!("{} / {}", lines[0], lines.last().unwrap());
    }
}
