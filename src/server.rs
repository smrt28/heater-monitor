use std::sync::{Arc, Mutex};
use crate::config::Config;
use crate::app_error::AppError;
use crate::storage::Storage;

pub async fn run_server(
    storage: Arc<Mutex<Storage>>,
    config: &Config) -> Result<(), AppError> {
    
    
    Ok(())
}