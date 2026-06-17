use std::time::Duration;

use alloy::eips::BlockNumberOrTag;
use alloy::hex;
use alloy::primitives::{Address, U256};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use eyre::{Context, Result, eyre};

use evm_rust_lab::abi::{decode_erc20_transfer, function_selector};
use evm_rust_lab::check::{self, Category, CheckReport};
use evm_rust_lab::color::{self, Cell};
use evm_rust_lab::input::{RpcEndpoint, parse_calldata};
use evm_rust_lab::raw;
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
    /// Run the full RPC readiness check suite.
    Check {
        #[arg(long, env = "EVM_RPC_URL")]
        rpc_url: String,
        /// Emit the readiness report as JSON.
        #[arg(long)]
        json: bool,
        /// Show every individual check, not just per-category status.
        #[arg(long)]
        verbose: bool,
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
    /// Query chain id, head, and client via a raw JSON-RPC path built and parsed
    /// entirely with reliakit-json (no serde, no Alloy provider).
    Ping {
        #[arg(long, env = "EVM_RPC_URL")]
        rpc_url: String,
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
                println!(
                    "{}",
                    report::to_json(&report::HealthDto::new(&status, &health))
                );
            } else {
                let overall = health.overall();
                let mut out = String::new();
                out.push_str(&color::title("evm-lab · RPC health"));
                out.push_str("\n\n");
                out.push_str(&color::heading("Target"));
                out.push('\n');
                out.push_str(&color::table(
                    &[vec![
                        Cell::styled("Endpoint", color::label("Endpoint")),
                        Cell::plain(color::host(endpoint.expose())),
                    ]],
                    3,
                ));
                out.push('\n');
                out.push_str(&color::heading("Result"));
                out.push('\n');

                let mut result = vec![vec![
                    Cell::styled(
                        color::health_word_plain(overall),
                        color::health_word(overall),
                    ),
                    match &status {
                        Ok(s) => Cell::plain(format!(
                            "chain {} · block {}",
                            s.chain_id, s.latest_block_number
                        )),
                        Err(_) => Cell::plain("endpoint unreachable"),
                    },
                ]];
                match &status {
                    Ok(s) => {
                        let ms = s.latency.as_millis();
                        result.push(vec![
                            Cell::styled("Latency", color::label("Latency")),
                            Cell::styled(format!("{ms} ms"), color::latency(ms)),
                        ]);
                        result.push(vec![
                            Cell::styled("Hash", color::label("Hash")),
                            Cell::plain(s.latest_block_hash.to_string()),
                        ]);
                        result.push(vec![
                            Cell::styled("TLS", color::label("TLS")),
                            Cell::plain(if endpoint.is_https() { "yes" } else { "no" }),
                        ]);
                        if fingerprint {
                            result.push(vec![
                                Cell::styled("Fingerprint", color::label("Fingerprint")),
                                Cell::plain(report::fingerprint(&TipDto::from_status(s))?),
                            ]);
                        }
                    }
                    Err(err) => {
                        result.push(vec![
                            Cell::styled("Reason", color::label("Reason")),
                            Cell::plain(endpoint.scrub(err.to_string())),
                        ]);
                    }
                }
                out.push_str(&color::table(&result, 3));
                print!("{out}");
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
                let mut out = String::new();
                out.push_str(&color::title("evm-lab · Block"));
                out.push_str("\n\n");
                out.push_str(&color::heading("Result"));
                out.push('\n');
                let mut result = vec![
                    row("Number", block.number.to_string()),
                    row("Hash", block.hash.to_string()),
                    row("Timestamp", block.timestamp.to_string()),
                    row("Tx count", block.tx_count.to_string()),
                ];
                if fingerprint {
                    result.push(row("Fingerprint", report::fingerprint(&dto)?));
                }
                out.push_str(&color::table(&result, 3));
                print!("{out}");
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
            let mut out = String::new();
            out.push_str(&color::title("evm-lab · ERC-20 balance"));
            out.push_str("\n\n");
            out.push_str(&color::heading("Result"));
            out.push('\n');
            out.push_str(&color::table(
                &[
                    row("Token", token.to_string()),
                    row("Holder", holder.to_string()),
                    row("Slot", slot.to_string()),
                    row("Balance", raw.to_string()),
                ],
                3,
            ));
            print!("{out}");
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
            let rows = read_erc20_balances(
                endpoint.expose(),
                token,
                &holders,
                U256::from(slot),
                max_in_flight,
            )
            .await
            .map_err(|e| eyre!("{}", endpoint.scrub(e.to_string())))?;
            if csv {
                print!("{}", report::balances_csv(&rows));
            } else {
                let mut out = String::new();
                out.push_str(&color::title("evm-lab · ERC-20 balances"));
                out.push_str("\n\n");
                out.push_str(&color::heading("Holders"));
                out.push('\n');
                let mut table_rows = vec![vec![
                    Cell::styled("Holder", color::label("Holder")),
                    Cell::styled("Raw balance", color::label("Raw balance")),
                ]];
                for (holder, balance) in &rows {
                    table_rows.push(vec![
                        Cell::plain(holder.to_string()),
                        Cell::plain(balance.to_string()),
                    ]);
                }
                out.push_str(&color::table(&table_rows, 4));
                print!("{out}");
            }
        }
        Command::Check {
            rpc_url,
            json,
            verbose,
        } => {
            let endpoint = RpcEndpoint::parse(&rpc_url)?;
            let report = check::run_checks(endpoint.expose())
                .await
                .map_err(|e| eyre!("{}", endpoint.scrub(e.to_string())))?;
            if json {
                println!("{}", check::to_json(&report));
            } else {
                print!(
                    "{}",
                    render_check(&report, color::host(endpoint.expose()), verbose)
                );
            }
            if !report.is_operational() {
                std::process::exit(1);
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
        Command::Ping { rpc_url } => {
            let endpoint = RpcEndpoint::parse(&rpc_url)?;
            let client = raw::RawClient::new(endpoint.expose());

            let scrub = |e: eyre::Report| eyre!("{}", endpoint.scrub(e.to_string()));
            let chain = client.call("eth_chainId", vec![]).await.map_err(scrub)?;
            let block = client
                .call("eth_blockNumber", vec![])
                .await
                .map_err(scrub)?;
            let version = client
                .call("web3_clientVersion", vec![])
                .await
                .map_err(scrub)?;

            let mut out = String::new();
            out.push_str(&color::title("evm-lab · ping (reliakit-json)"));
            out.push_str("\n\n");
            out.push_str(&color::heading("Target"));
            out.push('\n');
            out.push_str(&color::table(
                &[row("Endpoint", color::host(endpoint.expose()))],
                3,
            ));
            out.push('\n');
            out.push_str(&color::heading("Result"));
            out.push('\n');
            out.push_str(&color::table(
                &[
                    row(
                        "Chain",
                        raw::quantity(&chain).map_or_else(|| "-".to_owned(), |n| n.to_string()),
                    ),
                    row(
                        "Block",
                        raw::quantity(&block).map_or_else(|| "-".to_owned(), |n| n.to_string()),
                    ),
                    row("Client", version.as_str().unwrap_or("-").to_owned()),
                ],
                3,
            ));
            out.push('\n');
            out.push_str(&color::label(
                "JSON-RPC request and response handled entirely by reliakit-json.",
            ));
            out.push('\n');
            print!("{out}");
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

/// A two-column row: a muted label and a plain value.
fn row(label: &str, value: String) -> Vec<Cell> {
    vec![Cell::styled(label, color::label(label)), Cell::plain(value)]
}

/// Render the readiness report as a titled, sectioned report.
fn render_check(report: &CheckReport, host: String, verbose: bool) -> String {
    let overall = report.overall();
    let mut out = String::new();
    out.push_str(&color::title("evm-lab · RPC readiness"));
    out.push_str("\n\n");

    out.push_str(&color::heading("Target"));
    out.push('\n');
    out.push_str(&color::table(&[row("Endpoint", host)], 3));
    out.push('\n');

    out.push_str(&color::heading("Result"));
    out.push('\n');
    let mut result = vec![vec![
        Cell::styled(
            color::health_word_plain(overall),
            color::health_word(overall),
        ),
        Cell::plain(format!(
            "{} passed · {} failed",
            report.passed(),
            report.failed()
        )),
    ]];
    result.push(row(
        "Latency",
        format!("avg {} ms", report.avg_latency_ms()),
    ));
    result.push(row(
        "Chain",
        report
            .chain_id
            .map_or_else(|| "-".to_owned(), |c| c.to_string()),
    ));
    result.push(row(
        "Client",
        report.client.clone().unwrap_or_else(|| "-".to_owned()),
    ));
    result.push(row(
        "Head",
        report
            .head_lag_secs
            .map_or_else(|| "-".to_owned(), |s| format!("{s}s behind")),
    ));
    out.push_str(&color::table(&result, 3));
    out.push('\n');

    out.push_str(&color::heading("Checks"));
    out.push('\n');
    let mut rows = vec![vec![
        Cell::styled("Category", color::label("Category")),
        Cell::styled("Status", color::label("Status")),
        Cell::styled("Summary", color::label("Summary")),
    ]];
    for category in [
        Category::Core,
        Category::Head,
        Category::Capability,
        Category::Archive,
    ] {
        let outcome = report.category_outcome(category);
        let (passed, total) = report.category_counts(category);
        rows.push(vec![
            Cell::plain(category.label()),
            Cell::styled(outcome.label(), color::status(outcome.label())),
            Cell::plain(format!("{passed} / {total}")),
        ]);
    }
    out.push_str(&color::table(&rows, 4));

    if verbose {
        out.push('\n');
        out.push_str(&color::heading("Detail"));
        out.push('\n');
        let detail: Vec<Vec<Cell>> = report
            .checks
            .iter()
            .map(|c| {
                vec![
                    Cell::plain(c.method),
                    Cell::styled(c.outcome.label(), color::status(c.outcome.label())),
                    Cell::plain(c.detail.clone()),
                ]
            })
            .collect();
        out.push_str(&color::table(&detail, 3));
    }

    out
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
