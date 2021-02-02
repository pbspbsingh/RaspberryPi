use serde_json::Value;
use sqlx::types::chrono::NaiveDateTime;

use crate::db::POOL;
use crate::web::ws_health_info;

#[derive(Debug, sqlx::FromRow)]
pub struct SysInfo {
    pub s_id: i64,
    pub s_time: NaiveDateTime,
    pub cpu_avg: Option<f32>,
    pub cpu_temp: Option<f32>,
    pub memory: Option<f32>,
    pub extras: Value,
}

pub async fn load_sys_info(from: NaiveDateTime) -> anyhow::Result<Vec<SysInfo>> {
    Ok(
        sqlx::query_as("select * from sys_info where s_time>=? order by s_time")
            .bind(from)
            .fetch_all(POOL.get().unwrap())
            .await?,
    )
}

pub async fn save(
    cpu_avg: Option<f32>,
    cpu_temp: Option<f32>,
    memory: Option<f32>,
    extras: &Value,
) -> anyhow::Result<i64> {
    ws_health_info(cpu_avg, cpu_temp, memory, extras);
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
