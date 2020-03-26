use crate::tests::utils::{
    constants::{AGGREGATOR_REQUIRED_BALANCE, CKB_TOKEN_ID},
    contract_state::ContractState,
};
use crate::tests::{DUMMY_LOCK_BIN, DUMMY_LOCK_HASH, MAIN_CONTRACT_BIN, MAIN_CONTRACT_HASH};
use ckb_tool::testtool::{context::Context, tx_builder::TxBuilder};
use godwoken_types::prelude::*;
use godwoken_types::{
    core::{Index, ScriptHashType},
    packed::*,
};
use godwoken_utils::hash::new_blake2b;
use rand::{thread_rng, Rng};

pub fn default_tx_builder() -> TxBuilder {
    TxBuilder::default()
        .lock_script(
            Script::new_builder()
                .code_hash(DUMMY_LOCK_HASH.pack())
                .hash_type(ScriptHashType::Data.into())
                .build()
                .as_slice()
                .to_owned()
                .into(),
        )
        .type_script(
            Script::new_builder()
                .code_hash(MAIN_CONTRACT_HASH.pack())
                .hash_type(ScriptHashType::Data.into())
                .build()
                .as_slice()
                .to_owned()
                .into(),
        )
}

pub fn default_context() -> Context {
    let mut context = Context::default();
    context.deploy_contract(DUMMY_LOCK_BIN.to_owned());
    context.deploy_contract(MAIN_CONTRACT_BIN.clone());
    context
}

pub fn sign_block(privkey: &secp256k1::SecretKey, block: &AgBlock) -> [u8; 65] {
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
}

pub fn gen_accounts(start_i: Index, count: usize) -> impl Iterator<Item = Account> {
    (start_i..start_i + count as Index).map(|i| {
        let mut pubkey = [0u8; 20];
        let mut rng = thread_rng();
        rng.fill(&mut pubkey);
        Account::new_builder()
            .index(i.pack())
            .pubkey_hash(pubkey.pack())
            .build()
    })
}

pub fn prepare_accounts(contract_state: &mut ContractState, balances: Vec<u64>) -> Vec<Index> {
    let i = contract_state.account_count();
    let indexes: Vec<Index> = (i..(i + balances.len() as Index)).collect();
    for ((i, account), balance) in indexes
        .iter()
        .zip(gen_accounts(i, balances.len()))
        .zip(balances.into_iter())
    {
        contract_state.push_account(account);
        contract_state.update_account(*i, CKB_TOKEN_ID, balance as i128);
    }
    indexes
}

pub fn prepare_ag_account(contract_state: &mut ContractState) -> (Index, secp256k1::SecretKey) {
    let ag_index = contract_state.account_count();
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
        .index(ag_index.pack())
        .pubkey_hash(pubkey_hash.pack())
        .build();
    contract_state.push_account(account_ag);
    contract_state.update_account(ag_index, CKB_TOKEN_ID, AGGREGATOR_REQUIRED_BALANCE as i128);
    (ag_index, privkey)
}

pub fn gen_transfer_tx(
    sender: Index,
    to: Index,
    nonce: u32,
    token_id: [u8; 32],
    amount: u32,
    fee: u32,
) -> Tx {
    Tx::new_builder()
        .sender_index(sender.pack())
        .to_index(to.pack())
        .fee((token_id, fee as u64).pack())
        .amount((token_id, amount as u64).pack())
        .nonce(nonce.pack())
        .build()
}
