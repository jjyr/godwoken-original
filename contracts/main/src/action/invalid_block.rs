use crate::constants::STATE_CHECKPOINT_SIZE;
/// Invalid Block
/// 1. proof block exists
/// 2. re-run block txs from previous state to invalid state
/// 3. penalize aggregator, reward challenger
use crate::error::Error;
use crate::utils;
use alloc::vec;
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};

pub struct InvalidBlockVerifier<'a> {
    action: InvalidBlockReader<'a>,
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
}

impl<'a> InvalidBlockVerifier<'a> {
    pub fn new(
        old_state: GlobalStateReader<'a>,
        new_state: GlobalStateReader<'a>,
        invalid_block: InvalidBlockReader<'a>,
    ) -> InvalidBlockVerifier<'a> {
        InvalidBlockVerifier {
            action: invalid_block,
            old_state,
            new_state,
        }
    }

    fn verify_block(
        &self,
        block: &AgBlockReader<'a>,
        invalid_checkpoint: u32,
    ) -> Result<(), Error> {
        let block_index: u32 = block.number().unpack();
        let block_hash = {
            let mut hasher = utils::new_blake2b();
            hasher.update(block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let block_proof: Vec<[u8; 32]> = self
            .action
            .block_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let blocks_root = utils::compute_block_root(
            vec![(block_index as usize, block_hash)],
            block_index + 1,
            block_proof,
        )?;
        if &blocks_root != self.old_state.block_root().raw_data() {
            return Err(Error::InvalidBlockMerkleProof);
        }
        if invalid_checkpoint as usize >= block.state_checkpoints().len() {
            return Err(Error::OutOfIndexCheckpoint);
        }
        Ok(())
    }

    fn verify_txs(&self, block: &AgBlockReader<'a>, invalid_checkpoint: u32) -> Result<(), Error> {
        let txs = self.action.txs();
        if txs.len() > STATE_CHECKPOINT_SIZE || txs.len() == 0 {
            return Err(Error::IncorrectInvalidTxsSize);
        }

        let leaves: Vec<_> = {
            let base_index = invalid_checkpoint as usize * STATE_CHECKPOINT_SIZE;
            txs.iter()
                .enumerate()
                .map(|(i, tx)| {
                    let mut hasher = utils::new_blake2b();
                    hasher.update(tx.as_slice());
                    let mut hash = [0u8; 32];
                    hasher.finalize(&mut hash);
                    (base_index + i, hash)
                })
                .collect()
        };
        let txs_count: u32 = self.action.txs_count().unpack();
        let txs_proof: Vec<[u8; 32]> = self
            .action
            .txs_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let calculated_tx_root = utils::compute_tx_root(leaves, txs_count, txs_proof)?;
        if &calculated_tx_root != block.tx_root().raw_data() {
            return Err(Error::InvalidTxMerkleProof);
        }
        Ok(())
    }
    fn verify_account(&self, block: &AgBlockReader<'a>) -> Result<(), Error> {
        let leaves: Vec<_> = {
            let accounts = self.action.touched_accounts();
            accounts
                .iter()
                .map(|account| {
                    let index: u32 = account.index().unpack();
                    let mut hasher = utils::new_blake2b();
                    hasher.update(account.as_slice());
                    let mut hash = [0u8; 32];
                    hasher.finalize(&mut hash);
                    (index as usize, hash)
                })
                .collect()
        };
        let accounts_count: u32 = self.action.accounts_count().unpack();
        let accounts_proof: Vec<[u8; 32]> = self
            .action
            .touched_accounts_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let calculated_root = utils::compute_account_root(leaves, accounts_count, accounts_proof)?;
        if &calculated_root
            != block
                .state_checkpoints()
                .get(0)
                .expect("account root")
                .raw_data()
        {
            return Err(Error::InvalidAccountMerkleProof);
        }
        Ok(())
    }
    fn verify_state_transition(&self) -> Result<(), Error> {
        Ok(())
    }

    pub fn verify(&self) -> Result<(), Error> {
        let block = self.action.block();
        let invalid_checkpoint: u32 = self.action.invalid_checkpoint().unpack();
        self.verify_block(&block, invalid_checkpoint)?;
        self.verify_txs(&block, invalid_checkpoint)?;
        self.verify_account(&block)?;
        self.verify_state_transition()?;
        Ok(())
    }
}
