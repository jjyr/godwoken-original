use crate::{common, constants::CKB_TOKEN_ID, error::Error};
use alloc::vec::Vec;
use godwoken_types::{cache::KVMap, core::Index, packed::*, prelude::*};
use godwoken_utils::smt;

pub struct DepositVerifier<'a> {
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
    action: DepositReader<'a>,
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
        let account = self.action.account();
        let deposit_capacity = deposit_capacity()?;
        let index: Index = account.index().unpack();
        let mut kv: KVMap = self.action.token_kv().unpack();
        let proof = self.action.proof();
        let leaves_path = proof.leaves_path().unpack();
        let merkle_branches: Vec<([u8; 32], u8)> = proof.proof().unpack();
        let merkle_branches: Vec<(smt::H256, u8)> = merkle_branches
            .into_iter()
            .map(|(n, h)| (n.into(), h))
            .collect();

        // verify old state
        let old_account_root = self.old_state.account_root().unpack();
        common::verify_account_root(
            index,
            Some(account),
            kv.clone(),
            leaves_path.clone(),
            merkle_branches.clone(),
            &old_account_root,
        )?;

        // update balance
        let balance = kv.get(&CKB_TOKEN_ID).map(|balance| *balance).unwrap_or(0);
        let new_balance = balance + deposit_capacity;
        kv.insert(CKB_TOKEN_ID, new_balance);

        // verify new state
        let new_account_root = self.new_state.account_root().unpack();
        common::verify_account_root(
            index,
            Some(account),
            kv,
            leaves_path,
            merkle_branches,
            &new_account_root,
        )?;

        // verify global state
        let expected_state = self
            .old_state
            .to_entity()
            .as_builder()
            .account_root(new_account_root.pack())
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
