#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

use ckb_contract_std::setup;

#[no_mangle]
fn main() -> i8 {
    0
}

setup!(main);
