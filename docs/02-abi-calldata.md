# ABI Calldata

EVM contract calls are sent as calldata. Calldata is raw bytes, but the Ethereum ABI defines how to encode function calls and arguments so contracts can understand them.

## Function Selector

For a normal contract function call, the first 4 bytes are the function selector.

The selector is:

```text
first_4_bytes(keccak256("transfer(address,uint256)"))
```

For ERC-20 `transfer(address,uint256)`, the selector is:

```text
0xa9059cbb
```

## ABI-Encoded Arguments

After the selector, the remaining bytes are ABI-encoded arguments. Static types such as `address` and `uint256` are encoded into 32-byte words.

For:

```solidity
transfer(address to, uint256 amount)
```

the calldata layout is:

```text
4 bytes   function selector
32 bytes  recipient address, left-padded
32 bytes  raw uint256 amount
```

The amount is raw token units. ERC-20 decimals are display metadata; they are not automatically applied by the EVM.

See `examples/abi_decode_erc20_transfer.rs` for a small Alloy `sol!` decoding example.
