use std::collections::HashSet;
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

#[derive(Debug)]
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
        match load_allow().await {
            Err(e) => log::warn!("Failed to update allow list: {}", e),
            Ok(_) => log::info!(
                "Successfully updated allow list in {}ms",
                start.elapsed().as_millis()
            ),
        }
        let start = Instant::now();
        match load_block(block_file).await {
            Err(e) => log::warn!("Failed to update block list: {}", e),
            Ok(_) => log::info!(
                "Successfully updated block list in {}s",
                start.elapsed().as_secs()
            ),
        }
        time::sleep(Duration::from_secs(60 * 60)).await;
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
    let domains = load_block_list(block_file).await?;
    let domains = sort_domains(domains);
    filters.filters.push((
        String::from("BlockList Match"),
        Filter::DomainMatch(domains),
    ));
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
