use std::collections::HashSet;
use std::io::{Cursor, Read};

use axum::body::Body;
use axum::extract::WebSocketUpgrade;
use axum::http::Request;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::{routing, Json, Router, Server};
use http::{header, StatusCode};
use log::*;
use once_cell::sync::Lazy;
use routing::{get, post};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

pub use health::ws_health_info;
pub use queries::ws_dns_req;
pub use websocket::ws_sender;

use crate::web::config::{fetch_config, save_config};
use crate::web::dashboard::fetch_dashboard;
use crate::web::health::fetch_health_info;
use crate::web::queries::fetch_queries;
use crate::web::websocket::handle_ws;
use crate::{PiConfig, PI_CONFIG};

mod config;
mod dashboard;
mod health;
mod queries;
mod websocket;

static HOME_URLS: Lazy<HashSet<&str>> =
    Lazy::new(|| ["/", "/queries", "/filters", "/health"].into());
const STATIC_ASSETS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/static_assets.zip"));

pub async fn start_web_server() -> anyhow::Result<()> {
    info!("Static web assets zipped size: {}", STATIC_ASSETS.len());
    let PiConfig { web_port, .. } = PI_CONFIG.get().unwrap();

    let app = Router::new()
        .route("/config", get(fetch_config))
        .route("/config", post(save_config))
        .route("/dashboard/:days", get(fetch_dashboard))
        .route("/health/:days", get(fetch_health_info))
        .route("/queries/:days", get(fetch_queries))
        .route(
            "/websocket",
            get(|ws: WebSocketUpgrade| async { ws.on_upgrade(handle_ws) }),
        )
        .fallback(get(map_static_assets));

    info!("Starting web server at port {web_port}");
    Server::bind(&([0, 0, 0, 0], *web_port as u16).into())
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebError {
    pub error: String,
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        error!("Sending error response: '{}'", self.error);
        let mut response = Json(self).into_response();
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    }
}

impl From<anyhow::Error> for WebError {
    fn from(e: anyhow::Error) -> Self {
        WebError {
            error: e.to_string(),
        }
    }
}

async fn map_static_assets(request: Request<Body>) -> impl IntoResponse {
    let path = request.uri().path();
    let lookup_file = if HOME_URLS.contains(path) {
        "index.html"
    } else {
        &path[1..]
    };

    let mut response = None;
    if let Ok(mut zip) = ZipArchive::new(Cursor::new(STATIC_ASSETS)) {
        if let Ok(mut file) = zip.by_name(lookup_file) {
            let mime = mime_guess::from_path(file.name())
                .first()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "text/plain".into());
            let mut content = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut content).ok();
            trace!("Static file: {}", lookup_file);
            response = Some((StatusCode::OK, (header::CONTENT_TYPE, mime), content))
        }
    } else {
        error!("Failed to open static assets bytes as zip file!");
    }
    let response = match response {
        Some(res) => res,
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            (header::CONTENT_TYPE, "text/plain".into()),
            b"Resource not found, or something went wrong!".to_vec(),
        ),
    };
    (response.0, [response.1], response.2)
}
