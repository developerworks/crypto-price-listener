//! Logging module for configuring tracing output
//!
//! This module provides centralized logging configuration with both
//! stdout and file output, supporting daily log rotation.

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Log file configuration
const LOG_FILE_PREFIX: &str = "crypto-price-listener";
const LOG_DIRECTORY: &str = "logs";

/// Initialize tracing with file and stdout output
///
/// Returns a `WorkerGuard` that must be kept alive for the duration of the program
/// to ensure logs are properly flushed.
///
/// # Features
/// - Daily log rotation (logs are stored in `logs/` directory)
/// - Non-blocking file writes
/// - Output to both stdout and file
/// - Log level control via `RUST_LOG` environment variable
///
/// # Example
/// ```
/// let _guard = init_tracing()?;
/// ```
pub fn init_tracing() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all(LOG_DIRECTORY)?;

    // File appender with daily rotation
    let file_appender = tracing_appender::rolling::daily(LOG_DIRECTORY, LOG_FILE_PREFIX);
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // Stdout layer
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false);

    // File layer
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_target(false);

    // Environment filter (can be controlled via RUST_LOG env var)
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}
