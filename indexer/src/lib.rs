mod handlers;
mod models;

use handlers::ObjectDataHandler;
use handlers::PackageDataHandler;
use handlers::TransactionDigestHandler;

pub mod schema;

use anyhow::{Context, Result, bail};
use clap::Parser;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use prometheus::Registry;
use std::future::Future;
use std::pin::Pin;
use sui_indexer_alt_metrics::MetricsService;
use sui_indexer_alt_framework::{
    cluster::{Args, IndexerCluster},
    ingestion::IngestionConfig,
    postgres::DbArgs,
    Indexer,
    pipeline::sequential::SequentialConfig,
    service::Error,
};
use sui_rpc_api::client::{Client as RpcClient, HeadersInterceptor};
use url::Url;

// Embed database migrations into the binary so they run automatically on startup
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(Parser, Debug)]
struct CliArgs {
    /// Start indexing from genesis (checkpoint 0) instead of latest.
    #[arg(long)]
    from_genesis: bool,

    /// RPC URL used only to fetch the latest checkpoint (not for ingestion).
    #[arg(long)]
    latest_rpc_url: Option<Url>,

    #[clap(flatten)]
    cluster: Args,
}

#[derive(Debug, PartialEq, Eq)]
enum StartMode {
    Provided,
    Genesis,
    Latest(u64),
}

fn fetch_latest_checkpoint_sequence_boxed<'a>(
    rpc_api_url: &'a Url,
    rpc_username: Option<&'a str>,
    rpc_password: Option<&'a str>,
) -> Pin<Box<dyn Future<Output = Result<u64>> + 'a>> {
    Box::pin(async move {
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
    })
}

async fn resolve_start_checkpoint(
    args: &mut Args,
    from_genesis: bool,
    latest_rpc_url: Option<&Url>,
) -> Result<StartMode> {
    resolve_start_checkpoint_with(
        args,
        from_genesis,
        latest_rpc_url,
        fetch_latest_checkpoint_sequence_boxed,
    )
    .await
}

type FetchLatestFn =
    for<'a> fn(&'a Url, Option<&'a str>, Option<&'a str>) -> Pin<Box<dyn Future<Output = Result<u64>> + 'a>>;

async fn resolve_start_checkpoint_with(
    args: &mut Args,
    from_genesis: bool,
    latest_rpc_url: Option<&Url>,
    fetch_latest: FetchLatestFn,
) -> Result<StartMode>
{
    if args.indexer_args.first_checkpoint.is_some() {
        return Ok(StartMode::Provided);
    }

    if from_genesis {
        return Ok(StartMode::Genesis);
    }

    let rpc_api_url = latest_rpc_url
        .or(args.client_args.ingestion.rpc_api_url.as_ref())
        .context("default start requires --latest-rpc-url or --rpc-api-url to fetch latest")?;

    let latest = fetch_latest(
        rpc_api_url,
        args.client_args.ingestion.rpc_username.as_deref(),
        args.client_args.ingestion.rpc_password.as_deref(),
    )
    .await?;

    args.indexer_args.first_checkpoint = Some(latest);
    Ok(StartMode::Latest(latest))
}

pub async fn run() -> Result<()> {
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
    println!("[boot] args parsed");

    let main_args = cli.cluster;
    let packages_args = clone_args(&main_args);

    let latest_rpc_url = cli.latest_rpc_url.as_ref();
    let from_genesis = cli.from_genesis;

    tokio::try_join!(
        run_main_indexer(database_url.clone(), main_args, from_genesis, latest_rpc_url),
        run_packages_only_with_args(database_url, packages_args),
    )?;
    Ok(())
}

pub async fn run_packages_only() -> Result<()> {
    println!("[boot] package indexer start");
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in the environment")
        .parse::<Url>()
        .expect("Invalid database URL");

    let args = Args::parse();
    run_packages_only_with_args(database_url, args).await
}

fn clone_args(args: &Args) -> Args {
    Args {
        indexer_args: args.indexer_args.clone(),
        client_args: args.client_args.clone(),
        metrics_args: args.metrics_args.clone(),
    }
}

async fn run_main_indexer(
    database_url: Url,
    mut args: Args,
    from_genesis: bool,
    latest_rpc_url: Option<&Url>,
) -> Result<()> {
    match resolve_start_checkpoint(&mut args, from_genesis, latest_rpc_url).await? {
        StartMode::Provided => println!("[boot] using provided first_checkpoint"),
        StartMode::Genesis => println!("[boot] from-genesis enabled: starting at checkpoint 0"),
        StartMode::Latest(latest) => println!("[boot] start-latest: using checkpoint {latest}"),
    }

    println!("[boot] building cluster");
    let mut cluster = IndexerCluster::builder()
        .with_args(args)
        .with_database_url(database_url)
        .with_migrations(&MIGRATIONS)
        .build()
        .await?;
    println!("[boot] cluster built");

    println!("[boot] registering TransactionDigestHandler pipeline");
    cluster
        .sequential_pipeline(
            TransactionDigestHandler,
            SequentialConfig::default(),
        )
        .await?;
    println!("[boot] TransactionDigestHandler pipeline registered");

    println!("[boot] registering ObjectDataHandler pipeline");
    cluster
        .sequential_pipeline(
            ObjectDataHandler,
            SequentialConfig::default(),
        )
        .await?;
    println!("[boot] ObjectDataHandler pipeline registered");

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

async fn run_packages_only_with_args(database_url: Url, mut args: Args) -> Result<()> {
    println!("[boot] package indexer args parsed (forced first_checkpoint=0)");
    args.indexer_args.first_checkpoint = Some(0);
    args.metrics_args.metrics_address = "0.0.0.0:9185"
        .parse()
        .expect("invalid metrics address");

    let registry = Registry::new();
    let metrics = MetricsService::new(args.metrics_args.clone(), registry);
    let mut indexer = Indexer::new_from_pg(
        database_url,
        DbArgs::default(),
        args.indexer_args,
        args.client_args,
        IngestionConfig::default(),
        Some(&MIGRATIONS),
        None,
        metrics.registry(),
    )
    .await?;

    indexer
        .sequential_pipeline(PackageDataHandler, SequentialConfig::default())
        .await?;

    let mut service = indexer.run().await?;
    service = service.merge(metrics.run().await?);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> Args {
        Args::default()
    }

    #[tokio::test]
    async fn uses_provided_first_checkpoint() {
        let mut args = default_args();
        args.indexer_args.first_checkpoint = Some(7);

        let mode = resolve_start_checkpoint_with(&mut args, false, None, |_, _, _| {
            Box::pin(async {
                panic!("fetch_latest should not be called when first_checkpoint is set");
            })
        })
        .await
        .unwrap();

        assert_eq!(mode, StartMode::Provided);
        assert_eq!(args.indexer_args.first_checkpoint, Some(7));
    }

    #[tokio::test]
    async fn honors_from_genesis_flag() {
        let mut args = default_args();

        let mode = resolve_start_checkpoint_with(&mut args, true, None, |_, _, _| {
            Box::pin(async {
                panic!("fetch_latest should not be called when from_genesis is true");
            })
        })
        .await
        .unwrap();

        assert_eq!(mode, StartMode::Genesis);
        assert_eq!(args.indexer_args.first_checkpoint, None);
    }

    #[tokio::test]
    async fn errors_without_rpc_url_when_defaulting_to_latest() {
        let mut args = default_args();

        let err = resolve_start_checkpoint_with(&mut args, false, None, |_, _, _| {
            Box::pin(async { Ok(42) })
        })
        .await
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("default start requires --latest-rpc-url or --rpc-api-url"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn sets_latest_checkpoint_when_available() {
        let mut args = default_args();
        args.client_args.ingestion.rpc_api_url =
            Some(Url::parse("https://fullnode.testnet.sui.io:443").unwrap());

        let mode = resolve_start_checkpoint_with(&mut args, false, None, |_, _, _| {
            Box::pin(async { Ok(42) })
        })
        .await
        .unwrap();

        assert_eq!(mode, StartMode::Latest(42));
        assert_eq!(args.indexer_args.first_checkpoint, Some(42));
    }
}
