# evm-rust-lab

Practical Rust EVM examples for infra builders.

`evm-rust-lab` is a hands-on Rust repository for developers who want to learn EVM infrastructure by reading and running small examples. It uses Alloy for provider, RPC, ABI, and primitive types, and revm for local EVM simulation.

This repository is not a framework.
It is not a toy blockchain.
It is a collection of small, focused, working examples for developers who want to understand EVM infrastructure from the Rust side.

## What This Repo Is

- Practical Rust EVM examples using Alloy, revm, and real RPC workflows.
- A place to learn common infrastructure tasks such as reading chain state, decoding calldata, inspecting storage, and running local simulations.
- A compile-correct reference for developers who prefer small examples over large abstractions.

## What This Repo Is Not

- It is not Sentrix-specific.
- It is not a blockchain framework.
- It is not a replacement for client, node, or indexer codebases.
- It does not hide EVM concepts behind heavy abstractions.

## Who It Is For

- Rust developers learning Ethereum and EVM infrastructure.
- Protocol and infra engineers who want minimal examples they can adapt.
- Developers moving from JavaScript EVM tooling to Rust.

## Current Examples

- `rpc_fetch_block`: connect to an RPC endpoint and fetch basic block data.
- `rpc_chain_health`: measure basic RPC latency and print a health report.
- `abi_decode_erc20_transfer`: decode ERC-20 `transfer(address,uint256)` calldata.
- `storage_read_slot`: read a raw storage slot from a contract.
- `revm_simple_transfer`: simulate a simple value transfer with revm.

## Requirements

- Stable Rust
- An EVM-compatible RPC endpoint

Copy the example environment file:

```sh
cp .env.example .env
```

Then edit `EVM_RPC_URL` if you want to use another RPC endpoint.

## Run Examples

```sh
cargo run --example rpc_fetch_block
cargo run --example rpc_chain_health
cargo run --example abi_decode_erc20_transfer
cargo run --example storage_read_slot
cargo run --example revm_simple_transfer
```

## Philosophy

The examples in this repository should be small, direct, and useful. They should compile, use real crates from crates.io, and avoid fake APIs or pseudo-code. When an EVM topic has sharp edges, the code should expose the important concept without turning the example into a framework.
