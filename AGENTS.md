# AGENTS.md - AI Coding Assistant Project Guide

This document provides project background, build process, and development guidelines for AI coding assistants.

---

## Project Overview

**crypto-price-listener** is a Rust-based cryptocurrency price listening service. The application connects to Polymarket's real-time data stream via WebSocket, subscribes to cryptocurrency price updates, and persists received data to a MySQL database.

### Core Features
- Connect to Polymarket WebSocket service (`wss://ws-live-data.polymarket.com`)
- Subscribe to price updates on the `crypto_prices_chainlink` topic
- Write price data (trading pair symbol, timestamp, price) to MySQL database
- Structured logging using tracing

---

## Technology Stack

| Component | Purpose | Version |
|-----------|---------|---------|
| Rust | Programming language | Edition 2024 |
| Tokio | Async runtime | 1.50.0 |
| tokio-tungstenite | WebSocket client | 0.29.0 |
| sqlx | Async SQL toolkit | 0.8.6 |
| serde/serde_json | Serialization/Deserialization | 1.0.x |
| tracing | Logging | 0.1.x |
| anyhow | Error handling | 1.0.x |
| dotenv | Environment variable loading | 0.15.0 |
| rust_decimal | High-precision decimal numbers | 1.41.0 |

---

## Project Structure

```
crypto-price-listener/
├── Cargo.toml          # Project configuration and dependencies
├── Cargo.lock          # Dependency lock file
├── .env                # Environment variables configuration (gitignored)
├── .gitignore          # Git ignore rules
├── README.md           # Project documentation
├── AGENTS.md           # This file
└── src/
    └── main.rs         # Single entry point (all logic)
```

### Code Organization

The current project uses a **single-file architecture**, with all logic concentrated in `src/main.rs`:

- **Data Models** (`Subscription`, `SubscriptionItem`, `PriceUpdateMessage`, `PriceUpdatePayload`)
- **WebSocket Connection Management** - Uses `tokio_tungstenite::connect_async`
- **Message Processing Loop** - Parses JSON messages and extracts price data
- **Database Writes** - Executes insert operations using `sqlx::query!` macro

---

## Database Schema

### Table Structure

```sql
CREATE TABLE `crypto_prices` (
  `id` int unsigned NOT NULL AUTO_INCREMENT,
  `symbol` char(8) COLLATE utf8mb4_general_ci NOT NULL,
  `price` decimal(38,0) NOT NULL,
  `timestamp` bigint NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
```

### Field Descriptions
- `symbol`: Trading pair symbol (e.g., BTCUSD, ETHUSD)
- `price`: Price value, stored with full precision using `decimal(38,0)`
- `timestamp`: Unix timestamp in milliseconds

---

## Build and Run

### Prerequisites

1. **Rust toolchain** (>= 1.94.0)
2. **MySQL database** - Running and accessible via network
3. **Environment configuration** - Create `.env` file in project root

### Environment Configuration

Create `.env` file:

```bash
# Format: mysql://username:password@host:port/database
DATABASE_URL="mysql://root:password@localhost:3306/polymarket"
```

### Build Commands

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run (automatically reads .env)
cargo run
```

### sqlx Compile-time Checking

The project uses `sqlx::query!` macro for **compile-time SQL validation**. This requires:
- `.env` file must exist and contain a valid `DATABASE_URL`
- Database must be accessible with the appropriate table structure
- Compilation will fail if the database is unavailable

---

## Code Style Guide

### General Conventions

1. **Language**: Code comments and documentation use English
2. **Error Handling**: Use `anyhow::Result` for error propagation
3. **Logging**: Use `tracing` crate at `INFO` log level
4. **Async Style**: Use `tokio` runtime, functions marked as `async`

### Naming Conventions

- Structs: PascalCase (`PriceUpdateMessage`)
- Functions/Variables: snake_case (`price_update`, `full_accuracy_value`)
- Constants: SCREAMING_SNAKE_CASE (`URL`)

### Special Attributes

```rust
#[allow(dead_code)]  // For temporarily unused fields, avoids compiler warnings
#[serde(rename = "type")]  // Handle field names conflicting with Rust keywords
type_: String,
```

---

## Testing Strategy

The current project **does not include automated tests**. To add tests:

```bash
# Run tests
cargo test

# Example unit test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialization() {
        // Test JSON deserialization
    }
}
```

---

## Deployment Considerations

### Environment Variables

Production environment must set `DATABASE_URL`. Recommendations:
- Use secrets management service to store database credentials
- Avoid committing `.env` file to version control

### Database Connection Pool

Current configuration: Maximum 5 connections (`max_connections(5)`)
May need adjustment based on load.

### Error Recovery

WebSocket connection now automatically reconnects with exponential backoff:
- Initial retry delay: 1 second
- Maximum retry delay: 60 seconds
- Unlimited reconnection attempts (configurable)
- Graceful shutdown handling on SIGTERM/SIGINT

---

## Security Considerations

1. **Database Credentials**: `.env` file is added to `.gitignore` and won't be accidentally committed
2. **TLS Connection**: `tokio-tungstenite` uses `native-tls` feature for encrypted WebSocket connections
3. **SQL Injection Prevention**: Uses `sqlx::query!` macro with parameterized queries

---

## Extension Suggestions

For future enhancements, consider:
- Split single file into modules (`models/`, `db/`, `websocket/`)
- Add configuration management (support multiple trading pair subscriptions)
- Implement metrics export (Prometheus)
- Support other databases (PostgreSQL, SQLite)
