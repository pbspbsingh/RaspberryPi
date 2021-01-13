use sqlx::sqlite::SqlitePoolOptions;
use tokio::time::Duration;

use crate::db::POOL;

pub async fn init_db(db: impl AsRef<str>) -> anyhow::Result<()> {
    log::info!("Initializing database at file://{}", db.as_ref());
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_timeout(Duration::from_secs(15))
        .connect(&format!("sqlite://{}?mode=rwc", db.as_ref()))
        .await?;
    POOL.set(pool)
        .map_err(|_| anyhow::anyhow!("Couldn't set sqlite pool"))?;

    create_filters().await?;
    create_dns_requests().await?;

    log::info!("Database initialization done!");
    Ok(())
}

async fn create_filters() -> anyhow::Result<()> {
    sqlx::query(
        r"create table if not exists filters(
f_id INTEGER PRIMARY KEY,
ct DATETIME DEFAULT (strftime('%s','now')),
domain TEXT NOT NULL,
is_regex BOOLEAN NOT NULL,
enabled BOOLEAN NOT NULL,
is_allow BOOLEAN NOT NULL)",
    )
    .execute(POOL.get().unwrap())
    .await?;
    Ok(())
}

async fn create_dns_requests() -> anyhow::Result<()> {
    sqlx::query(
        r"create table if not exists dns_requests(
req_id INTEGER PRIMARY KEY,
req_time DATETIME DEFAULT (strftime('%s','now')),
req_type TEXT,
request TEXT,
response TEXT,
filtered BOOLEAN,
reason TEXT,
responded BOOLEAN NOT NULL)",
    )
    .execute(POOL.get().unwrap())
    .await?;
    Ok(())
}
