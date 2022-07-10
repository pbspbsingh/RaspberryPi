use std::collections::HashMap;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db::dns_requests::{fetch_dns_reqs, DnsRequest};
use crate::web::websocket::{send_ws_msg, WsMessage};
use crate::web::WebError;

#[derive(Debug, Serialize, Deserialize)]
struct WebQuery {
    id: i64,
    req_time: u64,
    requester: String,
    req_type: String,
    name: String,
    responded: bool,
    reply: Option<String>,
    filtered: Option<bool>,
    reason: Option<String>,
    resp_time: u64,
}

impl WebQuery {
    fn from(dr: DnsRequest) -> WebQuery {
        WebQuery {
            id: dr.req_id,
            req_time: dr.req_time.timestamp_millis() as u64,
            requester: dr.requester,
            req_type: dr.req_type.unwrap_or_else(|| "Unknown".into()),
            name: dr.request.unwrap_or_default(),
            responded: dr.responded,
            reply: dr.response,
            filtered: dr.filtered,
            reason: dr.reason,
            resp_time: dr.resp_ms as u64,
        }
    }
}

pub async fn fetch_queries(Path(limit): Path<u32>) -> Result<impl IntoResponse, WebError> {
    let dns_reqs = fetch_dns_reqs(limit).await?;
    let queries = dns_reqs.into_iter().map(WebQuery::from).collect::<Vec<_>>();
    Ok(Json(queries))
}

pub fn ws_dns_req(dr: DnsRequest) {
    let mut payload = HashMap::new();
    payload.insert("query", WebQuery::from(dr));
    if let Ok(s) = serde_json::to_string(&payload) {
        send_ws_msg(WsMessage::SendAll(s));
    }
}
