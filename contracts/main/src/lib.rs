#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
mod libc_alloc;

use godwoken_types::{packed::*, prelude::*};

#[global_allocator]
static HEAP: libc_alloc::LibCAllocator = libc_alloc::LibCAllocator;

#[alloc_error_handler]
fn oom_handler(_: core::alloc::Layout) -> ! {
    extern "C" { fn abort() -> !; }
    unsafe { abort() }
}

#[no_mangle]
fn contract_entry() -> isize {
    let block = GlobalState::new_builder().build();
    if block.as_slice().len() == 64 {
        return 0;
    } else {
        return -1;
    }
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
