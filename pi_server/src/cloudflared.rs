use std::process::Stdio;
use std::time::Duration;

use chrono::Local;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time;

use crate::{next_maintenance, PiConfig, PI_CONFIG};

const CF_URL: &str = "https://github.com/cloudflare/cloudflared/releases";

pub mod error {
    use std::sync::atomic::{AtomicU32, Ordering};

    pub(super) const ERROR_LIMIT: u32 = 100;

    static COUNT: AtomicU32 = AtomicU32::new(0);

    pub fn inc_count() {
        COUNT.fetch_add(1, Ordering::AcqRel);
    }

    pub(super) fn reset_count() {
        COUNT.store(0, Ordering::Release);
    }

    pub(super) fn count() -> u32 {
        COUNT.load(Ordering::Acquire)
    }
}

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
            log::error!("Download cloudflared from '{}' for your platform", CF_URL);
            eprintln!("Download cloudflared from '{}' for your platform", CF_URL);
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
        loop {
            let maintenance_time = next_maintenance();
            log::info!("Will restart the daemon at {}", maintenance_time);

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
            let wait_time = Duration::from_millis(2500);
            loop {
                if let Ok(len) = time::timeout(wait_time, stdout.read(&mut buff)).await {
                    let out = String::from_utf8_lossy(&buff[..len?]);
                    let out = out.trim();
                    if !out.is_empty() {
                        log::info!("stdout: {}", out);
                    }
                }
                if let Ok(len) = time::timeout(wait_time, stderr.read(&mut buff)).await {
                    let out = String::from_utf8_lossy(&buff[..len?]);
                    let out = out.trim();
                    if !out.is_empty() {
                        log::info!("stderr: {}", out);
                    }
                }

                if Local::now().naive_local() >= maintenance_time {
                    log::info!("Time to try updating cloudflared daemon");
                    self.update().await?;

                    log::info!("It's about time to restart the cloudflared daemon");
                    daemon.kill().await.ok();
                } else if error::count() >= error::ERROR_LIMIT {
                    log::error!(
                        "DNS errors ({}) are more than tolerance limit ({}), resetting cloudflare daemon",
                        error::count(),
                        error::ERROR_LIMIT,
                    );
                    daemon.kill().await.ok();
                } else if error::count() >= error::ERROR_LIMIT / 10 {
                    log::warn!("DNS errors count: {} ", error::count(),);
                }
                error::reset_count();

                match daemon.try_wait() {
                    Err(e) => {
                        log::warn!("Something went wrong while trying to get the status of child process: {}", e);
                        break;
                    }
                    Ok(Some(code)) => {
                        log::warn!("daemon died with status {}", code);
                        break;
                    }
                    Ok(None) => { /* Nothing to do */ }
                }
            }
        }
    }

    async fn update(&self) -> anyhow::Result<()> {
        let update_output = Command::new(&self.cmd).arg("update").output().await?;
        let mut buff = String::from_utf8_lossy(&update_output.stdout);
        buff += String::from_utf8_lossy(&update_output.stderr);
        log::info!("Updating cloudflared: {}", buff.trim());
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use chrono::{Datelike, Local, NaiveDate};

    #[test]
    fn test_datetime() {
        let now = dbg!(Local::now().naive_local());
        let next = NaiveDate::from_ymd(now.year(), now.month(), now.day() + 1).and_hms(2, 0, 0);

        println!("Hmm {}", next);
        println!("Kya {}, {}", now > next, next > now);
    }
}
