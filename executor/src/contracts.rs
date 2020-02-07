/// Built-in contracts
use crate::{error::Error, execution_context::ExecutionContext, traits::Contract};
use godwoken_types::{cache::TxWithHash, packed::*, prelude::*};

#[derive(Default)]
pub struct BasicAccount;

fn verify_signature<'a>(account: &AccountReader<'a>, tx: &TxWithHash) -> Result<(), Error> {
    let account_args = account.script().args().raw_data();
    let witness = tx.raw.witness().raw_data();
    godwoken_utils::secp256k1::verify_signature(&witness, &tx.tx_hash, &account_args)
        .map_err(|_| Error::InvalidSignature)
}

#[repr(u8)]
enum ErrorNum {
    InvalidArgs,
}

impl Contract for BasicAccount {
    fn call(&mut self, context: &mut ExecutionContext, tx: &TxWithHash) -> Result<(), Error> {
        let sender = context.sender()?;
        verify_signature(&sender.as_reader(), tx)?;
        let tx_args = tx.raw.args().raw_data();
        let tx_args = match BasicAccountArgsReader::verify(&tx_args, false) {
            Ok(()) => BasicAccountArgs::new_unchecked(tx_args.into()),
            Err(_) => return Err(Error::ContractCall(ErrorNum::InvalidArgs as u8)),
        };
        match tx_args.as_reader().to_enum() {
            BasicAccountArgsUnionReader::BATransfer(transfer) => {
                context.transfer(transfer.to().unpack(), transfer.amount().unpack())?;
            }
        }
        Ok(())
    }
}
