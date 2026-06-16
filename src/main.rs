use std::time::Duration;

use alloy::eips::BlockNumberOrTag;
use alloy::hex;
use alloy::primitives::{Address, U256};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use eyre::{Context, Result, eyre};

use evm_rust_lab::abi::{decode_erc20_transfer, function_selector};
use evm_rust_lab::input::{RpcEndpoint, parse_calldata};
use evm_rust_lab::report::{self, BlockDto, TipDto};
use evm_rust_lab::rpc::{check_rpc, fetch_block, read_erc20_balance, read_erc20_balances};
use evm_rust_lab::watch::watch;

/// Small, focused EVM infrastructure commands backed by Alloy and reliakit.
#[derive(Parser)]
#[command(name = "evm-lab", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Probe an RPC endpoint and report a criticality-aware health verdict.
    Health {
        #[arg(long, env = "EVM_RPC_URL")]
        rpc_url: String,
        /// Emit the chain tip as deterministic JSON.
        #[arg(long)]
        json: bool,
        /// Also print a reproducible canonical fingerprint of the tip.
        #[arg(long)]
        fingerprint: bool,
    },
    /// Fetch a block's number, hash, timestamp, and transaction count.
    Block {
        #[arg(long, env = "EVM_RPC_URL")]
        rpc_url: String,
        /// Block number, or "latest".
        #[arg(long, default_value = "latest")]
        number: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        fingerprint: bool,
    },
    /// Derive an ERC-20 balance storage slot and read it over JSON-RPC.
    Balance {
        #[arg(long, env = "EVM_RPC_URL")]
        rpc_url: String,
        /// Token contract address.
        token: Address,
        /// Holder address.
        holder: Address,
        /// Declaration slot of the token's balances mapping.
        #[arg(long, default_value_t = 0)]
        slot: u64,
    },
    /// Read balances for many holders with bounded concurrency.
    Balances {
        #[arg(long, env = "EVM_RPC_URL")]
        rpc_url: String,
        /// Token contract address.
        #[arg(long)]
        token: Address,
        /// One or more holder addresses.
        #[arg(required = true)]
        holders: Vec<Address>,
        #[arg(long, default_value_t = 0)]
        slot: u64,
        /// Maximum requests in flight at once.
        #[arg(long, default_value_t = 4)]
        max_in_flight: usize,
        /// Emit the result as RFC 4180 CSV.
        #[arg(long)]
        csv: bool,
    },
    /// Poll the chain tip under rate-limit and circuit-breaker control.
    Watch {
        #[arg(long, env = "EVM_RPC_URL")]
        rpc_url: String,
        /// Milliseconds between probes.
        #[arg(long, default_value_t = 3_000)]
        interval_ms: u64,
        /// Stop after this many probes (0 runs until interrupted).
        #[arg(long, default_value_t = 0)]
        ticks: u64,
    },
    /// Compute the 4-byte selector of a function signature (offline).
    Selector {
        /// For example: "transfer(address,uint256)".
        signature: String,
    },
    /// Decode ERC-20 transfer(address,uint256) calldata (offline).
    DecodeTransfer {
        /// Hex calldata, with or without a leading 0x.
        calldata: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    match Cli::parse().command {
        Command::Health {
            rpc_url,
            json,
            fingerprint,
        } => {
            let endpoint = RpcEndpoint::parse(&rpc_url)?;
            let status = check_rpc(endpoint.expose()).await;
            let health = report::rpc_health(&status);

            if json {
                // A single JSON object on stdout, nothing else.
                println!("{}", report::to_json(&report::HealthDto::new(&status, &health)));
            } else {
                match &status {
                    Ok(status) => {
                        println!("chain id: {}", status.chain_id);
                        println!("latest block: {}", status.latest_block_number);
                        println!("latest block hash: {}", status.latest_block_hash);
                        println!("latency: {} ms", status.latency.as_millis());
                        println!("tls: {}", endpoint.is_https());
                        if fingerprint {
                            println!("fingerprint: {}", report::fingerprint(&TipDto::from_status(status))?);
                        }
                    }
                    Err(err) => eprintln!("probe failed: {}", endpoint.scrub(err.to_string())),
                }
                println!("overall: {}", health.overall().as_str());
                for (name, detail) in health.reasons() {
                    println!("  - {name}: {}", detail.unwrap_or("(no detail)"));
                }
            }

            if !health.is_operational() {
                std::process::exit(1);
            }
        }
        Command::Block {
            rpc_url,
            number,
            json,
            fingerprint,
        } => {
            let endpoint = RpcEndpoint::parse(&rpc_url)?;
            let block = fetch_block(endpoint.expose(), parse_block_tag(&number)?)
                .await
                .map_err(|e| eyre!("{}", endpoint.scrub(e.to_string())))?;
            let dto = BlockDto::from_block(&block);
            if json {
                println!("{}", report::to_json(&dto));
            } else {
                println!("number: {}", block.number);
                println!("hash: {}", block.hash);
                println!("timestamp: {}", block.timestamp);
                println!("tx count: {}", block.tx_count);
                if fingerprint {
                    println!("fingerprint: {}", report::fingerprint(&dto)?);
                }
            }
        }
        Command::Balance {
            rpc_url,
            token,
            holder,
            slot,
        } => {
            let endpoint = RpcEndpoint::parse(&rpc_url)?;
            let raw = read_erc20_balance(endpoint.expose(), token, holder, U256::from(slot))
                .await
                .map_err(|e| eyre!("{}", endpoint.scrub(e.to_string())))?;
            println!("token: {token}");
            println!("holder: {holder}");
            println!("balances slot: {slot}");
            println!("raw balance (token base units): {raw}");
        }
        Command::Balances {
            rpc_url,
            token,
            holders,
            slot,
            max_in_flight,
            csv,
        } => {
            let endpoint = RpcEndpoint::parse(&rpc_url)?;
            let rows =
                read_erc20_balances(endpoint.expose(), token, &holders, U256::from(slot), max_in_flight)
                    .await
                    .map_err(|e| eyre!("{}", endpoint.scrub(e.to_string())))?;
            if csv {
                print!("{}", report::balances_csv(&rows));
            } else {
                for (holder, balance) in &rows {
                    println!("{holder}  {balance}");
                }
            }
        }
        Command::Watch {
            rpc_url,
            interval_ms,
            ticks,
        } => {
            let endpoint = RpcEndpoint::parse(&rpc_url)?;
            watch(endpoint.expose(), Duration::from_millis(interval_ms), ticks).await?;
        }
        Command::Selector { signature } => {
            println!("0x{}", hex::encode(function_selector(&signature)));
        }
        Command::DecodeTransfer { calldata } => {
            let transfer = decode_erc20_transfer(&parse_calldata(&calldata)?)?;
            println!("recipient: {}", transfer.to);
            println!("amount raw uint256: {}", transfer.amount);
        }
    }

    Ok(())
}

fn parse_block_tag(input: &str) -> Result<BlockNumberOrTag> {
    if input.eq_ignore_ascii_case("latest") {
        return Ok(BlockNumberOrTag::Latest);
    }
    let number = input
        .parse()
        .wrap_err("block must be a number or \"latest\"")?;
    Ok(BlockNumberOrTag::Number(number))
}
