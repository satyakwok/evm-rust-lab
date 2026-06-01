# evm-rust-lab

Practical Rust EVM examples using Alloy, revm, and real RPC workflows.

[![CI](https://github.com/satyakwok/evm-rust-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/satyakwok/evm-rust-lab/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)
![Status](https://img.shields.io/badge/status-experimental-yellow)

`evm-rust-lab` is a hands-on Rust EVM infrastructure lab for developers who want to learn by running small, focused examples instead of starting from a large framework.

## What You Can Do With It

- Check whether an EVM RPC endpoint is reachable and responding.
- Fetch chain ID, latest block number, latest block hash, and basic latency.
- Decode ERC-20 `transfer(address,uint256)` calldata.
- Read raw EVM storage slots.
- Run a minimal local EVM transfer simulation with revm.

## Quick Start

```sh
git clone https://github.com/satyakwok/evm-rust-lab
cd evm-rust-lab
cp .env.example .env
EVM_RPC_URL=https://ethereum.publicnode.com cargo run --example rpc_chain_health
```

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
cargo run --example revm_simple_transfer
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
- Add JSON output mode.
- Add optional CLI mode.
- Add WebSocket health checks.
- Add better examples for logs and event decoding.

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
