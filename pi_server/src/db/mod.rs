use once_cell::sync::OnceCell;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Executor, SqlitePool};
use tokio::time::Duration;

use crate::PiConfig;

pub mod dns_requests;
pub mod filters;
pub mod sys_info;

static POOL: OnceCell<SqlitePool> = OnceCell::new();

pub async fn init_db(config: &PiConfig) -> anyhow::Result<()> {
    log::info!("Connecting db sqlite://{}?mode=rw", config.db_path);
    let pool = SqlitePoolOptions::new()
        .max_connections(config.db_pool)
        .connect_timeout(Duration::from_secs(15))
        .connect(&format!("sqlite://{}?mode=rw", config.db_path))
        .await?;
    POOL.set(pool)
        .map_err(|_| anyhow::anyhow!("Couldn't set sqlite pool"))?;

    if !config.db_opt.is_empty() {
        log::info!("Executing '{}' on db", config.db_opt);
        POOL.get().unwrap().execute(&*config.db_opt).await?;
    }

    log::info!("Database initialization done!");
    Ok(())
}
