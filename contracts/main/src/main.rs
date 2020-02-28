#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

/// Main contract of Godwoken
/// This contract maintains the global state of accounts and blocks.
///
/// The main contract works like a state machine; operators update the global state through actions;
/// Each action state transition is trackable, it's for later challenge that required by Optimistic Rollup.
/// The end users transactions is not verified on-chain, instead the operator must verify them;
/// once operator submit a invalid action, that anyone watch the chain can send a challenge transaction to penalize operator's deposited coins.
///
/// This contract gurantee that anyone can read the operator ID, state transition action, and the global state from the chain for an invalid state transition `apply(S1, txs) -> S2`.
///
/// State transition actions:
///
/// 1. Registration
/// 2. Deposit
/// 3. Witdraw
/// 4. Send Tx
mod action;
mod common;
mod constants;
mod error;

use crate::common::{check_output, load_action, load_global_state};
use crate::constants::HASH_SIZE;
use crate::error::Error;
use alloc::format;
use ckb_contract_std::{ckb_constants::*, setup, syscalls};
use godwoken_types::packed::*;

#[no_mangle]
fn main() -> i8 {
    match contract_entry() {
        Ok(()) => 0,
        Err(err) => err as i8,
    }
}

setup!(main);

fn contract_entry() -> Result<(), Error> {
    // try get input type_hash
    if let Ok(type_hash) =
        syscalls::load_cell_by_field(HASH_SIZE, 0, 0, Source::GroupInput, CellField::TypeHash)
    {
        let lock_hash =
            syscalls::load_cell_by_field(HASH_SIZE, 0, 0, Source::GroupInput, CellField::LockHash)
                .expect("get lock hash");
        // do input verification
        // just check the output has same type constraint
        return check_output(&type_hash, &lock_hash);
    }
    // do output verification
    let action = load_action()?;
    let old_global_state = load_global_state(Source::Input)?;
    let new_global_state = load_global_state(Source::Output)?;
    match action.as_reader().to_enum() {
        ActionUnionReader::Deposit(deposit) => {
            crate::action::deposit::DepositVerifier::new(
                old_global_state.as_reader(),
                new_global_state.as_reader(),
                deposit,
            )
            .verify()?;
        }
        ActionUnionReader::Register(register) => {
            crate::action::register::RegisterVerifier::new(
                old_global_state.as_reader(),
                new_global_state.as_reader(),
                register,
            )
            .verify()?;
        }
        ActionUnionReader::SubmitBlock(submit_block) => {
            crate::action::submit_block::SubmitBlockVerifier::new(
                old_global_state.as_reader(),
                new_global_state.as_reader(),
                submit_block,
            )
            .verify()?;
        }
        ActionUnionReader::InvalidBlock(invalid_block) => {
            crate::action::invalid_block::InvalidBlockVerifier::new(
                old_global_state.as_reader(),
                new_global_state.as_reader(),
                invalid_block,
            )
            .verify()?;
        }
    }
    Ok(())
}
