use std::collections::HashSet;
use std::time::Instant;

use sqlx::types::chrono::NaiveDateTime;
use trust_dns_proto::op::Message;

use crate::db::POOL;
use crate::Timer;
use chrono::Local;

#[derive(Debug, sqlx::FromRow)]
pub struct DnsRequest {
    pub req_id: i64,
    pub req_time: NaiveDateTime,
    pub req_type: Option<String>,
    pub request: Option<String>,
    pub response: Option<String>,
    pub filtered: Option<bool>,
    pub reason: Option<String>,
    pub responded: bool,
    pub resp_ms: i64,
}

pub async fn fetch_dns_reqs(from: NaiveDateTime) -> anyhow::Result<Vec<DnsRequest>> {
    let start = Instant::now();
    let res = sqlx::query_as!(
        DnsRequest,
        "select * from dns_requests where req_time >= ?",
        from
    )
    .fetch_all(POOL.get().unwrap())
    .await?;
    log::debug!("Time taken to fetch dns requests: {}", start.t());
    Ok(res)
}

pub async fn save(
    msg: &Message,
    filtered: Option<bool>,
    reason: Option<String>,
    responded: bool,
    resp_ms: i64,
) -> anyhow::Result<i64> {
    let req_type = msg.queries().first().map(|q| q.query_type().to_string());
    let req = msg.queries().first().map(|q| q.name().to_string());
    let res = msg
        .answers()
        .iter()
        .map(|a| a.rdata().to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" ");

    let start = Instant::now();
    let done = sqlx::query!(
        r"
insert into 
dns_requests(req_type, request, response, filtered, reason, responded, resp_ms)
values(?, ?, ?, ?, ?, ?, ?)",
        req_type,
        req,
        res,
        filtered,
        reason,
        responded,
        resp_ms
    )
    .execute(POOL.get().unwrap())
    .await?;
    log::debug!("[{}] DnsRequest insertion time: {}", msg.id(), start.t());
    Ok(done.last_insert_rowid())
}

pub async fn agg_by_time(
    from: NaiveDateTime,
) -> anyhow::Result<Vec<(NaiveDateTime, i64, f64, Option<bool>)>> {
    let agg_time = (Local::now().naive_local() - from).num_seconds() / 50;
    let start = Instant::now();
    let res = sqlx::query_as(&format!(
        r"select 
datetime((strftime('%s', req_time) / {0}) * {0}, 'unixepoch') interval, 
count(req_id),
avg(resp_ms), 
filtered 
from dns_requests where req_time >= ? and responded = true
group by interval, filtered order by interval",
        agg_time
    ))
    .bind(from)
    .fetch_all(POOL.get().unwrap())
    .await?;
    log::info!(
        "Time taken to aggregate dns requests from {}: {}",
        from,
        start.t()
    );
    Ok(res)
}

pub async fn agg_by_type(from: NaiveDateTime) -> anyhow::Result<Vec<(String, i64)>> {
    let start = Instant::now();
    let res = sqlx::query_as(
        r"select req_type, count(req_id) 
from dns_requests where req_time >= ? and responded = true
group by req_type",
    )
    .bind(from)
    .fetch_all(POOL.get().unwrap())
    .await?;
    log::info!("Type aggregation time {}", start.t());
    Ok(res)
}

pub async fn agg_by_filtered(
    from: NaiveDateTime,
    is_filtered: bool,
) -> anyhow::Result<Vec<(String, i64)>> {
    let start = Instant::now();
    let res = sqlx::query_as(
        r"select request, count(req_id) cnt 
from dns_requests where req_time >= ? and filtered = ? and responded = true
group by request order by cnt desc limit 10",
    )
    .bind(from)
    .bind(is_filtered)
    .fetch_all(POOL.get().unwrap())
    .await?;
    log::info!("Request aggregation time {}", start.t());
    Ok(res)
}
