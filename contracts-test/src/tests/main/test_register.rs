use crate::tests::{
    utils::{
        constants::{AGGREGATOR_REQUIRED_BALANCE, NEW_ACCOUNT_REQUIRED_BALANCE},
        contract_state::ContractState,
        shortcut::{default_context, default_tx_builder, gen_accounts},
    },
    MAX_CYCLES,
};
use ckb_contract_tool::ckb_hash::blake2b_256;
use godwoken_types::packed::{Account, Action, Register, WitnessArgs};
use godwoken_types::prelude::*;

#[test]
fn test_account_register() {
    let mut context = ContractState::new();
    let global_state = context.get_global_state();
    // insert few entries
    let mut last_account: Option<Account> = None;
    let mut global_state = global_state;
    let mut original_amount = 0;
    let balances = vec![
        AGGREGATOR_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
    ];
    for (i, account) in gen_accounts(0, balances.clone()).enumerate() {
        let deposit_amount = balances[i];
        original_amount += deposit_amount;
        let register = match last_account {
            None => {
                // first account
                Register::new_builder().account(account.clone()).build()
            }
            Some(last_account) => {
                let (_, proof) = context.gen_account_merkle_proof(last_account.index().unpack());
                Register::new_builder()
                    .account(account.clone())
                    .last_account_hash(blake2b_256(last_account.as_slice()).pack())
                    .proof(
                        proof
                            .into_iter()
                            .map(|i| i.pack())
                            .collect::<Vec<_>>()
                            .pack(),
                    )
                    .build()
            }
        };
        let action = Action::new_builder().set(register).build();
        context.push_account(account.clone());
        let new_global_state = context.get_global_state();
        let witness = WitnessArgs::new_builder()
            .output_type(Some(action.as_bytes()).pack())
            .build();
        let mut context = default_context();
        let tx = default_tx_builder()
            .previous_output_data(global_state.as_slice().into())
            .input_capacity(original_amount)
            .output_capacity(original_amount + deposit_amount)
            .witnesses(vec![witness.as_slice().into()])
            .outputs_data(vec![new_global_state.as_slice().into()])
            .inject_and_build(&mut context)
            .expect("build tx");
        let verify_result = context.verify_tx(&tx, MAX_CYCLES);
        verify_result.expect("pass verification");
        last_account = Some(account);
        global_state = new_global_state;
    }
}
