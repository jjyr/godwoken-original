use crate::{error::Error, state::State};
use godwoken_types::{cache::KVMap, packed::*, prelude::*};

const NATIVE_TOKEN_ID: [u8; 32] = [0u8; 32];

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

    pub fn sender(&self) -> Result<(&Account, &KVMap), Error> {
        self.state
            .get_account(self.sender_index)
            .ok_or(Error::MissingAccount(self.sender_index))
    }

    pub fn transfer<'r>(&mut self, to_index: u32, payment: PaymentReader<'r>) -> Result<(), Error> {
        // check sender
        let (_sender, sender_kv) = self.sender()?;
        // check receiver
        let (_receiver, receiver_kv) = self
            .state
            .get_account(to_index)
            .ok_or(Error::MissingAccount(to_index))?;

        // get token type and amount
        let (token_type, amount) = match payment.to_enum() {
            PaymentUnionReader::Uint32(amount) => {
                let amount: u64 = Unpack::<u32>::unpack(&amount).into();
                (NATIVE_TOKEN_ID, amount)
            }
            PaymentUnionReader::UDTPayment(udt_payment) => {
                let udt_type: [u8; 32] = udt_payment.type_hash().unpack();
                let amount: u64 = Unpack::<u32>::unpack(&udt_payment.amount()).into();
                (udt_type, amount)
            }
        };
        // calculate new balance
        let sender_balance: u64 = sender_kv.get(&token_type).map(|v| *v).unwrap_or(0);
        if sender_balance < amount {
            return Err(Error::BalanceNotEnough(sender_balance, amount));
        }
        let sender_balance = sender_balance - amount;
        let receiver_balance: u64 = receiver_kv.get(&token_type).map(|v| *v).unwrap_or(0);
        let receiver_balance = receiver_balance
            .checked_add(amount)
            .ok_or(Error::BalanceOverflow)?;

        // update account balance
        self.state
            .update_account_state(self.sender_index, token_type, sender_balance)?;
        self.state
            .update_account_state(to_index, token_type, receiver_balance)?;
        Ok(())
    }
}
