use std::collections::VecDeque;
use std::time::SystemTime;


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
}
