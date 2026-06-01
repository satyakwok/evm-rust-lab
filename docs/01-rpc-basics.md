# RPC Basics

EVM infrastructure usually starts with JSON-RPC. A Rust application sends RPC requests to an execution node or RPC provider, and the provider returns chain data such as chain ID, block number, blocks, logs, receipts, balances, and storage.

## Chain ID

The chain ID identifies the network. Ethereum mainnet is `1`, Sepolia is `11155111`, and EVM-compatible chains use their own values.

Fetching the chain ID is a simple sanity check:

- It confirms that the RPC endpoint is reachable.
- It confirms that the endpoint is connected to the network you expect.
- It prevents accidental reads or writes against the wrong chain.

## Latest Block

The latest block number tells you where the RPC endpoint thinks the chain tip is. The latest block itself includes useful metadata such as hash, timestamp, gas data, and transaction references.

For infrastructure work, latest block data is often used to:

- check whether an RPC is syncing,
- compare multiple RPC endpoints,
- anchor state reads at a known block,
- detect stale infrastructure.

## Latency

Latency is the time between sending a request and receiving a response. A low-latency RPC matters for wallets, indexers, trading systems, monitoring, and any service that needs fresh chain data.

Latency alone is not enough to prove an RPC is healthy. A fast RPC can still be stale or return errors. A practical health check usually combines:

- basic request latency,
- chain ID,
- latest block number,
- latest block hash,
- error status.

See `examples/rpc_fetch_block.rs` and `examples/rpc_chain_health.rs` for small Alloy-based examples.
