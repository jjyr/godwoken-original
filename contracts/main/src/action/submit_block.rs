use crate::error::Error;
use crate::{common, constants::CKB_TOKEN_ID};
use alloc::vec;
use alloc::vec::Vec;
use godwoken_types::{cache::KVMap, packed::*, prelude::*};
use godwoken_utils::{
    hash::new_blake2b,
    mmr::{compute_account_root, compute_block_root, compute_new_block_root, merkle_root},
    secp256k1::verify_signature,
    smt::compute_root_with_proof,
};

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
        let changes = common::fetch_capacities();
        if changes.input != changes.output {
            return Err(Error::IncorrectCapacity);
        }
        Ok(())
    }

    /// verify aggregator
    /// 1. aggregator is valid
    /// 2. aggregator exsits in account root
    /// 3. aggregator's signature is according to pubkey hash
    fn check_aggregator(&self, ag_account: &AccountReader<'a>) -> Result<(), Error> {
        let kv: KVMap = self.action.kv().unpack();
        let kv_proof = self.action.kv_proof();
        let leaves_path = kv_proof.leaves_path().unpack();
        let proof: Vec<([u8; 32], u8)> = kv_proof.proof().unpack();
        let calculated_state_root = compute_root_with_proof(kv.clone(), leaves_path, proof)
            .map_err(|_| Error::InvalidKVMerkleProof)?;
        if calculated_state_root != ag_account.state_root().unpack() {
            return Err(Error::InvalidKVMerkleProof)?;
        }
        let balance = kv.get(&CKB_TOKEN_ID).map(|b| *b).unwrap_or(0);
        common::check_aggregator(&ag_account, balance)?;
        // verify merkle proof of aggregator
        let account_count: u32 = self.action.account_count().unpack();
        let account_proof: Vec<[u8; 32]> = self
            .action
            .account_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let ag_index: u32 = ag_account.index().unpack();
        let account_hash = {
            let mut hasher = new_blake2b();
            hasher.update(ag_account.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let calculated_root = compute_account_root(
            vec![(ag_index as usize, account_hash)],
            account_count,
            account_proof,
        )
        .map_err(|_| Error::InvalidAccountMerkleProof)?;
        let old_account_root = self.old_state.account_root().unpack();
        if calculated_root != old_account_root {
            return Err(Error::InvalidAccountMerkleProof);
        }
        // verify aggregator's signature
        let ag_pubkey_hash = ag_account.pubkey_hash().raw_data();
        let block = self.action.block();
        let sig_message = {
            let sig_block = block
                .to_entity()
                .as_builder()
                .ag_sig(Byte65::default())
                .build();
            let mut hasher = new_blake2b();
            hasher.update(sig_block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let ag_sig = block.ag_sig().unpack();
        verify_signature(&ag_sig[..], &sig_message[..], ag_pubkey_hash)
            .map_err(|_| Error::InvalidSignature)?;
        Ok(())
    }

    fn check_block(&self, ag_account: &AccountReader<'a>) -> Result<(), Error> {
        let block = self.action.block();
        let block_ag_index: u32 = block.ag_index().unpack();
        let ag_index: u32 = ag_account.index().unpack();
        if block_ag_index != ag_index {
            return Err(Error::IncorrectAgIndex);
        }
        // verify block state
        if block.previous_account_root().as_slice() != self.old_state.account_root().as_slice() {
            return Err(Error::InvalidAccountRoot);
        }
        if block.current_account_root().as_slice() != self.new_state.account_root().as_slice() {
            return Err(Error::InvalidAccountRoot);
        }
        // verify tx root
        let tx_hashes: Vec<[u8; 32]> = self
            .action
            .txs()
            .iter()
            .map(|tx| {
                let mut hasher = new_blake2b();
                hasher.update(tx.as_slice());
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);
                hash
            })
            .collect();
        let calculated_tx_root = merkle_root(tx_hashes);
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
                return Err(Error::InvalidBlockMerkleProof);
            }
        } else {
            let calculated_root = compute_block_root(
                vec![(block_number as usize - 1, last_block_hash)],
                block_number + 1,
                block_proof.clone(),
            )
            .map_err(|_| Error::InvalidBlockMerkleProof)?;
            if old_block_root != calculated_root {
                return Err(Error::InvalidBlockMerkleProof);
            }
        }
        // verify new state merkle proof
        let block_hash = {
            let mut hasher = new_blake2b();
            hasher.update(block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let new_block_root = self.new_state.block_root().unpack();
        let calculated_root = compute_new_block_root(
            last_block_hash,
            block_number - 1,
            block_hash,
            block_number,
            block_number + 1,
            block_proof,
        )
        .map_err(|_| Error::InvalidBlockMerkleProof)?;
        if new_block_root != calculated_root {
            return Err(Error::InvalidBlockMerkleProof);
        }
        Ok(())
    }

    pub fn verify(&self) -> Result<(), Error> {
        let ag_account = self.action.ag_account();
        self.check_balance()?;
        self.check_aggregator(&ag_account)?;
        self.check_block(&ag_account)?;
        self.check_state_transition()?;
        Ok(())
    }
}
