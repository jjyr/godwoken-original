use crate::constants::Error;
use godwoken_types::{packed::*, prelude::*};

pub struct RegisterVerifier(Register);

impl RegisterVerifier {
    pub fn new(register: Register) -> Self {
        RegisterVerifier(register)
    }

    pub fn verify(&self) -> Result<(), Error> {
        Ok(())
    }
}
