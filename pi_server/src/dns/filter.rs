use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use regex::{Regex, RegexSet};
use tokio::sync::RwLock;
use tokio::time;
use tokio::time::Duration;
use trust_dns_proto::rr::Name;

use crate::blocker::load_block_list;
use crate::db::filters::{fetch_filters, DbFilter};
use crate::dns::domain::{sort_domains, Domain};
use crate::dns::{ALLOW, BLOCK};
use crate::Timer;

const AN_HOUR: Duration = Duration::from_secs(60 * 60);
const BL_NAME: &str = "BlockList Match";

#[derive(Debug, Clone)]
enum Filter {
    Pattern(RegexSet),
    DomainMatch(HashSet<Domain>),
}

#[derive(Debug)]
pub struct Filters {
    filters: Vec<(String, Filter)>,
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
        for filter in &self.filters {
            match filter {
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

pub async fn update_filters(block_file: &str) -> anyhow::Result<()> {
    ALLOW
        .set(RwLock::new(Filters::default()))
        .map_err(|_| anyhow::anyhow!("Error setting ALLOW"))?;
    BLOCK
        .set(RwLock::new(Filters::default()))
        .map_err(|_| anyhow::anyhow!("Error setting BLOCK"))?;

    loop {
        log::debug!("Updating the filters...");

        let start = Instant::now();
        if let Err(e) = load_allow().await {
            log::warn!("Failed to update allow list: {}", e);
        } else {
            log::info!("Successfully updated allow list in {}", start.t());
        }

        let start = Instant::now();
        if let Err(e) = load_block(block_file).await {
            log::warn!("Failed to update block list: {}", e);
        } else {
            log::info!("Successfully updated block list in {}", start.t());
        }

        time::sleep(AN_HOUR).await;
    }
}

async fn load_allow() -> anyhow::Result<()> {
    let filters = load_db_filters(fetch_filters(true).await?).await?;
    let mut lock = ALLOW.get().unwrap().write().await;
    let _ = std::mem::replace(&mut *lock, filters);
    Ok(())
}

async fn load_block(block_file: &str) -> anyhow::Result<()> {
    let mut filters = load_db_filters(fetch_filters(false).await?).await?;

    let block_file = Path::new(block_file);
    let last_updated = if block_file.exists() {
        block_file.metadata()?.modified()?.elapsed()?
    } else {
        Duration::from_millis(0)
    };
    log::info!("Block list file was modified {} ago", last_updated.t());
    let mut bl_filter = None;
    if last_updated > AN_HOUR {
        let read_lock = BLOCK.get().unwrap().read().await;
        if let Some((_, f)) = read_lock.filters.iter().find(|f| f.0 == BL_NAME) {
            log::info!("Block list hasn't been updated lately, no need to read from disk");
            bl_filter = Some(f.clone());
        }
    }
    if bl_filter.is_none() {
        if let Ok(domains) = load_block_list(block_file).await {
            let domains = sort_domains(domains);
            bl_filter = Some(Filter::DomainMatch(domains));
        } else {
            log::warn!("Failed to read block_list file");
        }
    }
    if let Some(bl_filter) = bl_filter {
        filters.filters.push((BL_NAME.into(), bl_filter));
    }

    let mut lock = BLOCK.get().unwrap().write().await;
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
            (String::from("Regex Match"), Filter::Pattern(regex_set)),
            (String::from("Domain Match"), Filter::DomainMatch(domains)),
        ],
    })
}
