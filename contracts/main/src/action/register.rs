use crate::{
    common,
    constants::{CKB_TOKEN_ID, NEW_ACCOUNT_REQUIRED_BALANCE},
    error::Error,
};
use alloc::vec::Vec;
use godwoken_types::{cache::KVMap, packed::*, prelude::*};

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
        account: AccountReader<'a>,
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
        Ok(())
    }

    fn verify_account_state(&self, account: AccountReader<'a>, kv: KVMap) -> Result<(), Error> {
        let new_index: u64 = account.index().unpack();
        let old_account_count = self.old_state.account_count().unpack();
        if new_index != old_account_count {
            return Err(Error::InvalidAccountIndex);
        }

        let proof = self.action.proof();
        let leaves_path = proof.leaves_path().unpack();
        let merkle_branches: Vec<_> = Unpack::<Vec<([u8; 32], u8)>>::unpack(&proof.proof())
            .into_iter()
            .map(|(node, height)| (node.into(), height))
            .collect();
        // verify old state
        let old_account_root = self.old_state.account_root().unpack();
        if new_index == 0 {
            if old_account_root != [0u8; 32] {
                return Err(Error::InvalidAccountMerkleProof);
            }
        } else {
            let mut empty_kv = KVMap::default();
            empty_kv.insert(CKB_TOKEN_ID, 0);
            common::verify_account_root(
                new_index,
                None,
                empty_kv,
                leaves_path.clone(),
                merkle_branches.clone(),
                &old_account_root,
            )?;
        }

        // verify new state

        let new_account_root = self.new_state.account_root().unpack();
        common::verify_account_root(
            new_index,
            Some(account),
            kv,
            leaves_path,
            merkle_branches,
            &new_account_root,
        )?;

        let account_count: u64 = self.new_state.account_count().unpack();
        if account_count != old_account_count + 1 {
            return Err(Error::InvalidAccountCount);
        }

        Ok(())
    }

    pub fn verify(&self) -> Result<(), Error> {
        let account = self.action.account();
        let deposit_capacity = deposit_capacity()?;
        self.verify_account(account, deposit_capacity)?;
        let mut kv = KVMap::default();
        kv.insert(CKB_TOKEN_ID.into(), deposit_capacity.into());
        self.verify_account_state(account, kv)?;
        // verify global state
        let expected_state = self
            .old_state
            .to_entity()
            .as_builder()
            .account_root(self.new_state.account_root().to_entity())
            .account_count(self.new_state.account_count().to_entity())
            .build();
        if expected_state.as_slice() != self.new_state.as_slice() {
            return Err(Error::InvalidGlobalState);
        }
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
