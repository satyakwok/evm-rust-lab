use eyre::{ContextCompat, Result};
use revm::database::InMemoryDB;
use revm::primitives::{Address, TxKind, U256};
use revm::state::AccountInfo;
use revm::{Context, ExecuteEvm, MainBuilder, MainContext};

fn main() -> Result<()> {
    let sender = Address::from([0x11; 20]);
    let receiver = Address::from([0x22; 20]);

    let sender_start = U256::from(1_000_000_000_000_000_000u128);
    let transfer_value = U256::from(250_000_000_000_000_000u128);

    let mut db = InMemoryDB::default();
    db.insert_account_info(
        sender,
        AccountInfo {
            balance: sender_start,
            ..Default::default()
        },
    );
    db.insert_account_info(receiver, AccountInfo::default());

    let ctx = Context::mainnet().with_db(db);
    let mut evm = ctx.build_mainnet();

    let result_and_state = evm.transact(
        revm::context::TxEnv::builder()
            .caller(sender)
            .to(receiver)
            .value(transfer_value)
            .gas_limit(21_000)
            .gas_price(0)
            .kind(TxKind::Call(receiver))
            .build_fill(),
    )?;

    let sender_after = result_and_state
        .state
        .get(&sender)
        .map(|account| account.info.balance)
        .wrap_err("sender account was not present in the resulting state")?;
    let receiver_after = result_and_state
        .state
        .get(&receiver)
        .map(|account| account.info.balance)
        .wrap_err("receiver account was not present in the resulting state")?;

    println!("execution result: {:?}", result_and_state.result);
    println!("gas used: {}", result_and_state.result.tx_gas_used());
    println!("sender balance after execution: {sender_after}");
    println!("receiver balance after execution: {receiver_after}");

    Ok(())
}
