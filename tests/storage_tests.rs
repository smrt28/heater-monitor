use heat_monitor::storage::{Storage, StorageError};
use std::time::{Duration, SystemTime};
use heat_monitor::config::Config;


fn default_config() -> Config {
    Config {
        max_capacity: Some(10000000),
        port: 3000,
        sampling_interval: 35,
        averaging_interval: 120,
        listen_address: "0.0.0.0".to_string(),
        log_path: "test.log".to_string(),
        backlog: None,
        temp_sensor_url: "http://localhost:3000/temperature".to_string(),
    }
}

fn create_test_storage() -> Storage {

    Storage::new(&default_config()).unwrap()
}

fn create_test_storage_with_capacity(capacity: usize) -> Storage {
    let mut config = default_config();
    config.max_capacity = Some(capacity);
    Storage::new(&config).unwrap()
}

#[test]
fn test_new_storage_is_empty() {
    let storage = create_test_storage();
    assert!(storage.is_empty());
    assert_eq!(storage.len(), 0);
    assert!(storage.latest_sample().is_none());
    assert!(storage.oldest_sample().is_none());
}

#[test]
fn test_add_measurement() {
    let mut storage = create_test_storage();
    
    storage.add_measurement(23.5, 45.2);
    
    assert!(!storage.is_empty());
    assert_eq!(storage.len(), 1);
    
    let sample = storage.latest_sample().unwrap();
    assert_eq!(sample.temperature, 23.5);

    
    // Latest and oldest should be the same for single sample
    assert!(storage.latest_sample().unwrap().timestamp == storage.oldest_sample().unwrap().timestamp);
}

#[test]
fn test_add_multiple_measurements() {
    let mut storage = create_test_storage();
    
    storage.add_measurement(20.0, 40.0);
    std::thread::sleep(Duration::from_millis(10));
    storage.add_measurement(21.0, 41.0);
    std::thread::sleep(Duration::from_millis(10));
    storage.add_measurement(22.0, 42.0);
    
    assert_eq!(storage.len(), 3);
    
    let latest = storage.latest_sample().unwrap();
    let oldest = storage.oldest_sample().unwrap();
    
    assert_eq!(latest.temperature, 22.0);
    assert_eq!(oldest.temperature, 20.0);
    assert!(latest.timestamp > oldest.timestamp);
}

#[test]
fn test_capacity_limit() {
    let mut storage = create_test_storage_with_capacity(2);
    
    storage.add_measurement(20.0, 40.0);
    storage.add_measurement(21.0, 41.0);
    storage.add_measurement(22.0, 42.0); // Should evict the first one
    
    assert_eq!(storage.len(), 2);
    
    let oldest = storage.oldest_sample().unwrap();
    let latest = storage.latest_sample().unwrap();
    
    assert_eq!(oldest.temperature, 21.0); // First sample (20.0) should be evicted
    assert_eq!(latest.temperature, 22.0);
}

#[test]
fn test_get_samples_in_range_empty_storage() {
    let storage = create_test_storage();
    let now = SystemTime::now();
    let hour_ago = now - Duration::from_secs(3600);
    
    let result = storage.get_samples_in_range(hour_ago, now);
    assert!(matches!(result, Err(StorageError::NoDataAvailable)));
}

#[test]
fn test_get_samples_in_range_invalid_range() {
    let storage = create_test_storage();
    let now = SystemTime::now();
    let hour_ago = now - Duration::from_secs(3600);
    
    let result = storage.get_samples_in_range(now, hour_ago); // Invalid: from > to
    assert!(matches!(result, Err(StorageError::InvalidTimeRange)));
}

#[test]
fn test_get_samples_in_range_with_data() {
    let mut storage = create_test_storage();
    
    // We can't control exact timestamps, so we'll add samples and query recent range
    storage.add_measurement(20.0, 40.0);
    std::thread::sleep(Duration::from_millis(10));
    storage.add_measurement(21.0, 41.0);
    std::thread::sleep(Duration::from_millis(10));
    storage.add_measurement(22.0, 42.0);
    
    let now = SystemTime::now();
    let minute_ago = now - Duration::from_secs(60);
    
    let samples = storage.get_samples_in_range(minute_ago, now).unwrap();
    assert_eq!(samples.len(), 3); // All samples should be within the last minute
}

#[test]
fn test_per_minute_avg_fill_empty_storage() {
    let storage = create_test_storage();
    let now = SystemTime::now();
    let hour_ago = now - Duration::from_secs(3600);
    
    let result = storage.per_minute_avg_fill(hour_ago, now);
    assert!(matches!(result, Err(StorageError::NoDataAvailable)));
}

#[test]
fn test_per_minute_avg_fill_invalid_range() {
    let storage = create_test_storage();
    let now = SystemTime::now();
    let hour_ago = now - Duration::from_secs(3600);
    
    let result = storage.per_minute_avg_fill(now, hour_ago);
    assert!(matches!(result, Err(StorageError::InvalidTimeRange)));
}



#[test]
fn test_read_sample() {
    let mut storage = create_test_storage();
    
    storage.add_measurement(23.5, 45.2);
    
    let now = SystemTime::now();
    let minute_ago = now - Duration::from_secs(60);
    
    let result = storage.read_sample(minute_ago, Duration::from_secs(120));
    assert!(result.is_ok());
    
    let sample = result.unwrap();
    assert_eq!(sample.temperature, 23.5);
}

#[test]
fn test_read_sample_no_data() {
    let storage = create_test_storage();
    let now = SystemTime::now();
    let hour_ago = now - Duration::from_secs(3600);
    
    let result = storage.read_sample(hour_ago, Duration::from_secs(60));
    assert!(matches!(result, Err(StorageError::NoDataAvailable)));
}

#[test]
fn test_storage_ordering() {
    let mut storage = create_test_storage();
    
    // Add samples with small delays to ensure different timestamps
    storage.add_measurement(20.0, 40.0);
    std::thread::sleep(Duration::from_millis(10));
    storage.add_measurement(21.0, 41.0);
    std::thread::sleep(Duration::from_millis(10));
    storage.add_measurement(22.0, 42.0);
    
    let oldest = storage.oldest_sample().unwrap();
    let latest = storage.latest_sample().unwrap();
    
    assert_eq!(oldest.temperature, 20.0);
    assert_eq!(latest.temperature, 22.0);
    assert!(latest.timestamp >= oldest.timestamp);
}

#[test]
fn test_max_capacity_enforcement() {
    let mut storage = create_test_storage_with_capacity(3);
    
    // Add measurements up to capacity
    storage.add_measurement(10.0, 30.0);
    storage.add_measurement(15.0, 35.0);
    storage.add_measurement(20.0, 40.0);
    
    assert_eq!(storage.len(), 3);
    assert_eq!(storage.oldest_sample().unwrap().temperature, 10.0);
    assert_eq!(storage.latest_sample().unwrap().temperature, 20.0);
    
    // Add one more - should evict the oldest
    storage.add_measurement(25.0, 45.0);
    
    assert_eq!(storage.len(), 3); // Still at capacity
    assert_eq!(storage.oldest_sample().unwrap().temperature, 15.0); // 10.0 was evicted
    assert_eq!(storage.latest_sample().unwrap().temperature, 25.0); // New sample added
    
    // Add several more to test multiple evictions
    storage.add_measurement(30.0, 50.0);
    storage.add_measurement(35.0, 55.0);
    
    assert_eq!(storage.len(), 3);
    assert_eq!(storage.oldest_sample().unwrap().temperature, 25.0); // 15.0 and 20.0 evicted
    assert_eq!(storage.latest_sample().unwrap().temperature, 35.0);
}

#[test]
fn test_max_capacity_zero() {
    let mut storage = create_test_storage_with_capacity(0);
    
    // Adding to zero capacity should not store anything
    storage.add_measurement(20.0, 40.0);
    
    assert_eq!(storage.len(), 0);
    assert!(storage.is_empty());
    assert!(storage.latest_sample().is_none());
    assert!(storage.oldest_sample().is_none());
}

#[test]
fn test_max_capacity_one() {
    let mut storage = create_test_storage_with_capacity(1);
    
    // Add first measurement
    storage.add_measurement(20.0, 40.0);
    assert_eq!(storage.len(), 1);
    assert_eq!(storage.latest_sample().unwrap().temperature, 20.0);
    
    // Add second measurement - should replace the first
    storage.add_measurement(25.0, 45.0);
    assert_eq!(storage.len(), 1);
    assert_eq!(storage.latest_sample().unwrap().temperature, 25.0);
    assert_eq!(storage.oldest_sample().unwrap().temperature, 25.0);
}

#[test]
fn test_unlimited_capacity() {
    let mut storage = create_test_storage(); // No capacity limit
    
    // Add many measurements
    for i in 0..1000 {
        storage.add_measurement(i as f64, (i * 2) as f64);
    }
    
    assert_eq!(storage.len(), 1000);
    assert_eq!(storage.oldest_sample().unwrap().temperature, 0.0);
    assert_eq!(storage.latest_sample().unwrap().temperature, 999.0);
}

#[test]
fn test_capacity_with_range_queries() {
    let mut storage = create_test_storage_with_capacity(5);
    
    // Add measurements that will exceed capacity
    for i in 0..10 {
        storage.add_measurement(i as f64, (i * 2) as f64);
        std::thread::sleep(Duration::from_millis(1)); // Ensure different timestamps
    }
    
    // Should only have the last 5 measurements
    assert_eq!(storage.len(), 5);
    assert_eq!(storage.oldest_sample().unwrap().temperature, 5.0);
    assert_eq!(storage.latest_sample().unwrap().temperature, 9.0);
    
    // Range queries should work correctly with capacity-limited data
    let now = SystemTime::now();
    let minute_ago = now - Duration::from_secs(60);
    
    let samples = storage.get_samples_in_range(minute_ago, now).unwrap();
    assert_eq!(samples.len(), 5); // All remaining samples should be within range
}
