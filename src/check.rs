//! RPC readiness checks: a suite of category-grouped probes against an EVM
//! endpoint, each classified Pass / Warn / Fail and aggregated (via
//! [`reliakit_health`]) into an overall verdict. Output serializes through
//! [`reliakit_json`]; the probes use Alloy's own typed RPC methods.

use std::fmt::Display;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use alloy::eips::{BlockId, BlockNumberOrTag};
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{Filter, SyncStatus};
use eyre::Result;
use reliakit_derive::JsonEncode;
use reliakit_health::{Criticality, Health, HealthReport};
use reliakit_json::to_json_string;

/// A group of related checks. Core and Head are critical; Capability and Archive
/// can only degrade the verdict (an endpoint without archive state is still
/// usable for live work).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Core,
    Head,
    Capability,
    Archive,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Core => "Core",
            Category::Head => "Head",
            Category::Capability => "Capability",
            Category::Archive => "Archive",
        }
    }

    pub fn criticality(self) -> Criticality {
        match self {
            Category::Core | Category::Head => Criticality::Critical,
            Category::Capability | Category::Archive => Criticality::Optional,
        }
    }
}

/// The outcome of a single check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Warn,
    Fail,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Pass => "PASS",
            Outcome::Warn => "WARN",
            Outcome::Fail => "FAIL",
        }
    }

    pub fn health(self) -> Health {
        match self {
            Outcome::Pass => Health::Healthy,
            Outcome::Warn => Health::Degraded,
            Outcome::Fail => Health::Unhealthy,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Outcome::Pass => 0,
            Outcome::Warn => 1,
            Outcome::Fail => 2,
        }
    }
}

/// One executed check.
pub struct Check {
    pub category: Category,
    pub method: &'static str,
    pub outcome: Outcome,
    pub latency: Duration,
    pub detail: String,
}

impl Check {
    fn new(
        category: Category,
        method: &'static str,
        latency: Duration,
        outcome: Outcome,
        detail: impl Into<String>,
    ) -> Self {
        Check {
            category,
            method,
            outcome,
            latency,
            detail: detail.into(),
        }
    }
}

/// The full readiness result.
pub struct CheckReport {
    pub checks: Vec<Check>,
    pub chain_id: Option<u64>,
    pub client: Option<String>,
    pub head_lag_secs: Option<u64>,
}

impl CheckReport {
    /// Overall verdict: the worst effective status across checks, with each
    /// category's criticality applied.
    pub fn overall(&self) -> Health {
        let mut report = HealthReport::new();
        for check in &self.checks {
            report.push(
                check.method,
                check.outcome.health(),
                check.category.criticality(),
            );
        }
        report.overall()
    }

    pub fn passed(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.outcome == Outcome::Pass)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.outcome == Outcome::Fail)
            .count()
    }

    /// Mean check latency in milliseconds.
    pub fn avg_latency_ms(&self) -> u128 {
        if self.checks.is_empty() {
            return 0;
        }
        let total: u128 = self.checks.iter().map(|c| c.latency.as_millis()).sum();
        total / self.checks.len() as u128
    }

    /// The worst outcome within a category (for the per-category summary).
    pub fn category_outcome(&self, category: Category) -> Outcome {
        self.checks
            .iter()
            .filter(|c| c.category == category)
            .map(|c| c.outcome)
            .max_by_key(|o| o.rank())
            .unwrap_or(Outcome::Pass)
    }

    pub fn category_counts(&self, category: Category) -> (usize, usize) {
        let in_cat: Vec<_> = self
            .checks
            .iter()
            .filter(|c| c.category == category)
            .collect();
        let passed = in_cat.iter().filter(|c| c.outcome == Outcome::Pass).count();
        (passed, in_cat.len())
    }

    /// The overall verdict as a plain word: `OK`, `DEGRADED`, or `DOWN`.
    pub fn verdict_word(&self) -> &'static str {
        match self.overall() {
            Health::Healthy => "OK",
            Health::Degraded => "DEGRADED",
            Health::Unhealthy => "DOWN",
        }
    }

    /// Operational unless the verdict is `DOWN`.
    pub fn is_operational(&self) -> bool {
        self.overall() != Health::Unhealthy
    }
}

/// One check as a serialization-friendly object.
#[derive(JsonEncode)]
struct CheckDto {
    category: String,
    method: String,
    status: String,
    latency_ms: u64,
    detail: String,
}

/// The readiness result as a single JSON object.
#[derive(JsonEncode)]
struct ReadinessDto {
    overall: String,
    passed: u64,
    failed: u64,
    chain_id: Option<u64>,
    client: Option<String>,
    head_lag_secs: Option<u64>,
    checks: Vec<CheckDto>,
}

/// Render the report as deterministic JSON via [`reliakit_json`].
pub fn to_json(report: &CheckReport) -> String {
    let checks = report
        .checks
        .iter()
        .map(|c| CheckDto {
            category: c.category.label().to_owned(),
            method: c.method.to_owned(),
            status: c.outcome.label().to_owned(),
            latency_ms: c.latency.as_millis() as u64,
            detail: c.detail.clone(),
        })
        .collect();
    let dto = ReadinessDto {
        overall: report.verdict_word().to_owned(),
        passed: report.passed() as u64,
        failed: report.failed() as u64,
        chain_id: report.chain_id,
        client: report.client.clone(),
        head_lag_secs: report.head_lag_secs,
        checks,
    };
    to_json_string(&dto)
}

/// Time an async RPC call, returning the elapsed duration alongside its result.
macro_rules! timed {
    ($call:expr) => {{
        let started = Instant::now();
        let result = $call.await;
        (started.elapsed(), result)
    }};
}

fn ok(
    category: Category,
    method: &'static str,
    latency: Duration,
    detail: impl Into<String>,
) -> Check {
    Check::new(category, method, latency, Outcome::Pass, detail)
}

fn failed(category: Category, method: &'static str, latency: Duration, err: impl Display) -> Check {
    Check::new(category, method, latency, Outcome::Fail, err.to_string())
}

/// Run every readiness check against `rpc_url`.
pub async fn run_checks(rpc_url: &str) -> Result<CheckReport> {
    let provider = ProviderBuilder::new().connect(rpc_url).await?;
    let mut checks = Vec::new();
    let mut chain_id = None;
    let mut client = None;
    let mut head_lag_secs = None;

    // Core
    let (latency, result) = timed!(provider.get_chain_id());
    checks.push(match result {
        Ok(id) => {
            chain_id = Some(id);
            ok(Category::Core, "eth_chainId", latency, id.to_string())
        }
        Err(e) => failed(Category::Core, "eth_chainId", latency, e),
    });

    let (latency, result) =
        timed!(provider.raw_request::<(), String>("web3_clientVersion".into(), ()));
    checks.push(match result {
        Ok(version) => {
            client = Some(version.clone());
            ok(Category::Core, "web3_clientVersion", latency, version)
        }
        Err(e) => failed(Category::Core, "web3_clientVersion", latency, e),
    });

    let (latency, result) = timed!(provider.get_block_number());
    checks.push(match result {
        Ok(number) => ok(
            Category::Core,
            "eth_blockNumber",
            latency,
            number.to_string(),
        ),
        Err(e) => failed(Category::Core, "eth_blockNumber", latency, e),
    });

    // Head freshness
    let (latency, result) = timed!(provider.get_block_by_number(BlockNumberOrTag::Latest));
    checks.push(match result {
        Ok(Some(block)) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(block.header.timestamp);
            let lag = now.saturating_sub(block.header.timestamp);
            head_lag_secs = Some(lag);
            let (outcome, detail) = match lag {
                s if s <= 30 => (Outcome::Pass, format!("{s}s behind")),
                s if s <= 600 => (Outcome::Warn, format!("{s}s behind")),
                s => (Outcome::Fail, format!("{s}s behind")),
            };
            Check::new(
                Category::Head,
                "eth_getBlockByNumber",
                latency,
                outcome,
                detail,
            )
        }
        Ok(None) => Check::new(
            Category::Head,
            "eth_getBlockByNumber",
            latency,
            Outcome::Fail,
            "no latest block",
        ),
        Err(e) => failed(Category::Head, "eth_getBlockByNumber", latency, e),
    });

    // Capability
    let (latency, result) = timed!(provider.get_gas_price());
    checks.push(match result {
        Ok(wei) => ok(
            Category::Capability,
            "eth_gasPrice",
            latency,
            format!("{} gwei", wei / 1_000_000_000),
        ),
        Err(e) => failed(Category::Capability, "eth_gasPrice", latency, e),
    });

    let (latency, result) = timed!(provider.get_max_priority_fee_per_gas());
    checks.push(match result {
        Ok(wei) => ok(
            Category::Capability,
            "eth_maxPriorityFeePerGas",
            latency,
            format!("{} gwei", wei / 1_000_000_000),
        ),
        Err(e) => failed(Category::Capability, "eth_maxPriorityFeePerGas", latency, e),
    });

    let (latency, result) = timed!(provider.syncing());
    checks.push(match result {
        Ok(SyncStatus::None) => ok(Category::Capability, "eth_syncing", latency, "synced"),
        Ok(SyncStatus::Info(_)) => Check::new(
            Category::Capability,
            "eth_syncing",
            latency,
            Outcome::Warn,
            "node is syncing",
        ),
        Err(e) => failed(Category::Capability, "eth_syncing", latency, e),
    });

    let filter = Filter::new()
        .from_block(BlockNumberOrTag::Latest)
        .to_block(BlockNumberOrTag::Latest);
    let (latency, result) = timed!(provider.get_logs(&filter));
    checks.push(match result {
        Ok(_) => ok(
            Category::Capability,
            "eth_getLogs",
            latency,
            "log queries supported",
        ),
        // An unsupported or limited getLogs only degrades.
        Err(e) => Check::new(
            Category::Capability,
            "eth_getLogs",
            latency,
            Outcome::Warn,
            e.to_string(),
        ),
    });

    // Archive: historical state at an early block.
    let (latency, result) = timed!(
        provider
            .get_balance(Address::ZERO)
            .block_id(BlockId::number(1))
    );
    checks.push(match result {
        Ok(_) => ok(
            Category::Archive,
            "eth_getBalance",
            latency,
            "historical state available",
        ),
        Err(_) => Check::new(
            Category::Archive,
            "eth_getBalance",
            latency,
            Outcome::Warn,
            "no historical state (not an archive node)",
        ),
    });

    // Transport errors can echo the endpoint URL; keep it out of the report.
    for check in &mut checks {
        check.detail = check.detail.replace(rpc_url, "<redacted-rpc-url>");
    }

    Ok(CheckReport {
        checks,
        chain_id,
        client,
        head_lag_secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(category: Category, outcome: Outcome) -> Check {
        Check::new(category, "m", Duration::ZERO, outcome, "")
    }

    #[test]
    fn overall_degrades_when_optional_fails_but_core_passes() {
        let report = CheckReport {
            checks: vec![
                check(Category::Core, Outcome::Pass),
                check(Category::Archive, Outcome::Warn),
            ],
            chain_id: Some(1),
            client: None,
            head_lag_secs: Some(2),
        };
        assert_eq!(report.overall(), Health::Degraded);
        assert!(report.is_operational());
        assert_eq!(report.passed(), 1);
    }

    #[test]
    fn overall_down_when_core_fails() {
        let report = CheckReport {
            checks: vec![check(Category::Core, Outcome::Fail)],
            chain_id: None,
            client: None,
            head_lag_secs: None,
        };
        assert_eq!(report.verdict_word(), "DOWN");
        assert!(!report.is_operational());
    }

    #[test]
    fn category_outcome_is_the_worst() {
        let report = CheckReport {
            checks: vec![
                check(Category::Capability, Outcome::Pass),
                check(Category::Capability, Outcome::Warn),
            ],
            chain_id: None,
            client: None,
            head_lag_secs: None,
        };
        assert_eq!(report.category_outcome(Category::Capability), Outcome::Warn);
        assert_eq!(report.category_counts(Category::Capability), (1, 2));
    }
}
