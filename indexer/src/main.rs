use simple_sui_indexer::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}
