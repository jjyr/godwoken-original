use crate::tests::utils::{constants::CKB_TOKEN_ID, contract_state::ContractState};
/// Offchain Aggregator
use ckb_contract_tool::{ckb_hash::blake2b_256, TxBuilder};
use godwoken_types::{cache::KVMap, core::Index, packed::*, prelude::*};
use godwoken_utils::{mmr::merkle_root, smt};

pub struct Aggregator {
    contract_state: ContractState,
    txs_queue: Vec<Tx>,
}

pub struct SubmitBlockContext {
    pub block: AgBlock,
    pub txs: Vec<Tx>,
    pub account_proof: SMTProof,
    pub prev_global_state: GlobalState,
    pub prev_ag_account: Account,
    pub kv: KVMap,
}

impl SubmitBlockContext {
    pub fn complete_sig(&mut self, sig: [u8; 65]) {
        self.block = self.block.clone().as_builder().ag_sig(sig.pack()).build();
    }
}

impl Aggregator {
    pub fn new(contract_state: ContractState) -> Self {
        Aggregator {
            contract_state,
            txs_queue: Vec::new(),
        }
    }

    /// push a new user tx to pool
    pub fn push_tx(&mut self, tx: Tx) {
        self.txs_queue.push(tx);
    }

    /// generate submit block
    pub fn gen_submit_block(&mut self, ag_index: Index) -> SubmitBlockContext {
        let (leaves_path, merkle_branches) = self.contract_state.gen_account_merkle_proof(vec![
            smt::account_index_key(ag_index),
            smt::token_id_key(ag_index, &CKB_TOKEN_ID),
        ]);

        let prev_global_state = self.contract_state.get_global_state();
        let prev_account_root = prev_global_state.account_root().unpack();
        let account_count: u64 = prev_global_state.account_count().unpack();
        let (ag_account, kv) = {
            let ag_account = self
                .contract_state
                .get_account(ag_index)
                .expect("get aggregator account");
            let balance: u64 = self
                .contract_state
                .get_account_token(ag_index, &CKB_TOKEN_ID)
                .expect("get");
            let mut kv = KVMap::default();
            kv.insert(CKB_TOKEN_ID, balance);
            (ag_account, kv)
        };

        // TODO state should be revertable
        for tx in &self.txs_queue {
            self.contract_state.apply_tx(&tx, ag_index);
        }

        // new account root
        let new_account_root = self.contract_state.account_root();
        let block_number = self.contract_state.block_count();
        let txs = self.txs_queue.clone();
        let tx_root = merkle_root(
            self.txs_queue
                .iter()
                .map(|tx| blake2b_256(tx.as_slice()))
                .collect(),
        );
        let txs_count = self.txs_queue.len();
        let block = AgBlock::new_builder()
            .number(block_number.pack())
            .tx_root(tx_root.pack())
            .txs_count((txs_count as u32).pack())
            .ag_index(ag_index.pack())
            .prev_account_root(prev_account_root.pack())
            .account_root(new_account_root.pack())
            .account_count(account_count.pack())
            .build();
        let account_proof = SMTProof::new_builder()
            .leaves_path(leaves_path.pack())
            .proof(
                merkle_branches
                    .into_iter()
                    .map(|(node, height)| (node.into(), height))
                    .collect::<Vec<([u8; 32], u8)>>()
                    .pack(),
            )
            .build();
        SubmitBlockContext {
            block,
            txs,
            account_proof,
            prev_global_state,
            prev_ag_account: ag_account,
            kv,
        }
    }

    /// generate submit block tx
    pub fn complete_submit_block(&mut self, submit_block_context: SubmitBlockContext) -> TxBuilder {
        let SubmitBlockContext {
            block,
            txs,
            account_proof,
            prev_global_state,
            prev_ag_account,
            kv,
        } = submit_block_context;
        let block_number: u64 = block.number().unpack();
        let (_mmr_size, prev_block_proof) =
            self.contract_state.gen_block_merkle_proof(block_number);
        let submit_block = {
            let tx_vec = TxVec::new_builder().set(txs).build();
            SubmitBlock::new_builder()
                .txs(tx_vec)
                .block(block.clone())
                .block_proof(
                    prev_block_proof
                        .into_iter()
                        .map(|i| i.pack())
                        .collect::<Vec<_>>()
                        .pack(),
                )
                .ag_account(prev_ag_account)
                .token_kv(kv.pack())
                .account_proof(account_proof)
                .build()
        };

        let action = Action::new_builder().set(submit_block).build();
        // submit block
        self.contract_state.submit_block(block);
        let new_global_state = self.contract_state.get_global_state();

        // update tx witness
        let witness = WitnessArgs::new_builder()
            .output_type(Some(action.as_bytes()).pack())
            .build();
        let contract_balance = self.contract_state.balance();

        TxBuilder::default()
            .lock_script(self.contract_state.lock_script().as_slice().to_vec().into())
            .type_script(self.contract_state.type_script().as_slice().to_vec().into())
            .previous_output_data(prev_global_state.as_slice().into())
            .input_capacity(contract_balance)
            .output_capacity(contract_balance)
            .witnesses(vec![witness.as_slice().into()])
            .outputs_data(vec![new_global_state.as_slice().into()])
    }
}
