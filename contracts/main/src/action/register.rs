use crate::{
    common,
    constants::{CKB_TOKEN_ID, NEW_ACCOUNT_REQUIRED_BALANCE},
    error::Error,
};
use alloc::vec;
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};
use godwoken_utils::{
    hash::new_blake2b,
    mmr::{compute_account_root, compute_new_account_root},
    smt,
};

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

    /// verify account state
    fn verify_account(
        &self,
        account: &AccountReader<'a>,
        deposit_capacity: u64,
    ) -> Result<(), Error> {
        let nonce: u32 = account.nonce().unpack();
        if nonce != 0 {
            Err(Error::InvalidAccountNonce)?;
        }
        // Godwoken do not supports contract yet
        if account.script().to_opt().is_some() {
            Err(Error::InvalidAccountScript)?;
        }
        if deposit_capacity < NEW_ACCOUNT_REQUIRED_BALANCE {
            Err(Error::InvalidDepositAmount)?;
        }
        let calculated_state_root: [u8; 32] =
            smt::compute_root(vec![(CKB_TOKEN_ID.into(), deposit_capacity)])
                .expect("compute state root")
                .into();
        let state_root: [u8; 32] = account.state_root().unpack();
        if calculated_state_root != state_root {
            Err(Error::InvalidAccountStateRoot)?;
        }
        Ok(())
    }

    fn verify_state_transition(&self, account: &AccountReader<'a>) -> Result<(), Error> {
        if self.old_state.block_root().as_slice() != self.new_state.block_root().as_slice() {
            return Err(Error::InvalidGlobalState);
        }

        let new_index: u32 = account.index().unpack();
        let old_account_root = self.old_state.account_root().unpack();
        let last_index = new_index - 1;
        let last_account_hash = self.action.last_account_hash().unpack();
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
            let calculated_root = compute_account_root(
                vec![(last_index as usize, last_account_hash)],
                old_entries_count,
                proof_items.clone(),
            )
            .map_err(|_| Error::InvalidAccountMerkleProof)?;
            if old_account_root != calculated_root {
                return Err(Error::InvalidAccountMerkleProof);
            }
        }

        // verify new global state
        let new_account_hash = {
            let mut hasher = new_blake2b();
            hasher.update(account.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };

        let calculated_root = compute_new_account_root(
            last_account_hash,
            last_index,
            new_account_hash,
            new_index,
            new_index + 1,
            proof_items,
        )
        .map_err(|_| Error::InvalidAccountMerkleProof)?;

        let new_account_root = self.new_state.account_root().unpack();
        if new_account_root != calculated_root {
            return Err(Error::InvalidAccountMerkleProof);
        }

        Ok(())
    }

    pub fn verify(&self) -> Result<(), Error> {
        let account = self.action.account();
        let deposit_capacity = deposit_capacity()?;
        self.verify_account(&account, deposit_capacity)?;
        self.verify_state_transition(&account)?;
        Ok(())
    }
}

/// deposit capacity
fn deposit_capacity() -> Result<u64, Error> {
    let capacities = common::fetch_capacities();
    capacities
        .output
        .checked_sub(capacities.input)
        .ok_or(Error::IncorrectCapacity)
}
