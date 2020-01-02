use crate::constants::Error;
use godwoken_types::{packed::*, prelude::*};

pub struct SubmitBlockVerifier(SubmitBlock);

impl SubmitBlockVerifier {
    pub fn new(submit_block: SubmitBlock) -> Self {
        SubmitBlockVerifier(submit_block)
    }

    pub fn verify(&self) -> Result<(), Error> {
        Ok(())
    }
}
