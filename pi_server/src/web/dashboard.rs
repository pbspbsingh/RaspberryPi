use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use std::fmt::Display;
use std::time::Instant;

use chrono::{Duration, Local};
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

use crate::db::dns_requests::{agg_by_filtered, agg_by_time, agg_by_type, agg_failed_by_time};

use crate::web::WebError;
use crate::Timer;

const BASE_SERIES: &[&str] = &["Rejected", "Approved", "Passed"];

#[derive(Debug, Deserialize, Serialize)]
struct TimeSeries<T: Display = u64> {
    name: String,
    data: Vec<(u64, T)>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DashboardInfo {
    total_count: u64,
    reject_count: u64,
    dns_data: Vec<TimeSeries>,
    latency_data: Vec<TimeSeries<f64>>,
    queries: LinkedHashMap<String, u64>,
    top_approved: LinkedHashMap<String, u64>,
    top_rejected: LinkedHashMap<String, u64>,
}

pub async fn fetch_dashboard(Path(days): Path<u32>) -> Result<impl IntoResponse, WebError> {
    async fn _fetch_dashboard(days: u32) -> anyhow::Result<impl IntoResponse> {
        let start = Instant::now();
        let dns_data = BASE_SERIES
            .iter()
            .map(|name| name.to_string())
            .map(|name| TimeSeries { name, data: vec![] })
            .collect();
        let latency_data = BASE_SERIES
            .iter()
            .map(|name| name.to_string())
            .map(|name| TimeSeries { name, data: vec![] })
            .collect();
        let mut info = DashboardInfo {
            total_count: 0,
            reject_count: 0,
            dns_data,
            latency_data,
            queries: LinkedHashMap::with_capacity(10),
            top_approved: LinkedHashMap::with_capacity(10),
            top_rejected: LinkedHashMap::with_capacity(10),
        };
        let from = Local::now().naive_local() - Duration::days(days as i64);
        let agg_time = tokio::spawn(agg_by_time(from));
        let failed_agg_time = tokio::spawn(agg_failed_by_time(from));
        let agg_type = tokio::spawn(agg_by_type(from));
        let agg_filtered_true = tokio::spawn(agg_by_filtered(from, true));
        let agg_filtered_false = tokio::spawn(agg_by_filtered(from, false));

        for (time, count, res_time, filtered) in agg_time.await?? {
            let time = time.timestamp_millis() as u64;
            let count = count as u64;
            let res_time = (res_time * 100.).trunc() / 100.;

            info.total_count += count;
            assert!(info.dns_data.len() >= 3);
            assert!(info.latency_data.len() >= 3);
            match filtered {
                Some(false) => {
                    info.dns_data[0].data.push((time, count));
                    info.latency_data[0].data.push((time, res_time));
                    info.reject_count += count;
                }
                Some(true) => {
                    info.dns_data[1].data.push((time, count));
                    info.latency_data[1].data.push((time, res_time));
                }
                None => {
                    info.dns_data[2].data.push((time, count));
                    info.latency_data[2].data.push((time, res_time));
                }
            };
        }
        info.dns_data.insert(
            0,
            TimeSeries {
                name: "Failed".into(),
                data: failed_agg_time
                    .await??
                    .into_iter()
                    .map(|(time, count)| (time.timestamp_millis() as u64, count as u64))
                    .collect(),
            },
        );
        if let Ok(res) = agg_type.await.unwrap() {
            res.into_iter().for_each(|(k, v)| {
                info.queries.insert(k, v as u64);
            });
        }
        if let Ok(res) = agg_filtered_true.await.unwrap() {
            res.into_iter().for_each(|(k, v)| {
                let k = if k.ends_with('.') {
                    k[..k.len() - 1].to_string()
                } else {
                    k
                };
                info.top_approved.insert(k, v as u64);
            });
        }
        if let Ok(res) = agg_filtered_false.await.unwrap() {
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
        Ok(Json(info))
    }

    Ok(_fetch_dashboard(days).await?)
}
