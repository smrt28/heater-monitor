# Heater Monitor

A Rust-based temperature and humidity monitoring system that collects measurements every 15 seconds and provides HTTP API access to the data.

## Architecture

### Storage System (`src/storage.rs`)

The system uses an in-memory storage structure optimized for time-series data:

- **Data Structure**: `VecDeque<Sample>` for efficient insertion and memory management
- **Sample Rate**: Designed for 15-second intervals
- **Memory Management**: Optional capacity limits with automatic removal of oldest samples
- **Range Queries**: Efficient filtering for time-based data retrieval

#### Key Types

```rust
pub struct Sample {
    pub timestamp: SystemTime,
    pub temperature: f64,
    pub humidity: f64,
}

pub struct Storage {
    samples: VecDeque<Sample>,
    max_capacity: Option<usize>,
}
```

#### Key Methods

- `add_measurement(temp: f64, hum: f64)` - Adds new measurement with current timestamp
- `get_samples_in_range(from: SystemTime, to: SystemTime)` - Returns samples within time range
- `per_minute_avg_fill(from: SystemTime, to: SystemTime)` - Returns per-minute averages in reverse chronological order (most recent first)
- `latest_sample()` - Returns most recent measurement
- `len()` - Returns total number of stored samples

### HTTP Server (`src/server.rs`)

Provides REST API access to temperature data using Axum framework.

#### Endpoints

##### GET `/temps`

Returns temperature measurements as per-minute averages.

**Query Parameters:**
- `hours` (optional) - Number of hours to retrieve (default: 3)

**Response Format:**
```json
{
  "temperatures": [25.39, null, 24.8, 24.2],
  "latest_time": 1758294793,
  "oldest_time": 1758294553,
  "interval_minutes": 1,
  "count": 4
}
```

**Response Fields:**
- `temperatures` - Array of temperature values (°C), most recent first. `null` values indicate missing measurements
- `latest_time` - Unix timestamp of the most recent actual measurement (can be `null` if no measurements exist)
- `oldest_time` - Unix timestamp of the oldest measurement in the response (can be `null` if no measurements exist)
- `interval_minutes` - Time interval between measurements (always 1)
- `count` - Number of time slots returned (including nulls)

**Examples:**
- `/temps` - Last 3 hours (180 values)
- `/temps?hours=5` - Last 5 hours (300 values)
- `/temps?hours=1` - Last 1 hour (60 values)

**Data Characteristics:**
- One temperature value per minute (averaged from 15-second samples)
- Reverse chronological order (index 0 = most recent)
- `null` values represent minutes where thermometer was unavailable
- No leading/trailing nulls - only covers the actual data time range
- Returns empty array if no measurements exist in the requested period

### Error Handling

#### Storage Errors (`StorageError`)
- `InvalidTimeRange` - Start time is after end time
- `NoDataAvailable` - No measurements in requested time range

#### HTTP Errors
- All storage errors are converted to HTTP 500 with descriptive messages
- Invalid requests return appropriate HTTP status codes

## Usage Examples

### Adding Measurements
```rust
let mut storage = Storage::new();
storage.add_measurement(23.5, 45.2); // temp=23.5°C, humidity=45.2%
```

### Querying Data
```rust
let now = SystemTime::now();
let one_hour_ago = now - Duration::from_secs(3600);
let samples = storage.get_samples_in_range(one_hour_ago, now)?;
```

### HTTP API Usage
```bash
# Get last 3 hours
curl http://localhost:8080/temps

# Get last 8 hours  
curl http://localhost:8080/temps?hours=8

# Response interpretation:
# temperatures[0] = most recent minute
# temperatures[1] = 1 minute ago
# temperatures[2] = 2 minutes ago
# etc.
```

## Configuration

The server configuration is handled in `src/config.rs` and includes:
- Port configuration for HTTP server
- Other application settings

## Dependencies

Key dependencies include:
- `axum` - HTTP server framework
- `serde` - JSON serialization
- `tokio` - Async runtime
- `anyhow` - Error handling
- Standard library collections (`VecDeque`) for efficient data storage

## Performance Characteristics

- **Memory Usage**: Configurable with optional capacity limits
- **Query Performance**: O(n) for range queries where n is samples in range
- **Storage Performance**: O(1) for adding new measurements
- **Network Efficiency**: Per-minute aggregation reduces payload size significantly