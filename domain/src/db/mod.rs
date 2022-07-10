use log::*;
use once_cell::sync::OnceCell;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Connection, Pool, Sqlite, SqliteConnection};
use std::path::Path;

const DB_FILE: &str = "domain.db";

static DB: OnceCell<Pool<Sqlite>> = OnceCell::new();

pub mod block_list;
pub mod filters;

pub async fn init_db() -> anyhow::Result<bool> {
    let mut is_new = false;
    if !Path::new(DB_FILE).exists() {
        info!("Sqlite db:{DB_FILE} doesn't exit, creating it...");
        let mut connection =
            SqliteConnection::connect(&format!("sqlite://{DB_FILE}?mode=rwc")).await?;
        sqlx::query(include_str!("../../schema/init_db.sql"))
            .execute(&mut connection)
            .await?;
        is_new = true;
    }
    DB.set(
        SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&format!("sqlite://{DB_FILE}?mode=rw"))
            .await?,
    )
    .expect("Failed to set DB once_cell");
    sqlx::query("PRAGMA synchronous=OFF;")
        .execute(DB.get().unwrap())
        .await?;
    Ok(is_new)
}

pub(crate) async fn vacuum() -> anyhow::Result<()> {
    sqlx::query!("VACUUM;").execute(db()).await?;
    Ok(())
}

pub(crate) fn db() -> &'static Pool<Sqlite> {
    DB.get().unwrap()
}
