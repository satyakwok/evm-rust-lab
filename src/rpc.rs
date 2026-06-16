//! Async RPC workflows used by the CLI and examples.
//!
//! Public RPC endpoints flake, so every call here is wrapped in a resilience
//! stack: a bounded [`reliakit_retry`] policy with [`reliakit_backoff`], a
//! per-attempt deadline from [`reliakit_timeout`], and (for batches) a
//! [`reliakit_bulkhead`] concurrency cap.

use std::future::Future;
use std::time::{Duration, Instant};

use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{Address, B256, U256};
use alloy::providers::{Provider, ProviderBuilder};
use eyre::{ContextCompat, Result, eyre};
use reliakit_bulkhead::Bulkhead;
use reliakit_retry::{Backoff, RetryPolicy, retry_async};
use reliakit_timeout::{Deadline, Timeout};
use tokio::task::JoinSet;

use crate::storage::mapping_slot_address_key;

/// Total time budget for one logical RPC operation, across all retries.
const TOTAL_BUDGET_MS: u64 = 12_000;
/// Ceiling for any single attempt.
const ATTEMPT_BUDGET_MS: u64 = 5_000;

/// Up to three attempts, exponential backoff from 200ms capped at 2s.
fn rpc_retry_policy() -> RetryPolicy {
    let backoff =
        Backoff::exponential(Duration::from_millis(200), 2).with_max_delay(Duration::from_secs(2));
    RetryPolicy::new(3, backoff).expect("max_attempts is non-zero")
}

/// Remaining budget for the next attempt, or an error once the total deadline
/// has passed.
fn attempt_budget(started: Instant, deadline: Deadline) -> Result<Duration> {
    let now = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let remaining = deadline.remaining(now).min(ATTEMPT_BUDGET_MS);
    if remaining == 0 {
        return Err(eyre!("RPC time budget of {TOTAL_BUDGET_MS} ms exhausted"));
    }
    Ok(Duration::from_millis(remaining))
}

/// Run a fallible async RPC operation under the shared retry policy, awaiting a
/// Tokio timer between attempts and flattening the retry error to the last cause.
async fn with_retry<T, Op, Fut>(op: Op) -> Result<T>
where
    Op: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let policy = rpc_retry_policy();
    retry_async(&policy, op, |_| true, |delay| tokio::time::sleep(delay))
        .await
        .map_err(|err| err.into_last_error())
}

/// Reachability and chain-tip snapshot of an RPC endpoint.
pub struct RpcStatus {
    pub chain_id: u64,
    pub latest_block_number: u64,
    pub latest_block_hash: B256,
    pub latency: Duration,
}

/// Connect to `rpc_url` and read chain id and the latest block, timing the round trip.
pub async fn check_rpc(rpc_url: &str) -> Result<RpcStatus> {
    let started = Instant::now();
    let deadline = Timeout::new(TOTAL_BUDGET_MS).start(0);

    let (chain_id, latest_block_number, latest_block_hash) = with_retry(move || async move {
        let budget = attempt_budget(started, deadline)?;
        let work = async move {
            let provider = ProviderBuilder::new().connect(rpc_url).await?;
            let chain_id = provider.get_chain_id().await?;
            let latest_block_number = provider.get_block_number().await?;
            let latest_block = provider
                .get_block_by_number(BlockNumberOrTag::Latest)
                .await?
                .wrap_err("latest block was not returned by the RPC")?;
            Ok::<_, eyre::Report>((chain_id, latest_block_number, latest_block.hash()))
        };
        tokio::time::timeout(budget, work)
            .await
            .map_err(|_| eyre!("RPC attempt exceeded its time budget"))?
    })
    .await?;

    Ok(RpcStatus {
        chain_id,
        latest_block_number,
        latest_block_hash,
        latency: started.elapsed(),
    })
}

/// A single block's identifying fields.
pub struct BlockInfo {
    pub number: u64,
    pub hash: B256,
    pub timestamp: u64,
    pub tx_count: usize,
}

/// Fetch a block by number or tag (for example [`BlockNumberOrTag::Latest`]).
pub async fn fetch_block(rpc_url: &str, tag: BlockNumberOrTag) -> Result<BlockInfo> {
    let started = Instant::now();
    let deadline = Timeout::new(TOTAL_BUDGET_MS).start(0);

    with_retry(move || async move {
        let budget = attempt_budget(started, deadline)?;
        let work = async move {
            let provider = ProviderBuilder::new().connect(rpc_url).await?;
            let block = provider
                .get_block_by_number(tag)
                .await?
                .wrap_err("block was not returned by the RPC")?;
            Ok::<_, eyre::Report>(BlockInfo {
                number: block.header.number,
                hash: block.hash(),
                timestamp: block.header.timestamp,
                tx_count: block.transactions.len(),
            })
        };
        tokio::time::timeout(budget, work)
            .await
            .map_err(|_| eyre!("RPC attempt exceeded its time budget"))?
    })
    .await
}

/// Read an ERC-20 balance by deriving its storage slot from `balances_slot` (the
/// declaration slot of the token's balances mapping) and reading it directly,
/// without an `eth_call`.
pub async fn read_erc20_balance(
    rpc_url: &str,
    token: Address,
    holder: Address,
    balances_slot: U256,
) -> Result<U256> {
    let slot = mapping_slot_address_key(holder, balances_slot);
    let started = Instant::now();
    let deadline = Timeout::new(TOTAL_BUDGET_MS).start(0);

    with_retry(move || async move {
        let budget = attempt_budget(started, deadline)?;
        let work = async move {
            let provider = ProviderBuilder::new().connect(rpc_url).await?;
            let raw = provider.get_storage_at(token, slot.into()).await?;
            Ok::<_, eyre::Report>(raw)
        };
        tokio::time::timeout(budget, work)
            .await
            .map_err(|_| eyre!("RPC attempt exceeded its time budget"))?
    })
    .await
}

/// Read balances for many holders with at most `max_in_flight` requests in
/// flight at once, preserving input order. The cap is enforced by a
/// [`Bulkhead`] permit acquired before each spawn and released on completion.
pub async fn read_erc20_balances(
    rpc_url: &str,
    token: Address,
    holders: &[Address],
    balances_slot: U256,
    max_in_flight: usize,
) -> Result<Vec<(Address, U256)>> {
    let mut bulkhead = Bulkhead::new(max_in_flight.max(1));
    let mut tasks: JoinSet<Result<(usize, U256)>> = JoinSet::new();
    let mut slots: Vec<Option<U256>> = vec![None; holders.len()];

    for (index, &holder) in holders.iter().enumerate() {
        while !bulkhead.try_acquire_one() {
            collect_one(&mut tasks, &mut slots, &mut bulkhead).await?;
        }
        let url = rpc_url.to_owned();
        tasks.spawn(async move {
            let balance = read_erc20_balance(&url, token, holder, balances_slot).await?;
            Ok((index, balance))
        });
    }
    while !tasks.is_empty() {
        collect_one(&mut tasks, &mut slots, &mut bulkhead).await?;
    }

    Ok(holders
        .iter()
        .zip(slots)
        .map(|(&holder, balance)| (holder, balance.expect("every holder produced a balance")))
        .collect())
}

async fn collect_one(
    tasks: &mut JoinSet<Result<(usize, U256)>>,
    slots: &mut [Option<U256>],
    bulkhead: &mut Bulkhead,
) -> Result<()> {
    if let Some(joined) = tasks.join_next().await {
        let (index, balance) = joined.map_err(|e| eyre!("balance task failed: {e}"))??;
        slots[index] = Some(balance);
        bulkhead.release_one();
    }
    Ok(())
}
