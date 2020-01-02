use crate::constants::Error;
use godwoken_types::{packed::*, prelude::*};

pub struct DepositVerifier(Deposit);

impl DepositVerifier {
    pub fn new(deposit_action: Deposit) -> Self {
        DepositVerifier(deposit_action)
    }

    pub fn verify(&self) -> Result<(), Error> {
        Ok(())
    }
}
