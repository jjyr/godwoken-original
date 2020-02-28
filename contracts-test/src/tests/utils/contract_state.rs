use crate::tests::{DUMMY_LOCK_HASH, MAIN_CONTRACT_HASH};
use ckb_contract_tool::ckb_hash::{blake2b_256, new_blake2b};
use ckb_merkle_mountain_range::{leaf_index_to_pos, util::MemMMR, Merge};
use godwoken_types::{
    core::ScriptHashType,
    packed::{Account, AgBlock, GlobalState, Script, Tx},
    prelude::*,
};
use godwoken_utils::smt::SMT;

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
    accounts: Vec<(Account, SMT)>,
    block_root: [u8; 32],
    block_mmr: HashMMR,
    lock_data_hash: [u8; 32],
    type_data_hash: [u8; 32],
    block_count: u32,
}

impl ContractState {
    pub fn new() -> Self {
        ContractState {
            accounts: Vec::new(),
            block_root: [0u8; 32],
            block_mmr: Default::default(),
            lock_data_hash: *DUMMY_LOCK_HASH,
            type_data_hash: *MAIN_CONTRACT_HASH,
            block_count: 0,
        }
    }

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
            .block_root(self.block_root.pack())
            .build()
    }

    pub fn account_count(&self) -> u32 {
        self.accounts.len() as u32
    }

    pub fn block_count(&self) -> u32 {
        self.block_count
    }

    pub fn get_account(&self, index: u32) -> Option<&(Account, SMT)> {
        self.accounts.get(index as usize)
    }

    pub fn account_root(&self) -> [u8; 32] {
        let mut account_root = [0u8; 32];
        if self.accounts.is_empty() {
            return account_root;
        }
        let account_mmr_root = self.build_account_mmr().get_root().expect("mmr root");
        let entries_count: u32 = self.accounts.len() as u32;
        let mut hasher = new_blake2b();
        hasher.update(&entries_count.to_le_bytes());
        hasher.update(&account_mmr_root);
        hasher.finalize(&mut account_root);
        account_root
    }

    fn build_account_mmr(&self) -> HashMMR {
        let mut mmr = HashMMR::default();
        for (account, _kv_map) in &self.accounts {
            let account_hash = blake2b_256(account.as_slice());
            mmr.push(account_hash).expect("mmr push");
        }
        mmr
    }

    pub fn push_account(&mut self, account: Account) {
        let index = Unpack::<u32>::unpack(&account.index()) as usize;
        if index == self.accounts.len() {
            self.accounts.push((account, SMT::default()));
        } else {
            self.accounts[index].0 = account;
        }
    }

    pub fn gen_account_merkle_proof(&self, leaf_index: u32) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .build_account_mmr()
            .gen_proof(leaf_index_to_pos(leaf_index.into()))
            .expect("result");
        (proof.mmr_size(), proof.proof_items().to_owned())
    }

    pub fn submit_block(&mut self, block: AgBlock, count: u32) {
        let block_hash = blake2b_256(block.as_slice());
        self.block_mmr.push(block_hash).expect("mmr push");
        let block_mmr_root = self.block_mmr.get_root().expect("mmr root");
        let mut hasher = new_blake2b();
        hasher.update(&count.to_le_bytes());
        hasher.update(&block_mmr_root);
        hasher.finalize(&mut self.block_root);
        self.block_count += 1;
    }

    pub fn gen_block_merkle_proof(&self, leaf_index: u32) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .block_mmr
            .gen_proof(leaf_index_to_pos(leaf_index.into()))
            .expect("result");
        (proof.mmr_size(), proof.proof_items().to_owned())
    }

    pub fn update_account(&mut self, index: u32, token_type: [u8; 32], amount: i128) {
        let balance: u64 = self.accounts[index as usize]
            .1
            .get(&token_type.into())
            .expect("get")
            .into();
        let new_balance = (balance as i128 + amount) as u64;
        let new_state_root: [u8; 32] = (*self.accounts[index as usize]
            .1
            .update(token_type.into(), new_balance.into())
            .expect("update"))
        .into();
        let account = self.accounts[index as usize].0.clone();
        let nonce: u32 = account.nonce().unpack();
        let account = account
            .as_builder()
            .state_root(new_state_root.pack())
            .nonce((nonce + 1).pack())
            .build();
        self.push_account(account);
    }

    pub fn apply_tx(&mut self, tx: &Tx, fee_to: u32) {
        let (_tx_fee_token_type, tx_fee): ([u8; 32], u64) = tx.fee().unpack();
        let (token_type, amount): ([u8; 32], u64) = tx.amount().unpack();

        let sender_index: u32 = tx.sender_index().unpack();
        let to_index: u32 = tx.to_index().unpack();

        self.update_account(sender_index, token_type, -((amount + tx_fee) as i128));
        self.update_account(to_index, token_type, amount as i128);
        self.update_account(fee_to, token_type, tx_fee as i128);
    }
}
