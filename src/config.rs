use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
// use crate::temp_sensor::TempSensor;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub temp_sensor_url: String,
    #[allow(dead_code)]
    pub max_capacity: usize,
    pub interval: u64,
    pub port: u16,
    pub listen_address: String,
    pub log_path: String,
}

impl Config {
    pub fn read(path: PathBuf) -> Result<Config, anyhow::Error> {
        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str::<Config>(&contents)?)
    }
}
