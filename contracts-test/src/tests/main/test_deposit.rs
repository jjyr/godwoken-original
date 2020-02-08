use crate::tests::{
    utils::global_state::GlobalStateContext, DUMMY_LOCK_BIN, MAIN_CONTRACT_BIN, MAX_CYCLES,
};
use ckb_contract_tool::{Context, TxBuilder};
use godwoken_types::packed::{Account, Action, Deposit, WitnessArgs};
use godwoken_types::prelude::*;

#[test]
fn test_deposit() {
    let mut context = GlobalStateContext::new();
    // prepare a account account
    let account = Account::new_builder().build();
    context.push_account(account.clone());
    let global_state = context.get_global_state();

    let original_amount = 12u64;
    let deposit_amount = 42u64;

    // deposit money
    let new_account = {
        let balance: u64 = account.balance().unpack();
        account
            .clone()
            .as_builder()
            .balance((balance + deposit_amount).pack())
            .build()
    };
    let (_, proof) = context.gen_account_merkle_proof(account.index().unpack());
    let deposit = Deposit::new_builder()
        .old_account(account)
        .new_account(new_account.clone())
        .count(1u32.pack())
        .proof(
            proof
                .into_iter()
                .map(|i| i.pack())
                .collect::<Vec<_>>()
                .pack(),
        )
        .build();
    let action = Action::new_builder().set(deposit).build();
    let new_global_state = {
        let mut new_context = GlobalStateContext::new();
        new_context.push_account(new_account);
        new_context.get_global_state()
    };

    // update tx witness
    let witness = WitnessArgs::new_builder()
        .output_type(Some(action.as_bytes()).pack())
        .build();
    let contract_bin = MAIN_CONTRACT_BIN.to_owned();
    let mut context = Context::default();
    context.deploy_contract(DUMMY_LOCK_BIN.to_owned());
    context.deploy_contract(contract_bin.clone());
    let tx = TxBuilder::default()
        .lock_bin(DUMMY_LOCK_BIN.to_owned())
        .type_bin(contract_bin)
        .previous_output_data(global_state.as_slice().into())
        .input_capacity(original_amount)
        .output_capacity(original_amount + deposit_amount)
        .witnesses(vec![witness.as_slice().into()])
        .outputs_data(vec![new_global_state.as_slice().into()])
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    verify_result.expect("pass verification");
}
