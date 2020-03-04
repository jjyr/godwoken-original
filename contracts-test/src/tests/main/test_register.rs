use crate::tests::{
    utils::{
        constants::{AGGREGATOR_REQUIRED_BALANCE, CKB_TOKEN_ID, NEW_ACCOUNT_REQUIRED_BALANCE},
        contract_state::ContractState,
        shortcut::{default_context, default_tx_builder, gen_accounts},
    },
    MAX_CYCLES,
};
use godwoken_types::{
    core::Index,
    packed::{Action, Register, SMTProof, WitnessArgs},
    prelude::*,
};
use godwoken_utils::smt;

#[test]
fn test_account_register() {
    let mut context = ContractState::new();
    let mut global_state = context.get_global_state();
    // insert few entries
    let mut original_amount = 0;
    let balances = vec![
        AGGREGATOR_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
        NEW_ACCOUNT_REQUIRED_BALANCE,
    ];
    for (i, account) in gen_accounts(0, balances.len()).enumerate() {
        let deposit_amount = balances[i];
        original_amount += deposit_amount;
        let index: Index = account.index().unpack();
        let (leaves_path, merkle_branches) = context.gen_account_merkle_proof(vec![
            smt::account_index_key(index),
            smt::token_id_key(index, &CKB_TOKEN_ID),
        ]);
        let proof = SMTProof::new_builder()
            .leaves_path(leaves_path.pack())
            .proof(
                merkle_branches
                    .into_iter()
                    .map(|(node, height)| (node.into(), height))
                    .collect::<Vec<([u8; 32], u8)>>()
                    .pack(),
            )
            .build();
        let register = Register::new_builder()
            .account(account.clone())
            .proof(proof)
            .build();
        let action = Action::new_builder().set(register).build();
        context.push_account(account.clone());
        context.update_account(i as Index, CKB_TOKEN_ID, balances[i] as i128);
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
        global_state = new_global_state;
    }
}
