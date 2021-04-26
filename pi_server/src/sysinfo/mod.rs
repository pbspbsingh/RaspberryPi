use std::time::Duration;
use std::time::Instant;

use systemstat::{saturating_sub_bytes, Platform, System};
use tokio::time;

use crate::db::sys_info;
use crate::sysinfo::climate::read_climate_info;
use crate::Timer;

mod climate;

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
    let sys = System::new();
    let cpu_avg = sys.load_average().map(|avg| avg.one).ok();
    let cpu_temp = sys.cpu_temp().ok();
    let memory = sys
        .memory()
        .map(|mem| saturating_sub_bytes(mem.total, mem.free))
        .map(|mem| (mem.as_u64() as f32) / MB)
        .ok();
    log::info!("Time taken to read health info: {}", start.t());

    let (temperature, humidity) = read_climate_info().await;

    if let Err(e) = sys_info::save(cpu_avg, cpu_temp, memory, temperature, humidity).await {
        log::warn!("Failed to save sys_info: {}", e);
    }
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
