use crate::{constants::Error, utils};
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};
use ckb_contract_std::debug;

pub struct DepositVerifier<'a> {
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
    action: DepositReader<'a>,
}

impl<'a> DepositVerifier<'a> {
    /// deposit capacity
    fn deposit_capacity(&self) -> Result<u64, Error> {
        let capacities = utils::fetch_capacities();
        capacities
            .output
            .checked_sub(capacities.input)
            .ok_or(Error::IncorrectCapacity)
    }

    /// verify entry state
    fn verify_entry(&self, deposit_capacity: u64) -> Result<(), Error> {
        let old_entry = self.action.old_entry();
        let new_entry = self.action.new_entry();
        let entry_index: u32 = old_entry.index().unpack();
        if entry_index != new_entry.index().unpack() {
            Err(Error::InvalidEntryIndex)?;
        }
        if old_entry.pubkey_hash().as_slice() != new_entry.pubkey_hash().as_slice() {
            Err(Error::InvalidEntryPubkeyHash)?;
        }
        let nonce: u32 = old_entry.nonce().unpack();
        if nonce != new_entry.nonce().unpack() {
            Err(Error::InvalidEntryNonce)?;
        }
        let balance = old_entry.balance().unpack() + deposit_capacity;
        if balance != new_entry.balance().unpack() {
            Err(Error::InvalidEntryBalance)?;
        }
        Ok(())
    }

    fn verify_current_state(&self) -> Result<(), Error> {
        use ckb_merkle_mountain_range::{leaf_index_to_pos, MerkleProof};
        let old_entry_hash = {
            let mut hash = [0u8; 32];
            let mut hasher = utils::new_blake2b();
            hasher.update(self.action.old_entry().as_slice());
            hasher.finalize(&mut hash);
            hash
        };
        let entries_count: u32 = self.action.count().unpack();
        let proof_items: Vec<[u8; 32]> = self
            .action
            .proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        let old_account_root = self.old_state.account_root().unpack();
        let calculated_account_root = utils::compute_account_root(old_entry_hash, entries_count - 1, entries_count, proof_items)?;
        if calculated_account_root != old_account_root
        {
            return Err(Error::InvalidMerkleProof);
        }
        Ok(())
    }

    fn verify_state_transition(&self) -> Result<(), Error> {
        // TODO
        Ok(())
    }
}

impl<'a> DepositVerifier<'a> {
    pub fn new(
        old_state: GlobalStateReader<'a>,
        new_state: GlobalStateReader<'a>,
        deposit_action: DepositReader<'a>,
    ) -> DepositVerifier<'a> {
        DepositVerifier {
            old_state,
            new_state,
            action: deposit_action,
        }
    }

    pub fn verify(&self) -> Result<(), Error> {
        let deposit_capacity = self.deposit_capacity()?;
        self.verify_entry(deposit_capacity)?;
        self.verify_current_state()?;
        self.verify_state_transition()?;
        Ok(())
    }
}
