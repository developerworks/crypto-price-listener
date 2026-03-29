mod logging;
mod signal;

use anyhow::Result;
use dotenv::dotenv;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use std::time::Duration;
use tokio::time::{Instant, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Bytes, tungstenite::Message};
use tracing::{error, info, warn};

/// WebSocket reconnection configuration
const INITIAL_RETRY_DELAY_SECS: u64 = 1;
const MAX_RETRY_DELAY_SECS: u64 = 60;
const BACKOFF_MULTIPLIER: u64 = 2;

/// Maximum number of reconnection attempts (None for unlimited)
const MAX_RECONNECT_ATTEMPTS: Option<u64> = None;

#[derive(Serialize, Debug)]
struct Subscription {
    action: String,
    subscriptions: Vec<SubscriptionItem>,
}

#[derive(Serialize, Debug)]
struct SubscriptionItem {
    topic: String,
    #[serde(rename = "type")]
    type_: String,
    filters: String,
}
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct PriceUpdateMessage {
    connection_id: String,
    payload: PriceUpdatePayload,
    timestamp: i64,
    topic: String,
    #[serde(rename = "type")]
    type_: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct PriceUpdatePayload {
    full_accuracy_value: String,
    symbol: String,
    timestamp: i64,
    value: f64,
}

const URL: &str = "wss://ws-live-data.polymarket.com";

/// Create database connection pool
async fn create_db_pool() -> Result<sqlx::MySqlPool> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL env var not set"))?;
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    Ok(pool)
}

/// Create subscription message
fn create_subscription() -> Subscription {
    let subscription_items = vec![SubscriptionItem {
        topic: "crypto_prices_chainlink".to_string(),
        type_: "*".to_string(),
        filters: "".to_string(),
    }];
    Subscription {
        action: "subscribe".to_string(),
        subscriptions: subscription_items,
    }
}

/// Handle price update message
async fn handle_price_update(text: &str, pool: &sqlx::MySqlPool) -> Result<()> {
    if let Ok(price_update_message) = serde_json::from_str::<PriceUpdateMessage>(text) {
        let symbol = &price_update_message.payload.symbol;
        let timestamp = &price_update_message.payload.timestamp;
        let price = &price_update_message.payload.full_accuracy_value;
        info!("Price update received: {} {} {}", symbol, timestamp, price);

        let result = sqlx::query!(
            "INSERT INTO crypto_prices (symbol, timestamp, price) VALUES (?, ?, ?)",
            symbol,
            timestamp,
            price,
        )
        .execute(pool)
        .await?;
        info!(
            "Database write successful: {} rows affected, inserted ID {}",
            result.rows_affected(),
            result.last_insert_id()
        );
    }
    Ok(())
}

/// Run WebSocket connection (internal function, can be called by reconnection logic)
async fn run_websocket_connection(
    pool: &sqlx::MySqlPool,
    shutdown_rx: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    info!("Connecting to {}", URL);

    let (ws_stream, _) = connect_async(URL).await?;
    info!("WebSocket connected successfully");

    let (mut write, mut read) = ws_stream.split();

    // Send subscription message
    let subscription = create_subscription();
    write
        .send(Message::Text(serde_json::to_string(&subscription)?.into()))
        .await?;
    info!("Subscription message sent");

    // Message processing loop
    loop {
        tokio::select! {
            // Listen for WebSocket messages
            msg = read.next() => {
                match msg {
                    Some(Ok(msg)) => match msg {
                        Message::Text(text) => {
                            if let Err(e) = handle_price_update(&text, pool).await {
                                warn!("Failed to process price update: {}", e);
                            }
                        }
                        Message::Ping(_) => {
                            if let Err(e) = write.send(Message::Pong(Bytes::new())).await {
                                warn!("Failed to send Pong: {}", e);
                                return Err(e.into());
                            }
                        }
                        Message::Close(_) => {
                            warn!("Received server close connection message");
                            return Err(anyhow::anyhow!("Server closed connection"));
                        }
                        _ => {}
                    },
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        return Err(e.into());
                    }
                    None => {
                        warn!("WebSocket connection closed");
                        return Err(anyhow::anyhow!("Connection closed unexpectedly"));
                    }
                }
            }

            // Listen for shutdown signal
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received, gracefully closing connection...");
                    // Send close frame
                    let _ = write.send(Message::Close(None)).await;
                    return Ok(());
                }
            }
        }
    }
}

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

        match run_websocket_connection(&pool, &mut connection_shutdown_rx).await {
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
