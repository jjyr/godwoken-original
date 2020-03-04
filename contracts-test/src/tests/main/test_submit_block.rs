use crate::tests::{
    main::Error,
    utils::{
        aggregator::Aggregator,
        constants::CKB_TOKEN_ID,
        contract_state::ContractState,
        shortcut::{
            default_context, gen_transfer_tx, prepare_accounts, prepare_ag_account, sign_block,
        },
    },
    MAX_CYCLES,
};
use ckb_contract_tool::{ckb_error::assert_error_eq, ckb_script::ScriptError};

#[test]
fn test_submit_block() {
    let mut contract_state = ContractState::new();
    // prepare contract acccounts
    let account_indexes = prepare_accounts(&mut contract_state, vec![20, 100]);
    // prepare aggregator account
    let (ag_index, privkey) = prepare_ag_account(&mut contract_state);
    let mut aggregator = Aggregator::new(contract_state);
    // txs
    let transfer_tx = gen_transfer_tx(
        account_indexes[0],
        account_indexes[1],
        1,
        CKB_TOKEN_ID,
        15,
        3,
    );
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
    let transfer_tx = gen_transfer_tx(
        account_indexes[0],
        account_indexes[1],
        1,
        CKB_TOKEN_ID,
        15,
        3,
    );
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
    // decrease balance of aggregator
    contract_state.update_account(ag_index, CKB_TOKEN_ID, -1 as i128);
    let mut aggregator = Aggregator::new(contract_state);
    // txs
    let transfer_tx = gen_transfer_tx(
        account_indexes[0],
        account_indexes[1],
        1,
        CKB_TOKEN_ID,
        15,
        3,
    );
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
    let transfer_tx = gen_transfer_tx(
        account_indexes[0],
        account_indexes[1],
        1,
        CKB_TOKEN_ID,
        15,
        3,
    );
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
