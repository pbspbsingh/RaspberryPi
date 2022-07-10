use chrono::Local;
use std::future::Future;
use std::time::{Duration, Instant};

use futures_util::{Stream, StreamExt};
use log::*;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time;

use crate::db::block_list::{
    blocked_domain_last_updated, clear_blocked_domain, insert_blocked_domain, load_block_list,
    update_block_list, DbBlockList,
};
use crate::db::{db, vacuum};

pub async fn update_blocked_domains<F, S>(
    mut receiver: UnboundedReceiver<()>,
    fun: impl Fn(String) -> F,
) -> anyhow::Result<()>
where
    F: Future<Output = anyhow::Result<S>>,
    S: Stream<Item = String> + Unpin,
{
    loop {
        let should_refresh = match time::timeout(Duration::from_secs(30), receiver.recv()).await {
            Ok(Some(_)) => true,
            _ => should_update().await?,
        };
        if !should_refresh {
            continue;
        }
        info!("Time to update the blocked domains list.");

        let updated = Local::now().naive_local();
        let mut trans = db().begin().await?;

        let drop_count = clear_blocked_domain(&mut trans).await?;
        info!("Dropped {drop_count} blocked domains");

        let (mut start, mut insert_count, mut total_count) = (Instant::now(), 0, 0);
        for mut bl in load_block_list().await? {
            let DbBlockList {
                src, retry_count, ..
            } = &bl;
            if *retry_count > 3 {
                warn!("{src} has been retried for {retry_count}, skipping");
                continue;
            }

            debug!("Loading blocked domains from {src}");
            let (retry_count, domain_count) = match fun(src.clone()).await {
                Ok(mut domain_stream) => {
                    let mut domain_count = 0;
                    while let Some(domain) = domain_stream.next().await {
                        if insert_blocked_domain(&mut trans, &domain, src, updated).await {
                            insert_count += 1;
                            domain_count += 1;
                        }
                        total_count += 1;

                        if start.elapsed().as_secs() >= 10 {
                            debug!("Inserted {insert_count}/{total_count} domains so far");
                            start = Instant::now();
                        }
                    }
                    (0, domain_count)
                }
                Err(e) => {
                    warn!("Failed to get the domains from {src}: {e} ");
                    (retry_count + 1, -1)
                }
            };
            bl.retry_count = retry_count;
            bl.domain_count = domain_count;
            bl.last_updated = updated;
            update_block_list(&mut trans, bl).await?;
        }
        trans.commit().await?;
        vacuum().await?;
        info!("Inserted {insert_count} of {total_count} blocked domains");
    }
}

async fn should_update() -> anyhow::Result<bool> {
    use chrono::Duration;

    let block_list = load_block_list().await?;
    let unprocessed = block_list.iter().filter(|bl| bl.domain_count == -1).count();

    if block_list.len() == unprocessed {
        debug!("All block list are unprocessed, refreshing the blocked domains");
        Ok(true)
    } else if let Some(last_updated) = blocked_domain_last_updated().await? {
        debug!("Blocked domains were last updated at: {last_updated}");
        Ok(Local::now().naive_local() - last_updated > Duration::days(7))
    } else {
        debug!("No entry found in blocked domain, forcing refresh");
        Ok(true)
    }
}
