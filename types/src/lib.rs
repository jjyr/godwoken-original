#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod cache;
mod conversion;
pub mod core;
mod extension;
#[doc(hidden)]
mod generated;
pub mod prelude;
pub use generated::packed;

//re-exports
pub use molecule::bytes;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::collections;
        use std::vec;
    } else {
        use alloc::collections;
        use alloc::vec;
    }
}
