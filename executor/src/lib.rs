#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod error;
mod execution_context;
pub mod executor;
pub mod state;
pub mod traits;
mod types;
