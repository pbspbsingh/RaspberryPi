use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

use chrono::{Duration, Local, NaiveDateTime, Timelike};
use log::*;
use once_cell::sync::OnceCell;
use rustlearn::multiclass::OneVsRestWrapper;
use rustlearn::prelude::*;
use rustlearn::trees::decision_tree::{DecisionTree, Hyperparameters};
use tokio::sync::RwLock;

use crate::db::sys_info::load_sys_info;
use crate::{Timer, PI_CONFIG};

const MAX_RETRY: u32 = 5;
const ACCEPTABLE_TEMP_DIFF: f32 = 5.0;
const ACCEPTABLE_HUMID_DIFF: f32 = 10.0;

static LAST_READ_TEMP: AtomicI64 = AtomicI64::new(-10_000_000);
static LAST_READ_HUMID: AtomicI64 = AtomicI64::new(-10_000_000);

static MODEL_LAST_UPDATED: OnceCell<RwLock<NaiveDateTime>> = OnceCell::new();

static TEMPERATURE_MODEL: OnceCell<RwLock<OneVsRestWrapper<DecisionTree>>> = OnceCell::new();
static HUMIDITY_MODEL: OnceCell<RwLock<OneVsRestWrapper<DecisionTree>>> = OnceCell::new();

pub async fn read_climate_info() -> (Option<f32>, Option<f32>) {
    let start = Instant::now();
    let mut last_reading = None;
    let (mut final_temp, mut final_humidity) = (None, None);
    if let Some(pin) = PI_CONFIG.get().unwrap().dht22_pin {
        for try_cnt in 0..MAX_RETRY {
            if let Some((measured_temp, measured_humid)) = read_pin(pin).await {
                last_reading = Some((measured_temp, measured_humid));
                if final_temp.is_none() {
                    if let Some(predicted_temp) = predict_temperature().await {
                        info!("Measured temperature: {measured_temp:.2}C, Predicted temperature: {predicted_temp:.2}C");
                        if (measured_temp - predicted_temp).abs() < ACCEPTABLE_TEMP_DIFF {
                            final_temp = Some(measured_temp);
                        }
                    }
                }
                if final_temp.is_none() {
                    let last_temp = LAST_READ_TEMP.load(Ordering::Relaxed) as f32 / 1000.0;
                    info!("Measured temperature: {measured_temp:.2}C, Last temperature: {last_temp:.2}C");
                    if (measured_temp - last_temp).abs() < ACCEPTABLE_TEMP_DIFF {
                        final_temp = Some(measured_temp);
                    }
                }

                if final_humidity.is_none() {
                    if let Some(predicted_humid) = predict_humidity().await {
                        info!("Measured humidity: {measured_humid:.2}, Predicted humidity: {predicted_humid:.2}");
                        if (measured_humid - predicted_humid).abs() < ACCEPTABLE_HUMID_DIFF {
                            final_humidity = Some(measured_humid);
                        }
                    }
                }
                if final_humidity.is_none() {
                    let last_humid = LAST_READ_HUMID.load(Ordering::Relaxed) as f32 / 1000.0;
                    info!("Measured humidity: {measured_humid:.2}, Last humidity: {last_humid:.2}");
                    if (measured_humid - last_humid).abs() < ACCEPTABLE_HUMID_DIFF {
                        final_humidity = Some(measured_humid);
                    }
                }

                if final_temp.is_some() && final_humidity.is_some() {
                    info!("Read temperature/humidity in {try_cnt} retries");
                    break;
                }
            }
            if (final_temp.is_none() || final_humidity.is_none()) && try_cnt != MAX_RETRY - 1 {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
    if let Some((t, h)) = last_reading {
        if final_temp.is_none() {
            warn!("Neither ML nor last temperature worked, using the last read value: {t:.2}C");
            final_temp = Some(t);
        }
        if final_humidity.is_none() {
            warn!("Neither ML nor last humidity worked, using the last read value: {h:.2}");
            final_humidity = Some(h);
        }
    }
    if let Some(temp) = final_temp {
        LAST_READ_TEMP.store((temp * 1000.0) as i64, Ordering::SeqCst);
    }
    if let Some(humid) = final_humidity {
        LAST_READ_HUMID.store((humid * 1000.0) as i64, Ordering::SeqCst);
    }
    info!("Reading temperature/humidity took {}", start.t());
    (final_temp, final_humidity)
}

async fn read_pin(pin: u32) -> Option<(f32, f32)> {
    tokio::task::spawn_blocking(move || match dht22::try_reading(pin) {
        Ok(reading) => Some((reading.temperature(), reading.humidity())),
        Err(e) => {
            warn!("Failed to read temperature/humidity: {e:?}");
            None
        }
    })
    .await
    .unwrap_or(None)
}

async fn predict_temperature() -> Option<f32> {
    train_models().await;

    if let Some(temp_model) = TEMPERATURE_MODEL.get() {
        let read_lock = temp_model.read().await;
        let curr_time = to_seconds(Local::now().naive_local());
        match read_lock.predict(&Array::from(vec![curr_time])) {
            Ok(result) => {
                return Some(result.get(0, 0));
            }
            Err(e) => warn!("Failed to predict temperature: {e}"),
        }
    } else {
        warn!("Temperature model is not initialized yet");
    }
    None
}

async fn predict_humidity() -> Option<f32> {
    train_models().await;

    if let Some(hum_model) = HUMIDITY_MODEL.get() {
        let read_lock = hum_model.read().await;
        let curr_time = to_seconds(Local::now().naive_local());
        match read_lock.predict(&Array::from(vec![curr_time])) {
            Ok(result) => {
                return Some(result.get(0, 0));
            }
            Err(e) => warn!("Failed to predict humidity: {e}"),
        }
    } else {
        warn!("Humidity model is not initialized yet");
    }
    None
}

async fn train_models() {
    if let Some(last_update) = MODEL_LAST_UPDATED.get() {
        let read_lock = last_update.read().await;
        let diff = Local::now().naive_local() - *read_lock;
        if diff < Duration::hours(1) {
            debug!(
                "ML models were last updated {} ago, skipping re-training",
                diff.to_std().unwrap().t()
            );
            return;
        }
    }
    let train_start = Instant::now();
    let sys_infos = load_sys_info(Local::now().naive_local() - Duration::days(1))
        .await
        .unwrap_or_else(|_| Vec::new());
    if sys_infos.len() < 1000 {
        warn!(
            "Data points {} is not enough for training machine learning model",
            sys_infos.len()
        );
        return;
    }
    info!(
        "Training machine learning models with {} data points",
        sys_infos.len()
    );
    let (mut s1, mut s2) = (false, false);
    let start = Instant::now();
    let (times, temps): (Vec<f32>, Vec<f32>) = sys_infos
        .iter()
        .filter_map(|s| s.temperature.map(|t| (to_seconds(s.s_time), t)))
        .unzip();
    let mut temp_model = Hyperparameters::new(1).one_vs_rest();
    match temp_model.fit(&Array::from(times), &Array::from(temps)) {
        Ok(_) => {
            if let Some(temperature_model) = TEMPERATURE_MODEL.get() {
                let mut write_lock = temperature_model.write().await;
                *write_lock = temp_model;
            } else {
                TEMPERATURE_MODEL.set(RwLock::new(temp_model)).ok();
            }
            info!("ML training for temperature succeeded in {}", start.t());
            s1 = true;
        }
        Err(e) => warn!("ML training for temperature failed: {e:?}"),
    };

    let start = Instant::now();
    let (times, humids): (Vec<f32>, Vec<f32>) = sys_infos
        .into_iter()
        .filter_map(|s| s.humidity.map(|h| (to_seconds(s.s_time), h)))
        .unzip();
    let mut humid_model = Hyperparameters::new(1).one_vs_rest();
    match humid_model.fit(&Array::from(times), &Array::from(humids)) {
        Ok(()) => {
            if let Some(humidity_model) = HUMIDITY_MODEL.get() {
                let mut write_lock = humidity_model.write().await;
                *write_lock = humid_model;
            } else {
                HUMIDITY_MODEL.set(RwLock::new(humid_model)).ok();
            }
            info!("ML training for humidity succeeded in {}", start.t());
            s2 = true;
        }
        Err(e) => warn!("ML training for humidity failed: {e:?}"),
    };

    if s1 && s2 {
        if let Some(last_updated) = MODEL_LAST_UPDATED.get() {
            let mut write_lock = last_updated.write().await;
            *write_lock = Local::now().naive_local();
        } else {
            MODEL_LAST_UPDATED
                .set(RwLock::new(Local::now().naive_local()))
                .ok();
        }
    } else {
        warn!("One or more ML learning failed, not updating last_updated flag");
    }
    info!("Machine learning training completed in {}", train_start.t());
}

fn to_seconds(time: NaiveDateTime) -> f32 {
    (time.hour() * 60 * 60 + time.minute() * 60 + time.second()) as f32
}

#[cfg(test)]
mod test {
    use std::sync::atomic::Ordering;

    use once_cell::sync::OnceCell;

    use crate::sysinfo::climate::LAST_READ_TEMP;

    #[test]
    fn test_atomic() {
        for i in 0..10 {
            let val = LAST_READ_TEMP.load(Ordering::Relaxed) as f32 / 1000.;
            dbg!(val);
            LAST_READ_TEMP.store(i * 1000, Ordering::SeqCst);
        }
    }

    static CELL: OnceCell<u32> = OnceCell::<u32>::new();

    #[test]
    fn test_cell() {
        for i in (0..10).rev() {
            dbg!(CELL.get_or_init(|| i));
        }
    }
}
