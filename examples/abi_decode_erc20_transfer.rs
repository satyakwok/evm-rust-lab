use alloy::hex;
use alloy::sol;
use alloy::sol_types::SolCall;
use eyre::Result;

sol! {
    function transfer(address to, uint256 amount);
}

fn main() -> Result<()> {
    // First 4 bytes are the function selector for transfer(address,uint256).
    // Remaining bytes are ABI-encoded arguments: address recipient, then uint256 amount.
    let calldata = hex::decode(
        "a9059cbb\
         0000000000000000000000001111111111111111111111111111111111111111\
         0000000000000000000000000000000000000000000000000de0b6b3a7640000",
    )?;

    let decoded = transferCall::abi_decode(&calldata)?;

    println!("function: transfer");
    println!("recipient: {}", decoded.to);
    println!("amount raw uint256: {}", decoded.amount);

    Ok(())
}
