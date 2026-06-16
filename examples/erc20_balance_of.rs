use alloy::primitives::{Address, U256, address};
use alloy::providers::{Provider, ProviderBuilder};
use dotenvy::dotenv;
use evm_rust_lab::mapping_slot_address_key;
use eyre::{Context, Result};

// USDC on Ethereum mainnet. Its `balanceOf` mapping is declared at storage
// slot 9, so a holder balance lives at keccak256(abi.encode(holder, 9)).
const TOKEN: Address = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
const BALANCES_SLOT: U256 = U256::from_limbs([9, 0, 0, 0]);
const HOLDER: Address = address!("d8da6bf26964af9d7eed9e03e53415d37aa96045");

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc_url = std::env::var("EVM_RPC_URL")
        .wrap_err("EVM_RPC_URL is not set. Copy .env.example to .env first")?;

    let provider = ProviderBuilder::new().connect(&rpc_url).await?;

    let slot = mapping_slot_address_key(HOLDER, BALANCES_SLOT);
    let raw = provider.get_storage_at(TOKEN, slot.into()).await?;

    println!("token: {TOKEN}");
    println!("holder: {HOLDER}");
    println!("balances mapping slot: {BALANCES_SLOT}");
    println!("derived storage slot: {slot}");
    println!("raw balance (token base units): {raw}");

    Ok(())
}
