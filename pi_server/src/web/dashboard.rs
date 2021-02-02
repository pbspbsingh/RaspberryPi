use std::time::Instant;

use chrono::{Duration, Local};
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};
use warp::reply::json;
use warp::{Rejection, Reply};

use crate::db::dns_requests::{agg_by_filtered, agg_by_time, agg_by_type};
use crate::web::WebError;
use crate::Timer;

#[derive(Debug, Serialize, Deserialize)]
struct DashboardInfo {
    total_count: u64,
    reject_count: u64,
    passed: Vec<(u64, u64)>,
    approved: Vec<(u64, u64)>,
    rejected: Vec<(u64, u64)>,
    passed_ms: Vec<(u64, f64)>,
    approved_ms: Vec<(u64, f64)>,
    rejected_ms: Vec<(u64, f64)>,
    queries: LinkedHashMap<String, u64>,
    top_approved: LinkedHashMap<String, u64>,
    top_rejected: LinkedHashMap<String, u64>,
}

pub async fn fetch_dashboard(days: u32) -> Result<impl Reply, Rejection> {
    let start = Instant::now();
    let mut info = DashboardInfo {
        total_count: 0,
        reject_count: 0,
        passed: vec![],
        passed_ms: vec![],
        approved: vec![],
        approved_ms: vec![],
        rejected: vec![],
        rejected_ms: vec![],
        queries: LinkedHashMap::with_capacity(10),
        top_approved: LinkedHashMap::with_capacity(10),
        top_rejected: LinkedHashMap::with_capacity(10),
    };
    let from = Local::now().naive_local() - Duration::days(days as i64);
    for (time, count, res_time, filtered) in agg_by_time(from).await.map_err(WebError::new)? {
        let time = time.timestamp_millis() as u64;
        let count = count as u64;
        let res_time = (res_time * 100.).trunc() / 100.; // format!("{:.2}", res_time).parse().unwrap();

        info.total_count += count;
        match filtered {
            None => {
                info.passed.push((time, count));
                info.passed_ms.push((time, res_time));
            }
            Some(true) => {
                info.approved.push((time, count));
                info.approved_ms.push((time, res_time));
            }
            Some(false) => {
                info.rejected.push((time, count));
                info.rejected_ms.push((time, res_time));
                info.reject_count += count;
            }
        };
    }
    if let Ok(res) = agg_by_type(from).await {
        res.into_iter().for_each(|(k, v)| {
            info.queries.insert(k, v as u64);
        });
    }
    if let Ok(res) = agg_by_filtered(from, true).await {
        res.into_iter().for_each(|(k, v)| {
            let k = if k.ends_with('.') {
                k[..k.len() - 1].to_string()
            } else {
                k
            };
            info.top_approved.insert(k, v as u64);
        });
    }
    if let Ok(res) = agg_by_filtered(from, false).await {
        res.into_iter().for_each(|(k, v)| {
            let k = if k.ends_with('.') {
                k[..k.len() - 1].to_string()
            } else {
                k
            };
            info.top_rejected.insert(k, v as u64);
        });
    }
    log::info!(
        "Total time to aggregate data for {} day(s): {}",
        days,
        start.t()
    );
    Ok(json(&info))
}
