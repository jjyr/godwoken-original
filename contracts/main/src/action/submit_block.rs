use crate::error::Error;
use godwoken_types::{packed::*, prelude::*};

pub struct SubmitBlockVerifier<'a>(SubmitBlockReader<'a>);

impl<'a> SubmitBlockVerifier<'a> {
    pub fn new(submit_block: SubmitBlockReader<'a>) -> Self {
        SubmitBlockVerifier(submit_block)
    }

    pub fn verify(&self) -> Result<(), Error> {
        Ok(())
    }
}
