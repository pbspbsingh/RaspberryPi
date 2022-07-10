use std::collections::{HashMap, HashSet};

use axum::response::IntoResponse;
use axum::{Form, Json};
use chrono::Local;
use itertools::{Either, Itertools};
use regex::Regex;
use serde::{Deserialize, Serialize};
use trust_dns_proto::rr::Name;

use crate::downloader::signal_blocked_domain_refresh;
use domain::db::block_list::{load_block_list, save_block_list, DbBlockList};
use domain::db::filters::{load_all_filters, save_filters, DbFilter};
use domain::reload_filters;

use crate::web::WebError;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    approve_rules: Vec<String>,
    reject_rules: Vec<String>,
    block_list: Vec<(String, i64)>,
}

pub async fn fetch_config() -> Result<impl IntoResponse, WebError> {
    let (approve_rules, reject_rules) =
        load_all_filters().await?.into_iter().partition_map(|dbf| {
            let mut expr = dbf.expr;
            if dbf.is_regex {
                expr = format!("* {expr}");
            }
            if !dbf.enabled {
                expr = format!("# {expr}");
            }
            if dbf.is_allow {
                Either::Left(expr)
            } else {
                Either::Right(expr)
            }
        });
    let block_list = load_block_list()
        .await?
        .into_iter()
        .map(|bl| (bl.src, bl.domain_count))
        .collect();
    let config = Config {
        approve_rules,
        reject_rules,
        block_list,
    };
    Ok(Json(config))
}

pub async fn save_config(
    Form(form): Form<HashMap<String, String>>,
) -> Result<impl IntoResponse, WebError> {
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
    log::info!("Saving {} filters", configs.len());
    save_filters(configs).await?;
    log::info!("Reloading the filters...");
    reload_filters().await?;

    if let Some(block_list) = form.get("updatedBlockList") {
        let old_block_list = load_block_list().await?;
        let old_block_list = old_block_list
            .iter()
            .map(|bl| bl.src.trim())
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
            let last_updated = Local::now().naive_local();
            save_block_list(
                new_block_list
                    .into_iter()
                    .map(|line| DbBlockList {
                        bl_id: -1,
                        src: line.into(),
                        retry_count: 0,
                        domain_count: -1,
                        last_updated,
                    })
                    .collect::<Vec<_>>(),
            )
            .await?;
            signal_blocked_domain_refresh();
        } else {
            log::warn!("Block list hasn't been updated, nothing to do!");
        }
    }

    fetch_config().await
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
        if let Err(e) = Regex::new(rule) {
            log::warn!("Can't parse {rule} as regex: {e:?}",);
            return None;
        }
        true
    } else {
        if let Err(e) = Name::from_str_relaxed(rule) {
            log::warn!("Can't parse {rule} as domain name: {e:?}");
            return None;
        }
        false
    };
    Some(DbFilter {
        f_id: -1,
        create_time: Local::now().naive_local(),
        expr: rule.to_lowercase(),
        is_regex,
        is_allow,
        enabled,
    })
}

#[cfg(test)]
mod test {
    use crate::web::config::extract_filter;

    #[test]
    fn test1() {
        dbg!(extract_filter("#*facebook.com", true));
    }
}
