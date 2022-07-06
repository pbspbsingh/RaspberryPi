use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::db::dns_requests::{fetch_dns_reqs, DnsRequest};
use crate::web::websocket::{send_ws_msg, WsMessage};
use crate::web::WebError;

#[derive(Debug, Serialize, Deserialize)]
struct Query {
    id: i64,
    req_time: u64,
    req_type: String,
    name: String,
    responded: bool,
    reply: Option<String>,
    filtered: Option<bool>,
    reason: Option<String>,
    resp_time: u64,
}

pub async fn fetch_queries(Path(limit): Path<u32>) -> Result<impl IntoResponse, WebError> {
    let dns_reqs = fetch_dns_reqs(limit).await.map_err(anyhow::Error::from)?;
    let queries = dns_reqs
        .into_iter()
        .map(
            |DnsRequest {
                 req_id,
                 req_time,
                 req_type,
                 request,
                 responded,
                 response,
                 filtered,
                 reason,
                 resp_ms,
             }| Query {
                id: req_id,
                req_time: req_time.timestamp_millis() as u64,
                req_type: req_type.unwrap_or_else(|| "Unknown".to_string()),
                name: request
                    .map(|s| {
                        if s.ends_with('.') {
                            s[..s.len() - 1].to_string()
                        } else {
                            s
                        }
                    })
                    .unwrap_or_else(|| "".to_string()),
                responded,
                reply: response,
                filtered,
                reason,
                resp_time: resp_ms as u64,
            },
        )
        .collect::<Vec<_>>();
    Ok(Json(queries))
}

#[allow(clippy::too_many_arguments)]
pub fn ws_dns_req(
    req_id: i64,
    req_time: NaiveDateTime,
    req_type: Option<String>,
    request: Option<String>,
    response: String,
    filtered: Option<bool>,
    reason: Option<String>,
    responded: bool,
    resp_ms: i64,
) {
    let query = Query {
        id: req_id,
        req_time: req_time.timestamp_millis() as u64,
        req_type: req_type.unwrap_or_else(|| "Unknown".to_string()),
        name: request
            .map(|s| {
                if s.ends_with('.') {
                    s[..s.len() - 1].to_owned()
                } else {
                    s
                }
            })
            .unwrap_or_else(|| "".to_string()),
        responded,
        reply: Some(response),
        filtered,
        reason: reason.map(String::from),
        resp_time: resp_ms as u64,
    };
    let mut payload = HashMap::new();
    payload.insert("query", query);
    if let Ok(s) = serde_json::to_string(&payload) {
        send_ws_msg(WsMessage::SendAll(s));
    }
}
