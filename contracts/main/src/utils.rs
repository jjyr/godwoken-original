use crate::constants::{Error, HASH_SIZE};
use ckb_contract_std::{ckb_constants::*, syscalls};
use godwoken_types::{packed::*, prelude::*};

/// check output type hash, make sure there is exactly one contract in output
pub fn check_output_type_hash(type_hash: &[u8]) -> Result<(), Error> {
    let mut output_contracts = 0;
    for i in 0.. {
        match syscalls::load_cell_by_field(HASH_SIZE, 0, i, Source::Output, CellField::TypeHash) {
            Ok(output_type_hash) => {
                if type_hash == &output_type_hash[..] {
                    output_contracts += 1;
                }
                if output_contracts > 1 {
                    return Err(Error::InvalidOutputTypeHash);
                }
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(_) => continue,
        }
    }
    if output_contracts != 1 {
        return Err(Error::InvalidOutputTypeHash);
    }
    Ok(())
}
