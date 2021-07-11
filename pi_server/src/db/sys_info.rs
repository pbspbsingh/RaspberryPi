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
    pub temperature: Option<f32>,
    pub humidity: Option<f32>,
}

pub async fn load_sys_info(from: NaiveDateTime) -> anyhow::Result<Vec<SysInfo>> {
    Ok(sqlx::query_as!(
        SysInfo,
        "select * from sys_info where s_time>=? order by s_time",
        from
    )
    .fetch_all(POOL.get().unwrap())
    .await?)
}

pub async fn save(
    cpu_avg: Option<f32>,
    cpu_temp: Option<f32>,
    memory: Option<f32>,
    temperature: Option<f32>,
    humidity: Option<f32>,
) -> anyhow::Result<i64> {
    ws_health_info(cpu_avg, cpu_temp, memory, temperature, humidity);
    Ok(sqlx::query!(
        "insert into sys_info(cpu_avg, cpu_temp, memory, temperature, humidity) values(?, ?, ?, ?, ?)",
        cpu_avg,
        cpu_temp,
        memory,
        temperature,
        humidity
    )
        .execute(POOL.get().unwrap())
        .await?
        .last_insert_rowid())
}
