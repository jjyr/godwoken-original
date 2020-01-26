use crate::{constants::NEW_ACCOUNT_REQUIRED_BALANCE, error::Error, utils};
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};

pub struct RegisterVerifier<'a> {
    action: RegisterReader<'a>,
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
}

impl<'a> RegisterVerifier<'a> {
    pub fn new(
        old_state: GlobalStateReader<'a>,
        new_state: GlobalStateReader<'a>,
        register_action: RegisterReader<'a>,
    ) -> RegisterVerifier<'a> {
        RegisterVerifier {
            old_state,
            new_state,
            action: register_action,
        }
    }

    /// verify entry state
    fn verify_entry(&self, deposit_capacity: u64) -> Result<(), Error> {
        let entry = self.action.entry();
        if entry.is_aggregator() {
            utils::check_aggregator(&entry)?;
        }
        let nonce: u32 = entry.nonce().unpack();
        if nonce != 0 {
            Err(Error::InvalidEntryNonce)?;
        }
        let balance = entry.balance().unpack();
        if balance != deposit_capacity || balance < NEW_ACCOUNT_REQUIRED_BALANCE {
            Err(Error::InvalidEntryBalance)?;
        }
        Ok(())
    }

    fn verify_state_transition(&self) -> Result<(), Error> {
        if self.old_state.block_root().as_slice() != self.new_state.block_root().as_slice() {
            return Err(Error::InvalidGlobalState);
        }

        let entry = self.action.entry();
        let new_index: u32 = entry.index().unpack();
        let old_account_root = self.old_state.account_root().unpack();
        let last_index = new_index - 1;
        let last_entry_hash = self.action.last_entry_hash().unpack();
        let proof_items: Vec<[u8; 32]> = self
            .action
            .proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        // verify old global state
        if new_index == 0 {
            if old_account_root != [0u8; 32] || proof_items.len() != 0 {
                return Err(Error::InvalidAccountMerkleProof);
            }
        } else {
            let old_entries_count = new_index;
            let calculated_root = utils::compute_account_root(
                last_entry_hash,
                last_index,
                old_entries_count,
                proof_items.clone(),
            )?;
            if old_account_root != calculated_root {
                return Err(Error::InvalidAccountMerkleProof);
            }
        }

        // verify new global state
        let new_entry_hash = {
            let mut hasher = utils::new_blake2b();
            hasher.update(entry.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };

        let calculated_root = utils::compute_new_account_root(
            last_entry_hash,
            last_index,
            new_entry_hash,
            new_index,
            new_index + 1,
            proof_items,
        )?;

        let new_account_root = self.new_state.account_root().unpack();
        if new_account_root != calculated_root {
            return Err(Error::InvalidAccountMerkleProof);
        }

        Ok(())
    }

    pub fn verify(&self) -> Result<(), Error> {
        let deposit_capacity = deposit_capacity()?;
        self.verify_entry(deposit_capacity)?;
        self.verify_state_transition()?;
        Ok(())
    }
}

/// deposit capacity
fn deposit_capacity() -> Result<u64, Error> {
    let capacities = utils::fetch_capacities();
    capacities
        .output
        .checked_sub(capacities.input)
        .ok_or(Error::IncorrectCapacity)
}
