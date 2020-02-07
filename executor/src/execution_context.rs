use crate::{error::Error, state::State};
use godwoken_types::{packed::*, prelude::*};

pub struct ExecutionContext<'a> {
    state: &'a mut State,
    sender_index: u32,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(state: &'a mut State, sender_index: u32) -> Self {
        ExecutionContext {
            state,
            sender_index,
        }
    }

    pub fn sender(&self) -> Result<&Account, Error> {
        self.state
            .get_account(self.sender_index)
            .ok_or(Error::MissingAccount(self.sender_index))
    }

    pub fn transfer(&mut self, receiver_index: u32, amount: u64) -> Result<(), Error> {
        // check sender
        let sender = self.sender()?;
        let sender_balance: u64 = sender.balance().unpack();
        if sender_balance < amount.into() {
            return Err(Error::BalanceNotEnough(sender_balance, amount));
        }
        let new_sender_balance = sender_balance + amount;
        // check receiver
        let receiver = self
            .state
            .get_account(receiver_index)
            .ok_or(Error::MissingAccount(receiver_index))?;
        let receiver_balance: u64 = receiver.balance().unpack();
        let new_receiver_balance = receiver_balance
            .checked_add(amount)
            .ok_or(Error::BalanceOverflow)?;
        // uodate account balance
        self.state
            .update_account_balance(self.sender_index, new_sender_balance)?;
        self.state
            .update_account_balance(receiver_index, new_receiver_balance)?;
        Ok(())
    }
}
