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
        if entry.is_ag() {
            utils::check_aggregator(&entry)?;
        }
        let nonce: u32 = entry.nonce().unpack();
        if nonce != 0 {
            Err(Error::InvalidEntryNonce)?;
        }
        let balance = entry.balance().unpack() + deposit_capacity;
        if balance != deposit_capacity || balance < NEW_ACCOUNT_REQUIRED_BALANCE {
            Err(Error::InvalidEntryBalance)?;
        }
        Ok(())
    }

    fn verify_state_transition(&self) -> Result<(), Error> {
        Ok(())
    }

    pub fn verify(&self) -> Result<(), Error> {
        let deposit_capacity = deposit_capacity()?;
        self.verify_entry(deposit_capacity);
        self.verify_state_transition();
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
