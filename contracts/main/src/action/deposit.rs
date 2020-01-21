use crate::{error::Error, utils};
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};

pub struct DepositVerifier<'a> {
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
    action: DepositReader<'a>,
}

impl<'a> DepositVerifier<'a> {
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

    fn verify_state_transition(&self) -> Result<(), Error> {
        if self.old_state.block_root().as_slice() != self.new_state.block_root().as_slice() {
            return Err(Error::InvalidGlobalState);
        }
        let entries_count: u32 = self.action.count().unpack();
        let proof_items: Vec<[u8; 32]> = self
            .action
            .proof()
            .iter()
            .map(|item| item.unpack())
            .collect();
        // verify old_entry
        let old_entry = self.action.old_entry();
        let old_account_root = self.old_state.account_root().unpack();
        verify_entry_state(
            old_entry,
            &old_account_root,
            entries_count,
            proof_items.clone(),
        )?;
        // verify new_entry
        let new_entry = self.action.new_entry();
        let new_account_root = self.new_state.account_root().unpack();
        verify_entry_state(new_entry, &new_account_root, entries_count, proof_items)?;
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

/// verify entry state according to merkle root
fn verify_entry_state<'a>(
    entry: AccountEntryReader<'a>,
    account_root: &[u8; 32],
    entries_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<(), Error> {
    let entry_hash = {
        let mut hash = [0u8; 32];
        let mut hasher = utils::new_blake2b();
        hasher.update(entry.as_slice());
        hasher.finalize(&mut hash);
        hash
    };
    let entry_index = entry.index().unpack();
    let calculated_account_root =
        utils::compute_account_root(entry_hash, entry_index, entries_count, proof_items)?;
    if &calculated_account_root != account_root {
        return Err(Error::InvalidMerkleProof);
    }
    Ok(())
}
