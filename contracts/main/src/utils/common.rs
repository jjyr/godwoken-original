use crate::constants::{AGGREGATOR_CODE_HASH, AGGREGATOR_REQUIRED_BALANCE, HASH_SIZE};
use crate::error::Error;
use ckb_contract_std::{ckb_constants::*, syscalls};
use core::mem::size_of;
use godwoken_types::{bytes::Bytes, packed::*, prelude::*};

const BUF_LEN: usize = 4096;

pub fn check_aggregator<'a>(entry: &AccountEntryReader<'a>) -> Result<(), Error> {
    if !entry.is_aggregator() {
        return Err(Error::InvalidAggregator);
    }

    let balance: u64 = entry.balance().unpack();
    if balance < AGGREGATOR_REQUIRED_BALANCE {
        return Err(Error::InvalidAggregator);
    }

    let code_hash: [u8; 32] = entry.script().code_hash().unpack();
    if code_hash != AGGREGATOR_CODE_HASH {
        return Err(Error::InvalidAggregator);
    }
    Ok(())
}

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
    let witness_args = match WitnessArgsReader::verify(&buf, false) {
        Ok(()) => WitnessArgs::new_unchecked(buf.into()),
        Err(_) => return Err(Error::InvalidWitness),
    };
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

pub fn load_global_state(source: Source) -> Result<GlobalState, Error> {
    const GLOBAL_STATE_SIZE: usize = 64;

    let buf = syscalls::load_cell_data(GLOBAL_STATE_SIZE, 0, 0, source).expect("load global state");
    match GlobalStateReader::verify(&buf, false) {
        Ok(()) => Ok(GlobalState::new_unchecked(buf.into())),
        Err(_) => Err(Error::InvalidGlobalState),
    }
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
