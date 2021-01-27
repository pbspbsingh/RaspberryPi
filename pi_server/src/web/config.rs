use std::collections::HashMap;
use std::time::Instant;

use chrono::Local;
use futures_util::TryFutureExt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use warp::reply::json;
use warp::{Rejection, Reply};

use crate::db::filters::{fetch_filters, save_filters, DbFilter};
use crate::dns::domain::Domain;
use crate::dns::filter;
use crate::web::WebError;
use crate::Timer;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    approve_rules: Vec<String>,
    reject_rules: Vec<String>,
}

pub async fn get_config() -> Result<impl Reply, Rejection> {
    let start = Instant::now();
    let mut config = Config {
        approve_rules: vec![],
        reject_rules: vec![],
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
                .filter(|x| x.len() > 0)
                .filter_map(|r| extract_filter(r, true)),
        );
    }
    if let Some(rules) = form.get("rejectRules") {
        configs.extend(
            rules
                .split('\n')
                .map(str::trim)
                .filter(|x| x.len() > 0)
                .filter_map(|r| extract_filter(r, false)),
        );
    }
    log::info!("Inserting {} values in filters", configs.len());
    save_filters(configs).await.map_err(WebError::new)?;
    tokio::spawn(async {
        filter::load_allow().await.ok();
        filter::load_block(None).await.ok();
    });
    Ok("Success!")
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
        }
    }
}
