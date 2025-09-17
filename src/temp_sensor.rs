use std::fmt;
use std::fmt::{Display, Formatter};
use regex::Regex;
use crate::app_error::AppError;

pub struct Measurement {
    pub humidity: f64,
    pub temperature: f64,
}

impl Display for Measurement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "temperature: {}, humidity: {}", self.temperature, self.humidity)
    }
}

pub struct TempSensor {
    url: String,
}


impl TempSensor {
    pub fn new(url: &String) -> Self {
        Self {
            url: url.clone(),
        }
    }

    pub async fn query(&self) -> Result<Measurement, AppError> {
        let text = reqwest::get(&self.url).await?.text().await?;
        let re = Regex::new(r"teplota:\s*<b>\s*(\d+\.\d+)\s*%\s*(\d+\.\d+)\s*&deg;C")?;
        if let Some(caps) = re.captures(&text) {
            return Ok(Measurement {
                humidity: caps[1].parse()
                    .map_err(|e| AppError::TemperatureSensorError(format!("Failed to parse humidity: {}", e)))?,
                temperature: caps[2].parse()
                    .map_err(|e| AppError::TemperatureSensorError(format!("Failed to parse temperature: {}", e)))?,
            });
        }
        Err(AppError::TemperatureSensorError("failed to parse measurement".to_string()))
    }

}
