
use std::sync::{Arc, Mutex};
use crate::config::Config;
use crate::app_error::AppError;
use crate::storage::{Sample, SampleSpec, Storage};
use anyhow::Context;

use axum::{routing::{get}, extract::{State}, Router, Json};
use axum::serve;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    storage: Arc<Mutex<Storage>>,
}

pub async fn run_server(
    storage: Arc<Mutex<Storage>>,
    config: &Config) -> Result<(), AppError> {
    let state = AppState { storage };
    let app = Router::new()
        .route("/temps", get(temps))
        .fallback(get(fallback))
        .with_state(state);


    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;
    Ok(())
}



async fn temps(State(state): State<AppState>) -> Result<Json<Sample>, AppError> {
    Ok(axum::Json(state.storage.lock()?
        .sample(SampleSpec::Time(std::time::Duration::from_secs(3600)))))

}

async fn fallback() -> &'static str {
    "Not found"
}