//! EVM storage slot math.

use alloy::primitives::{Address, B256, U256, keccak256};

/// Storage slot of an entry in a Solidity `mapping(address => _)`.
///
/// Solidity stores `m[key]` at `keccak256(abi.encode(key, slot))`, where `slot`
/// is the declaration slot of the mapping. ERC-20 `balanceOf` is the canonical
/// case: a balance never lives at a raw slot, it is derived from the holder
/// address and the balances mapping slot.
pub fn mapping_slot_address_key(key: Address, mapping_slot: U256) -> B256 {
    // abi.encode left-pads the 20-byte address to a 32-byte word, then appends
    // the slot as a 32-byte big-endian word.
    let mut buf = [0u8; 64];
    buf[12..32].copy_from_slice(key.as_slice());
    buf[32..64].copy_from_slice(&mapping_slot.to_be_bytes::<32>());
    keccak256(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::dyn_abi::DynSolValue;
    use alloy::primitives::address;

    #[test]
    fn mapping_slot_matches_independent_abi_encoding() {
        let key = address!("d8da6bf26964af9d7eed9e03e53415d37aa96045");
        let slot = U256::from(3);

        // Cross-check against alloy's own ABI encoder via an independent path:
        // abi.encode(address, uint256) is the concatenation of both words.
        let mut encoded = DynSolValue::Address(key).abi_encode();
        encoded.extend(DynSolValue::Uint(slot, 256).abi_encode());
        let expected = keccak256(&encoded);

        assert_eq!(mapping_slot_address_key(key, slot), expected);
    }

    #[test]
    fn mapping_slot_depends_on_key_and_slot() {
        let a = address!("1111111111111111111111111111111111111111");
        let b = address!("2222222222222222222222222222222222222222");
        assert_ne!(
            mapping_slot_address_key(a, U256::ZERO),
            mapping_slot_address_key(b, U256::ZERO)
        );
        assert_ne!(
            mapping_slot_address_key(a, U256::ZERO),
            mapping_slot_address_key(a, U256::from(1))
        );
    }
}
