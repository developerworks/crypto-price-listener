
# Overview

**Project Overview:** This is a Rust-based crypto price listener that connects to Polymarket's WebSocket API to receive real-time cryptocurrency price updates and stores them in a MySQL database.

**Technology Stack:**

- Rust (Edition 2024)
- Tokio for async runtime
- tokio-tungstenite for WebSocket connections
- sqlx for database operations with MySQL
- serde/serde_json for serialization
- tracing for logging
- anyhow for error handling
- dotenv for environment configuration
- rust_decimal for decimal precision

**Architecture:** It's a simple single-file application (main.rs) that:

- Connects to a WebSocket endpoint (wss://ws-live-data.polymarket.com)
- Subscribes to crypto price updates
- Inserts received data into MySQL database

**Database:** MySQL with a table crypto_prices storing symbol, price, and timestamp

**Build Process:** Standard Cargo build system

**Configuration:** Environment variables via `.env` file (DATABASE_URL)

## sqlx::query! compile check

You need set environment variable in `.env` file

```
echo "DATABASE_URL=mysql://root:password@localhost:3306/polymarket" > .env
```

**DDL:**

```sql
CREATE TABLE `crypto_prices` (
  `id` int unsigned NOT NULL AUTO_INCREMENT,
  `symbol` char(8) COLLATE utf8mb4_general_ci NOT NULL,
  `price` decimal(38,0) NOT NULL,
  `timestamp` bigint NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=1975 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci
```