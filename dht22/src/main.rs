use std::time::Duration;

use dht22::try_reading;

fn main() {
    for _ in 0..20 {
        match try_reading(17) {
            Err(e) => eprintln!("Failed: {:?}", e),
            Ok(r) => println!("{:#?}", r),
        }
        println!();
        std::thread::sleep(Duration::from_secs(5));
    }
}
