use crate::tests::utils::{constants::AGGREGATOR_REQUIRED_BALANCE, contract_state::ContractState};
use crate::tests::{DUMMY_LOCK_BIN, DUMMY_LOCK_HASH, MAIN_CONTRACT_BIN, MAIN_CONTRACT_HASH};
use ckb_contract_tool::{Context, TxBuilder};
use godwoken_types::bytes::Bytes;
use godwoken_types::prelude::*;
use godwoken_types::{core::ScriptHashType, packed::*};
use godwoken_utils::hash::new_blake2b;
use rand::thread_rng;

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

pub fn prepare_accounts(
    contract_state: &mut ContractState,
    accounts_balance: Vec<u64>,
) -> Vec<u32> {
    let i = contract_state.account_count();
    let indexes: Vec<u32> = (i..(i + accounts_balance.len() as u32)).collect();
    for (i, balance) in indexes.iter().zip(accounts_balance.iter()) {
        let account = Account::new_builder()
            .balance(balance.pack())
            .index(i.pack())
            .build();
        contract_state.push_account(account);
    }
    indexes
}

pub fn prepare_ag_account(contract_state: &mut ContractState) -> (u32, secp256k1::SecretKey) {
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
        .balance(AGGREGATOR_REQUIRED_BALANCE.pack())
        .index(ag_index.pack())
        .script(
            AccountScript::new_builder()
                .args(Bytes::from(pubkey_hash.to_vec()).pack())
                .build(),
        )
        .is_ag(1u8.into())
        .build();
    contract_state.push_account(account_ag);
    (ag_index, privkey)
}
