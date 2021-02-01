use std::collections::HashSet;
use std::io::{Cursor, Read};

use http::header;
use http::response::Response;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use warp::filters::compression;
use warp::filters::path::FullPath;
use warp::hyper::StatusCode;
use warp::reject::Reject;
use warp::{filters, Filter};
use zip::ZipArchive;

pub use queries::ws_dns_req;
pub use websocket::ws_sender;

use crate::web::config::{get_config, save_config};
use crate::web::dashboard::fetch_dashboard;
use crate::web::queries::fetch_queries;
use crate::web::websocket::handle_ws;
use crate::{PiConfig, PI_CONFIG};

mod config;
mod dashboard;
mod queries;
mod websocket;

static HOME_URLS: OnceCell<HashSet<&str>> = OnceCell::new();
const STATIC_ASSETS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/static_assets.zip"));

#[derive(Debug, Serialize, Deserialize)]
pub struct WebError {
    pub error: String,
}

impl Reject for WebError {}

impl WebError {
    pub fn new(e: anyhow::Error) -> Self {
        WebError {
            error: e.to_string(),
        }
    }
}

pub async fn start_web_server() -> anyhow::Result<()> {
    let PiConfig { web_port, .. } = PI_CONFIG.get().unwrap();
    let dashboard = warp::get()
        .and(warp::path!("dashboard" / u32))
        .and_then(fetch_dashboard);
    let queries = warp::get()
        .and(warp::path!("queries" / u32))
        .and_then(fetch_queries);
    let config_fetch = warp::get().and(warp::path("config")).and_then(get_config);
    let config_save = warp::post()
        .and(warp::path("config"))
        .and(warp::body::form())
        .and_then(save_config);
    let websocket = warp::get()
        .and(warp::path("websocket"))
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(handle_ws));
    let assets = warp::get()
        .and(filters::path::full())
        .map(map_static_assets);

    log::info!("Starting web server at port {}", web_port);

    let filters = dashboard
        .or(queries)
        .or(config_fetch)
        .or(config_save)
        .or(websocket)
        .or(assets);
    Ok(warp::serve(filters.with(compression::gzip()))
        .run(([0, 0, 0, 0], *web_port as u16))
        .await)
}

fn map_static_assets(path: FullPath) -> http::Result<Response<Vec<u8>>> {
    let response = Response::builder();
    let home_urls = HOME_URLS.get_or_init(|| {
        ["/", "/queries", "/filters", "/health"]
            .iter()
            .map(|s| *s)
            .collect::<HashSet<_>>()
    });
    let lookup_file = if home_urls.contains(path.as_str()) {
        "index.html"
    } else {
        &path.as_str()[1..]
    };
    if let Ok(mut zip) = ZipArchive::new(Cursor::new(STATIC_ASSETS)) {
        if let Ok(mut file) = zip.by_name(lookup_file) {
            let mime = mime_guess::from_path(file.name())
                .first()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "text/plain".into());
            let mut content = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut content).ok();
            log::trace!("Static file: {}", lookup_file);
            return response
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(content);
        }
    } else {
        log::error!("Failed to open static assets bytes as zip file!");
    }
    log::warn!("File not found: {}", lookup_file);
    response
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(b"Not Found!".to_vec())
}
