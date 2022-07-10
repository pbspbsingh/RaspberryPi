use crate::db::db;
use chrono::NaiveDateTime;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbFilter {
    pub f_id: i64,
    pub create_time: NaiveDateTime,
    pub expr: String,
    pub is_regex: bool,
    pub enabled: bool,
    pub is_allow: bool,
}

pub async fn load_filters() -> anyhow::Result<Vec<DbFilter>> {
    Ok(sqlx::query_as!(
        DbFilter,
        r"select * from filters where enabled=true order by create_time desc"
    )
    .fetch_all(db())
    .await?)
}

pub async fn load_all_filters() -> anyhow::Result<Vec<DbFilter>> {
    Ok(
        sqlx::query_as!(DbFilter, r"select * from filters order by create_time desc")
            .fetch_all(db())
            .await?,
    )
}

pub async fn save_filters(filters: impl IntoIterator<Item = DbFilter>) -> anyhow::Result<()> {
    let mut trans = db().begin().await?;
    sqlx::query!("delete from filters")
        .execute(&mut trans)
        .await?;
    for filter in filters {
        sqlx::query!(
            r#"
            insert into filters(expr, is_regex, enabled, is_allow)
            values(?, ?, ?, ?)
            "#,
            filter.expr,
            filter.is_regex,
            filter.enabled,
            filter.is_allow,
        )
        .execute(&mut trans)
        .await?;
    }
    Ok(trans.commit().await?)
}
