# evm-rust-lab

Practical Rust EVM examples using Alloy, revm, and real RPC workflows.

[![CI](https://github.com/satyakwok/evm-rust-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/satyakwok/evm-rust-lab/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)
![Status](https://img.shields.io/badge/status-experimental-yellow)

`evm-rust-lab` is a hands-on Rust EVM infrastructure lab. It ships a small library, a set of runnable examples, and an `evm-lab` CLI — with every RPC call wrapped in the [reliakit](https://crates.io/crates/reliakit) resilience stack (retry, backoff, timeout, circuit breaker, rate limiter, bulkhead).

## What You Can Do With It

- Check whether an EVM RPC endpoint is reachable and report a criticality-aware health verdict.
- Fetch chain ID, latest block number, latest block hash, and basic latency.
- Decode ERC-20 `transfer(address,uint256)` calldata and compute function selectors.
- Derive ERC-20 balance storage slots and read them over JSON-RPC, single or batched.
- Watch the chain tip under rate-limit and circuit-breaker control.
- Run a minimal local EVM transfer simulation with revm.

## Quick Start

```sh
git clone https://github.com/satyakwok/evm-rust-lab
cd evm-rust-lab
cp .env.example .env
export EVM_RPC_URL=https://ethereum.publicnode.com
cargo run --bin evm-lab -- health
```

## evm-lab CLI

`EVM_RPC_URL` is read from the environment or `--rpc-url`; it is validated as an
HTTP(S) URL and held as a redacted secret (RPC URLs often embed an API key).

```sh
evm-lab health --json --fingerprint
evm-lab block --number latest
evm-lab balance 0xA0b8...eB48 0xd8dA...6045 --slot 9
evm-lab balances --token 0xA0b8...eB48 --slot 9 --csv 0xd8dA...6045 0x28C6...1d60
evm-lab watch --interval-ms 3000 --ticks 5
evm-lab selector "transfer(address,uint256)"
evm-lab decode-transfer 0xa9059cbb...
```

| Command | What it does |
| --- | --- |
| `health` | Probes an endpoint; criticality-aware verdict (reachability critical, latency degrade-only). |
| `block` | Fetches a block's number, hash, timestamp, and tx count. |
| `balance` | Derives one ERC-20 balance storage slot and reads it. |
| `balances` | Reads many holders with bounded concurrency; text or CSV. |
| `watch` | Polls the chain tip under a token-bucket rate limiter and circuit breaker. |
| `selector` | Computes a 4-byte function selector (offline). |
| `decode-transfer` | Decodes ERC-20 `transfer` calldata (offline). |

`--json` (health, block) emits deterministic JSON; `--fingerprint` adds a
reproducible canonical encoding of the snapshot; `--csv` (balances) emits RFC 4180.

## Built on reliakit

The RPC and CLI layers dogfood the [reliakit](https://crates.io/crates/reliakit)
reliability crates:

| Crate | Where it is used |
| --- | --- |
| `reliakit-retry` / `reliakit-backoff` | Every RPC call: bounded attempts, exponential backoff. |
| `reliakit-timeout` / `reliakit-core` | Per-attempt deadline against a total time budget. |
| `reliakit-bulkhead` | `balances` batch concurrency cap. |
| `reliakit-ratelimit` / `reliakit-circuit` | `watch` poll-rate limiting and fast-fail. |
| `reliakit-collections` | Rolling latency window in `watch`. |
| `reliakit-decide` | Latency-scored verdict in `watch`. |
| `reliakit-health` | Criticality-aware `health` aggregation. |
| `reliakit-secret` / `reliakit-primitives` / `reliakit-validate` | RPC URL and hex input validation and redaction. |
| `reliakit-json` / `reliakit-csv` / `reliakit-codec` / `reliakit-derive` | `--json`, `--csv`, and `--fingerprint` output. |

## Example Output

```text
RPC URL: https://ethereum.publicnode.com
latency: 103 ms
chain id: 1
latest block: ...
latest block hash: 0x...
status: OK
```

Block numbers, hashes, and latency will vary by network and RPC endpoint.

## Current Examples

| Example | What it demonstrates |
| --- | --- |
| `rpc_chain_health` | Checks whether an EVM-compatible RPC endpoint is reachable and responding. |
| `rpc_fetch_block` | Fetches chain ID, latest block number, latest block hash, and timestamp. |
| `abi_decode_erc20_transfer` | Decodes ERC-20 `transfer(address,uint256)` calldata. |
| `storage_read_slot` | Reads a raw EVM storage slot through JSON-RPC. |
| `erc20_balance_of` | Derives an ERC-20 balance storage slot and reads it over JSON-RPC. |
| `revm_simple_transfer` | Runs a minimal local value transfer simulation with revm. |

## Run Examples

```sh
cargo run --example rpc_chain_health
```

```sh
cargo run --example rpc_fetch_block
```

```sh
cargo run --example abi_decode_erc20_transfer
```

```sh
cargo run --example storage_read_slot
```

```sh
cargo run --example erc20_balance_of
```

```sh
cargo run --example revm_simple_transfer
```

## Library Helpers

The pure building blocks the examples rely on live in `src/lib.rs` and are unit
tested without a network connection:

- `mapping_slot_address_key` derives the storage slot of a `mapping(address => _)` entry (for example ERC-20 `balanceOf`).
- `function_selector` computes a 4-byte selector from a function signature.
- `erc20_transfer_calldata` builds ABI-encoded `transfer(address,uint256)` calldata.

```sh
cargo test
```

## Why This Exists

Most EVM learning material starts at smart contracts or JavaScript tooling. This repo focuses on the lower-level Rust side: RPC calls, ABI decoding, storage reads, and execution simulation.

This repository is not a framework.
It is not a toy blockchain.
It is a collection of small, focused, working examples for developers who want to understand EVM infrastructure from the Rust side.

## What This Repo Is

- A practical Rust EVM infrastructure lab.
- A collection of compile-correct examples.
- A reference for developers learning Alloy, revm, and EVM RPC workflows.
- A foundation for future EVM diagnostics tooling.

## What This Repo Is Not

- It is not a blockchain framework.
- It is not a toy chain.
- It is not a wallet, explorer, indexer, or production RPC service.
- It is not tied to any single EVM network.

## Who It Is For

- Rust developers entering Ethereum/EVM infrastructure.
- Web3 developers moving from JavaScript tooling to Rust.
- Protocol and infrastructure engineers who want small runnable references.
- Builders working on EVM-compatible chains, RPC tooling, explorers, or execution systems.

## Requirements

- Rust stable
- Cargo
- An EVM-compatible RPC endpoint

## Configuration

The RPC examples read `EVM_RPC_URL` from `.env`:

```sh
EVM_RPC_URL=https://ethereum.publicnode.com
```

Any EVM-compatible RPC endpoint can be used:

```sh
EVM_RPC_URL=https://mainnet.base.org cargo run --example rpc_chain_health
```

```sh
EVM_RPC_URL=https://arb1.arbitrum.io/rpc cargo run --example rpc_chain_health
```

## Roadmap

- Add block movement checks.
- Add RPC method compatibility checks.
- Add WebSocket health checks.
- Add better examples for logs and event decoding.

Done: JSON output mode, `evm-lab` CLI, batched balance reads, chain-tip watch.

## Design Philosophy

- Small examples over large abstractions.
- Real RPC calls over mocked behavior.
- Compile-correct code over pseudo-code.
- Practical infrastructure workflows over theory.
- Neutral examples that work with any EVM-compatible chain.

## License

Licensed under either of:

- MIT, see `LICENSE-MIT`
- Apache-2.0, see `LICENSE-APACHE`
