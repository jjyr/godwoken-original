#![no_std]

#[no_mangle]
fn contract_entry() -> isize {
    return 0;
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
