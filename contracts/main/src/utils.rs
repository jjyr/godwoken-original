use crate::constants::{Error, HASH_SIZE};
use ckb_contract_std::{ckb_constants::*, syscalls};
use core::mem::size_of;
use godwoken_types::{bytes::Bytes, packed::*, prelude::*};

const BUF_LEN: usize = 4096;

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

pub fn load_action() -> Result<Action, Error> {
    let buf = syscalls::load_witness(BUF_LEN, 0, 0, Source::GroupOutput).expect("load witness");
    let witness_args = WitnessArgs::new_unchecked(buf.into());
    witness_args
        .output_type()
        .to_opt()
        .ok_or(Error::InvalidWitness)
        .and_then(|buf| {
            let buf: Bytes = buf.unpack();
            match ActionReader::verify(&buf, false) {
                Ok(()) => Ok(Action::new_unchecked(buf)),
                Err(_) => Err(Error::InvalidWitness),
            }
        })
}

pub struct CapacityChange {
    pub input: u64,
    pub output: u64,
}

/* fetch input cell's capacity and output cell's capacity */
pub fn fetch_capacities() -> CapacityChange {
    let input = {
        let raw = syscalls::load_cell_by_field(
            size_of::<u64>(),
            0,
            0,
            Source::Input,
            CellField::Capacity,
        )
        .expect("load capacity");
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&raw);
        u64::from_le_bytes(buf)
    };
    let output = {
        let raw = syscalls::load_cell_by_field(
            size_of::<u64>(),
            0,
            0,
            Source::Output,
            CellField::Capacity,
        )
        .expect("load capacity");
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&raw);
        u64::from_le_bytes(buf)
    };

    CapacityChange { input, output }
}
