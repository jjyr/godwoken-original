#![allow(clippy::all)]
#![allow(unused_imports)]

use ckb_types::packed as blockchain;
mod godwoken;

pub mod packed {
    pub use super::godwoken::*;
    pub use molecule::prelude::{Byte, ByteReader};
}
