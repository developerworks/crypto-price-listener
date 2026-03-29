//! Configuration constants for the application
//!
//! This module centralizes all configurable constants used throughout
//! the crypto-price-listener application.

/// WebSocket endpoint URL
pub const URL: &str = "wss://ws-live-data.polymarket.com";

/// WebSocket reconnection configuration
pub const INITIAL_RETRY_DELAY_SECS: u64 = 1;
pub const MAX_RETRY_DELAY_SECS: u64 = 60;
pub const BACKOFF_MULTIPLIER: u64 = 2;

/// Maximum number of reconnection attempts (None for unlimited)
pub const MAX_RECONNECT_ATTEMPTS: Option<u64> = None;
