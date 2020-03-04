use crate::error::Error;
use alloc::vec::Vec;
use godwoken_types::{cache::KVMap, packed::*, prelude::*};

struct AccountInner {
    account: Account,
    kv: KVMap,
    nonce: u32,
}

/// account states
pub struct State(Vec<AccountInner>);

impl State {
    pub fn new<'a>(mut accounts: Vec<(AccountReader<'a>, KVMap)>) -> Self {
        accounts.sort_unstable_by_key(|(account, _)| {
            let index: u64 = account.index().unpack();
            index
        });
        State(
            accounts
                .into_iter()
                .map(|(account, kv)| {
                    let nonce: u32 = account.nonce().unpack();
                    AccountInner {
                        account: account.to_entity(),
                        kv,
                        nonce,
                    }
                })
                .collect(),
        )
    }

    fn get_inner_index(&self, index: u64) -> Result<usize, usize> {
        self.0
            .binary_search_by_key(&index, |account| account.account.index().unpack())
    }

    pub fn get_account(&self, index: u64) -> Option<(&Account, &KVMap)> {
        self.get_inner_index(index)
            .ok()
            .and_then(|i| self.0.get(i))
            .map(|inner| (&inner.account, &inner.kv))
    }

    pub fn update_account_state(
        &mut self,
        index: u64,
        key: [u8; 32],
        value: u64,
    ) -> Result<(), Error> {
        let i = self
            .get_inner_index(index)
            .map_err(|_| Error::MissingAccount(index))?;
        self.0[i].kv.insert(key, value);
        Ok(())
    }

    pub fn inc_nonce(&mut self, index: u64) -> Result<(), Error> {
        let i = self
            .get_inner_index(index)
            .map_err(|_| Error::MissingAccount(index))?;
        let nonce = self.0[i].nonce;
        let new_nonce: u32 = nonce.checked_add(1).expect("no overflow");
        self.0[i].nonce = new_nonce;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Account, &KVMap)> {
        self.0.iter().map(|inner| (&inner.account, &inner.kv))
    }
}
