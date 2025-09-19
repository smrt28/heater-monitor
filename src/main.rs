
mod config;
mod app_error;
mod temp_sensor;
mod storage;
mod server;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::config::Config;
use anyhow::Result;
use log::{error, info};
use std::fs::OpenOptions;
use crate::server::run_server;
use crate::temp_sensor::TempSensor;
use crate::storage::Storage;
use clap::Parser;
use daemonize::Daemonize;

#[derive(Parser)]
#[command(name = "heater-monitor")]
#[command(about = "A temperature and humidity monitoring system")]
struct Args {
    #[cfg(debug_assertions)]
    config_path: Option<PathBuf>,

    #[cfg(not(debug_assertions))]
    config_path: PathBuf,

    #[arg(short = 'd', long = "daemon")]
    daemon: bool,
}



async fn run_app(args: Args) -> Result<(), Box<dyn std::error::Error>> {

    #[cfg(debug_assertions)]
    let config_path = args.config_path.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("config.toml")
    });

    #[cfg(not(debug_assertions))]
    let config_path = args.config_path;

    let config = Config::read(config_path.clone())?;
    info!("Loaded config from: {}", config_path.display());

    let storage = Arc::new(Mutex::new(
        Storage::new(&config)
    ));
    info!("Storage initialized");

    {
        let temp_sensor = TempSensor::new(&config.temp_sensor_url);
        let interval = config.interval;
        let storage = storage.clone();
        info!("Starting temperature monitoring task with {}s interval", interval);
        let _handle = tokio::spawn(async move {
            let mut cnt: usize = 0;
            loop {
                if let Ok(val) = temp_sensor.query().await {
                    if cnt % 50 == 0 {
                        // log every 50th measurement
                        info!("Measurements: {}, Temperature: {}°C, Humidity: {}%",
                            cnt, val.temperature, val.humidity);
                    }
                    
                    if let Ok(mut storage) = storage.lock() {
                        cnt += 1;
                        storage.add_measurement(val.temperature, val.humidity);
                    } else {
                        error!("failed to lock storage");
                    }
                } else {
                    error!("failed to query temperature sensor");
                }
                tokio::time::sleep(std::time::Duration::from_secs(interval as u64)).await;
            }
        });
    }

    info!("Starting HTTP server on port {}", config.port);
    let _res = run_server(storage, &config).await?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // In debug mode, force foreground operation
    #[cfg(debug_assertions)]
    let daemon_mode = false;

    #[cfg(not(debug_assertions))]
    let daemon_mode = args.daemon;

    // Initialize logging first, before any log statements
    if daemon_mode {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/heater-monitor.log")?;

        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .target(env_logger::Target::Stderr)
            .init();

        info!("Daemonizing...");

        let daemonize = Daemonize::new()
            .pid_file("/tmp/heater-monitor.pid")
            .chown_pid_file(true)
            .working_directory("/tmp")
            .umask(0o027)
            .stderr(log_file);

        match daemonize.start() {
            Ok(_) => {},
            Err(e) => {
                error!("Daemonization failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .target(env_logger::Target::Stdout)
            .init();
        info!("Running in foreground mode");
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_app(args))
}
