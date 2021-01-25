use serde_json::Value;
use sqlx::types::chrono::NaiveDateTime;

use crate::db::POOL;

#[derive(Debug, sqlx::FromRow)]
pub struct SysInfo {
    s_id: i64,
    s_time: NaiveDateTime,
    cpu_avg: Option<f32>,
    cpu_temp: Option<f32>,
    memory: Option<f32>,
    extras: Value,
}

pub async fn fetch(from: NaiveDateTime) -> anyhow::Result<Vec<SysInfo>> {
    Ok(sqlx::query_as("select * from sys_info where s_time>=?")
        .bind(from)
        .fetch_all(POOL.get().unwrap())
        .await?)
}

pub async fn save(
    cpu_avg: Option<f32>,
    cpu_temp: Option<f32>,
    memory: Option<f32>,
    extras: &Value,
) -> anyhow::Result<i64> {
    Ok(sqlx::query!(
        "insert into sys_info(cpu_avg, cpu_temp, memory, extras) values(?, ?, ?, ?)",
        cpu_avg,
        cpu_temp,
        memory,
        extras
    )
    .execute(POOL.get().unwrap())
    .await?
    .last_insert_rowid())
}
