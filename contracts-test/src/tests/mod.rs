#[macro_use]
mod utils;
mod dummy_lock;
mod main;

use ckb_tool::ckb_types::{bytes::Bytes, packed::CellOutput, prelude::*};
use lazy_static::lazy_static;

const DUMMY_LOCK_PATH: &str = "../contracts/binaries/dummy-lock";
const MAIN_CONTRACT_PATH: &str = "../contracts/binaries/godwoken-main";

lazy_static! {
    pub static ref DUMMY_LOCK_BIN: Bytes = std::fs::read(DUMMY_LOCK_PATH).expect("read").into();
    pub static ref MAIN_CONTRACT_BIN: Bytes =
        std::fs::read(MAIN_CONTRACT_PATH).expect("read").into();
    pub static ref DUMMY_LOCK_HASH: [u8; 32] = CellOutput::calc_data_hash(&DUMMY_LOCK_BIN).unpack();
    pub static ref MAIN_CONTRACT_HASH: [u8; 32] =
        CellOutput::calc_data_hash(&MAIN_CONTRACT_BIN).unpack();
}

pub const MAX_CYCLES: u64 = 30_000_000;
