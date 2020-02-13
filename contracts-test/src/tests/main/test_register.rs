use crate::tests::{
    utils::{
        constants::{AGGREGATOR_REQUIRED_BALANCE, NEW_ACCOUNT_REQUIRED_BALANCE},
        contract_state::ContractState,
        shortcut::{default_context, default_tx_builder},
    },
    MAX_CYCLES,
};
use ckb_contract_tool::ckb_hash::blake2b_256;
use godwoken_types::bytes::Bytes;
use godwoken_types::packed::{Account, AccountScript, Action, Register, WitnessArgs};
use godwoken_types::prelude::*;
use rand::{thread_rng, Rng};

#[test]
fn test_account_register() {
    let mut context = ContractState::new();
    let global_state = context.get_global_state();
    // insert few entries
    let mut last_account: Option<Account> = None;
    let mut global_state = global_state;
    let mut original_amount = 0;
    for index in 0u32..=5u32 {
        let is_aggregator = index < 2;
        let deposit_amount = if is_aggregator {
            AGGREGATOR_REQUIRED_BALANCE
        } else {
            NEW_ACCOUNT_REQUIRED_BALANCE
        };
        original_amount += deposit_amount;
        let account = {
            let mut pubkey = [0u8; 20];
            let mut rng = thread_rng();
            rng.fill(&mut pubkey);
            Account::new_builder()
                .index(index.pack())
                .script(
                    AccountScript::new_builder()
                        .args(Bytes::from(pubkey.to_vec()).pack())
                        .build(),
                )
                .is_ag({
                    if is_aggregator {
                        1.into()
                    } else {
                        0.into()
                    }
                })
                .balance(deposit_amount.pack())
                .build()
        };
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
