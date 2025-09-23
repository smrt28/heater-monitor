use std::time::{Duration, SystemTime};
use std::collections::VecDeque;
use crate::app_error::AppError;
use crate::config::Config;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use log::{debug, error, info, warn};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub timestamp: SystemTime,
    pub temperature: f64,
 //   #[allow(dead_code)]
 //   pub humidity: f64,
}

impl Sample {
    fn serialize(&self) -> Result<String, AppError> {
        Ok(format!("t1 {} {}",
           self.timestamp.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
           self.temperature))
    }

    #[allow(dead_code)]
    fn deserialize(line: &str) -> Result<Sample, AppError> {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();

        if parts.len() != 3 || parts[0] != "t1" {
            return Err(AppError::ParseError(format!("Invalid sample format: {}", line)));
        }

        let timestamp_secs: u64 = parts[1].parse()
            .map_err(|_| AppError::ParseError(format!("Invalid timestamp: {}", parts[1])))?;

        let temperature: f64 = parts[2].parse()
            .map_err(|_| AppError::ParseError(format!("Invalid temperature: {}", parts[2])))?;

        if temperature > 1000.0 || temperature < -1000.0 {
            return Err(AppError::ParseError(format!("Invalid temperature range: {}", temperature)));
        }

        let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp_secs);

        Ok(Sample {
            timestamp,
            temperature,
        })
    }
}

#[derive(Debug)]
pub struct Storage {
    pub(crate) samples: VecDeque<Sample>,
    file_store: Option<File>,
    last_sample_time: Option<SystemTime>,
    config: Config,
    last: Option<Sample>,
}

#[derive(Debug)]
pub enum StorageError {
    InvalidTimeRange,
    NoDataAvailable,
}

impl Storage {
    fn read_samples_from_file(&mut self, file_path: &str) -> Result<(), AppError> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let mut line = line?;
            if let Ok(sample) = Sample::deserialize(&line) {
                self.push_raw_sample(sample);
            } else {
                line.truncate(100);
                error!("Failed to parse sample from file: {}", &line);
            }
        }

        Ok(())
    }

    pub fn new(config: &Config) -> Result<Self, AppError> {
        let mut rv = Self {
            samples: VecDeque::new(),
            file_store: None,
            last_sample_time: None,
            config: config.clone(),
            last: None,
        };

        if let Some(file_path) = &config.backlog {
            if !rv.read_samples_from_file(file_path).is_ok() {
                info!("Failed to read samples from file");
            };
        }

        info!("Storage initialized by {}", rv.samples.len());

        rv.file_store = if let Some(file_path) = &config.backlog {
            Some(File::options()
                .create(true)
                .append(true)
                .open(file_path)?)
        } else {
            None
        };

        Ok(rv)
    }


    pub fn push_raw_sample(&mut self, sample: Sample) {
        if let Some(last_sample_time) = self.last_sample_time {
            if last_sample_time > sample.timestamp {
                error!("Sample timestamp is in the past");
                return;
            }
        }

        self.last_sample_time = Some(sample.timestamp);
        if let Some(capacity) = self.config.max_capacity {
            if capacity == 0 {
                // Don't store anything if capacity is zero
                return;
            }
            if self.samples.len() >= capacity {
                self.samples.pop_front();
            }
        }
        self.samples.push_back(sample.clone());
        self.last = Some(sample);
    }

    pub fn add_measurement(&mut self, temp: f64, _hum: f64) {
        let sample = Sample {
            timestamp: SystemTime::now(),
            temperature: temp,
         //   humidity: hum,
        };

        if let Some(file_store) = &mut self.file_store {
            if let Ok(mut s) = sample.serialize() {
                s.push('\n');
                if !file_store.write(s.as_bytes()).is_ok() {
                    info!("Failed to write sample to file");
                };
            }
        }

        self.push_raw_sample(sample);
    }

    pub fn get_samples_in_range(&self, from: SystemTime, to: SystemTime) -> Result<Vec<&Sample>, StorageError> {
        if from > to {
            return Err(StorageError::InvalidTimeRange);
        }

        let samples: Vec<&Sample> = self.samples
            .iter()
            .filter(|sample| sample.timestamp >= from && sample.timestamp <= to)
            .collect();

        if samples.is_empty() {
            return Err(StorageError::NoDataAvailable);
        }

        Ok(samples)
    }

    pub fn per_minute_avg_fill(&self, from: SystemTime, to: SystemTime) -> Result<Vec<Option<f64>>, StorageError> {
        if from > to {
            return Err(StorageError::InvalidTimeRange);
        }

        debug!("per_minute_avg_fill from:{:?} to:{:?}", from, to);

        let samples = self.get_samples_in_range(from, to)?;
        if samples.is_empty() {
            return Ok(Vec::new());
        }

        for(prev, curr) in samples.iter().zip(samples.iter().skip(1)) {
            if prev.timestamp > curr.timestamp {
                warn!("Sample timestamp is in the past");
                return Err(StorageError::InvalidTimeRange);
            }
        }

        let mut timestamp = samples.first().unwrap().timestamp;
        let mut count = 0;
        let mut sum:f64 = 0.0;
        let mut it = samples.iter().peekable();
        let mut previous_average: Option<f64> = None;
        let mut averages = Vec::new();
        let mut no_samples_count = 0;
        let mut fin = false;

        let interval = Duration::from_secs(self.config.averaging_interval as u64);

        let mut loops = 0;

        while !fin {
            loops += 1;
            if loops > 1000000 {
                // prevent infinite loop and so OOM killer
                error!("Looping forever");
                panic!("Looping forever");
            }

            match it.peek() {
                Some(curr) => {
                    if curr.timestamp < timestamp + interval {
                        sum += curr.temperature;
                        count += 1;
                        it.next();
                        continue;
                    }
                }
                None => {
                    fin = true;
                }
            };


            if count > 0 {
                no_samples_count = 0;
                previous_average = Some(sum / count as f64);
            } else {
                no_samples_count += 1;
            }

            timestamp = timestamp + interval;

            if no_samples_count > 5 {
                previous_average = None;
            }

            averages.push(previous_average);
            if averages.len() > 100000 {
                error!("Too many entries in averages, something is wrong");
                break;
            }
            count = 0;
            sum = 0.0;
        }

        Ok(averages)
    }

    pub fn get_last_sample(&self) -> Option<&Sample> {
        self.last.as_ref()
    }

    #[allow(dead_code)]
    pub fn read_sample(&self, from: SystemTime, duration: Duration) -> Result<Sample, StorageError> {
        let to = from + duration;
        let samples = self.get_samples_in_range(from, to)?;
        
        if let Some(sample) = samples.first() {
            Ok((*sample).clone())
        } else {
            Err(StorageError::NoDataAvailable)
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn latest_sample(&self) -> Option<&Sample> {
        self.samples.back()
    }

    pub fn oldest_sample(&self) -> Option<&Sample> {
        self.samples.front()
    }

    // Helper method for testing - only available when testing
    #[cfg(any(test, feature = "test-helpers"))]
    #[allow(dead_code)]
    pub fn add_sample_direct(&mut self, sample: Sample) {
        self.samples.push_back(sample);
    }
}
