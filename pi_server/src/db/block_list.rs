use std::time::Instant;

use chrono::{Local, NaiveDateTime};

use crate::db::POOL;
use crate::Timer;

pub struct BlockListItem {
    pub b_id: i64,
    pub b_src: String,
    pub b_count: Option<i64>,
    pub b_last_updated: NaiveDateTime,
}

pub async fn db_block_list() -> anyhow::Result<Vec<BlockListItem>> {
    Ok(sqlx::query_as!(
        BlockListItem,
        "select * from block_list order by b_count desc"
    )
    .fetch_all(POOL.get().unwrap())
    .await?)
}

pub async fn update_block_list(bl: impl AsRef<[BlockListItem]>) -> anyhow::Result<()> {
    let mut transaction = POOL.get().unwrap().begin().await?;
    let now = Local::now().naive_local();
    for bi in bl.as_ref() {
        sqlx::query!(
            "update block_list set b_count = ?, b_last_updated =? where b_src=?",
            bi.b_count,
            now,
            bi.b_src
        )
        .execute(&mut transaction)
        .await?;
    }
    Ok(transaction.commit().await?)
}

pub async fn replace_block_list(block_list: impl IntoIterator<Item = &str>) -> anyhow::Result<()> {
    let start = Instant::now();
    let mut transaction = POOL.get().unwrap().begin().await?;
    sqlx::query!("delete from block_list")
        .execute(&mut transaction)
        .await?;
    let mut count = 0;
    let now = Local::now().naive_local();
    for src in block_list {
        sqlx::query!(
            "insert into block_list(b_src, b_count, b_last_updated) values(?, ?, ?)",
            src,
            -1,
            now
        )
        .execute(&mut transaction)
        .await?;
        count += 1;
    }
    log::info!("Inserted {} sources in block list in {}", count, start.t());
    Ok(transaction.commit().await?)
}
