use std::path::Path;

use chrono::Local;
use once_cell::sync::OnceCell;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Connection, Executor, SqliteConnection, SqlitePool};

use crate::{next_maintenance, timer::Timer, PiConfig, PI_CONFIG};

pub mod dns_requests;
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
        .connect(&format!("sqlite://{}?mode=rw", db_path))
        .await?;
    POOL.set(pool)
        .map_err(|_| anyhow::anyhow!("Couldn't set sqlite pool"))?;

    if !db_opt.is_empty() {
        log::info!("Executing '{}' on db", db_opt);
        POOL.get().unwrap().execute(&**db_opt).await?;
    }
    tokio::spawn(clean_old_entries());
    log::info!("Database initialization done!");
    Ok(())
}

async fn create_db(db_path: &str) -> anyhow::Result<()> {
    let mut con = SqliteConnection::connect(&format!("sqlite://{}?mode=rwc", db_path)).await?;
    let _ = sqlx::query(include_str!("../../schema/create_tables.sql"))
        .execute(&mut con)
        .await?;
    log::info!("Created a new database file '{}'", db_path);
    Ok(())
}

async fn clean_old_entries() {
    async fn delete() -> anyhow::Result<()> {
        let overflow = Local::now().naive_local() - chrono::Duration::days(30);
        log::info!("Will try to entries older than {:?}", overflow);
        let count = sqlx::query!("delete from dns_requests where req_time < ?", overflow)
            .execute(POOL.get().unwrap())
            .await?
            .rows_affected();
        if count > 0 {
            log::warn!("Deleted {} entries from dns_requests table", count);
        }

        let count = sqlx::query!("delete from sys_info where s_time < ?", overflow)
            .execute(POOL.get().unwrap())
            .await?
            .rows_affected();
        if count > 0 {
            log::warn!("Deleted {} entries from sys_info table", count);
        }

        Ok(())
    }

    loop {
        let sleep_duration = next_maintenance() - Local::now().naive_local();
        let sleep_duration = sleep_duration.to_std().unwrap();
        log::info!(
            "Waiting for {} before cleaning up database",
            sleep_duration.t()
        );
        tokio::time::sleep(sleep_duration).await;
        delete().await.ok();
    }
}
