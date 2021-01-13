use once_cell::sync::OnceCell;
use sqlx::types::chrono::{DateTime, Local};
use sqlx::SqlitePool;

pub use init::init_db;
use trust_dns_proto::op::Message;

mod init;

static POOL: OnceCell<SqlitePool> = OnceCell::new();

#[derive(Debug, sqlx::FromRow)]
pub struct DbFilter {
    pub f_id: i64,
    pub ct: DateTime<Local>,
    pub domain: String,
    pub is_regex: bool,
    pub enabled: bool,
    pub is_allow: bool,
}

pub async fn fetch_filters(is_allow: bool) -> anyhow::Result<Vec<DbFilter>> {
    Ok(
        sqlx::query_as("select * from filters where enabled=true and is_allow=?")
            .bind(is_allow)
            .fetch_all(POOL.get().unwrap())
            .await?,
    )
}

pub async fn save_dns_request(
    response: &Message,
    filtered: Option<bool>,
    reason: Option<String>,
    responded: bool,
) -> anyhow::Result<i64> {
    let req_type = response
        .queries()
        .first()
        .map(|q| q.query_type().to_string());
    let req = response.queries().first().map(|q| q.name().to_string());
    let res = response.answers().first().map(|a| a.rdata().to_string());
    let done = sqlx::query(
        r"insert into 
dns_requests(req_type, request, response, filtered, reason, responded)
values(?, ?, ?, ?, ?, ?)",
    )
    .bind(req_type)
    .bind(req)
    .bind(res)
    .bind(filtered)
    .bind(reason)
    .bind(responded)
    .execute(POOL.get().unwrap())
    .await?;
    Ok(done.last_insert_rowid())
}
