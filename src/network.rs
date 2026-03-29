//! Network module for WebSocket connection management
//!
//! This module handles WebSocket connections, message processing,
//! and subscription management for the crypto price listener.

use crate::config::URL;
use crate::database::handle_price_update;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio_tungstenite::{connect_async, tungstenite::Bytes, tungstenite::Message};
use tracing::{error, info, warn};

/// Subscription request message
#[derive(Serialize, Debug)]
pub struct Subscription {
    pub action: String,
    pub subscriptions: Vec<SubscriptionItem>,
}

/// Individual subscription item
#[derive(Serialize, Debug)]
pub struct SubscriptionItem {
    pub topic: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub filters: String,
}

/// Create subscription message for crypto price updates
///
/// Subscribes to the `crypto_prices_chainlink` topic with all event types.
pub fn create_subscription() -> Subscription {
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

/// Run WebSocket connection
///
/// Establishes a WebSocket connection to the Polymarket endpoint,
/// sends subscription message, and processes incoming messages.
///
/// # Arguments
/// * `pool` - Database connection pool for persisting price updates
/// * `shutdown_rx` - Receiver for shutdown signal
///
/// # Errors
/// Returns an error if connection fails or is closed unexpectedly
pub async fn run_websocket_connection(
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
