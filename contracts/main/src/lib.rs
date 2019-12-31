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
mod libc_alloc;
mod action;

use godwoken_types::{packed::*, prelude::*};
use ckb_contract_std::syscalls;

#[global_allocator]
static HEAP: libc_alloc::LibCAllocator = libc_alloc::LibCAllocator;

#[alloc_error_handler]
fn oom_handler(_: core::alloc::Layout) -> ! {
    extern "C" { fn abort() -> !; }
    unsafe { abort() }
}

#[no_mangle]
fn contract_entry() -> isize {
    let tx_hash = syscalls::load_tx_hash(32, 0);
    if 32 == tx_hash.map(|h| h.len()).unwrap() {
        return 0;
    } else {
        return -1;
    }
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    syscalls::exit(42);
    loop {}
}
