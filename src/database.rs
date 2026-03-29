//! Database module for connection management and data persistence
//!
//! This module provides database connection pooling and functions
//! for persisting price update data to MySQL.

use anyhow::Result;
use serde::Deserialize;
use sqlx::mysql::MySqlPoolOptions;
use tracing::info;

/// Price update message from WebSocket
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct PriceUpdateMessage {
    pub connection_id: String,
    pub payload: PriceUpdatePayload,
    pub timestamp: i64,
    pub topic: String,
    #[serde(rename = "type")]
    pub type_: String,
}

/// Price update payload containing actual price data
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct PriceUpdatePayload {
    pub full_accuracy_value: String,
    pub symbol: String,
    pub timestamp: i64,
    pub value: f64,
}

/// Create database connection pool
///
/// Reads DATABASE_URL from environment variables and creates
/// a connection pool with 5 max connections.
///
/// # Errors
/// Returns an error if DATABASE_URL is not set or connection fails
pub async fn create_db_pool() -> Result<sqlx::MySqlPool> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL env var not set"))?;
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    Ok(pool)
}

/// Handle price update message
///
/// Parses the JSON text as PriceUpdateMessage and inserts the data
/// into the crypto_prices table.
///
/// # Arguments
/// * `text` - JSON string containing the price update
/// * `pool` - Database connection pool
///
/// # Errors
/// Returns an error if database insert fails
pub async fn handle_price_update(text: &str, pool: &sqlx::MySqlPool) -> Result<()> {
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
