/// Invalid Block
/// 1. proof block exists
/// 2. re-run block txs from previous state to invalid state
/// 3. penalize aggregator, reward challenger
use crate::error::Error;
use crate::utils;
use alloc::vec;
use alloc::vec::Vec;
use godwoken_types::{packed::*, prelude::*};

pub struct InvalidBlockVerifier<'a> {
    action: InvalidBlockReader<'a>,
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
}

impl<'a> InvalidBlockVerifier<'a> {
    pub fn new(
        old_state: GlobalStateReader<'a>,
        new_state: GlobalStateReader<'a>,
        invalid_block: InvalidBlockReader<'a>,
    ) -> InvalidBlockVerifier<'a> {
        InvalidBlockVerifier {
            action: invalid_block,
            old_state,
            new_state,
        }
    }

    pub fn verify(&self) -> Result<(), Error> {
        Ok(())
    }
}
