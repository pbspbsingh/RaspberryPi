use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use std::env;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

const STATIC_ASSETS_DIR: &str = "../pi_client/build";
const STATIC_ASSETS_ZIP: &str = "static_assets.zip";

fn main() {
    println!("cargo:rerun-if-changed={}", STATIC_ASSETS_DIR);
    if let Err(e) = build_zip() {
        eprintln!("Failed to build static assets zip file: {}", e);
    }
}

fn build_zip() -> anyhow::Result<()> {
    let output = Path::new(&env::var_os("OUT_DIR").unwrap()).join(STATIC_ASSETS_ZIP);
    let mut writer = ZipWriter::new(File::create(output)?);
    if Path::new(STATIC_ASSETS_DIR).exists() {
        let options = FileOptions::default().compression_method(CompressionMethod::Bzip2);
        let files = bfs(STATIC_ASSETS_DIR)?;
        for file in &files {
            let name = file.display().to_string();
            let name = &name[STATIC_ASSETS_DIR.len() + 1..];
            writer.start_file(name, options)?;
            writer.write_all(&std::fs::read(file)?)?;
        }
    }
    Ok(())
}

fn bfs(build_dir: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut res = Vec::new();

    let mut queue = VecDeque::new();
    queue.push_back(Path::new(build_dir).to_path_buf());

    while !queue.is_empty() {
        let polled = queue.pop_front().unwrap();
        if polled.is_file() {
            res.push(polled);
        } else if polled.is_dir() {
            let dir = polled.read_dir()?;
            for dir_entry in dir {
                let file = dir_entry?;
                queue.push_back(file.path());
            }
        }
    }
    Ok(res)
}
