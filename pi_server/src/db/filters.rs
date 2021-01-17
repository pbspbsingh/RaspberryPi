use sqlx::types::chrono::NaiveDateTime;

use crate::db::POOL;

#[derive(Debug, sqlx::FromRow)]
pub struct DbFilter {
    pub f_id: i64,
    pub ct: NaiveDateTime,
    pub expr: String,
    pub is_regex: bool,
    pub enabled: bool,
    pub is_allow: bool,
}

pub async fn fetch_filters(is_allow: bool) -> anyhow::Result<Vec<DbFilter>> {
    Ok(sqlx::query_as!(
        DbFilter,
        "select * from filters where enabled=? and is_allow=?",
        true,
        is_allow
    )
    .fetch_all(POOL.get().unwrap())
    .await?)
}
