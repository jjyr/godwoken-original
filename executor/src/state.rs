use crate::error::Error;
use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};

/// account states
pub struct State(Vec<Account>);

impl State {
    pub fn new(mut accounts: Vec<Account>) -> Self {
        accounts.sort_unstable_by_key(|account| {
            let index: u32 = account.index().unpack();
            index
        });
        State(accounts)
    }

    fn get_inner_index(&self, index: u32) -> Result<usize, usize> {
        self.0
            .binary_search_by_key(&index, |account| account.index().unpack())
    }

    pub fn get_account(&self, index: u32) -> Option<&Account> {
        self.get_inner_index(index).ok().and_then(|i| self.0.get(i))
    }

    fn update_account<F: FnOnce(&Account) -> Account>(
        &mut self,
        index: u32,
        f: F,
    ) -> Result<(), Error> {
        if let Ok(i) = self.get_inner_index(index) {
            let new_account = f(&self.0[i]);
            self.0[i] = new_account;
            Ok(())
        } else {
            Err(Error::MissingAccount(index))
        }
    }

    pub fn update_account_balance(&mut self, index: u32, balance: u64) -> Result<(), Error> {
        self.update_account(index, |account| {
            account
                .to_owned()
                .as_builder()
                .balance(balance.pack())
                .build()
        })
    }

    pub fn inc_nonce(&mut self, index: u32) -> Result<(), Error> {
        self.update_account(index, |account| {
            let nonce: u32 = account.nonce().unpack();
            let new_nonce: u32 = nonce.checked_add(1).expect("no overflow");
            account
                .to_owned()
                .as_builder()
                .nonce(new_nonce.pack())
                .build()
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Account> {
        self.0.iter()
    }
}
