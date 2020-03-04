use crate::tests::{DUMMY_LOCK_HASH, MAIN_CONTRACT_HASH};
use ckb_contract_tool::ckb_hash::{blake2b_256, new_blake2b};
use ckb_merkle_mountain_range::{leaf_index_to_pos, util::MemMMR, Merge};
use godwoken_types::{
    core::{Index, ScriptHashType, TokenID},
    packed::{Account, AgBlock, GlobalState, Script, Tx},
    prelude::*,
};
use godwoken_utils::smt::{self, Value, SMT};

pub struct HashMerge;

impl Merge for HashMerge {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut merge_result = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize(&mut merge_result);
        merge_result
    }
}

type HashMMR = MemMMR<[u8; 32], HashMerge>;

pub struct ContractState {
    account_smt: SMT,
    block_mmr: HashMMR,
    lock_data_hash: [u8; 32],
    type_data_hash: [u8; 32],
    block_count: u64,
    account_count: u64,
}

impl ContractState {
    pub fn new() -> Self {
        ContractState {
            account_smt: SMT::default(),
            block_mmr: Default::default(),
            lock_data_hash: *DUMMY_LOCK_HASH,
            type_data_hash: *MAIN_CONTRACT_HASH,
            block_count: 0,
            account_count: 0,
        }
    }

    /// TODO
    /// return balance of contract
    pub fn balance(&self) -> u64 {
        0
    }

    pub fn type_script(&self) -> Script {
        Script::new_builder()
            .code_hash(self.type_data_hash.pack())
            .hash_type(ScriptHashType::Data.into())
            .build()
    }

    pub fn lock_script(&self) -> Script {
        Script::new_builder()
            .code_hash(self.lock_data_hash.pack())
            .hash_type(ScriptHashType::Data.into())
            .build()
    }

    pub fn get_global_state(&self) -> GlobalState {
        GlobalState::new_builder()
            .account_root(self.account_root().pack())
            .block_root(self.block_root().pack())
            .account_count(self.account_count.pack())
            .block_count(self.block_count.pack())
            .build()
    }

    pub fn account_count(&self) -> u64 {
        self.account_count
    }

    pub fn block_count(&self) -> u64 {
        self.block_count
    }

    pub fn get_account(&self, index: Index) -> Option<Account> {
        let key = smt::account_index_key(index);
        self.account_smt.get(&key).map(|v| v.into()).ok()
    }

    pub fn get_account_token(&self, index: Index, token: &TokenID) -> Option<u64> {
        let key = smt::token_id_key(index, token);
        self.account_smt.get(&key).map(|v| v.into()).ok()
    }

    pub fn block_root(&self) -> [u8; 32] {
        if self.block_count == 0 {
            return [0u8; 32];
        }
        self.block_mmr.get_root().expect("root")
    }

    pub fn account_root(&self) -> [u8; 32] {
        (*self.account_smt.root()).into()
    }

    pub fn push_account(&mut self, account: Account) {
        let index: Index = account.index().unpack();
        let key = smt::account_index_key(index);
        let is_new = self.account_smt.get(&key).expect("get").is_zero();
        self.account_smt
            .update(key, Value::from(account))
            .expect("update");
        if is_new {
            self.account_count += 1;
        }
    }

    pub fn gen_account_merkle_proof(
        &self,
        keys: Vec<smt::H256>,
    ) -> (Vec<Vec<u8>>, Vec<(smt::H256, u8)>) {
        let proof = self.account_smt.merkle_proof(keys).expect("merkle_proof");
        (proof.leaves_path().to_owned(), proof.proof().to_owned())
    }

    pub fn submit_block(&mut self, block: AgBlock) {
        let block_hash = blake2b_256(block.as_slice());
        self.block_mmr.push(block_hash).expect("mmr push");
        self.block_count += 1;
    }

    pub fn gen_block_merkle_proof(&self, index: u64) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .block_mmr
            .gen_proof(leaf_index_to_pos(index))
            .expect("result");
        (proof.mmr_size(), proof.proof_items().to_owned())
    }

    pub fn update_account(&mut self, index: Index, token_type: [u8; 32], amount: i128) {
        let token_key = smt::token_id_key(index, &token_type);
        let balance: u64 = self.account_smt.get(&token_key).expect("get").into();
        let new_balance = (balance as i128 + amount) as u64;
        self.account_smt
            .update(token_key, new_balance.into())
            .expect("update");
    }

    pub fn apply_tx(&mut self, tx: &Tx, fee_to: Index) {
        let (fee_token_type, tx_fee): ([u8; 32], u64) = tx.fee().unpack();
        let (token_type, amount): ([u8; 32], u64) = tx.amount().unpack();

        let sender_index: Index = tx.sender_index().unpack();
        let to_index: Index = tx.to_index().unpack();

        self.update_account(sender_index, token_type, -((amount + tx_fee) as i128));
        self.update_account(to_index, token_type, amount as i128);
        self.update_account(fee_to, fee_token_type, tx_fee as i128);

        // increase account's nonce
        let sender_key = smt::account_index_key(sender_index);
        let sender: Account = self.account_smt.get(&sender_key).expect("get").into();
        let nonce: u32 = sender.nonce().unpack();
        self.account_smt
            .update(
                sender_key,
                sender.as_builder().nonce((nonce + 1).pack()).build().into(),
            )
            .expect("update");
    }
}
