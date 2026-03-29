mod config;
mod database;
mod logging;
mod network;
mod signal;

use anyhow::Result;
use config::{
    BACKOFF_MULTIPLIER, INITIAL_RETRY_DELAY_SECS, MAX_RECONNECT_ATTEMPTS, MAX_RETRY_DELAY_SECS,
};
use database::create_db_pool;
use dotenv::dotenv;
use std::time::Duration;
use tokio::time::{Instant, sleep};
use tracing::{error, info, warn};



#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Initialize tracing and keep the guard alive
    let _tracing_guard = logging::init_tracing()?;

    let pool = create_db_pool().await?;
    info!("Database connection pool created successfully");
    info!("Starting crypto price listener service...");

    // Create shutdown signal channel
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    // Setup Ctrl+C signal handler
    signal::setup_ctrl_c_handler(shutdown_tx.clone());

    // Reconnection loop
    let mut retry_delay_secs = INITIAL_RETRY_DELAY_SECS;
    let mut connection_count = 0u64;

    loop {
        // Check if shutdown signal received
        if *shutdown_rx.borrow() {
            info!("Service stopped");
            break;
        }

        // Check max reconnection attempts
        if let Some(max_attempts) = MAX_RECONNECT_ATTEMPTS
            && connection_count >= max_attempts
        {
            error!(
                "Max reconnection attempts ({}) reached, giving up",
                max_attempts
            );
            break;
        }

        connection_count += 1;
        info!("Connection attempt # {}", connection_count);

        let start_time = Instant::now();

        // Clone shutdown_rx for this connection
        let mut connection_shutdown_rx = shutdown_rx.clone();

        match network::run_websocket_connection(&pool, &mut connection_shutdown_rx).await {
            Ok(()) => {
                // Normal shutdown (signal received)
                info!("Connection closed normally, service stopped");
                break;
            }
            Err(e) => {
                let duration = start_time.elapsed();
                warn!(
                    "Connection abnormally terminated (uptime: {:.2?}): {}",
                    duration, e
                );

                // Check if shutdown signal received
                if *shutdown_rx.borrow() {
                    info!("Service stopped, no more reconnection attempts");
                    break;
                }

                // Exponential backoff retry
                warn!(
                    "Will attempt to reconnect in {} seconds...",
                    retry_delay_secs
                );

                // Use tokio::select to listen for both shutdown signal and delay
                tokio::select! {
                    _ = sleep(Duration::from_secs(retry_delay_secs)) => {}
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("Shutdown signal received, canceling reconnection");
                            break;
                        }
                    }
                }

                // Increase delay (exponential backoff)
                retry_delay_secs =
                    (retry_delay_secs * BACKOFF_MULTIPLIER).min(MAX_RETRY_DELAY_SECS);
            }
        }
    }

    // Close database connection pool
    info!("Closing database connection pool...");
    pool.close().await;
    info!("Service fully stopped");

    Ok(())
}
