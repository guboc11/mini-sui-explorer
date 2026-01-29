mod handlers;
mod models;

use handlers::ObjectDataHandler;
use handlers::TransactionDigestHandler;

pub mod schema;

use anyhow::{Context, Result, bail};
use clap::Parser;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use sui_indexer_alt_framework::{
    cluster::{Args, IndexerCluster},
    pipeline::sequential::SequentialConfig,
    service::Error,
};
use sui_rpc_api::client::{Client as RpcClient, HeadersInterceptor};
use tokio;
use url::Url;

// Embed database migrations into the binary so they run automatically on startup
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(Parser, Debug)]
struct CliArgs {
    /// Start indexing from genesis (checkpoint 0) instead of latest.
    #[arg(long)]
    from_genesis: bool,

    #[clap(flatten)]
    cluster: Args,
}

async fn fetch_latest_checkpoint_sequence(
    rpc_api_url: &Url,
    rpc_username: Option<&str>,
    rpc_password: Option<&str>,
) -> Result<u64> {
    let client = RpcClient::new(rpc_api_url.as_str())
        .with_context(|| format!("Failed to create RPC client for {rpc_api_url}"))?;
    let client = if let Some(username) = rpc_username {
        let mut headers = HeadersInterceptor::new();
        headers.basic_auth(username.to_string(), rpc_password.map(str::to_string));
        client.with_headers(headers)
    } else {
        client
    };
    let mut client = client;
    let summary = client
        .get_latest_checkpoint()
        .await
        .with_context(|| format!("Failed to fetch latest checkpoint from {rpc_api_url}"))?;
    Ok(summary.data().sequence_number)
}

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
    let cli = CliArgs::parse();
    let mut args = cli.cluster;
    println!("[boot] args parsed");

    if args.indexer_args.first_checkpoint.is_some() {
        println!("[boot] using provided first_checkpoint");
    } else if cli.from_genesis {
        println!("[boot] from-genesis enabled: starting at checkpoint 0");
    } else {
        let rpc_api_url = args
            .client_args
            .ingestion
            .rpc_api_url
            .as_ref()
            .context("default start requires --rpc-api-url (or RPC_API_URL) to fetch latest")?;
        let latest = fetch_latest_checkpoint_sequence(
            rpc_api_url,
            args.client_args.ingestion.rpc_username.as_deref(),
            args.client_args.ingestion.rpc_password.as_deref(),
        )
        .await?;
        println!("[boot] start-latest: using checkpoint {latest}");
        args.indexer_args.first_checkpoint = Some(latest);
    }

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
