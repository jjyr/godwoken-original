use crate::constants::Error;
use godwoken_types::{packed::*, prelude::*};

pub struct RegisterVerifier<'a>(RegisterReader<'a>);

impl<'a> RegisterVerifier<'a> {
    pub fn new(register: RegisterReader<'a>) -> Self {
        RegisterVerifier(register)
    }

    pub fn verify(&self) -> Result<(), Error> {
        Ok(())
    }
}
