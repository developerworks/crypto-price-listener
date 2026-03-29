//! Signal handling module for graceful shutdown
//!
//! This module provides signal handling functionality to gracefully shutdown
//! the application when receiving system signals like Ctrl+C.

use tokio::sync::watch::Sender;
use tracing::{error, info};

/// Spawn a task to handle Ctrl+C signal for graceful shutdown
///
/// # Arguments
/// * `shutdown_tx` - The sender half of a watch channel to signal shutdown
///
/// # Example
/// ```
/// let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
/// setup_ctrl_c_handler(shutdown_tx);
/// ```
pub fn setup_ctrl_c_handler(shutdown_tx: Sender<bool>) {
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("Failed to listen for Ctrl+C signal: {}", e);
            return;
        }
        info!("Ctrl+C signal received, shutting down service...");
        let _ = shutdown_tx.send(true);
    });
}
