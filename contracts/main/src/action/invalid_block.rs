use crate::constants::{CHALLENGE_REWARD_RATE, CKB_TOKEN_ID};
/// Invalid Block
/// 1. proof block exists
/// 2. re-run block txs from previous state to invalid state
/// 3. penalize aggregator, reward challenger
use crate::error::Error;
use alloc::vec;
use alloc::vec::Vec;
use godwoken_executor::{executor::Executor, state::State};
use godwoken_types::{
    cache::{AccountWithKV, KVMap, TxWithHash},
    packed::*,
    prelude::*,
};
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
        if block.is_penalized_block() {
            // A penalized block can't be invalid since it is generated on-chain
            return Err(Error::TryRevertPenalizedBlock);
        }
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

    fn verify_txs_len(&self, txs: &TxVecReader<'a>) -> Result<(), Error> {
        if txs.len() == 0 {
            return Err(Error::IncorrectInvalidTxsSize);
        }
        Ok(())
    }

    /// verify txs
    /// return tx_with_hashes for later use
    fn verify_txs_root(&self, block: &AgBlockReader<'a>, txs: &[TxWithHash]) -> Result<(), Error> {
        let leaves: Vec<_> = {
            txs.iter()
                .enumerate()
                .map(|(i, tx)| (i, tx.tx_hash.clone()))
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
        if &calculated_root != block.previous_account_root().raw_data() {
            return Err(Error::InvalidAccountMerkleProof);
        }
        Ok(())
    }

    /// return Ok if the block state is invalid
    fn verify_invalid_state(
        &self,
        state: &mut State,
        block: &AgBlockReader<'a>,
        tx_with_hashes: Vec<TxWithHash>,
        accounts_count: u32,
        accounts_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        let executor = Executor::new();
        let ag_index: u32 = block.ag_index().unpack();
        for tx in tx_with_hashes {
            if executor.run(state, tx, ag_index).is_err() {
                // errors occured, represents the block is invalid
                return Ok(());
            }
        }
        // check new account root
        let leaves: Vec<_> =
            accounts_to_proof_leaves(state.iter().map(|(account, _kv)| account.as_reader()));
        let calculated_root = compute_account_root(leaves, accounts_count, accounts_proof)
            .map_err(|_| Error::InvalidAccountMerkleProof)?;
        if &calculated_root != block.current_account_root().raw_data() {
            // block is invalid
            return Ok(());
        }
        Err(Error::TryRevertValidBlock)
    }

    pub fn calculate_reverted_account_root(
        &self,
        state: &mut State,
        ag_index: u32,
        challenger_index: u32,
        accounts_count: u32,
        accounts_proof: Vec<[u8; 32]>,
    ) -> Result<[u8; 32], Error> {
        let (ag, ag_kv) = state.get_account(ag_index).ok_or(Error::MissingAgAccount)?;
        let (chal, chal_kv) = state
            .get_account(challenger_index)
            .ok_or(Error::MissingChallengerAccount)?;
        // calculate reward
        let reward_amount = {
            let balance: u64 = ag_kv.get(&CKB_TOKEN_ID).map(|b| *b).unwrap_or(0);
            balance.saturating_mul(CHALLENGE_REWARD_RATE.0) / CHALLENGE_REWARD_RATE.1
        };
        let chal_balance: u64 = chal_kv.get(&CKB_TOKEN_ID).map(|b| *b).unwrap_or(0);

        let ag_index: u32 = ag.index().unpack();
        let chal_index: u32 = chal.index().unpack();

        state
            .update_account_state(ag_index, CKB_TOKEN_ID, 0)
            .expect("update aggregator");
        state
            .update_account_state(
                chal_index,
                CKB_TOKEN_ID,
                chal_balance.saturating_add(reward_amount),
            )
            .expect("update challenger");
        state.sync_state().expect("compute account state");

        let leaves =
            accounts_to_proof_leaves(state.iter().map(|(account, _kv)| account.as_reader()));
        let account_root = compute_account_root(leaves, accounts_count, accounts_proof)
            .map_err(|_| Error::InvalidAccountMerkleProof)?;
        Ok(account_root)
    }

    pub fn verify_penalize_and_new_state(
        &self,
        state: &mut State,
        block: &AgBlockReader<'a>,
        accounts_count: u32,
        accounts_proof: Vec<[u8; 32]>,
        block_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        let ag_index: u32 = block.ag_index().unpack();
        let challenger_index: u32 = self.action.challenger_index().unpack();
        let account_root = self.calculate_reverted_account_root(
            state,
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
        let mut state = State::new(
            self.action
                .touched_accounts()
                .iter()
                .zip(self.action.touched_accounts_kv().iter())
                .zip(self.action.touched_accounts_kv_proof().iter())
                .map(|((account, kv), kv_proof)| {
                    let kv: KVMap = kv.unpack();
                    let leaves_path = kv_proof.leaves_path().unpack();
                    let proof: Vec<([u8; 32], u8)> = kv_proof.proof().unpack();
                    AccountWithKV {
                        account,
                        kv,
                        proof,
                        leaves_path,
                    }
                })
                .collect(),
        );
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
        self.verify_txs_len(&txs)?;
        let tx_with_hashes = build_tx_hashes(&txs);
        self.verify_txs_root(&block, &tx_with_hashes)?;
        self.verify_accounts(&block, accounts_count, accounts_proof.clone())?;
        self.verify_invalid_state(
            &mut state,
            &block,
            tx_with_hashes,
            accounts_count,
            accounts_proof.clone(),
        )?;
        self.verify_penalize_and_new_state(
            &mut state,
            &block,
            accounts_count,
            accounts_proof,
            block_proof,
        )?;
        Ok(())
    }
}

fn build_tx_hashes<'a>(txs: &'a TxVecReader<'a>) -> Vec<TxWithHash<'a>> {
    txs.iter()
        .map(|tx| {
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
