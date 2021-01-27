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

pub async fn fetch_filters(is_allow: Option<bool>) -> anyhow::Result<Vec<DbFilter>> {
    let query = if let Some(is_allow) = is_allow {
        sqlx::query_as!(
            DbFilter,
            "select * from filters where enabled=true and is_allow=? order by ct desc",
            is_allow
        )
        .fetch_all(POOL.get().unwrap())
        .await
    } else {
        sqlx::query_as!(DbFilter, "select * from filters")
            .fetch_all(POOL.get().unwrap())
            .await
    };
    Ok(query?)
}

pub async fn save_filters(filters: Vec<DbFilter>) -> anyhow::Result<()> {
    let mut transaction = POOL.get().unwrap().begin().await?;
    sqlx::query!("delete from filters")
        .execute(&mut transaction)
        .await?;
    log::debug!("Removed old rows from filters table.");
    let mut count = 0;
    for DbFilter {
        expr,
        is_regex,
        enabled,
        is_allow,
        ..
    } in filters
    {
        sqlx::query!(
            "insert into filters (expr, is_regex, enabled, is_allow) values(?, ?, ?, ?)",
            expr,
            is_regex,
            enabled,
            is_allow
        )
        .execute(&mut transaction)
        .await?;
        count += 1;
    }
    log::info!("Filters table updated with {} rows", count);
    Ok(transaction.commit().await?)
}
