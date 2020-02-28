use crate::{common, constants::CKB_TOKEN_ID, error::Error};
use alloc::vec;
use alloc::vec::Vec;
use godwoken_types::{cache::KVMap, packed::*, prelude::*};
use godwoken_utils::{hash::new_blake2b, mmr::compute_account_root, smt::compute_root_with_proof};

pub struct DepositVerifier<'a> {
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
    action: DepositReader<'a>,
}

impl<'a> DepositVerifier<'a> {
    /// verify account state
    fn verify_account(
        &self,
        old_account: AccountReader<'a>,
        new_account: AccountReader<'a>,
        deposit_capacity: u64,
    ) -> Result<(), Error> {
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

        let mut kv_map: KVMap = self.action.old_kv().unpack();
        let kv_proof = self.action.kv_proof();
        let leaves_path = kv_proof.leaves_path().unpack();
        let proof: Vec<([u8; 32], u8)> = kv_proof.proof().unpack();
        let calculated_state_root =
            compute_root_with_proof(kv_map.clone(), leaves_path.clone(), proof.clone())
                .map_err(|_| Error::InvalidKVMerkleProof)?;
        let old_state_root: [u8; 32] = old_account.state_root().unpack();

        if calculated_state_root != old_state_root {
            return Err(Error::InvalidKVMerkleProof);
        }

        let balance = kv_map
            .get(&CKB_TOKEN_ID)
            .map(|balance| *balance)
            .unwrap_or(0);
        let new_balance = balance + deposit_capacity;
        kv_map.insert(CKB_TOKEN_ID, new_balance);
        let calculated_state_root = compute_root_with_proof(
            kv_map.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            leaves_path,
            proof,
        )
        .map_err(|_| Error::InvalidKVMerkleProof)?;
        let new_state_root: [u8; 32] = new_account.state_root().unpack();

        if calculated_state_root != new_state_root {
            return Err(Error::InvalidKVMerkleProof);
        }
        Ok(())
    }

    fn verify_state_transition(
        &self,
        old_account: AccountReader<'a>,
        new_account: AccountReader<'a>,
    ) -> Result<(), Error> {
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
        let old_account_root = self.old_state.account_root().unpack();
        verify_account_state(
            old_account,
            &old_account_root,
            entries_count,
            proof_items.clone(),
        )?;
        // verify new_account
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
        let old_account = self.action.old_account();
        let new_account = self.action.new_account();
        let deposit_capacity = deposit_capacity()?;
        self.verify_account(old_account, new_account, deposit_capacity)?;
        self.verify_state_transition(old_account, new_account)?;
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

/// verify account state according to merkle root
fn verify_account_state<'a>(
    account: AccountReader<'a>,
    account_root: &[u8; 32],
    entries_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<(), Error> {
    let account_hash = {
        let mut hash = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(account.as_slice());
        hasher.finalize(&mut hash);
        hash
    };
    let account_index: u32 = account.index().unpack();
    let calculated_account_root = compute_account_root(
        vec![(account_index as usize, account_hash)],
        entries_count,
        proof_items,
    )
    .map_err(|_| Error::InvalidAccountMerkleProof)?;
    if &calculated_account_root != account_root {
        return Err(Error::InvalidAccountMerkleProof);
    }
    Ok(())
}
