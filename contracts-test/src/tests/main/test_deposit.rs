use crate::tests::{
    utils::{
        constants::CKB_TOKEN_ID,
        contract_state::ContractState,
        shortcut::{default_context, default_tx_builder, gen_accounts},
    },
    MAX_CYCLES,
};
use godwoken_types::{
    cache::KVMap,
    core::Index,
    packed::{Action, Deposit, SMTProof, WitnessArgs},
    prelude::*,
};
use godwoken_utils::smt;

#[test]
fn test_deposit() {
    let mut context = ContractState::new();

    let original_amount = 12u64;
    let deposit_amount = 42u64;

    // prepare a account
    let account = gen_accounts(0, 1).next().unwrap();
    let index: Index = 0;
    context.update_account(index, CKB_TOKEN_ID, original_amount as i128);
    context.push_account(account.clone());

    let (leaves_path, merkle_branches) = context.gen_account_merkle_proof(vec![
        smt::account_index_key(index),
        smt::token_id_key(index, &CKB_TOKEN_ID),
    ]);

    let global_state = context.get_global_state();
    let mut kv = KVMap::default();
    kv.insert(CKB_TOKEN_ID, original_amount);

    // deposit money
    context.update_account(index, CKB_TOKEN_ID, deposit_amount as i128);
    let new_global_state = context.get_global_state();

    let deposit = Deposit::new_builder()
        .account(account)
        .token_kv(kv.pack())
        .proof(
            SMTProof::new_builder()
                .leaves_path(leaves_path.pack())
                .proof(
                    merkle_branches
                        .into_iter()
                        .map(|(node, height)| (node.into(), height))
                        .collect::<Vec<([u8; 32], u8)>>()
                        .pack(),
                )
                .build(),
        )
        .build();
    let action = Action::new_builder().set(deposit).build();

    // update tx witness
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
}
