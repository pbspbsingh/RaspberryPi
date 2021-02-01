use std::path::Path;

use once_cell::sync::OnceCell;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Executor, SqlitePool};
use tokio::time::Duration;

use crate::{PiConfig, PI_CONFIG};

pub mod block_list;
pub mod dns_requests;
pub mod filters;
pub mod sys_info;

static POOL: OnceCell<SqlitePool> = OnceCell::new();

pub async fn init_db() -> anyhow::Result<()> {
    let PiConfig {
        db_path,
        db_pool,
        db_opt,
        ..
    } = PI_CONFIG.get().unwrap();
    log::info!("Connecting db sqlite://{}?mode=rw", db_path);
    if !Path::new(db_path).exists() {
        log::info!("DB hasn't been created yet, creating it...");
        create_db(db_path).await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(*db_pool)
        .connect_timeout(Duration::from_secs(15))
        .connect(&format!("sqlite://{}?mode=rw", db_path))
        .await?;
    POOL.set(pool)
        .map_err(|_| anyhow::anyhow!("Couldn't set sqlite pool"))?;

    if !db_opt.is_empty() {
        log::info!("Executing '{}' on db", db_opt);
        POOL.get().unwrap().execute(&**db_opt).await?;
    }

    log::info!("Database initialization done!");
    Ok(())
}

async fn create_db(db_path: &str) -> anyhow::Result<()> {
    let pool = SqlitePoolOptions::new()
        .connect_timeout(Duration::from_secs(15))
        .max_connections(1)
        .connect(&format!("sqlite://{}?mode=rwc", db_path))
        .await?;
    let _ = sqlx::query_file!("src/migration.sql")
        .execute(&pool)
        .await?;
    log::info!("Created a new database file '{}'", db_path);
    Ok(())
}
