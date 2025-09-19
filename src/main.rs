
mod config;
mod app_error;
mod temp_sensor;
mod storage;
mod server;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::config::Config;
use anyhow::Result;
use log::error;
// use regex::Regex;
use crate::server::run_server;
use crate::temp_sensor::TempSensor;
use crate::storage::Storage;
use std::env;

fn get_config_path() -> Result<PathBuf> {
    #[cfg(not(debug_assertions))]
    {
        let args: Vec<String> = env::args().collect();
        let config_path = args.get(1).ok_or_else(|| {
            anyhow::anyhow!("Config file path must be passed as first argument")
        })?;
        Ok(PathBuf::from(config_path))
    }
    #[cfg(debug_assertions)]
    {
        let config_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("config.toml");
        let config_path = config_file.to_str().ok_or_else(|| {
            anyhow::anyhow!("Invalid UTF-8 in config path")
        })?;
        Ok(PathBuf::from(config_path))
    }
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let config = Config::read(get_config_path()?)?;

    let storage = Arc::new(Mutex::new(
        Storage::new(&config)
    ));

    {
        let temp_sensor = TempSensor::new(&config.temp_sensor_url);
        let interval = config.interval;
        let storage = storage.clone();
        let _handle = tokio::spawn(async move {
            loop {
                if let Ok(val) = temp_sensor.query().await {
                    println!("{}", val);

                    if let Ok(mut storage) = storage.lock() {
                        storage.add_measurement(val.temperature, val.humidity);
                    } else {
                        error!("failed to lock storage");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(interval as u64)).await;
            }
        });
    }

    let _res = run_server(storage, &config).await?;
    Ok(())
}
