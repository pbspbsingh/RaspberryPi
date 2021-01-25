use std::time::Duration;

use serde_json::json;
use systemstat::{saturating_sub_bytes, Platform, System};

use crate::db::sys_info;

const MB: f32 = 1024.0 * 1024.0;

pub async fn load_sys_info() -> anyhow::Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        let sys = System::new();
        let cpu_avg = sys.load_average().map(|avg| avg.one).ok();
        let cpu_temp = sys.cpu_temp().ok();
        let memory = sys
            .memory()
            .map(|mem| saturating_sub_bytes(mem.total, mem.free))
            .map(|mem| (mem.as_u64() as f32) / MB)
            .ok();
        if let Err(e) = sys_info::save(cpu_avg, cpu_temp, memory, &json!({})).await {
            log::warn!("Failed to save sys_info: {}", e);
        }
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