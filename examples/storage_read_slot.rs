use alloy::primitives::{U256, address};
use alloy::providers::{Provider, ProviderBuilder};
use dotenvy::dotenv;
use eyre::{Context, Result};

const CONTRACT_ADDRESS: alloy::primitives::Address =
    address!("0000000000000000000000000000000000000000");
const STORAGE_SLOT: U256 = U256::ZERO;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc_url = std::env::var("EVM_RPC_URL")
        .wrap_err("EVM_RPC_URL is not set. Copy .env.example to .env first")?;

    let provider = ProviderBuilder::new().connect(&rpc_url).await?;
    let raw_value = provider
        .get_storage_at(CONTRACT_ADDRESS, STORAGE_SLOT)
        .await?;

    // ERC-20 balances are usually stored in a mapping. Reading a balance requires
    // keccak256(abi.encode(address, mapping_slot)), not a direct read of one raw slot.
    println!("contract address: {CONTRACT_ADDRESS}");
    println!("slot: {STORAGE_SLOT}");
    println!("raw value: {raw_value}");

    Ok(())
}
