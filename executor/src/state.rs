use crate::error::Error;
use alloc::{borrow::ToOwned, vec::Vec};
use godwoken_types::{
    cache::{AccountWithKV, KVMap},
    packed::*,
    prelude::*,
};
use godwoken_utils::smt::compute_root_with_proof;

struct AccountInner {
    account: Account,
    kv: KVMap,
    leaves_path: Vec<Vec<u8>>,
    proof: Vec<([u8; 32], u8)>,
    nonce: u32,
}

/// account states
pub struct State(Vec<AccountInner>);

impl State {
    pub fn new<'a>(mut accounts: Vec<AccountWithKV<'a>>) -> Self {
        accounts.sort_unstable_by_key(|account_with_kv| {
            let index: u32 = account_with_kv.account.index().unpack();
            index
        });
        State(
            accounts
                .into_iter()
                .map(|account_with_kv| {
                    let AccountWithKV {
                        account,
                        kv,
                        leaves_path,
                        proof,
                    } = account_with_kv;
                    let nonce: u32 = account.nonce().unpack();
                    AccountInner {
                        account: account.to_entity(),
                        kv,
                        proof,
                        leaves_path,
                        nonce,
                    }
                })
                .collect(),
        )
    }

    fn get_inner_index(&self, index: u32) -> Result<usize, usize> {
        self.0
            .binary_search_by_key(&index, |account| account.account.index().unpack())
    }

    pub fn get_account(&self, index: u32) -> Option<(&Account, &KVMap)> {
        self.get_inner_index(index)
            .ok()
            .and_then(|i| self.0.get(i))
            .map(|inner| (&inner.account, &inner.kv))
    }

    pub fn update_account_state(
        &mut self,
        index: u32,
        key: [u8; 32],
        value: u64,
    ) -> Result<(), Error> {
        let i = self
            .get_inner_index(index)
            .map_err(|_| Error::MissingAccount(index))?;
        self.0[i].kv.insert(key, value);
        Ok(())
    }

    pub fn inc_nonce(&mut self, index: u32) -> Result<(), Error> {
        let i = self
            .get_inner_index(index)
            .map_err(|_| Error::MissingAccount(index))?;
        let nonce = self.0[i].nonce;
        let new_nonce: u32 = nonce.checked_add(1).expect("no overflow");
        self.0[i].nonce = new_nonce;
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Account, &KVMap)> {
        self.0.iter().map(|inner| (&inner.account, &inner.kv))
    }

    /// update account
    pub fn sync_state(&mut self) -> Result<(), Error> {
        for i in 0..self.0.len() {
            let AccountInner {
                account,
                leaves_path,
                proof,
                kv,
                nonce,
            } = &self.0[i];
            let new_state_root =
                compute_root_with_proof(kv.to_owned(), leaves_path.to_owned(), proof.to_owned())
                    .map_err(|_| Error::InvalidMerkleProof)?;
            let new_account = account
                .to_owned()
                .as_builder()
                .state_root(new_state_root.pack())
                .nonce(nonce.pack())
                .build();
            self.0[i].account = new_account;
        }
        Ok(())
    }
}
