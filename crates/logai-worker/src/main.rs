use clickhouse::Client;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use futures::StreamExt;
use logai_core::LogEntry;
use tracing::{info, error};
use serde_json::json;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};

const COLLECTION_NAME: &str = "log_embeddings";
const VECTOR_SIZE: u64 = 384; // all mini LML6V2 output 384 dimensions

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
    create_logs_table(&clickhouse).await?;
    info!("Clickhouse ready!");

    // Conncect to qdrant
    info!("Connecting to Qdrant...");
    let qdrant = Qdrant::from_url("http://localhost:6334").build()?;
    setup_qdrant_collection(&qdrant).await?;
    info!("Qdrant ready!");

    // Load embedding model (running locally)
    info!("Loading embedding model (First time downloads 30mb)..");
    let mut  model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),)?;
    info!("Embedding model loaded!");

    //Subscribe to logs.ingest
    info!("Subscribing to logs.ingest...");
    let mut subscriber = nats.subscribe("logs.ingest").await?;
    info!("Worker ready! Waiting for logs...");

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
                // Store in ClickHouse (exisitng)
                if let Err(e) = insert_log(&clickhouse, &entry).await {
                    error!("ClickHouse insert failed: {}", e);
                } 

                // Generate mebdding & store in Qdrant 
                if let Err(e) = embed_and_store(&mut model, &qdrant, &entry).await {
                    error!("Qdrant Store failed: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to parse messgae: {}", e);
            }
        }
    }
    Ok(())

}

/// Setuping the qdrant collection like creating a table

async fn setup_qdrant_collection(qdrant: &Qdrant) -> Result<(), Box<dyn std::error::Error>> {
    // check if collection already exists or not
    let collection = qdrant.list_collections().await?;
    let exists = collection
    .collections
    .iter()
    .any(|c| c.name == COLLECTION_NAME);

    if !exists {
        info!("Creating Qdrant collection: {}", COLLECTION_NAME);
        qdrant
        .create_collection(
            CreateCollectionBuilder::new(COLLECTION_NAME)
                        .vectors_config(VectorParamsBuilder::new(VECTOR_SIZE, Distance::Cosine))
        )
        .await?;
    info!("Collection Created");
    } else {
        info!("Qdrant collection already exists");
    }
    Ok(())
}

/// Generate embedding for a log and store in Qdrant

async fn embed_and_store(
    model: &mut TextEmbedding,
    qdrant: &Qdrant,
    entry: &LogEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create text to embed: combine service + Level + message
    let text_to_embed = format!(
        "service:{} level:{:?} {}",
        entry.service, entry.level, entry.message
    );

    // Generate embedding (text -> 384D vector)
    let documents: Vec<String> = vec![text_to_embed.clone()];
    let embeddings = model.embed(documents, None)?;
    let vector = embeddings.into_iter().next().ok_or("No embeddings generated")?;

    if vector.is_empty() {
        return Err("Embedding returned empty vector".into());
    }
    info!("Generated embedding with {} dimensions", vector.len());

    // Create point with metadata (payload)
    let payload: Payload = json!({
        "log_id": entry.id.to_string(),
        "service": entry.service,
        "level": format!("{:?}", entry.level),
        "message": entry.message,
        "timestamp": entry.timestamp.to_rfc3339(),
        "timestamp_unix": entry.timestamp.timestamp(),
    })
    .try_into()
    .unwrap();

    let point = PointStruct::new(entry.id.to_string(), vector, payload,);

    //Upsert (insert or update) into the Qdrant
    qdrant.upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, vec![point]).wait(true)).await?;

    info!(id = %entry.id, "Embedded & stored in Qdrant");
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