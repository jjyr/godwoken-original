use crate::common;
use crate::constants::{CHALLENGE_REWARD_RATE, STATE_CHECKPOINT_SIZE};
/// Invalid Block
/// 1. proof block exists
/// 2. re-run block txs from previous state to invalid state
/// 3. penalize aggregator, reward challenger
use crate::error::Error;
use alloc::vec;
use alloc::vec::Vec;
use godwoken_executor::{executor::Executor, state::State};
use godwoken_types::{cache::TxWithHash, packed::*, prelude::*};
use godwoken_utils::{
    hash::new_blake2b,
    mmr::{compute_account_root, compute_block_root, compute_tx_root},
};

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
        block_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        let block_index: u32 = block.number().unpack();
        let block_hash = {
            let mut hasher = new_blake2b();
            hasher.update(block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let block_root = compute_block_root(
            vec![(block_index as usize, block_hash)],
            block_index + 1,
            block_proof,
        )
        .map_err(|_| Error::InvalidBlockMerkleProof)?;
        if &block_root != self.old_state.block_root().raw_data() {
            return Err(Error::InvalidBlockMerkleProof);
        }
        Ok(())
    }

    fn verify_checkpoint_and_txs_len(
        &self,
        checkpoint_index: u32,
        block: &AgBlockReader<'a>,
        txs: &TxVecReader<'a>,
    ) -> Result<(), Error> {
        if checkpoint_index as usize >= block.state_checkpoints().len() {
            return Err(Error::OutOfIndexCheckpoint);
        }
        if txs.len() > STATE_CHECKPOINT_SIZE || txs.len() == 0 {
            return Err(Error::IncorrectInvalidTxsSize);
        }
        Ok(())
    }

    /// verify txs
    /// return tx_with_hashes for later use
    fn verify_txs_root(
        &self,
        block: &AgBlockReader<'a>,
        checkpoint_index: u32,
        txs: &[TxWithHash],
    ) -> Result<(), Error> {
        let leaves: Vec<_> = {
            let base_index = checkpoint_index as usize * STATE_CHECKPOINT_SIZE;
            txs.iter()
                .enumerate()
                .map(|(i, tx)| (base_index + i, tx.tx_hash.clone()))
                .collect()
        };
        let txs_count: u32 = self.action.txs_count().unpack();
        let txs_proof: Vec<[u8; 32]> = self
            .action
            .txs_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let calculated_tx_root = compute_tx_root(leaves, txs_count, txs_proof)
            .map_err(|_| Error::InvalidTxMerkleProof)?;
        if &calculated_tx_root != block.tx_root().raw_data() {
            return Err(Error::InvalidTxMerkleProof);
        }
        Ok(())
    }

    fn verify_accounts(
        &self,
        block: &AgBlockReader<'a>,
        accounts_count: u32,
        accounts_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        let leaves = accounts_to_proof_leaves(self.action.touched_accounts().iter());
        let calculated_root = compute_account_root(leaves, accounts_count, accounts_proof)
            .map_err(|_| Error::InvalidAccountMerkleProof)?;
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

    /// return Ok if the block state is invalid
    fn verify_invalid_state(
        &self,
        block: &AgBlockReader<'a>,
        checkpoint_index: u32,
        tx_with_hashes: Vec<TxWithHash>,
        accounts_count: u32,
        accounts_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        let mut state = State::new(
            self.action
                .touched_accounts()
                .iter()
                .map(|account| account.to_entity())
                .collect(),
        );
        let executor = Executor::new();
        let ag_index: u32 = block.ag_index().unpack();
        for tx in tx_with_hashes {
            if executor.run(&mut state, tx, ag_index).is_err() {
                // errors occured, represents the block is invalid
                return Ok(());
            }
        }
        // check new account root
        let leaves: Vec<_> =
            accounts_to_proof_leaves(state.iter().map(|account| account.as_reader()));
        let calculated_root = compute_account_root(leaves, accounts_count, accounts_proof)
            .map_err(|_| Error::InvalidAccountMerkleProof)?;
        if &calculated_root
            != block
                .state_checkpoints()
                .get((checkpoint_index + 1) as usize)
                .expect("get invalid checkpoint")
                .raw_data()
        {
            // block is invalid
            return Ok(());
        }
        Err(Error::TryRevertValidBlock)
    }

    pub fn calculate_reverted_account_root(
        &self,
        block: &AgBlockReader<'a>,
        ag_index: u32,
        challenger_index: u32,
        accounts_count: u32,
        accounts_proof: Vec<[u8; 32]>,
    ) -> Result<[u8; 32], Error> {
        let mut state = State::new(
            self.action
                .touched_accounts()
                .iter()
                .map(|account| account.to_entity())
                .collect(),
        );
        let ag_account = state.get_account(ag_index).ok_or(Error::MissingAgAccount)?;
        let challenger_account = state
            .get_account(challenger_index)
            .ok_or(Error::MissingChallengerAccount)?;
        let reward_amount = {
            let ag_balance: u64 = ag_account.balance().unpack();
            ag_balance.saturating_mul(CHALLENGE_REWARD_RATE.0) / CHALLENGE_REWARD_RATE.1
        };
        let challenger_balance: u64 = challenger_account.balance().unpack();
        state.update_account_balance(ag_index, 0);
        state.update_account_balance(
            challenger_index,
            challenger_balance.saturating_add(reward_amount),
        );
        let leaves = accounts_to_proof_leaves(state.iter().map(|account| account.as_reader()));
        let account_root = compute_account_root(leaves, accounts_count, accounts_proof)
            .map_err(|_| Error::InvalidAccountMerkleProof)?;
        Ok(account_root)
    }

    pub fn verify_penalize_and_new_state(
        &self,
        block: &AgBlockReader<'a>,
        accounts_count: u32,
        accounts_proof: Vec<[u8; 32]>,
        block_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        let ag_index: u32 = block.ag_index().unpack();
        let challenger_index: u32 = self.action.challenger_index().unpack();
        let account_root = self.calculate_reverted_account_root(
            block,
            ag_index,
            challenger_index,
            accounts_count,
            accounts_proof,
        )?;
        if self.new_state.account_root().raw_data() != account_root {
            return Err(Error::InvalidNewAccountRoot);
        }
        // generate a new block to instead the invalid one
        let new_block = AgBlock::new_penalized_block(block, account_root, challenger_index);
        let block_index: u32 = block.number().unpack();
        let block_hash = {
            let mut hasher = new_blake2b();
            hasher.update(new_block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let block_root = compute_block_root(
            vec![(block_index as usize, block_hash)],
            block_index + 1,
            block_proof,
        )
        .map_err(|_| Error::InvalidBlockMerkleProof)?;
        if self.new_state.block_root().raw_data() != block_root {
            return Err(Error::InvalidNewBlockRoot);
        }
        Ok(())
    }

    /// Invalid a block
    /// 1. proof that block/txs/accounts are actually exists
    /// 2. run txs, compare the state to the block's account_root
    /// 3. generate a reverted block to instead the invalid block
    /// 4. put a penalize tx in reverted block
    /// 5. verify new account root and block root
    pub fn verify(&self) -> Result<(), Error> {
        let block = self.action.block();
        let txs = self.action.txs();
        let checkpoint_index: u32 = self.action.checkpoint_index().unpack();
        let accounts_count: u32 = self.action.accounts_count().unpack();
        let accounts_proof: Vec<[u8; 32]> = self
            .action
            .touched_accounts_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let block_proof: Vec<[u8; 32]> = self
            .action
            .block_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        self.verify_block(&block, block_proof.clone())?;
        self.verify_checkpoint_and_txs_len(checkpoint_index, &block, &txs)?;
        let tx_with_hashes = build_tx_hashes(&txs);
        self.verify_txs_root(&block, checkpoint_index, &tx_with_hashes)?;
        self.verify_accounts(&block, accounts_count, accounts_proof.clone())?;
        self.verify_invalid_state(
            &block,
            checkpoint_index,
            tx_with_hashes,
            accounts_count,
            accounts_proof.clone(),
        )?;
        self.verify_penalize_and_new_state(&block, accounts_count, accounts_proof, block_proof)?;
        Ok(())
    }
}

fn build_tx_hashes<'a>(txs: &'a TxVecReader<'a>) -> Vec<TxWithHash<'a>> {
    txs.iter()
        .enumerate()
        .map(|(i, tx)| {
            let mut hasher = new_blake2b();
            hasher.update(tx.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            TxWithHash {
                raw: tx,
                tx_hash: hash.clone(),
            }
        })
        .collect()
}

fn accounts_to_proof_leaves<'a>(
    iter: impl Iterator<Item = AccountReader<'a>>,
) -> Vec<(usize, [u8; 32])> {
    iter.map(|account| {
        let index: u32 = account.index().unpack();
        let mut hasher = new_blake2b();
        hasher.update(account.as_slice());
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        (index as usize, hash)
    })
    .collect()
}
