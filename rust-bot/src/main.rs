//! Hanisha Rust Bot - High-performance auto-filter service
//! Ported from Python ShobanaFilterBot for Koyeb Free Tier
#![deny(missing_docs)]

use std::sync::Arc;
use tracing_subscriber::fmt;

mod config;
mod database;
mod indexer;
mod filter;
mod cache;
mod sweep;
mod keepalive;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing subscriber with env_filter
    fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing_info!("Starting Hanisha Rust Bot - Koyeb Free Tier (0.1 vCPU, 512 MB RAM)");

    // ===== Load Configuration =====
    let config = config::load_config().await;

    // Validate required configuration
    if config.database_uri.is_empty() {
        tracing_error!("DATABASE_URI environment variable is required");
        std::process::exit(1);
    }

    // ===== Initialize MongoDB Connection =====
    let client_options = mongodb::options::ClientOptions::parse(&config.database_uri)
        .await
        .expect("Failed to parse MongoDB URI");

    // Set max pool size to 5 for Koyeb free tier
    client_options.max_pool_size = Some(5.into());

    let mongo_client = mongodb::Client::with_options(client_options)
        .await
        .expect("Failed to create MongoDB client");

    // Ping MongoDB to verify connection
    mongo_client
        .database(&config.database_name)
        .run_command(
            mongodb::bson::doc! {"ping": 1},
            None,
        )
        .await
        .expect("Failed to ping MongoDB");

    tracing_info!("Connected to MongoDB: {}", config.database_name);

    // ===== Initialize Shared State =====
    let shared = Arc::new(database::SharedState::new(mongo_client, config.clone()).await);

    // ===== Start Keep-Alive HTTP Server =====
    let keepalive_config = keepalive::KeepAliveConfig::from_env();
    let shared_http = Arc::clone(&shared);
    tokio::spawn(async move {
        if let Err(e) = keepalive::start_server(keepalive_config).await {
            tracing_error!("Keep-alive server error: {:?}", e);
        }
    });

    // ===== Start Auto-Delete Sweep Loop =====
    // Single background task every 15 seconds - does not spawn thousands of sleeps
    let shared_sweep = Arc::clone(&shared);
    tokio::spawn(async move {
        if let Err(e) = sweep::start_sweep_loop(shared_sweep).await {
            tracing_error!("Auto-delete sweep error: {:?}", e);
        }
    });

    // ===== Start Telegram Bot Client =====
    // Initialize the bot with all handlers and start polling/updates
    shared.start_bot().await?;

    Ok(())
}