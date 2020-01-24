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
        let account_mmr_size: u64 = self.action.account_mmr_size().unpack();
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
        let ag_pubkey_hash = ag_entry.pubkey_hash().unpack();
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
        Ok(())
    }

    fn check_state_transition(&self) -> Result<(), Error> {
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
