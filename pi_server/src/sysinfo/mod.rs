use std::thread;
use std::time::Duration;
use std::time::Instant;

use systemstat::{saturating_sub_bytes, Platform, System};
use tokio::{task, time};

use crate::db::sys_info;
use crate::{Timer, PI_CONFIG};

const MB: f32 = 1024.0 * 1024.0;
const MINUTE: Duration = Duration::from_secs(60);

pub async fn load_sys_info() -> anyhow::Result<()> {
    let mut delay = time::Instant::now() + MINUTE;
    loop {
        time::sleep_until(delay).await;
        delay += MINUTE;

        read_sys_info().await
    }
}

async fn read_sys_info() {
    let start = Instant::now();
    let (temperature, humidity) = task::spawn_blocking(read_environment_info)
        .await
        .unwrap_or((None, None));

    let sys = System::new();
    let cpu_avg = sys.load_average().map(|avg| avg.one).ok();
    let cpu_temp = sys.cpu_temp().ok();
    let memory = sys
        .memory()
        .map(|mem| saturating_sub_bytes(mem.total, mem.free))
        .map(|mem| (mem.as_u64() as f32) / MB)
        .ok();
    log::info!("Time taken to read health info: {}", start.t());

    if let Err(e) = sys_info::save(cpu_avg, cpu_temp, memory, temperature, humidity).await {
        log::warn!("Failed to save sys_info: {}", e);
    }
}

fn read_environment_info() -> (Option<f32>, Option<f32>) {
    if let Some(pin) = PI_CONFIG.get().unwrap().dht22_pin {
        let start = Instant::now();
        for i in 0..3 {
            match dht22::try_reading(pin) {
                Ok(reading) => {
                    log::debug!("Time taken to read temperature/humidity: {}", start.t());
                    return (Some(reading.temperature()), Some(reading.humidity()));
                }
                Err(e) => log::warn!(
                    "Reading temperature/humidity failed at {} retry due to {:?}",
                    i + 1,
                    e
                ),
            }
            thread::sleep(Duration::from_secs(2));
        }
    }
    (None, None)
}

#[cfg(test)]
mod test {
    use serde_json::Value;
    use systemstat::{Platform, System};

    #[test]
    fn sys_test() {
        let sys = System::new();
        match sys.load_average() {
            Err(e) => eprintln!("Couldn't load cpu info: {}", e),
            Ok(ag) => {
                dbg!(ag);
            }
        };
        match sys.cpu_temp() {
            Err(e) => eprintln!("Couldn't load cpu temp: {}", e),
            Ok(temp) => println!("Temperature: {} C", temp),
        };

        match sys.memory() {
            Ok(mem) => println!(
                "Memory: {} free / {} [{:?}]",
                mem.free, mem.total, mem.platform_memory
            ),
            Err(x) => println!("Memory: error: {}", x),
        }
    }

    #[test]
    fn test2() {
        let j = serde_json::json!({ "hello": "boss"});
        dbg!(j);
        dbg!(Value::from("234.33"));
    }
}
