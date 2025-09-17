use std::arch::x86_64::_mulx_u32;
use std::cmp::PartialEq;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use serde::Serialize;
use uuid::Timestamp;
use crate::app_error::AppError;

#[derive(Debug, PartialEq)]
enum Status {
    Ok,
    Error,
}

struct Measurement {
    temp: Option<f64>,
    hum: Option<f64>,
    timestamp: SystemTime,
    status: Status,
}

pub struct Storage {
    measurements: VecDeque<Measurement>,
    max_measurements: usize,
}


pub enum SampleSpec {
    Time(std::time::Duration),
}

#[derive(Debug, Serialize)]
pub struct Sample {
     temp: Vec<f64>,
}

impl Sample {
    pub fn create_empty() -> Self {
        Self {
            temp: Vec::new(),
        }
    }
}

impl Storage {
    pub fn new() -> Self {
        Self {
            measurements: VecDeque::new(),
            max_measurements: 100000,
        }
    }

    pub fn set_max_measurements(&mut self, max_measurements: usize) {
        self.max_measurements = max_measurements;
    }

    pub fn add_measurement(&mut self, temp: f64, hum: f64) {
        self.measurements.push_front(Measurement{
            temp: Some(temp),
            hum: Some(hum),
            timestamp: SystemTime::now(),
            status: Status::Ok,
        });
    }

    pub fn per_minute_avg_fill2(&self, from: SystemTime, to: SystemTime) -> Vec<f64> {
        if to <= from {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut index = 0;

        // Find first measurement at/after `from`
        while index < self.measurements.len() && self.measurements[index].timestamp < from {
            index += 1;
        }

        let mut minute_start = from;
        let mut minute_end = minute_start + Duration::from_secs(60);

        while minute_start < to {
            let mut sum = 0.0;
            let mut count = 0usize;

            // Consume all measurements in this minute
            while index < self.measurements.len()
                && self.measurements[index].timestamp < minute_end
            {
                let m = &self.measurements[index];
                if m.status == Status::Ok {
                    if let Some(v) = m.temp {
                        sum += v;
                        count += 1;
                    }
                }
                index += 1;
            }

            // Average or forward-fill
            if count > 0 {
                result.push(sum / count as f64);
            } else {
                let prev = result.last().copied().unwrap_or(f64::NAN);
                result.push(prev);
            }

            // If no more data, fill remaining minutes with last value and bail
            if index >= self.measurements.len() {
                let last = *result.last().unwrap(); // safe: we just pushed
                let mut ms = minute_end;
                while ms < to {
                    result.push(last);
                    ms += Duration::from_secs(60);
                }
                break;
            }

            minute_start = minute_end;
            minute_end += Duration::from_secs(60);
        }

        result
    }

    pub fn per_minute_avg_fill(&self, from: SystemTime, to: SystemTime) -> Result<Vec<f64>, AppError> {
        assert!(to >= from, "invalid range");
        let total_secs = to.duration_since(from)?.as_secs();
        let buckets = ((total_secs + 59) / 60) as usize; // ceil to full minutes
        if buckets == 0 {
            return Ok(Vec::new());
        }

        let mut sums = vec![0.0f64; buckets];
        let mut counts = vec![0usize; buckets];

        // Fill sums/counts (early-exit thanks to sorted data)
        for m in self.measurements.iter() {
            if m.timestamp >= to {
                break;
            }
            if m.timestamp < from {
                continue;
            }
            if m.status != Status::Ok {
                continue;
            }
            if let Some(v) = m.temp {
                let idx = (m.timestamp.duration_since(from)?.as_secs() / 60) as usize;
                if idx < buckets {
                    sums[idx] += v;
                    counts[idx] += 1;
                }
            }
        }

        // Build averages, then forward-fill
        let mut out = vec![f64::NAN; buckets];
        for i in 0..buckets {
            if counts[i] > 0 {
                out[i] = sums[i] / counts[i] as f64;
            } else if i > 0 {
                out[i] = out[i - 1]; // forward-fill
            }
        }
        Ok(out)
    }

    /// Convenience: last `minutes` buckets ending at now.
    pub fn last_minutes_avg_fill(&self, from: SystemTime) -> Vec<f64> {
        let to = SystemTime::now();
        if (to < from) {
            return Vec::new();
        }
        if let Ok(res) = self.per_minute_avg_fill(from, to) {
            return res;
        }
        return Vec::new();
    }


    pub fn sample(&self, spec: SampleSpec) -> Sample {
        match spec {
            SampleSpec::Time(t) => {
                let Some(start) = self.measurements.front() else {
                    return Sample::create_empty();
                };
                return Sample {
                    temp: self.last_minutes_avg_fill(SystemTime::now() - t)
                }
            }
        }


        return Sample::create_empty();
    }
}
