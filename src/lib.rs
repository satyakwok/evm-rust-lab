//! Practical Rust EVM building blocks using Alloy, revm, and real RPC workflows.
//!
//! The pure helpers ([`abi`], [`storage`]) are unit tested without a network
//! connection. [`rpc`] holds the async workflows — wrapped in the reliakit
//! resilience stack (retry, backoff, timeout, bulkhead) — shared by the
//! `evm-lab` binary and the example programs. [`input`] validates and redacts
//! CLI inputs, [`report`] serializes output, and [`watch`] polls the chain tip
//! under rate-limit and circuit-breaker control.

pub mod abi;
pub mod check;
pub mod color;
pub mod input;
pub mod raw;
pub mod report;
pub mod rpc;
pub mod storage;
pub mod watch;

pub use abi::{Erc20Transfer, decode_erc20_transfer, erc20_transfer_calldata, function_selector};
pub use storage::mapping_slot_address_key;
