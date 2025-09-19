use std::time::{Duration, SystemTime};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Sample {
    pub timestamp: SystemTime,
    pub temperature: f64,
    pub humidity: f64,
}

#[derive(Debug)]
pub struct Storage {
    samples: VecDeque<Sample>,
    max_capacity: Option<usize>,
}

#[derive(Debug)]
pub enum StorageError {
    InvalidTimeRange,
    NoDataAvailable,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            max_capacity: None,
        }
    }

    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_capacity),
            max_capacity: Some(max_capacity),
        }
    }

    pub fn add_measurement(&mut self, temp: f64, hum: f64) {
        let sample = Sample {
            timestamp: SystemTime::now(),
            temperature: temp,
            humidity: hum,
        };

        if let Some(capacity) = self.max_capacity {
            if self.samples.len() >= capacity {
                self.samples.pop_front();
            }
        }

        self.samples.push_back(sample);
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

        let samples = self.get_samples_in_range(from, to)?;
        if samples.is_empty() {
            return Ok(Vec::new());
        }
        
        // Find the actual time range of our data
        let first_sample_time = samples.iter().map(|s| s.timestamp).min().unwrap();
        let last_sample_time = samples.iter().map(|s| s.timestamp).max().unwrap();
        
        // Start from the minute containing the first sample
        let start_minute = first_sample_time.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default().as_secs() / 60 * 60;
        let start_time = SystemTime::UNIX_EPOCH + Duration::from_secs(start_minute);
        
        // End at the minute containing the last sample or 'to', whichever is earlier
        let end_time = std::cmp::min(last_sample_time, to);
        let end_minute = end_time.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default().as_secs() / 60 * 60;
        let actual_end_time = SystemTime::UNIX_EPOCH + Duration::from_secs(end_minute);
        
        let duration_secs = actual_end_time.duration_since(start_time)
            .map_err(|_| StorageError::InvalidTimeRange)?
            .as_secs();
        
        let minutes = (duration_secs / 60) + 1;
        let mut averages = Vec::new();
        
        for minute in 0..minutes {
            let minute_start = start_time + Duration::from_secs(minute * 60);
            let minute_end = minute_start + Duration::from_secs(60);
            
            let minute_samples: Vec<&Sample> = samples
                .iter()
                .filter(|sample| sample.timestamp >= minute_start && sample.timestamp < minute_end)
                .cloned()
                .collect();
            
            if !minute_samples.is_empty() {
                let avg = minute_samples
                    .iter()
                    .map(|s| s.temperature)
                    .sum::<f64>() / minute_samples.len() as f64;
                averages.push(Some(avg));
            } else {
                // Add null for missing measurements within the data range
                averages.push(None);
            }
        }

        // Reverse to get most recent first
        averages.reverse();
        Ok(averages)
    }

    pub fn read_sample(&self, from: SystemTime, duration: Duration) -> Result<Sample, StorageError> {
        let to = from + duration;
        let samples = self.get_samples_in_range(from, to)?;
        
        if let Some(sample) = samples.first() {
            Ok((*sample).clone())
        } else {
            Err(StorageError::NoDataAvailable)
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn latest_sample(&self) -> Option<&Sample> {
        self.samples.back()
    }

    pub fn oldest_sample(&self) -> Option<&Sample> {
        self.samples.front()
    }
}
