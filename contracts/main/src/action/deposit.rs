use crate::{error::Error, utils};
use alloc::vec;
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};

pub struct DepositVerifier<'a> {
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
    action: DepositReader<'a>,
}

impl<'a> DepositVerifier<'a> {
    /// verify account state
    fn verify_account(&self, deposit_capacity: u64) -> Result<(), Error> {
        let old_account = self.action.old_account();
        let new_account = self.action.new_account();
        let account_index: u32 = old_account.index().unpack();
        if account_index != new_account.index().unpack() {
            Err(Error::InvalidAccountIndex)?;
        }
        if old_account.script().as_slice() != new_account.script().as_slice() {
            Err(Error::InvalidAccountScript)?;
        }
        let nonce: u32 = old_account.nonce().unpack();
        if nonce != new_account.nonce().unpack() {
            Err(Error::InvalidAccountNonce)?;
        }
        let balance = old_account.balance().unpack() + deposit_capacity;
        if balance != new_account.balance().unpack() {
            Err(Error::InvalidAccountBalance)?;
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
        // verify old_account
        let old_account = self.action.old_account();
        let old_account_root = self.old_state.account_root().unpack();
        verify_account_state(
            old_account,
            &old_account_root,
            entries_count,
            proof_items.clone(),
        )?;
        // verify new_account
        let new_account = self.action.new_account();
        let new_account_root = self.new_state.account_root().unpack();
        verify_account_state(new_account, &new_account_root, entries_count, proof_items)?;
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
        self.verify_account(deposit_capacity)?;
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

/// verify account state according to merkle root
fn verify_account_state<'a>(
    account: AccountReader<'a>,
    account_root: &[u8; 32],
    entries_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<(), Error> {
    let account_hash = {
        let mut hash = [0u8; 32];
        let mut hasher = utils::new_blake2b();
        hasher.update(account.as_slice());
        hasher.finalize(&mut hash);
        hash
    };
    let account_index: u32 = account.index().unpack();
    let calculated_account_root = utils::compute_account_root(
        vec![(account_index as usize, account_hash)],
        entries_count,
        proof_items,
    )?;
    if &calculated_account_root != account_root {
        return Err(Error::InvalidAccountMerkleProof);
    }
    Ok(())
}
