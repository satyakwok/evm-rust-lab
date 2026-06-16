//! A polling chain-tip watcher that exercises the time-driven resilience crates.
//!
//! Each tick is gated by a [`reliakit_ratelimit`] token bucket and a
//! [`reliakit_circuit`] breaker (both driven by a [`reliakit_core::MonotonicClock`]);
//! recent latencies are kept in a [`reliakit_collections::RingBuffer`] and a
//! [`reliakit_decide`] reasoner turns the latest latency into a verdict.

use std::time::Duration;

use eyre::Result;
use reliakit_circuit::CircuitBreaker;
use reliakit_collections::RingBuffer;
use reliakit_core::{Clock, MonotonicClock};
use reliakit_decide::{Action, Curve, Reasoner, Score};
use reliakit_ratelimit::RateLimiter;

use crate::rpc::check_rpc;

/// Poll the chain tip every `interval`. With `ticks == 0` it runs until
/// interrupted; otherwise it stops after `ticks` successful gate passes.
pub async fn watch(rpc_url: &str, interval: Duration, ticks: u64) -> Result<()> {
    let clock = MonotonicClock::new();
    let interval_ms = u64::try_from(interval.as_millis())
        .unwrap_or(u64::MAX)
        .max(1);

    // One probe per interval.
    let mut limiter = RateLimiter::new(1, 1, interval_ms);
    // Open after three consecutive failures; probe again after five intervals.
    let mut breaker = CircuitBreaker::new(3, interval_ms.saturating_mul(5));
    let mut latencies: RingBuffer<u64> = RingBuffer::new(16).expect("capacity is non-zero");

    let mut done = 0u64;
    while ticks == 0 || done < ticks {
        if !breaker.allow(clock.now()) {
            println!("circuit OPEN, skipping probe");
            tokio::time::sleep(interval).await;
            continue;
        }
        if !limiter.try_acquire_one(clock.now()) {
            let wait = limiter.retry_after(clock.now(), 1).unwrap_or(interval_ms);
            tokio::time::sleep(Duration::from_millis(wait)).await;
            continue;
        }

        match check_rpc(rpc_url).await {
            Ok(status) => {
                breaker.on_success();
                let ms = u64::try_from(status.latency.as_millis()).unwrap_or(u64::MAX);
                latencies.push(ms);
                println!(
                    "block {} | {ms} ms | avg {} ms | {}",
                    status.latest_block_number,
                    average(&latencies),
                    verdict(ms),
                );
            }
            Err(err) => {
                breaker.on_failure(clock.now());
                // The endpoint may be embedded in transport errors; keep it out of logs.
                println!(
                    "probe failed: {}",
                    err.to_string().replace(rpc_url, "<redacted-rpc-url>")
                );
            }
        }

        done += 1;
        if ticks == 0 || done < ticks {
            tokio::time::sleep(interval).await;
        }
    }
    Ok(())
}

fn average(buffer: &RingBuffer<u64>) -> u64 {
    if buffer.is_empty() {
        return 0;
    }
    let sum: u128 = buffer.iter().map(|&v| u128::from(v)).sum();
    u64::try_from(sum / buffer.len() as u128).unwrap_or(u64::MAX)
}

/// Score the latency (0ms..=2000ms mapped to 0.0..=1.0) and let the reasoner pick
/// between a steady state and an alert.
fn verdict(latency_ms: u64) -> &'static str {
    let signal = Score::from_ratio(u32::try_from(latency_ms.min(2_000)).unwrap_or(2_000), 2_000);

    let mut reasoner = Reasoner::new();
    reasoner.add(
        Action::new("steady")
            .with_base(Score::from_ratio(1, 2))
            .consider(Curve::Inverse, signal),
    );
    reasoner.add(Action::new("alert").consider(Curve::Linear, signal));

    reasoner.decide().map_or("steady", |decision| decision.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_low_latency_is_steady() {
        assert_eq!(verdict(50), "steady");
    }

    #[test]
    fn verdict_high_latency_is_alert() {
        assert_eq!(verdict(1_900), "alert");
    }
}
