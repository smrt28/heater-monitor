use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
// use crate::temp_sensor::TempSensor;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub temp_sensor_url: String,
    #[allow(dead_code)]
    pub max_capacity: Option<usize>,
    pub sampling_interval: u64,
    pub port: u16,
    pub listen_address: String,
    pub log_path: String,
    #[allow(dead_code)]
    pub backlog: Option<String>,
    pub averaging_interval: u32,
}

impl Config {
    pub fn read(path: PathBuf) -> Result<Config, anyhow::Error> {
        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str::<Config>(&contents)?)
    }
}
