use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use regex::{Regex, RegexSet};
use tokio::time;
use tokio::time::Duration;
use trust_dns_proto::rr::Name;

use crate::blocker::load_block_list;
use crate::db::filters::{fetch_filters, DbFilter};
use crate::dns::domain::{sort_domains, Domain};
use crate::dns::{ALLOW, BLOCK};
use crate::{PiConfig, Timer, PI_CONFIG};

const AN_HOUR: Duration = Duration::from_secs(60 * 60);
const BL_NAME: &str = "BlockList Match";

#[derive(Debug)]
enum Filter {
    Pattern(RegexSet),
    DomainMatch(HashSet<Domain>),
}

#[derive(Debug)]
pub struct Filters {
    filters: Vec<(String, Arc<Filter>)>,
}

impl Filters {
    pub fn default() -> Self {
        Filters { filters: vec![] }
    }

    pub fn check(&self, name: &Name) -> Option<&str> {
        let domain = match Domain::from_name(name) {
            None => return None,
            Some(d) => d,
        };
        let domain_name = domain.name();
        for (name, filter) in &self.filters {
            match (name, filter.as_ref()) {
                (name, Filter::Pattern(regex_set)) => {
                    if regex_set.is_match(&domain_name) {
                        return Some(name);
                    }
                }
                (name, Filter::DomainMatch(dom_match)) => {
                    let mut domain = Some(domain.clone());
                    while let Some(d) = domain {
                        if dom_match.contains(&d) {
                            return Some(name);
                        }
                        domain = d.parent();
                    }
                }
            }
        }
        None
    }
}

pub async fn update_filters() -> anyhow::Result<()> {
    loop {
        log::debug!("Updating the filters...");

        let start = Instant::now();
        if let Err(e) = load_allow().await {
            log::warn!("Failed to update allow list: {}", e);
        } else {
            log::info!("Successfully updated allow list in {}", start.t());
        }

        let start = Instant::now();
        if let Err(e) = load_block().await {
            log::warn!("Failed to update block list: {}", e);
        } else {
            log::info!("Successfully updated block list in {}", start.t());
        }

        time::sleep(AN_HOUR).await;
    }
}

pub async fn load_allow() -> anyhow::Result<()> {
    let filters = load_db_filters(fetch_filters(Some(true)).await?).await?;
    let mut lock = ALLOW.write().await;
    let _ = std::mem::replace(&mut *lock, filters);
    Ok(())
}

pub async fn load_block() -> anyhow::Result<()> {
    let mut filters = load_db_filters(fetch_filters(Some(false)).await?).await?;

    let PiConfig { block_list, .. } = PI_CONFIG.get().unwrap();
    let block_file = Path::new(block_list);
    let last_updated = if block_file.exists() {
        block_file.metadata()?.modified()?.elapsed()?
    } else {
        Duration::from_secs(AN_HOUR.as_secs() * 2)
    };

    log::info!("Block list file was modified {} ago", last_updated.t());
    let mut bl_filter = None;
    if last_updated > AN_HOUR {
        let read_lock = BLOCK.read().await;
        if let Some((_, f)) = read_lock.filters.iter().find(|f| f.0 == BL_NAME) {
            log::info!("Block list hasn't been updated lately, no need to read from disk");
            bl_filter = Some(Arc::clone(f));
        }
    }
    if bl_filter.is_none() {
        if let Ok(domains) = load_block_list(block_file).await {
            let domains = tokio::task::spawn_blocking(|| sort_domains(domains)).await?;
            bl_filter = Some(Arc::new(Filter::DomainMatch(domains)));
        } else {
            log::warn!("Failed to read block_list file");
        }
    }
    if let Some(bl_filter) = bl_filter {
        filters.filters.push((BL_NAME.into(), bl_filter));
    }

    let mut lock = BLOCK.write().await;
    let _ = std::mem::replace(&mut *lock, filters);
    Ok(())
}

async fn load_db_filters(db_filters: Vec<DbFilter>) -> anyhow::Result<Filters> {
    let (regex, domain): (Vec<_>, Vec<_>) = db_filters.into_iter().partition(|f| f.is_regex);
    let regex_set = RegexSet::new(
        regex
            .into_iter()
            .map(|f| f.expr)
            .filter(|s| Regex::new(s).is_ok()),
    )?;
    let domains = sort_domains(
        domain
            .into_iter()
            .filter_map(|f| Domain::parse(f.expr))
            .collect(),
    );
    Ok(Filters {
        filters: vec![
            (
                String::from("Regex Match"),
                Arc::new(Filter::Pattern(regex_set)),
            ),
            (
                String::from("Domain Match"),
                Arc::new(Filter::DomainMatch(domains)),
            ),
        ],
    })
}
