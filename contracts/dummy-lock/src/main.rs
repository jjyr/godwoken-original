#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

use ckb_std::{entry, default_alloc};

#[no_mangle]
fn main() -> i8 {
    0
}

entry!(main);
default_alloc!();
