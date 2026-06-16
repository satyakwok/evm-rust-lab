//! Output serialization and health classification.
//!
//! Read commands can emit either human text or deterministic JSON
//! ([`reliakit_json`]). The [`TipDto`] also derives a canonical binary encoding
//! ([`reliakit_codec`]) so a chain tip can be fingerprinted reproducibly.

use std::time::Duration;

use eyre::{Result, eyre};
use reliakit_codec::encode_to_vec;
use reliakit_csv::CsvWriter;
use reliakit_derive::{CanonicalEncode, JsonEncode};
use reliakit_health::{Criticality, Health, HealthReport};
use reliakit_json::to_json_string;

use crate::rpc::{BlockInfo, RpcStatus};

/// A chain-tip identity in a serialization-friendly shape (all fields
/// stringified so the JSON and canonical encodings stay stable across
/// platforms). Latency is deliberately excluded so the fingerprint of a given
/// tip is reproducible.
#[derive(JsonEncode, CanonicalEncode)]
pub struct TipDto {
    pub chain_id: String,
    pub latest_block: String,
    pub latest_block_hash: String,
}

impl TipDto {
    pub fn from_status(status: &RpcStatus) -> Self {
        TipDto {
            chain_id: status.chain_id.to_string(),
            latest_block: status.latest_block_number.to_string(),
            latest_block_hash: status.latest_block_hash.to_string(),
        }
    }
}

/// A health probe result as a single JSON object (verdict plus the tip, with
/// empty tip fields when the endpoint was unreachable).
#[derive(JsonEncode)]
pub struct HealthDto {
    pub overall: String,
    pub chain_id: String,
    pub latest_block: String,
    pub latest_block_hash: String,
    pub latency_ms: String,
}

impl HealthDto {
    pub fn new(status: &Result<RpcStatus>, health: &HealthReport) -> Self {
        let overall = health.overall().as_str().to_owned();
        match status {
            Ok(status) => HealthDto {
                overall,
                chain_id: status.chain_id.to_string(),
                latest_block: status.latest_block_number.to_string(),
                latest_block_hash: status.latest_block_hash.to_string(),
                latency_ms: status.latency.as_millis().to_string(),
            },
            Err(_) => HealthDto {
                overall,
                chain_id: String::new(),
                latest_block: String::new(),
                latest_block_hash: String::new(),
                latency_ms: String::new(),
            },
        }
    }
}

/// A block in serialization-friendly shape.
#[derive(JsonEncode, CanonicalEncode)]
pub struct BlockDto {
    pub number: String,
    pub hash: String,
    pub timestamp: String,
    pub tx_count: String,
}

impl BlockDto {
    pub fn from_block(block: &BlockInfo) -> Self {
        BlockDto {
            number: block.number.to_string(),
            hash: block.hash.to_string(),
            timestamp: block.timestamp.to_string(),
            tx_count: block.tx_count.to_string(),
        }
    }
}

/// Deterministic JSON text for any serializable DTO.
pub fn to_json<T: reliakit_json::JsonEncode>(value: &T) -> String {
    to_json_string(value)
}

/// A reproducible content fingerprint: the canonical binary encoding, hex-encoded.
pub fn fingerprint<T: reliakit_codec::CanonicalEncode>(value: &T) -> Result<String> {
    let bytes = encode_to_vec(value).map_err(|e| eyre!("canonical encode failed: {e:?}"))?;
    Ok(format!("0x{}", alloy::hex::encode(bytes)))
}

/// Classify round-trip latency into a health status: fast is healthy, slow is
/// degraded, very slow is unhealthy.
pub fn latency_health(latency: Duration) -> Health {
    let ms = latency.as_millis();
    if ms < 500 {
        Health::Healthy
    } else if ms < 2_000 {
        Health::Degraded
    } else {
        Health::Unhealthy
    }
}

/// Build a health report for an RPC probe: reachability is critical, latency can
/// only degrade.
pub fn rpc_health(status: &Result<RpcStatus>) -> HealthReport {
    match status {
        Ok(status) => HealthReport::new()
            .critical("rpc-reachability", Health::Healthy)
            .with(
                "rpc-latency",
                latency_health(status.latency),
                Criticality::Optional,
            )
            .detail(format!("{} ms", status.latency.as_millis())),
        // The raw error may embed the (secret) RPC URL, so keep it out of the
        // report; the scrubbed cause is printed separately by the caller.
        Err(_) => HealthReport::new()
            .critical("rpc-reachability", Health::Unhealthy)
            .detail("endpoint unreachable"),
    }
}

/// Render a holder/balance table as deterministic CSV (RFC 4180).
pub fn balances_csv(rows: &[(alloy::primitives::Address, alloy::primitives::U256)]) -> String {
    let mut writer = CsvWriter::new();
    writer.write_record(["holder", "raw_balance"]);
    for (holder, balance) in rows {
        writer.write_record([holder.to_string(), balance.to_string()]);
    }
    writer.into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_block() -> BlockDto {
        BlockDto {
            number: "10".to_owned(),
            hash: "0xabc".to_owned(),
            timestamp: "123".to_owned(),
            tx_count: "2".to_owned(),
        }
    }

    #[test]
    fn latency_health_thresholds() {
        assert_eq!(latency_health(Duration::from_millis(100)), Health::Healthy);
        assert_eq!(latency_health(Duration::from_millis(800)), Health::Degraded);
        assert_eq!(
            latency_health(Duration::from_millis(3_000)),
            Health::Unhealthy
        );
    }

    #[test]
    fn block_fingerprint_is_deterministic() {
        assert_eq!(
            fingerprint(&sample_block()).unwrap(),
            fingerprint(&sample_block()).unwrap()
        );
    }

    #[test]
    fn tip_json_has_expected_keys() {
        let dto = TipDto {
            chain_id: "1".to_owned(),
            latest_block: "100".to_owned(),
            latest_block_hash: "0xff".to_owned(),
        };
        let json = to_json(&dto);
        assert!(json.contains("\"chain_id\":\"1\""), "{json}");
        assert!(json.contains("\"latest_block\":\"100\""), "{json}");
    }
}
