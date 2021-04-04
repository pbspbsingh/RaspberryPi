use std::time::{Duration, Instant};

use gpio_cdev::{Chip, Line, LineRequestFlags};
use thiserror::Error;

pub use am2302::Reading;

use crate::am2302::ReadingError;

mod am2302;

const LOW: u8 = 0;
const HIGH: u8 = 1;
const BITS_NEEDED: usize = 40;
const MAX_READINGS: usize = 83;

const GPIO_FILE: &str = "/dev/gpiochip0";

#[derive(Debug, Error)]
pub enum DHT22Error {
    #[error("Device is not ready, try again later")]
    DeviceNotReady,

    #[error("Failed to initialize")]
    InitError(#[from] gpio_cdev::Error),

    #[error("Failed to read value from pin")]
    ValueInputError,

    #[error("Insufficient data points")]
    InsufficientReading(usize),

    #[error("Reading error")]
    ReadingError(#[from] ReadingError),
}

pub fn try_reading(gpio_pin: u32) -> Result<Reading, DHT22Error> {
    let line = init(gpio_pin)?;

    let events = read_events(line)?;

    process_events(&events)
}

pub fn init(gpio_pin: u32) -> Result<Line, DHT22Error> {
    let mut chip = Chip::new(GPIO_FILE)?;
    let line = chip.get_line(gpio_pin)?;

    let output = line.request(LineRequestFlags::OUTPUT, HIGH, "pull-down")?;
    output.set_value(LOW)?;

    std::thread::sleep(Duration::from_millis(3));
    Ok(line)
}

fn read_events(line: Line) -> Result<Vec<Event>, DHT22Error> {
    use DHT22Error::*;

    let input = line.request(LineRequestFlags::INPUT, HIGH, "read-data")?;

    let start = Instant::now();
    let timeout = Duration::from_secs_f32(2.5);

    let mut events = Vec::with_capacity(MAX_READINGS);
    let mut prev_state = input.get_value().map_err(|_| ValueInputError)?;
    while start.elapsed() < timeout && events.len() < MAX_READINGS {
        let curr_state = input.get_value().map_err(|_| ValueInputError)?;
        if prev_state != curr_state {
            let event_type = if prev_state == LOW && curr_state == HIGH {
                EvenType::RisingEdge
            } else {
                EvenType::FallingEdge
            };
            events.push(Event::new(event_type));

            prev_state = curr_state;
        }
    }
    /*println!(
        "Events count: {} in {}",
        events.len(),
        start.elapsed().as_millis()
    );*/
    Ok(events)
}

fn process_events(events: &[Event]) -> Result<Reading, DHT22Error> {
    let data_points = events
        .windows(2)
        .filter_map(|pair| {
            let prev = &pair[0];
            let next = &pair[1];
            match next.event_type {
                EvenType::FallingEdge => Some(next.timestamp - prev.timestamp),
                EvenType::RisingEdge => None,
            }
        })
        .map(|elapsed| if elapsed.as_micros() > 35 { HIGH } else { LOW })
        .collect::<Vec<_>>();
    // println!("Data points {}", data_points.len());
    let mut err = DHT22Error::InsufficientReading(data_points.len());
    for data in data_points.windows(BITS_NEEDED) {
        err = match Reading::from_binary_slice(data) {
            Ok(reading) => return Ok(reading),
            Err(e) => DHT22Error::ReadingError(e),
        };
    }
    Err(err)
}

#[derive(Debug, Copy, Clone)]
enum EvenType {
    RisingEdge,
    FallingEdge,
}

#[derive(Debug, Copy, Clone)]
struct Event {
    timestamp: Instant,
    event_type: EvenType,
}

impl Event {
    pub fn new(event_type: EvenType) -> Self {
        Event {
            timestamp: Instant::now(),
            event_type,
        }
    }
}
