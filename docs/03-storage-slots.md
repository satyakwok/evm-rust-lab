# Storage Slots

Every EVM contract has persistent storage addressed by 32-byte slots. Low-level RPC calls such as `eth_getStorageAt` read the raw value at a slot.

This is useful for infrastructure work because storage reads can inspect contract state without calling contract functions. It is also easy to misuse because Solidity's high-level layout rules are more complex than a simple slot number.

## Raw Slots

Simple fixed-size state variables are often stored directly in slots, depending on Solidity packing rules.

For example, a contract might store one `uint256` at slot `0`. Reading slot `0` would return the raw 32-byte value.

## Mappings Require Hashing

Mappings do not store values directly at the mapping slot. Solidity computes the storage location using a hash.

For a mapping like:

```solidity
mapping(address => uint256) balances;
```

the balance for an address is stored at:

```text
keccak256(abi.encode(address, mapping_slot))
```

That means reading an ERC-20 balance is not the same as reading slot `0`. You need the correct mapping slot and the hashed key location.

## Low-Level Warning

Raw slot reading is a low-level tool. It depends on compiler layout, inheritance, packing, proxy patterns, and contract upgrades. Prefer contract calls for normal application logic, and use storage reads when you specifically need low-level infrastructure visibility.

See `examples/storage_read_slot.rs` for a direct raw slot read using Alloy.
