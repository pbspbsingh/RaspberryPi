use trust_dns_proto::op::Message;

use crate::db::POOL;
use sqlx::types::chrono::NaiveDateTime;
use std::collections::HashSet;
use std::time::Instant;

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

pub async fn get() -> anyhow::Result<Vec<DnsRequest>> {
    Ok(sqlx::query_as!(DnsRequest, "select * from dns_requests")
        .fetch_all(POOL.get().unwrap())
        .await?)
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
    log::debug!(
        "[{}] DnsRequest insertion time: {}ms",
        msg.id(),
        start.elapsed().as_millis()
    );
    Ok(done.last_insert_rowid())
}
