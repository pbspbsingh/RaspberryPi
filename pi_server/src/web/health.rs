use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{Duration, Local};
use serde::{Deserialize, Serialize};

use crate::db::sys_info::load_sys_info;
use crate::web::websocket::{send_ws_msg, WsMessage};
use crate::web::WebError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Health {
    name: &'static str,
    data: Vec<(u64, f64)>,
}

pub async fn fetch_health_info(Path(days): Path<u32>) -> Result<impl IntoResponse, WebError> {
    let from = Local::now().naive_local() - Duration::days(days as i64);
    let mut temperature = Vec::new();
    let mut humidity = Vec::new();
    let mut cpu_avg = Vec::new();
    let mut memory = Vec::new();
    let mut cpu_temp = Vec::new();

    for si in load_sys_info(from).await.map_err(anyhow::Error::from)? {
        let time = si.s_time.timestamp_millis() as u64;
        if let Some(temp) = si.temperature {
            temperature.push((time, truncate(temp)));
        }
        if let Some(h) = si.humidity {
            humidity.push((time, truncate(h)));
        }
        if let Some(ca) = si.cpu_avg {
            cpu_avg.push((time, truncate(ca)));
        }
        if let Some(m) = si.memory {
            memory.push((time, truncate(m)));
        }
        if let Some(ct) = si.cpu_temp {
            cpu_temp.push((time, truncate(ct)));
        }
    }
    Ok(Json([
        Health {
            name: "Temperature ºC",
            data: temperature,
        },
        Health {
            name: "Humidity",
            data: humidity,
        },
        Health {
            name: "CPU Average (Per Minute)",
            data: cpu_avg,
        },
        Health {
            name: "CPU Temperature (C)",
            data: cpu_temp,
        },
        Health {
            name: "Memory Usage",
            data: memory,
        },
    ]))
}

pub fn ws_health_info(
    cpu_avg: Option<f32>,
    cpu_temp: Option<f32>,
    memory: Option<f32>,
    temperature: Option<f32>,
    humidity: Option<f32>,
) {
    let now = Local::now().naive_local().timestamp_millis();
    let message = serde_json::json!({
        "health": {
            "time": now,
            "temperature": temperature.map(truncate),
            "humidity": humidity.map(truncate),
            "cpu_avg": cpu_avg.map(truncate),
            "cpu_temp": cpu_temp.map(truncate),
            "memory": memory.map(truncate),
        }
    })
    .to_string();
    log::info!("{}", message);
    send_ws_msg(WsMessage::SendAll(message));
}

fn truncate(num: f32) -> f64 {
    (num as f64 * 1000.).trunc() / 1000.
}
