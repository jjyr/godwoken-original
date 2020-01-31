use crate::constants::STATE_CHECKPOINTS_INTERVAL;
use crate::error::Error;
use crate::utils;
use alloc::vec;
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};

pub struct SubmitBlockVerifier<'a> {
    action: SubmitBlockReader<'a>,
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
}

impl<'a> SubmitBlockVerifier<'a> {
    pub fn new(
        old_state: GlobalStateReader<'a>,
        new_state: GlobalStateReader<'a>,
        submit_block: SubmitBlockReader<'a>,
    ) -> SubmitBlockVerifier<'a> {
        SubmitBlockVerifier {
            action: submit_block,
            old_state,
            new_state,
        }
    }

    fn check_balance(&self) -> Result<(), Error> {
        let changes = utils::fetch_capacities();
        if changes.input != changes.output {
            return Err(Error::IncorrectCapacity);
        }
        Ok(())
    }

    /// verify aggregator
    /// 1. aggregator is valid
    /// 2. aggregator exsits in account root
    /// 3. aggregator's signature is according to pubkey hash
    fn check_aggregator(&self) -> Result<(), Error> {
        let ag_entry = self.action.ag_entry();
        utils::check_aggregator(&ag_entry)?;
        // verify merkle proof of aggregator
        let account_count: u32 = self.action.account_count().unpack();
        let account_proof: Vec<[u8; 32]> = self
            .action
            .account_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let ag_index: u32 = ag_entry.index().unpack();
        let entry_hash = {
            let mut hasher = utils::new_blake2b();
            hasher.update(ag_entry.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let calculated_root =
            utils::compute_account_root(entry_hash, ag_index, account_count, account_proof)?;
        let old_account_root = self.old_state.account_root().unpack();
        if calculated_root != old_account_root {
            return Err(Error::InvalidAccountMerkleProof);
        }
        // verify aggregator's signature
        let ag_pubkey_hash = ag_entry.script().args().raw_data();
        let block = self.action.block();
        let sig_message = {
            let sig_block = block
                .to_entity()
                .as_builder()
                .ag_sig(Byte65::default())
                .build();
            let mut hasher = utils::new_blake2b();
            hasher.update(sig_block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let ag_sig = block.ag_sig().unpack();
        utils::verify_ag_signature(ag_sig, sig_message, ag_pubkey_hash)?;
        Ok(())
    }

    fn check_block(&self) -> Result<(), Error> {
        let block = self.action.block();
        // verify block state checkpoints
        let checkpoints_count = state_checkpoints_count(self.action.txs().len());
        use ckb_contract_std::debug;
        debug!("required checkpoints {}  count {}", checkpoints_count, block.state_checkpoints().len());
        if block.state_checkpoints().len() != checkpoints_count {
            return Err(Error::IncorrectNumberOfCheckpoints);
        }
        if block
            .state_checkpoints()
            .get(0)
            .expect("old account root")
            .as_slice()
            != self.old_state.account_root().as_slice()
        {
            return Err(Error::InvalidAccountRoot);
        }
        if block
            .state_checkpoints()
            .get(checkpoints_count - 1)
            .expect("new account root")
            .as_slice()
            != self.new_state.account_root().as_slice()
        {
            return Err(Error::InvalidAccountRoot);
        }
        // verify tx root
        let tx_hashes: Vec<[u8; 32]> = self
            .action
            .txs()
            .iter()
            .map(|tx| {
                let mut hasher = utils::new_blake2b();
                hasher.update(tx.as_slice());
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);
                hash
            })
            .collect();
        let calculated_tx_root = utils::merkle_root(&tx_hashes);
        let tx_root = self.action.block().tx_root().unpack();
        if tx_root != calculated_tx_root {
            return Err(Error::InvalidTxRoot);
        }
        Ok(())
    }

    fn check_state_transition(&self) -> Result<(), Error> {
        // verify old state merkle proof
        let block = self.action.block();
        let block_number: u32 = block.number().unpack();
        let block_proof: Vec<[u8; 32]> = self
            .action
            .block_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let last_block_hash = self.action.last_block_hash().unpack();
        let old_block_root = self.old_state.block_root().unpack();
        if block_number == 0 {
            if old_block_root != [0u8; 32] || block_proof.len() != 0 {
                return Err(Error::InvalidAccountMerkleProof);
            }
        } else {
            let calculated_root = utils::compute_block_root(
                last_block_hash,
                block_number - 1,
                block_number + 1,
                block_proof.clone(),
            )?;
            if old_block_root != calculated_root {
                return Err(Error::InvalidBlockMerkleProof);
            }
        }
        // verify new state merkle proof
        let block_hash = {
            let mut hasher = utils::new_blake2b();
            hasher.update(block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let new_block_root = self.new_state.block_root().unpack();
        let calculated_root = utils::compute_new_block_root(
            last_block_hash,
            block_number - 1,
            block_hash,
            block_number,
            block_number + 1,
            block_proof,
        )?;
        if new_block_root != calculated_root {
            return Err(Error::InvalidBlockMerkleProof);
        }
        Ok(())
    }

    pub fn verify(&self) -> Result<(), Error> {
        self.check_balance()?;
        self.check_aggregator()?;
        self.check_block()?;
        self.check_state_transition()?;
        Ok(())
    }
}

/// required count of checkpoints for txs_len
/// checkpoints: old_state | cp1 | cp2 ....
fn state_checkpoints_count(txs_len: usize) -> usize {
    let cp_count = txs_len.saturating_sub(1) / STATE_CHECKPOINTS_INTERVAL + 1;
    // plus 1 for old_state
    cp_count + 1
}
