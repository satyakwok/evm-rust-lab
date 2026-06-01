# evm-rust-lab

Practical Rust EVM examples using Alloy, revm, and real RPC workflows.

[![CI](https://github.com/satyakwok/evm-rust-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/satyakwok/evm-rust-lab/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)
![Status](https://img.shields.io/badge/status-experimental-yellow)

`evm-rust-lab` is a practical Rust EVM infrastructure lab for developers who want to learn by reading and running small, working examples. It uses Alloy for RPC, ABI, providers, and primitives, and revm for local EVM simulation.

This repository is not a framework.
It is not a toy blockchain.
It is a collection of small, focused, working examples for developers who want to understand EVM infrastructure from the Rust side.

## What This Repo Is

- A collection of practical Rust EVM examples.
- A learning repo for Alloy providers, RPC reads, ABI decoding, raw storage inspection, and revm simulation.
- A compile-correct reference for developers who prefer direct examples over large abstractions.

## What This Repo Is Not

- It is not a blockchain framework.
- It is not a toy chain.
- It is not a node, indexer, wallet, or production RPC service.
- It is not tied to any specific EVM network or vendor.

## Who It Is For

- Rust developers learning Ethereum and EVM infrastructure.
- Protocol and infrastructure engineers who want minimal examples they can adapt.
- Developers moving from JavaScript EVM tooling to Rust.
- Anyone learning how RPC, ABI encoding, storage, and local EVM simulation fit together.

## Current Examples

| Example | What it demonstrates |
| --- | --- |
| `rpc_fetch_block` | Connect to an RPC endpoint and fetch chain/block data. |
| `rpc_chain_health` | Measure basic RPC latency and print a health report. |
| `abi_decode_erc20_transfer` | Decode ERC-20 `transfer(address,uint256)` calldata. |
| `storage_read_slot` | Read a raw EVM storage slot from an RPC endpoint. |
| `revm_simple_transfer` | Simulate a simple in-memory value transfer with revm. |

## Quick Start

```sh
git clone https://github.com/satyakwok/evm-rust-lab
cd evm-rust-lab
cp .env.example .env
```

The default `.env.example` uses:

```sh
EVM_RPC_URL=https://ethereum.publicnode.com
```

You can replace it with any EVM-compatible RPC endpoint, for example:

```sh
EVM_RPC_URL=https://mainnet.base.org
EVM_RPC_URL=https://arb1.arbitrum.io/rpc
```

## Run Examples

Fetch latest block data:

```sh
cargo run --example rpc_fetch_block
```

Check basic RPC health:

```sh
cargo run --example rpc_chain_health
```

Decode ERC-20 transfer calldata:

```sh
cargo run --example abi_decode_erc20_transfer
```

Read a raw storage slot:

```sh
cargo run --example storage_read_slot
```

Run a local revm transfer simulation:

```sh
cargo run --example revm_simple_transfer
```

## Example Output

Example `rpc_chain_health` output:

```text
RPC URL: https://ethereum.publicnode.com
latency: 832 ms
chain id: 1
latest block: 25224808
latest block hash: 0x75c76693c08c8e6ffa71e22043d4cdb043d80843c7bef32d0e69d8464d9af67a
status: OK
```

Block numbers, hashes, and latency will vary.

## Requirements

- Stable Rust
- An EVM-compatible RPC endpoint

## Philosophy

The examples should be small, direct, and useful. They should compile, use real crates from crates.io, and avoid fake APIs or pseudo-code. When an EVM topic has sharp edges, the code should expose the important concept without turning the example into a framework.

## License

Licensed under either of:

- MIT, see `LICENSE-MIT`
- Apache-2.0, see `LICENSE-APACHE`
