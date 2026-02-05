use clickhouse::Client;
use futures::StreamExt;
use logai_core::LogEntry;
use tracing::{info, error};

#[tokio::main]

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    //connect to NATS
    info!("Connecting to NATS...");
    let nats = async_nats::connect("localhost:4222").await?;
    info!("Connected to NATS!");

    // connect to clickhouese
    info!("Connecting to ClickHouse...");
    let clickhouse = Client::default()
        .with_url("http://localhost:8123")
        .with_database("logai");
    info!("Connected to Clickhouse!");

    // Create table if not exists
    create_logs_table(&clickhouse).await?;


    //Subscribe to logs.ingest
    info!("Subscribing to logs.ingest...");
    let mut subscriber = nats.subscribe("logs.ingest").await?;

    //process messages
    while let Some(message) = subscriber.next().await {
        match serde_json::from_slice::<LogEntry>(&message.payload) {
            Ok(entry) => {
                info!(
                    id = %entry.id,
                    level = ?entry.level,
                    service = %entry.service,
                    "Received Log"
                );
                //Insert clickhouse
                if let Err(e) = insert_log(&clickhouse, &entry).await {
                    error!("Failed to insert log: {}", e);
                } 
            }
            Err(e) => {
                error!("Failed to parse messgae: {}", e);
            }
        }
    }
    Ok(())

}

async fn create_logs_table(client: &Client) -> Result<(), clickhouse::error::Error> {
    client.query(r#"
        CREATE TABLE IF NOT EXISTS logs (
            id UUID,
            timestamp DateTime64(3),
            level String,
            service String,
            message String,
            raw String,
            trace_id Nullable(String),
            span_id Nullable(String),
            error_category Nullable(String),
            fields String,
            ingested_at DateTime64(3)
        ) ENGINE = MergeTree()
        ORDER BY (service, timestamp)
        PARTITION BY toYYYYMM(timestamp)
    "#).execute().await?;

    info!("Logs table ready");
    Ok(())
}

async fn insert_log(client: &Client, entry: &LogEntry) -> Result<(), clickhouse::error::Error> {
    client.query(r#"
    INSERT INTO logs (id, timestamp, level, service, message, raw, trace_id, span_id, error_category, fields, ingested_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(entry.id)
    .bind(entry.timestamp.timestamp_millis())
    .bind(format!("{:?}", entry.level))
    .bind(&entry.service)
    .bind(&entry.message)
    .bind(&entry.raw)
    .bind(&entry.trace_id)
    .bind(&entry.span_id)
    .bind(entry.error_category.map(|e| format!("{:?}", e)))
    .bind(serde_json::to_string(&entry.fields).unwrap_or_else(|_| "{}".to_string()))
    .bind(entry.ingested_at.timestamp_millis())
    .execute()
    .await?;

    info!(id = %entry.id, "Log stored in Clickhouse");
    Ok(())
}