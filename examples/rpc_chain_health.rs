use alloy::eips::BlockNumberOrTag;
use alloy::providers::{Provider, ProviderBuilder};
use dotenvy::dotenv;
use eyre::{Context, ContextCompat, Result};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc_url = std::env::var("EVM_RPC_URL")
        .wrap_err("EVM_RPC_URL is not set. Copy .env.example to .env first")?;

    let started = Instant::now();
    let health = check_rpc(&rpc_url).await;
    let latency = started.elapsed();

    println!("RPC URL: {rpc_url}");
    println!("latency: {} ms", latency.as_millis());

    match health {
        Ok(report) => {
            println!("chain id: {}", report.chain_id);
            println!("latest block: {}", report.latest_block_number);
            println!("latest block hash: {}", report.latest_block_hash);
            println!("status: OK");
            Ok(())
        }
        Err(err) => {
            println!("status: FAILED");
            Err(err)
        }
    }
}

struct HealthReport {
    chain_id: u64,
    latest_block_number: u64,
    latest_block_hash: alloy::primitives::B256,
}

async fn check_rpc(rpc_url: &str) -> Result<HealthReport> {
    let provider = ProviderBuilder::new().connect(rpc_url).await?;

    let chain_id = provider.get_chain_id().await?;
    let latest_block_number = provider.get_block_number().await?;
    let latest_block = provider
        .get_block_by_number(BlockNumberOrTag::Latest)
        .await?
        .wrap_err("latest block was not returned by the RPC")?;

    Ok(HealthReport {
        chain_id,
        latest_block_number,
        latest_block_hash: latest_block.hash(),
    })
}
