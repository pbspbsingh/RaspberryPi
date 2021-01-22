use std::io::{Cursor, Read};

use http::header;
use http::response::Response;
use warp::filters::path::FullPath;
use warp::hyper::StatusCode;
use warp::{filters, Filter};
use zip::ZipArchive;

use crate::PiConfig;

const STATIC_ASSETS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/static_assets.zip"));

pub async fn start_web_server(config: &PiConfig) -> anyhow::Result<()> {
    let filters = warp::get()
        .and(filters::path::full())
        .map(map_static_assets);
    log::info!("Starting web server at port {}", config.web_port);
    Ok(warp::serve(filters)
        .run(([0, 0, 0, 0], config.web_port as u16))
        .await)
}

fn map_static_assets(path: FullPath) -> http::Result<Response<Vec<u8>>> {
    let response = Response::builder();
    let lookup_file = if path.as_str() == "/" {
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
