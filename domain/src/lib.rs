use log::*;

use db::init_db;
pub use filters::{check_filters, reload_filters};

pub mod block_list;
pub mod db;
mod filters;

pub async fn init() -> anyhow::Result<()> {
    info!("Initializing domain db...");
    let _ = init_db().await?;

    info!("Initializing filters...");
    reload_filters().await?;

    Ok(())
}
