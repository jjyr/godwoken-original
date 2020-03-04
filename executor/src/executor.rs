use crate::{error::Error, execution_context::ExecutionContext, state::State};
use godwoken_types::{cache::TxWithHash, packed::*, prelude::*};

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Executor {}
    }

    fn verify_tx<'a>(&self, sender: &Account, tx: &TxWithHash) -> Result<(), Error> {
        // check nonce
        let nonce: u32 = sender.nonce().unpack();
        let tx_nonce = tx.raw.nonce().unpack();
        if nonce + 1 != tx_nonce {
            return Err(Error::InvalidNonce(nonce + 1, tx_nonce));
        }
        // check signature
        let pubkey_hash = sender.pubkey_hash().raw_data();
        let witness = tx.raw.witness().raw_data();
        godwoken_utils::secp256k1::verify_signature(&witness, &tx.tx_hash, &pubkey_hash)
            .map_err(|_| Error::InvalidSignature)?;
        Ok(())
    }

    pub fn run(&self, state: &mut State, tx: TxWithHash, ag_index: u64) -> Result<(), Error> {
        let sender_index: u64 = tx.raw.sender_index().unpack();
        let to_index: u64 = tx.raw.to_index().unpack();
        let (sender, _kv) = state
            .get_account(sender_index)
            .ok_or(Error::MissingAccount(sender_index))?;
        self.verify_tx(&sender, &tx)?;
        if sender.script().to_opt().is_some() {
            // do not support contract yet
            return Err(Error::ContractCall(1));
        }
        let mut context = ExecutionContext::new(state, sender_index);
        // charge tx fee
        context.transfer(ag_index, tx.raw.fee())?;
        // transfer
        context.transfer(to_index, tx.raw.amount())?;
        state.inc_nonce(sender_index)?;
        Ok(())
    }
}
