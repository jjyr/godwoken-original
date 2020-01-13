use crate::{constants::Error, utils};
use godwoken_types::{packed::*, prelude::*};

pub struct DepositVerifier<'a>(DepositReader<'a>);

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
        let old_entry = self.0.old_entry();
        let new_entry = self.0.new_entry();
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

    fn verify_state(&self) -> Result<(), Error> {
        // TODO
        Ok(())
    }

    fn verify_state_transition(&self) -> Result<(), Error> {
        // TODO
        Ok(())
    }
}

impl<'a> DepositVerifier<'a> {
    pub fn new(deposit_action: DepositReader<'a>) -> DepositVerifier<'a> {
        DepositVerifier(deposit_action)
    }

    pub fn verify(&self) -> Result<(), Error> {
        let deposit_capacity = self.deposit_capacity()?;
        self.verify_entry(deposit_capacity)?;
        self.verify_state()?;
        self.verify_state_transition()?;
        Ok(())
    }
}
