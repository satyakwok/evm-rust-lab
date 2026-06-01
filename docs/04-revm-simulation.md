# revm Simulation

`revm` is a Rust EVM implementation. It can execute EVM transactions against a database of account state, code, balances, nonces, and storage.

Infrastructure engineers use revm for local execution workflows such as:

- transaction simulation,
- gas estimation experiments,
- state transition testing,
- forked-state execution,
- MEV and mempool analysis,
- contract behavior inspection.

## Transaction Simulation

At a high level, a simulation needs:

- a database containing the starting state,
- block environment values,
- transaction environment values,
- an EVM configured for the desired hardfork rules.

The EVM executes the transaction and returns an execution result plus state changes. The result tells you whether execution succeeded, reverted, or halted. The state changes tell you how accounts, balances, storage, and code changed during execution.

## Simple Transfers

A plain ETH value transfer is the smallest useful simulation:

- create a sender account with balance,
- create or load a receiver account,
- build a transaction from sender to receiver,
- execute it in memory,
- inspect the resulting balances and gas used.

This is not a blockchain node. It is local EVM execution over a supplied state database.

See `examples/revm_simple_transfer.rs` for a minimal in-memory revm transfer.
