use sqlx::types::chrono::NaiveDateTime;

use crate::db::POOL;
use crate::web::ws_health_info;

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
    #[derive(Debug, sqlx::FromRow)]
    struct _SysInfo {
        pub s_id: i64,
        pub s_time: NaiveDateTime,
        pub cpu_avg: Option<f64>,
        pub cpu_temp: Option<f64>,
        pub memory: Option<f64>,
        pub temperature: Option<f64>,
        pub humidity: Option<f64>,
    }
    let results = sqlx::query_as!(
        _SysInfo,
        "select * from sys_info where s_time>=? order by s_time",
        from
    )
    .fetch_all(POOL.get().unwrap())
    .await?
    .into_iter()
    .map(|s| SysInfo {
        s_id: s.s_id,
        s_time: s.s_time,
        cpu_avg: s.cpu_avg.map(|f| f as f32),
        cpu_temp: s.cpu_temp.map(|f| f as f32),
        memory: s.memory.map(|f| f as f32),
        temperature: s.temperature.map(|f| f as f32),
        humidity: s.humidity.map(|f| f as f32),
    })
    .collect();
    Ok(results)
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
        r#"
        insert into sys_info(cpu_avg, cpu_temp, memory, temperature, humidity) 
        values(?, ?, ?, ?, ?)
        "#,
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
