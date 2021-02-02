use chrono::{Duration, Local};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use warp::reply::json;
use warp::{Rejection, Reply};

use crate::db::sys_info::load_sys_info;
use crate::web::websocket::{send_ws_msg, WsMessage};
use crate::web::WebError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Health {
    name: &'static str,
    data: Vec<(u64, f64)>,
}

pub async fn fetch_health_info(days: u32) -> Result<impl Reply, Rejection> {
    let from = Local::now().naive_local() - Duration::days(days as i64);
    let mut cpu_avg = Vec::new();
    let mut memory = Vec::new();
    let mut cpu_temp = Vec::new();
    for si in load_sys_info(from).await.map_err(WebError::new)? {
        let time = si.s_time.timestamp_millis() as u64;
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
    Ok(json(&[
        Health {
            name: "CPU Average (Per Minute)",
            data: cpu_avg,
        },
        Health {
            name: "Memory Usage",
            data: memory,
        },
        Health {
            name: "CPU Temperature (C)",
            data: cpu_temp,
        },
    ]))
}

pub fn ws_health_info(
    cpu_avg: Option<f32>,
    cpu_temp: Option<f32>,
    memory: Option<f32>,
    extras: &Value,
) {
    let now = Local::now().naive_local().timestamp_millis();
    let message = serde_json::json!({
        "health": {
            "time": now,
            "cpu_avg": cpu_avg.map(truncate),
            "cpu_temp": cpu_temp.map(truncate),
            "memory": memory.map(truncate),
            "extras": extras,
        }
    })
    .to_string();
    send_ws_msg(WsMessage::SendAll(message));
}

fn truncate(num: f32) -> f64 {
    (num as f64 * 1000.).trunc() / 1000.
}
