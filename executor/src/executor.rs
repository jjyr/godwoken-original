use crate::{
    contracts::BasicAccount, error::Error, execution_context::ExecutionContext, state::State,
    traits::Contract,
};
use alloc::boxed::Box;
use godwoken_types::{cache::TxWithHash, packed::*, prelude::*};

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Executor {}
    }

    fn load_contract(&self, code_hash: [u8; 32]) -> Option<Box<dyn Contract>> {
        // currently, only support one contract
        if code_hash == [0u8; 32] {
            return Some(Box::new(BasicAccount::default()));
        }
        None
    }

    fn verify_tx<'a>(&self, sender: &Account, tx: &TxReader<'a>) -> Result<(), Error> {
        let nonce: u32 = sender.nonce().unpack();
        if nonce + 1 != tx.nonce().unpack() {
            return Ok(());
        }
        Ok(())
    }

    fn charge_fee<'a>(
        &self,
        context: &mut ExecutionContext,
        tx: &TxReader<'a>,
        ag_index: u32,
    ) -> Result<(), Error> {
        let fee: u32 = tx.fee().unpack();
        context.transfer(ag_index, fee.into())
    }

    pub fn run(&self, state: &mut State, tx: TxWithHash, ag_index: u32) -> Result<(), Error> {
        // 1. find account
        // 2. load contract
        // 3. execute and update state
        let sender_index: u32 = tx.raw.account_index().unpack();
        let sender_account = state.get_account(sender_index).unwrap();
        // .ok_or(Error::MissingAccount)?;
        self.verify_tx(&sender_account, &tx.raw)?;
        // load contract
        let script = sender_account.script();
        let code_hash: [u8; 32] = script.code_hash().unpack();
        let mut contract = self.load_contract(code_hash).unwrap();
        let mut context = ExecutionContext::new(state, sender_index);
        // carge tx fee then call contract
        self.charge_fee(&mut context, &tx.raw, ag_index)?;
        contract.call(&mut context, &tx)?;
        state.inc_nonce(sender_index)?;
        Ok(())
    }
}
