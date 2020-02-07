use crate::{error::Error, execution_context::ExecutionContext};
use godwoken_types::cache::TxWithHash;

pub trait Contract {
    fn call(&mut self, context: &mut ExecutionContext, tx: &TxWithHash) -> Result<(), Error>;
}
