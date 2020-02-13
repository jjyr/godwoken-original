use crate::tests::{
    main::Error,
    utils::{
        aggregator::Aggregator,
        contract_state::ContractState,
        shortcut::{default_context, prepare_accounts, prepare_ag_account, sign_block},
    },
    MAX_CYCLES,
};
use ckb_contract_tool::{ckb_error::assert_error_eq, ckb_script::ScriptError};
use godwoken_types::packed::Tx;
use godwoken_types::prelude::*;

fn new_transfer_tx(from: u32, to: u32, amount: u32) -> Tx {
    Tx::new_builder()
        .account_index(from.pack())
        .fee(3u32.pack())
        .nonce(1u32.pack())
        .args({
            let mut args = vec![0u8; 8];
            args[..4].copy_from_slice(&to.to_le_bytes());
            args[4..].copy_from_slice(&amount.to_le_bytes());
            args.pack()
        })
        .build()
}

#[test]
fn test_submit_block() {
    let mut contract_state = ContractState::new();
    // prepare contract acccounts
    let account_indexes = prepare_accounts(&mut contract_state, vec![20, 100]);
    // prepare aggregator account
    let (ag_index, privkey) = prepare_ag_account(&mut contract_state);
    let mut aggregator = Aggregator::new(contract_state);
    // txs
    let transfer_tx = new_transfer_tx(account_indexes[0], account_indexes[1], 15);
    aggregator.push_tx(transfer_tx);
    // generate block and sign
    let mut submit_context = aggregator.gen_submit_block(ag_index);
    let ag_sig = sign_block(&privkey, &submit_context.block);
    submit_context.complete_sig(ag_sig);
    // run
    let mut context = default_context();
    let tx = aggregator
        .complete_submit_block(submit_context)
        .inject_and_build(&mut context)
        .expect("tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    verify_result.expect("pass verification");
}

#[test]
fn test_submit_with_non_ag_account() {
    let mut contract_state = ContractState::new();
    // prepare contract acccounts
    let account_indexes = prepare_accounts(&mut contract_state, vec![20, 100]);
    // prepare aggregator account
    let (_ag_index, privkey) = prepare_ag_account(&mut contract_state);
    let mut aggregator = Aggregator::new(contract_state);
    // txs
    let transfer_tx = new_transfer_tx(account_indexes[0], account_indexes[1], 15);
    aggregator.push_tx(transfer_tx);
    // generate block and sign
    let mut submit_context = aggregator.gen_submit_block(account_indexes[0]);
    let ag_sig = sign_block(&privkey, &submit_context.block);
    submit_context.complete_sig(ag_sig);
    // run
    let mut context = default_context();
    let tx = aggregator
        .complete_submit_block(submit_context)
        .inject_and_build(&mut context)
        .expect("tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    assert_error_eq!(
        verify_result.unwrap_err(),
        ScriptError::ValidationFailure(Error::InvalidAggregator as i8)
    );
}

#[test]
fn test_with_non_sufficient_balance() {
    let mut contract_state = ContractState::new();
    // prepare contract acccounts
    let account_indexes = prepare_accounts(&mut contract_state, vec![20, 100]);
    // prepare aggregator account
    let (ag_index, privkey) = prepare_ag_account(&mut contract_state);
    // decrease balance of aggregator account
    {
        let ag_account = contract_state
            .get_account(ag_index)
            .expect("get aggregator")
            .to_owned();
        let least_balance: u64 = ag_account.balance().unpack();
        let ag_account = ag_account
            .as_builder()
            .balance((least_balance - 1).pack())
            .build();
        contract_state.push_account(ag_account);
    }
    let mut aggregator = Aggregator::new(contract_state);
    // txs
    let transfer_tx = new_transfer_tx(account_indexes[0], account_indexes[1], 15);
    aggregator.push_tx(transfer_tx);
    // generate block and sign
    let mut submit_context = aggregator.gen_submit_block(ag_index);
    let ag_sig = sign_block(&privkey, &submit_context.block);
    submit_context.complete_sig(ag_sig);
    // run
    let mut context = default_context();
    let tx = aggregator
        .complete_submit_block(submit_context)
        .inject_and_build(&mut context)
        .expect("tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    assert_error_eq!(
        verify_result.unwrap_err(),
        ScriptError::ValidationFailure(Error::InvalidAggregator as i8)
    );
}

#[test]
fn test_with_wrong_ag_sig() {
    let mut contract_state = ContractState::new();
    // prepare contract acccounts
    let account_indexes = prepare_accounts(&mut contract_state, vec![20, 100]);
    // prepare aggregator account
    let (ag_index, _privkey) = prepare_ag_account(&mut contract_state);
    let mut aggregator = Aggregator::new(contract_state);
    // txs
    let transfer_tx = new_transfer_tx(account_indexes[0], account_indexes[1], 15);
    aggregator.push_tx(transfer_tx);
    // generate block and sign
    let mut submit_context = aggregator.gen_submit_block(ag_index);
    let ag_sig = [0u8; 65];
    submit_context.complete_sig(ag_sig);
    // run
    let mut context = default_context();
    let tx = aggregator
        .complete_submit_block(submit_context)
        .inject_and_build(&mut context)
        .expect("tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    assert_error_eq!(
        verify_result.unwrap_err(),
        ScriptError::ValidationFailure(Error::InvalidSignature as i8)
    );
}
