use crate::db::db;
use chrono::NaiveDateTime;

use itertools::Itertools;

use sqlx::{Sqlite, Transaction};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbBlockList {
    pub bl_id: i64,
    pub src: String,
    pub retry_count: i64,
    pub domain_count: i64,
    pub last_updated: NaiveDateTime,
}

pub async fn load_block_list() -> anyhow::Result<Vec<DbBlockList>> {
    Ok(sqlx::query_as!(
        DbBlockList,
        r#"select * from block_list order by domain_count desc"#
    )
    .fetch_all(db())
    .await?)
}

pub async fn save_block_list(list: impl IntoIterator<Item = DbBlockList>) -> anyhow::Result<()> {
    let mut trans = db().begin().await?;
    sqlx::query!("delete from block_list")
        .execute(&mut trans)
        .await?;
    for bl in list {
        let _ = sqlx::query!(
            r"
            insert into block_list(src, last_updated)
            values(?, ?)
            ",
            bl.src,
            bl.last_updated,
        )
        .execute(&mut trans)
        .await?;
    }
    Ok(trans.commit().await?)
}

pub async fn find_blocked_domain(
    name: impl AsRef<str>,
) -> anyhow::Result<Option<(String, String)>> {
    fn sub_names(name: &str) -> Vec<&str> {
        let name = name.strip_suffix('.').unwrap_or(name);
        let mut result = Vec::with_capacity(name.split('.').count());
        result.push(name);
        result.extend(
            name.match_indices('.')
                .map(|(idx, _)| name[idx + 1..].trim())
                .filter(|x| !x.is_empty()),
        );
        result
    }

    let names = sub_names(name.as_ref());
    let query = format!(
        "select domain_name, source from blocked_domains where domain_name in ({})",
        (0..names.len()).map(|_| '?').join(", ")
    );
    let mut query = sqlx::query_as(&query);
    for name in names {
        query = query.bind(name);
    }
    Ok(query.fetch_optional(db()).await?)
}

pub(crate) async fn blocked_domain_last_updated() -> anyhow::Result<Option<NaiveDateTime>> {
    Ok(sqlx::query_as::<_, (i64, NaiveDateTime)>(
        r#"select bd_id, updated from blocked_domains order by bd_id desc limit 5"#,
    )
    .fetch_optional(db())
    .await?
    .map(|(_, updated)| updated))
}

pub(crate) async fn clear_blocked_domain(
    trans: &mut Transaction<'_, Sqlite>,
) -> anyhow::Result<u64> {
    Ok(sqlx::query!("delete from blocked_domains")
        .execute(trans)
        .await?
        .rows_affected())
}

pub(crate) async fn update_block_list(
    trans: &mut Transaction<'_, Sqlite>,
    bl: DbBlockList,
) -> anyhow::Result<()> {
    sqlx::query!(
        r"
        update block_list 
        set retry_count=?, domain_count=?, last_updated=?
        where bl_id=?
        ",
        bl.retry_count,
        bl.domain_count,
        bl.last_updated,
        bl.bl_id,
    )
    .execute(trans)
    .await?;
    Ok(())
}

pub(crate) async fn insert_blocked_domain(
    trans: &mut Transaction<'_, Sqlite>,
    domain: &str,
    src: &str,
    updated: NaiveDateTime,
) -> bool {
    sqlx::query!(
        r"insert into blocked_domains(domain_name, source, updated) values(?, ?, ?)",
        domain,
        src,
        updated,
    )
    .execute(trans)
    .await
    .is_ok()
}
