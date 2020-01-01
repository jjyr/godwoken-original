#![no_std]
#![feature(alloc_error_handler)]

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
use crate::utils::check_output_type_hash;
use ckb_contract_std::{ckb_constants::*, syscalls};
use godwoken_types::{packed::*, prelude::*};

#[global_allocator]
static HEAP: libc_alloc::LibCAllocator = libc_alloc::LibCAllocator;

#[alloc_error_handler]
fn oom_handler(_: core::alloc::Layout) -> ! {
    extern "C" {
        fn abort() -> !;
    }
    unsafe { abort() }
}

#[no_mangle]
fn contract_entry() -> isize {
    match contract_main() {
        Ok(_) => 0,
        Err(err) => err as isize,
    }
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    syscalls::exit(42);
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
    Ok(())
}
