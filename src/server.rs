
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use crate::config::Config;
use crate::app_error::AppError;
use crate::storage::{Storage, StorageError};
use anyhow::Context;
use serde::{Deserialize, Serialize};

use axum::{routing::{get}, extract::{State, Query, Path}, Router, Json};
use axum::response::{Html, Response};
use axum::http::{StatusCode, header};
// use axum::serve;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    storage: Arc<Mutex<Storage>>,
}

#[derive(Deserialize)]
struct TempsQuery {
    hours: Option<u64>,
}

#[derive(Serialize)]
struct TempsResponse {
    temperatures: Vec<Option<f64>>,
    latest_time: Option<u64>,
    oldest_time: Option<u64>,
    interval_minutes: u64,
    count: usize,
}

pub async fn run_server(
    storage: Arc<Mutex<Storage>>,
    config: &Config) -> Result<(), AppError> {
    let state = AppState { storage };
    let app = Router::new()
        .route("/", get(index))
        .route("/temps", get(temps))
        .route("/assets/{*file}", get(serve_asset))
        .fallback(get(fallback))
        .with_state(state);


    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}

async fn serve_asset(Path(file): Path<String>) -> Result<Response, StatusCode> {
    match file.as_str() {
        "chartjs-adapter-date-fns.bundle.min.js" => {
            let content = include_str!("../assets/chartjs-adapter-date-fns.bundle.min.js");
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/javascript")
                .body(content.into())
                .unwrap())
        }
        _ => Err(StatusCode::NOT_FOUND)
    }
}

async fn temps(
    State(state): State<AppState>,
    Query(params): Query<TempsQuery>
) -> Result<Json<TempsResponse>, AppError> {
    let hours = params.hours.unwrap_or(3);
    let now = SystemTime::now();
    let from = now - Duration::from_secs(hours * 3600);
    
    let storage = state.storage.lock()?;
    let temperatures = storage.per_minute_avg_fill(from, now)
        .map_err(|e| match e {
            StorageError::InvalidTimeRange => AppError::InternalError("Invalid time range".to_string()),
            StorageError::NoDataAvailable => AppError::InternalError("No data available for the requested time range".to_string()),
        })?;
    
    // Get the timestamps of the latest and oldest actual measurements
    let latest_time = storage.latest_sample()
        .map(|sample| sample.timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());
    
    let oldest_time = storage.oldest_sample()
        .map(|sample| sample.timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());
    
    let response = TempsResponse {
        count: temperatures.len(),
        latest_time,
        oldest_time,
        interval_minutes: 1,
        temperatures,
    };
    
    Ok(Json(response))
}

async fn fallback() -> &'static str {
    "Not found"
}