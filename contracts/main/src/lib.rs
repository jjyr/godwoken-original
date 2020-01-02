#![no_std]
#![feature(alloc_error_handler, panic_info_message)]

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
extern crate alloc;
mod action;
mod constants;
mod libc_alloc;
mod utils;

use crate::constants::{Error, HASH_SIZE};
use crate::utils::{check_output_type_hash, load_action};
use alloc::{
    format,
    string::{String, ToString},
};
use ckb_contract_std::{ckb_constants::*, debug, syscalls};
use godwoken_types::{packed::*, prelude::*};

#[global_allocator]
static HEAP: libc_alloc::LibCAllocator = libc_alloc::LibCAllocator;

#[alloc_error_handler]
fn oom_handler(_: core::alloc::Layout) -> ! {
    syscalls::exit(Error::OutOfMemory as i8);
    loop {}
}

#[no_mangle]
fn contract_entry() -> isize {
    match contract_main() {
        Ok(_) => 0,
        Err(err) => err as isize,
    }
}

#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    let mut s = String::new();
    if let Some(p) = panic_info.payload().downcast_ref::<&str>() {
        s.push_str(&format!("panic occurred: {:?}", p));
    } else {
        s.push_str(&format!("panic occurred"));
    }
    if let Some(m) = panic_info.message() {
        s.push_str(&format!(" {:?}", m));
    }
    if let Some(location) = panic_info.location() {
        s.push_str(&format!(
            ", in file {}:{}",
            location.file(),
            location.line()
        ));
    } else {
        s.push_str(&format!(", but can't get location information..."));
    }

    syscalls::debug(s);
    syscalls::exit(Error::Panic as i8);
    loop {}
}

fn contract_main() -> Result<(), Error> {
    // try get input type_hash
    if let Ok(type_hash) =
        syscalls::load_cell_by_field(HASH_SIZE, 0, 0, Source::GroupInput, CellField::TypeHash)
    {
        // do input verification
        // just check the output has same type constraint
        return check_output_type_hash(&type_hash);
    }
    // do output verification
    let action = load_action().expect("load action");
    match action.to_enum() {
        ActionUnion::Deposit(deposit) => {
            crate::action::deposit::DepositVerifier::new(deposit).verify()?;
        }
        ActionUnion::Register(register) => {
            crate::action::register::RegisterVerifier::new(register).verify()?;
        }
        ActionUnion::SubmitBlock(submit_block) => {
            crate::action::submit_block::SubmitBlockVerifier::new(submit_block).verify()?;
        }
        _ => panic!("not support action {}", action.item_id()),
    }
    Ok(())
}
