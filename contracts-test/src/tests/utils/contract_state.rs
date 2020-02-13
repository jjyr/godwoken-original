use crate::tests::{DUMMY_LOCK_HASH, MAIN_CONTRACT_HASH};
use ckb_contract_tool::ckb_hash::{blake2b_256, new_blake2b};
use ckb_merkle_mountain_range::{leaf_index_to_pos, util::MemMMR, Merge};
use godwoken_types::bytes::Bytes;
use godwoken_types::prelude::*;
use godwoken_types::{
    core::ScriptHashType,
    packed::{Account, AgBlock, GlobalState, Script, Tx},
};

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
    account_entries: Vec<Account>,
    block_root: [u8; 32],
    block_mmr: HashMMR,
    lock_data_hash: [u8; 32],
    type_data_hash: [u8; 32],
    block_count: u32,
}

impl ContractState {
    pub fn new() -> Self {
        ContractState {
            account_entries: Vec::new(),
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
        self.account_entries.len() as u32
    }

    pub fn block_count(&self) -> u32 {
        self.block_count
    }

    pub fn get_account(&self, index: u32) -> Option<&Account> {
        self.account_entries.get(index as usize)
    }

    pub fn account_root(&self) -> [u8; 32] {
        let mut account_root = [0u8; 32];
        if self.account_entries.is_empty() {
            return account_root;
        }
        let account_mmr_root = self.build_account_mmr().get_root().expect("mmr root");
        let entries_count: u32 = self.account_entries.len() as u32;
        let mut hasher = new_blake2b();
        hasher.update(&entries_count.to_le_bytes());
        hasher.update(&account_mmr_root);
        hasher.finalize(&mut account_root);
        account_root
    }

    fn build_account_mmr(&self) -> HashMMR {
        let mut mmr = HashMMR::default();
        for account in &self.account_entries {
            let account_hash = blake2b_256(account.as_slice());
            mmr.push(account_hash).expect("mmr push");
        }
        mmr
    }

    pub fn push_account(&mut self, account: Account) {
        let index = Unpack::<u32>::unpack(&account.index()) as usize;
        if index == self.account_entries.len() {
            self.account_entries.push(account);
        } else {
            self.account_entries[index] = account;
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

    pub fn apply_tx(&mut self, tx: &Tx, fee_to: u32) {
        let tx_fee: u64 = {
            let tx_fee: u32 = tx.fee().unpack();
            tx_fee.into()
        };
        let args: Bytes = tx.args().unpack();
        let to_index: u64 = {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&args[..4]);
            u32::from_le_bytes(buf).into()
        };
        let amount: u64 = {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&args[4..]);
            u32::from_le_bytes(buf).into()
        };
        let from_account =
            &self.account_entries[Unpack::<u32>::unpack(&tx.account_index()) as usize];
        let from_account = from_account
            .clone()
            .as_builder()
            .balance({
                let balance: u64 = from_account.balance().unpack();
                (balance - amount - tx_fee).pack()
            })
            .nonce({
                let nonce: u32 = from_account.nonce().unpack();
                (nonce + 1).pack()
            })
            .build();
        let to_account = &self.account_entries[to_index as usize];
        let to_account = to_account
            .clone()
            .as_builder()
            .balance({
                let balance: u64 = to_account.balance().unpack();
                (balance + amount).pack()
            })
            .build();
        let fee_account = &self.account_entries[fee_to as usize];
        let fee_account = fee_account
            .clone()
            .as_builder()
            .balance({
                let balance: u64 = fee_account.balance().unpack();
                (balance + tx_fee).pack()
            })
            .build();
        self.push_account(from_account);
        self.push_account(to_account);
        self.push_account(fee_account);
    }
}
