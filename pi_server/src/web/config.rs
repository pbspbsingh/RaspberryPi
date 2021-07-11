use std::collections::{HashMap, HashSet};
use std::time::Instant;

use chrono::Local;
use futures_util::TryFutureExt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use warp::reply::json;
use warp::{Rejection, Reply};

use crate::db::block_list::{db_block_list, replace_block_list};
use crate::db::filters::{fetch_filters, save_filters, DbFilter};
use crate::dns::domain::Domain;
use crate::dns::filter;
use crate::web::WebError;
use crate::{blocker, Timer};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    approve_rules: Vec<String>,
    reject_rules: Vec<String>,
    block_list: Vec<(String, Option<i64>)>,
}

pub async fn get_config() -> Result<impl Reply, Rejection> {
    let start = Instant::now();
    let mut config = Config {
        approve_rules: vec![],
        reject_rules: vec![],
        block_list: vec![],
    };
    let filters = fetch_filters(None).map_err(WebError::new).await?;
    let size = filters.len();
    for DbFilter {
        mut expr,
        is_allow,
        is_regex,
        enabled,
        ..
    } in filters
    {
        if is_regex {
            expr = format!("* {}", expr);
        }
        if !enabled {
            expr = format!("# {}", expr);
        }
        if is_allow {
            config.approve_rules.push(expr);
        } else {
            config.reject_rules.push(expr);
        }
    }
    config.block_list.extend(
        db_block_list()
            .await
            .map_err(WebError::new)?
            .into_iter()
            .map(|bl| (bl.b_src, bl.b_count)),
    );
    log::debug!("Fetched {} rules in {}", size, start.t());
    Ok(json(&config))
}

pub async fn save_config(form: HashMap<String, String>) -> Result<impl Reply, Rejection> {
    let mut configs = Vec::new();
    if let Some(rules) = form.get("approveRules") {
        configs.extend(
            rules
                .split('\n')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .filter_map(|r| extract_filter(r, true)),
        );
    }
    if let Some(rules) = form.get("rejectRules") {
        configs.extend(
            rules
                .split('\n')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .filter_map(|r| extract_filter(r, false)),
        );
    }
    let mut bl_updated = false;
    if let Some(block_list) = form.get("updatedBlockList") {
        let old_block_list = db_block_list().await.map_err(WebError::new)?;
        let old_block_list = old_block_list
            .iter()
            .map(|bl| bl.b_src.trim())
            .collect::<HashSet<_>>();
        let new_block_list = block_list
            .split('\n')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<HashSet<_>>();
        if old_block_list != new_block_list {
            log::info!(
                "Block list has been updated {} vs {}",
                old_block_list.len(),
                new_block_list.len()
            );
            replace_block_list(new_block_list)
                .await
                .map_err(WebError::new)?;
            bl_updated = true;
        } else {
            log::info!("Block list hasn't been updated, nothing to do!");
        }
    }
    log::info!("Inserting {} values in filters", configs.len());
    save_filters(configs).await.map_err(WebError::new)?;
    tokio::spawn(async move {
        filter::load_allow().await.ok();
        filter::load_block().await.ok();
        if bl_updated {
            blocker::fetch_block_list(false).await.ok();
        }
    });
    get_config().await
}

fn extract_filter(mut rule: &str, is_allow: bool) -> Option<DbFilter> {
    let enabled = if rule.starts_with('#') {
        rule = rule[1..].trim();
        false
    } else {
        true
    };
    let is_regex = if rule.starts_with('*') {
        rule = rule[1..].trim();
        if Regex::new(rule).is_err() {
            log::warn!("Can't parse {} as regex", rule);
            return None;
        }
        true
    } else {
        if Domain::parse(rule).is_none() {
            log::warn!("Can't parse {} as domain name", rule);
            return None;
        }
        false
    };
    Some(DbFilter {
        f_id: -1,
        ct: Local::now().naive_local(),
        expr: rule.to_string(),
        is_regex,
        is_allow,
        enabled,
    })
}

#[cfg(test)]
mod test {
    use regex::Regex;

    #[test]
    fn test1() {
        match Regex::new(".*duh.*") {
            Ok(r) => {
                dbg!(r.is_match("www.duho.com"));
            }
            Err(e) => {
                dbg!(e);
            }
        };
    }
}
