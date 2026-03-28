use anyhow::Result;
use dotenv::dotenv;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use tokio_tungstenite::{connect_async, tungstenite::Bytes, tungstenite::Message};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

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

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let database_url = std::env::var("DATABASE_URL").unwrap();
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;

    info!("Staring crypto prices listener...");

    info!("Connecting to {}", URL);

    let (ws_stream, _) = connect_async(URL).await?;
    info!("Connected to {}", URL);
    let (mut write, mut read) = ws_stream.split();

    let subscription_items = vec![SubscriptionItem {
        topic: "crypto_prices_chainlink".to_string(),
        type_: "*".to_string(),
        filters: "".to_string(),
    }];
    let subscription = Subscription {
        action: "subscribe".to_string(),
        subscriptions: subscription_items,
    };
    info!("Sending subscription: {:?}", subscription);

    write
        .send(Message::Text(serde_json::to_string(&subscription)?.into()))
        .await?;

    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                if let Ok(price_update_message) = serde_json::from_str::<PriceUpdateMessage>(&text)
                {
                    let symbol = &price_update_message.payload.symbol;
                    let timestamp = &price_update_message.payload.timestamp;
                    let price = &price_update_message.payload.full_accuracy_value;
                    info!("{} {} {}", symbol, timestamp, price);

                    let result = sqlx::query!(
                        "INSERT INTO crypto_prices (symbol, timestamp, price) VALUES (?, ?, ?)",
                        symbol,
                        timestamp,
                        price,
                    )
                    .execute(&pool)
                    .await?;
                    info!(
                        "affected rows {}, inserted id {}",
                        result.rows_affected(),
                        result.last_insert_id()
                    );
                }
            }
            Message::Ping(_) => write.send(Message::Pong(Bytes::new())).await?,
            _ => {}
        }
    }

    Ok(())
}
