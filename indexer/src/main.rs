mod handlers;
mod models;

use handlers::ObjectDataHandler;
use handlers::TransactionDigestHandler;

pub mod schema;

use anyhow::{Result, bail};
use clap::Parser;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use sui_indexer_alt_framework::{
    cluster::{Args, IndexerCluster},
    pipeline::sequential::SequentialConfig,
    service::Error,
};
use tokio;
use url::Url;

// Embed database migrations into the binary so they run automatically on startup
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[tokio::main]
async fn main() -> Result<()> {
    println!("[boot] main() start");
    // Load .env data
    dotenvy::dotenv().ok();
    println!("[boot] .env loaded");

    // Local database URL created in step 3 above
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in the environment")
        .parse::<Url>()
        .expect("Invalid database URL");
    println!("[boot] DATABASE_URL parsed");

    // Parse command-line arguments (checkpoint range, URLs, performance settings)
    let args = Args::parse();
    println!("[boot] args parsed");

    // Build and configure the indexer cluster
    println!("[boot] building cluster");
    let mut cluster = IndexerCluster::builder()
        .with_args(args) // Apply command-line configuration
        .with_database_url(database_url) // Set up database URL
        .with_migrations(&MIGRATIONS) // Enable automatic schema migrations
        .build()
        .await?;
    println!("[boot] cluster built");

    // Register our custom sequential pipeline with the cluster
    println!("[boot] registering TransactionDigestHandler pipeline");
    cluster
        .sequential_pipeline(
            TransactionDigestHandler,    // Our processor/handler implementation
            SequentialConfig::default(), // Use default batch sizes and checkpoint lag
        )
        .await?;
    println!("[boot] TransactionDigestHandler pipeline registered");

    println!("[boot] registering ObjectDataHandler pipeline");
    cluster
        .sequential_pipeline(
            ObjectDataHandler,          // Object data processor/handler implementation
            SequentialConfig::default(), // Use default batch sizes and checkpoint lag
        )
        .await?;
    println!("[boot] ObjectDataHandler pipeline registered");

    // Start the indexer and wait for completion
    println!("[boot] starting cluster");
    let service = cluster.run().await?;
    println!("[boot] cluster started, entering service.main()");
    match service.main().await {
        Ok(()) | Err(Error::Terminated) => Ok(()),
        Err(Error::Aborted) => {
            bail!("Indexer aborted due to an unexpected error")
        }
        Err(Error::Task(e)) => {
            bail!(e)
        }
    }
}
