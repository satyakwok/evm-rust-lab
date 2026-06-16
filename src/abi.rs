//! ABI selectors, calldata building, and decoding.

use alloy::primitives::{Address, U256, keccak256};
use alloy::sol;
use alloy::sol_types::SolCall;
use eyre::{Result, eyre};

sol! {
    function transfer(address to, uint256 amount);
}

/// 4-byte selector for a Solidity function signature such as
/// `"transfer(address,uint256)"`.
pub fn function_selector(signature: &str) -> [u8; 4] {
    let hash = keccak256(signature.as_bytes());
    [hash[0], hash[1], hash[2], hash[3]]
}

/// ABI-encoded calldata for ERC-20 `transfer(address,uint256)`: the 4-byte
/// selector followed by the recipient and amount, each as a 32-byte word.
pub fn erc20_transfer_calldata(to: Address, amount: U256) -> Vec<u8> {
    let mut data = Vec::with_capacity(68);
    data.extend_from_slice(&function_selector("transfer(address,uint256)"));
    let mut recipient = [0u8; 32];
    recipient[12..32].copy_from_slice(to.as_slice());
    data.extend_from_slice(&recipient);
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    data
}

/// Decoded ERC-20 `transfer(address,uint256)` call.
pub struct Erc20Transfer {
    pub to: Address,
    pub amount: U256,
}

/// Decode ERC-20 `transfer(address,uint256)` calldata (selector plus arguments).
pub fn decode_erc20_transfer(calldata: &[u8]) -> Result<Erc20Transfer> {
    let decoded = transferCall::abi_decode(calldata)
        .map_err(|e| eyre!("not valid transfer(address,uint256) calldata: {e}"))?;
    Ok(Erc20Transfer {
        to: decoded.to,
        amount: decoded.amount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, hex};

    #[test]
    fn transfer_selector_is_a9059cbb() {
        assert_eq!(
            function_selector("transfer(address,uint256)"),
            [0xa9, 0x05, 0x9c, 0xbb]
        );
    }

    #[test]
    fn transfer_calldata_matches_known_encoding() {
        // 1 ETH (1e18) to 0x1111...1111, the vector decoded in the ABI example.
        let expected = hex::decode(
            "a9059cbb\
             0000000000000000000000001111111111111111111111111111111111111111\
             0000000000000000000000000000000000000000000000000de0b6b3a7640000",
        )
        .unwrap();
        let calldata = erc20_transfer_calldata(
            address!("1111111111111111111111111111111111111111"),
            U256::from(1_000_000_000_000_000_000u64),
        );
        assert_eq!(calldata, expected);
    }

    #[test]
    fn decode_round_trips_with_encode() {
        let to = address!("1111111111111111111111111111111111111111");
        let amount = U256::from(1_000_000_000_000_000_000u64);
        let calldata = erc20_transfer_calldata(to, amount);

        let decoded = decode_erc20_transfer(&calldata).unwrap();
        assert_eq!(decoded.to, to);
        assert_eq!(decoded.amount, amount);
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode_erc20_transfer(&[0x00, 0x01, 0x02]).is_err());
    }
}
