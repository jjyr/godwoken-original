use super::DUMMY_LOCK_BIN;
use ckb_contract_tool::{Context, TxBuilder};

#[test]
fn test_dummy_lock() {
    const EXPECTED_CYCLES: u64 = 6288;
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
