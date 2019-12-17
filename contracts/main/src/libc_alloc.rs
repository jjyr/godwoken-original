use core::alloc::{GlobalAlloc, Layout};

pub struct LibCAllocator;

unsafe impl Sync for LibCAllocator {}

unsafe impl GlobalAlloc for LibCAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        extern "C" { 
            fn malloc(size: usize) -> *mut u8;
        }
        let size = layout.size();
        malloc(size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        extern "C" { fn free(ptr: *mut u8); }
        free(ptr)
    }
}
