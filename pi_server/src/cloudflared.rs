use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::{PiConfig, PI_CONFIG};

pub async fn init_cloudflare<'a>() -> anyhow::Result<Cloudflared<'a>> {
    let PiConfig {
        cloudflared_path,
        cloudflared_port,
        ..
    } = PI_CONFIG.get().unwrap();
    log::info!(
        "Using cloudflared path: '{}', port: {}",
        cloudflared_path,
        cloudflared_port
    );

    let version_output = match Command::new(cloudflared_path)
        .arg("--version")
        .output()
        .await
    {
        Err(e) => {
            log::error!("Failed to start cloudflared process: {}", e);
            log::error!("Download cloudflared from 'https://dl.equinox.io/cloudflare/cloudflared/stable' for your platform");
            eprintln!("Download cloudflared from 'https://dl.equinox.io/cloudflare/cloudflared/stable' for your platform");
            return Err(e.into());
        }
        Ok(cmd) => cmd,
    };
    if !version_output.status.success() {
        log::error!("cloudflared failed to start: {}", version_output.status);
        return Err(anyhow::anyhow!(
            "Failed to start cloudflared daemon: {}",
            version_output.status
        ));
    } else {
        let output = String::from_utf8_lossy(&version_output.stdout);
        log::info!("cloudflared version: {}", output.trim());
    }

    Ok(Cloudflared {
        cmd: cloudflared_path,
        port: *cloudflared_port,
    })
}

pub struct Cloudflared<'a> {
    cmd: &'a str,
    port: u32,
}

impl<'a> Cloudflared<'a> {
    pub async fn start_daemon(&self) -> anyhow::Result<()> {
        self.update().await?;

        log::info!(
            "Starting cloudflared daemon: `{} proxy-dns --port {}`",
            self.cmd,
            self.port
        );
        let mut daemon = Command::new(&self.cmd)
            .arg("proxy-dns")
            .arg("--port")
            .arg(self.port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let mut stdout = daemon.stdout.take().unwrap();
        let mut stderr = daemon.stderr.take().unwrap();
        let mut buff = [0u8; 1024];
        loop {
            match tokio::time::timeout(Duration::from_secs(5), stdout.read(&mut buff)).await {
                Err(_) => {}
                Ok(len) => log::info!("stdout: {}", String::from_utf8_lossy(&buff[..len?])),
            }
            match tokio::time::timeout(Duration::from_secs(5), stderr.read(&mut buff)).await {
                Err(_) => {}
                Ok(len) => log::info!("stderr: {}", String::from_utf8_lossy(&buff[..len?])),
            }
            if daemon.id().is_none() {
                log::warn!("oops, cloudflared process has died");
                break;
            }
        }
        Err(anyhow::anyhow!("cloudflared daemon died unexpectedly!"))
    }

    async fn update(&self) -> anyhow::Result<()> {
        let update_output = Command::new(&self.cmd).arg("update").output().await?;
        let mut buff = String::from_utf8_lossy(&update_output.stdout);
        buff += String::from_utf8_lossy(&update_output.stderr);
        log::info!("Updating cloudflared: {}", buff.trim());
        Ok(())
    }
}
