use crate::tests::{
    utils::{
        constants::NATIVE_TOKEN_ID,
        contract_state::ContractState,
        shortcut::{default_context, default_tx_builder, gen_accounts},
    },
    MAX_CYCLES,
};
use godwoken_types::{
    cache::KVMap,
    packed::{Action, Deposit, SMTProof, WitnessArgs},
    prelude::*,
};
use godwoken_utils::smt::SMT;

#[test]
fn test_deposit() {
    let mut context = ContractState::new();

    let original_amount = 12u64;
    let deposit_amount = 42u64;

    // prepare a account
    let account = gen_accounts(0, vec![original_amount]).next().unwrap();
    context.push_account(account.clone());
    let global_state = context.get_global_state();
    let mut tree = SMT::default();
    tree.update(NATIVE_TOKEN_ID.into(), original_amount.into())
        .expect("update");
    let mut kv = KVMap::default();
    kv.insert(NATIVE_TOKEN_ID, original_amount);
    let kv_proof = tree
        .merkle_proof(vec![NATIVE_TOKEN_ID.into()])
        .expect("gen merkle proof");

    // deposit money
    let new_account = {
        tree.update(
            NATIVE_TOKEN_ID.into(),
            (original_amount + deposit_amount).into(),
        )
        .expect("update");
        let root: [u8; 32] = (*tree.root()).into();
        account.clone().as_builder().state_root(root.pack()).build()
    };
    let (_, proof) = context.gen_account_merkle_proof(account.index().unpack());
    let deposit = Deposit::new_builder()
        .old_account(account)
        .new_account(new_account.clone())
        .old_kv(kv.pack())
        .kv_proof(
            SMTProof::new_builder()
                .leaves_path(kv_proof.leaves_path().pack())
                .proof({
                    let proof: Vec<([u8; 32], u8)> = kv_proof
                        .proof()
                        .iter()
                        .map(|(node, height)| ((*node).into(), *height))
                        .collect();
                    proof.pack()
                })
                .build(),
        )
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
        let mut new_context = ContractState::new();
        new_context.push_account(new_account);
        new_context.get_global_state()
    };

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
