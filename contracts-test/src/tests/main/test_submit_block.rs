use crate::tests::{
    utils::{
        constants::AGGREGATOR_REQUIRED_BALANCE, global_state::GlobalStateContext,
        types_utils::merkle_root,
    },
    DUMMY_LOCK_BIN, MAIN_CONTRACT_BIN, MAX_CYCLES,
};
use ckb_contract_tool::{
    ckb_hash::{blake2b_256, new_blake2b},
    Context, TxBuilder,
};
use godwoken_types::bytes::Bytes;
use godwoken_types::packed::{
    Account, AccountScript, Action, AgBlock, SubmitBlock, Tx, TxVec, WitnessArgs,
};
use godwoken_types::prelude::*;
use rand::thread_rng;

#[test]
fn test_submit_block() {
    let mut context = GlobalStateContext::new();

    // prepare account entries
    let account_a = Account::new_builder()
        .balance(20u64.pack())
        .index(0u32.pack())
        .build();
    let account_b = Account::new_builder()
        .balance(100u64.pack())
        .index(1u32.pack())
        .build();
    let (privkey, pubkey) = {
        let mut rng = thread_rng();
        let privkey = secp256k1::SecretKey::random(&mut rng);
        let pubkey = secp256k1::PublicKey::from_secret_key(&privkey);
        (privkey, pubkey)
    };
    let pubkey_hash = {
        let pubkey_bytes = pubkey.serialize_compressed();
        let mut hasher = new_blake2b();
        hasher.update(&pubkey_bytes);
        let mut hash = [0u8; 20];
        hasher.finalize(&mut hash);
        hash
    };
    let account_ag = Account::new_builder()
        .balance(AGGREGATOR_REQUIRED_BALANCE.pack())
        .index(2u32.pack())
        .script(
            AccountScript::new_builder()
                .args(Bytes::from(pubkey_hash.to_vec()).pack())
                .build(),
        )
        .is_ag(1u8.into())
        .build();
    context.push_account(account_a.clone());
    context.push_account(account_b.clone());
    context.push_account(account_ag.clone());
    // aggregator proof
    let (_account_mmr_size, account_proof) =
        context.gen_account_merkle_proof(account_ag.index().unpack());

    let global_state = context.get_global_state();
    let previous_account_root = global_state.account_root().unpack();

    let transfer_tx = Tx::new_builder()
        .account_index(account_a.index())
        .fee(3u32.pack())
        .nonce(1u32.pack())
        .args({
            let mut args = vec![0u8; 8];
            let to_index: u32 = account_b.index().unpack();
            args[..4].copy_from_slice(&to_index.to_le_bytes());
            args[4..].copy_from_slice(&15u32.to_le_bytes());
            args.pack()
        })
        .build();

    context.apply_tx(&transfer_tx, account_ag.index().unpack());

    // new account root
    let new_account_root = context.account_root();

    let original_amount = 120u64;
    // send money
    let tx_root = merkle_root(&[blake2b_256(transfer_tx.as_slice()).pack()]);

    let block = AgBlock::new_builder()
        .number(0u32.pack())
        .tx_root(tx_root)
        .ag_index(account_ag.index())
        .previous_account_root(previous_account_root.pack())
        .current_account_root(new_account_root.pack())
        .build();
    let ag_sig = {
        let mut hasher = new_blake2b();
        hasher.update(block.as_slice());
        let mut block_hash = [0u8; 32];
        hasher.finalize(&mut block_hash);
        let msg = secp256k1::Message::parse(&block_hash);
        let (signature, rec_id) = secp256k1::sign(&msg, &privkey);
        let mut sig = [0u8; 65];
        sig[..64].copy_from_slice(&signature.serialize());
        sig[64] = rec_id.serialize();
        sig
    };
    let block = block.as_builder().ag_sig(ag_sig.pack()).build();

    let (_block_mmr_size, block_proof) = context.gen_block_merkle_proof(0);
    let submit_block = {
        let txs = TxVec::new_builder().set(vec![transfer_tx]).build();
        SubmitBlock::new_builder()
            .txs(txs)
            .block(block.clone())
            .block_proof(
                block_proof
                    .into_iter()
                    .map(|i| i.pack())
                    .collect::<Vec<_>>()
                    .pack(),
            )
            .ag_account(account_ag)
            .account_proof(
                account_proof
                    .into_iter()
                    .map(|i| i.pack())
                    .collect::<Vec<_>>()
                    .pack(),
            )
            .account_count(3u32.pack())
            .build()
    };
    let action = Action::new_builder().set(submit_block).build();

    // submit block
    context.submit_block(block, 1);
    let new_global_state = context.get_global_state();

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
        .output_capacity(original_amount)
        .witnesses(vec![witness.as_slice().into()])
        .outputs_data(vec![new_global_state.as_slice().into()])
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    verify_result.expect("pass verification");
}
