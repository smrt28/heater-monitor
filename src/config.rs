use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
use crate::temp_sensor::TempSensor;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub temp_sensor_url: String,
    pub max_measurements: usize,
    pub interval: u64,
    pub port: u16,
}

impl Config {
    pub fn read(path: PathBuf) -> Result<Config, anyhow::Error> {
        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str::<Config>(&contents)?)
    }
}
