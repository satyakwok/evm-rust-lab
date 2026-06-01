use alloy::eips::BlockNumberOrTag;
use alloy::providers::{Provider, ProviderBuilder};
use dotenvy::dotenv;
use eyre::{Context, ContextCompat, Result};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc_url = std::env::var("EVM_RPC_URL")
        .wrap_err("EVM_RPC_URL is not set. Copy .env.example to .env first")?;

    let provider = ProviderBuilder::new().connect(&rpc_url).await?;

    let chain_id = provider.get_chain_id().await?;
    let latest_block_number = provider.get_block_number().await?;
    let latest_block = provider
        .get_block_by_number(BlockNumberOrTag::Latest)
        .await?
        .wrap_err("latest block was not returned by the RPC")?;

    println!("chain id: {chain_id}");
    println!("latest block number: {latest_block_number}");
    println!("latest block hash: {}", latest_block.hash());
    println!("timestamp: {}", latest_block.header.timestamp);

    Ok(())
}
