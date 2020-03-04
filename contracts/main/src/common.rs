/// common module contains serveral reusable functions
use crate::constants::{AGGREGATOR_REQUIRED_BALANCE, HASH_SIZE};
use crate::error::Error;
use alloc::vec::Vec;
use ckb_contract_std::{ckb_constants::*, syscalls};
use core::mem::size_of;
use godwoken_types::{bytes::Bytes, cache::KVMap, core::Index, packed::*, prelude::*};
use godwoken_utils::smt::{self, Value, ValueTrait};

const BUF_LEN: usize = 4096;

pub fn check_aggregator<'a>(account: AccountReader<'a>, balance: u64) -> Result<(), Error> {
    if balance < AGGREGATOR_REQUIRED_BALANCE {
        return Err(Error::InvalidAggregator);
    }
    if account.script().to_opt().is_some() {
        return Err(Error::InvalidAggregator);
    }
    Ok(())
}

/// check output type hash, make sure there is exactly one contract in output
pub fn check_output(type_hash: &[u8], lock_hash: &[u8]) -> Result<(), Error> {
    let mut n = 0;
    let mut contract_index = 0;
    // make sure there only 1 output has contract type hash
    for i in 0.. {
        match syscalls::load_cell_by_field(HASH_SIZE, 0, i, Source::Output, CellField::TypeHash) {
            Ok(output_type_hash) => {
                if type_hash == &output_type_hash[..] {
                    n += 1;
                    contract_index = i;
                }
                if n > 1 {
                    return Err(Error::InvalidOutputTypeHash);
                }
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(_) => continue,
        }
    }
    if n != 1 {
        return Err(Error::InvalidOutputTypeHash);
    }
    // check the lock hash
    if let Ok(output_lock_hash) = syscalls::load_cell_by_field(
        HASH_SIZE,
        0,
        contract_index,
        Source::Output,
        CellField::LockHash,
    ) {
        if &output_lock_hash[..] == lock_hash {
            return Ok(());
        }
    }
    Err(Error::InvalidOutputLockHash)
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
    const GLOBAL_STATE_SIZE: usize = 80;

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

/// verify account state according to merkle root
pub fn verify_account_root<'a>(
    index: Index,
    account: Option<AccountReader<'a>>,
    token_kv: KVMap,
    leaves_path: Vec<Vec<u8>>,
    merkle_branches: Vec<(smt::H256, u8)>,
    account_root: &[u8; 32],
) -> Result<(), Error> {
    // verify account and kv
    let mut leaves: Vec<_> = token_kv
        .iter()
        .map(|(k, v)| (smt::token_id_key(index, k), Value::from(*v).to_h256()))
        .collect();
    let value = account
        .map(|account| Value::from(account.to_entity()))
        .unwrap_or_else(|| Value::zero());
    leaves.push((smt::account_index_key(index.into()), value.to_h256()));
    let calculated_root: [u8; 32] =
        smt::compute_root_with_proof(leaves, leaves_path, merkle_branches.clone())
            .map_err(|_| Error::InvalidAccountMerkleProof)?
            .into();
    if &calculated_root != account_root {
        return Err(Error::InvalidAccountMerkleProof);
    }
    Ok(())
}
