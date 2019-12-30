#[macro_use]
mod types_utils;
mod test_main;

use ckb_contract_tool::{ckb_types::bytes::Bytes, Context, TxBuilder};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DUMMY_LOCK_BIN: Bytes =
        Bytes::from(&include_bytes!("../../../contracts/binaries/dummy_lock")[..]);
    pub static ref MAIN_CONTRACT_BIN: Bytes =
        Bytes::from(&include_bytes!("../../../contracts/binaries/main")[..]);
    pub static ref EXPERIMENTAL_BIN: Bytes =
        Bytes::from(&include_bytes!("../../../contracts/binaries/main")[..]);
}

pub const MAX_CYCLES: u64 = 500_000;

#[test]
fn test_dummy_lock() {
    const EXPECTED_CYCLES: u64 = 2155;
    let contract_bin = DUMMY_LOCK_BIN.to_owned();
    let mut context = Context::default();
    context.deploy_contract(contract_bin.clone());
    let tx = TxBuilder::default()
        .lock_bin(contract_bin)
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, EXPECTED_CYCLES);
    let cycles = verify_result.expect("pass verification");
    assert_eq!(cycles, EXPECTED_CYCLES);
}

#[test]
fn test_experimental_contract() {
    const EXPECTED_CYCLES: u64 = 7158;
    let contract_bin = EXPERIMENTAL_BIN.to_owned();
    let mut context = Context::default();
    context.deploy_contract(contract_bin.clone());
    let tx = TxBuilder::default()
        .lock_bin(contract_bin)
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, EXPECTED_CYCLES);
    let cycles = verify_result.expect("pass verification");
    assert_eq!(cycles, EXPECTED_CYCLES);
}
