use crate::tests::{DUMMY_LOCK_BIN, DUMMY_LOCK_HASH};
use ckb_tool::testtool::{context::Context, tx_builder::TxBuilder};
use godwoken_types::{core::ScriptHashType, packed::*, prelude::*};

#[test]
fn test_dummy_lock() {
    const EXPECTED_CYCLES: u64 = 6288;
    let mut context = Context::default();
    context.deploy_contract(DUMMY_LOCK_BIN.clone());
    let tx = TxBuilder::default()
        .lock_script(
            Script::new_builder()
                .code_hash(DUMMY_LOCK_HASH.pack())
                .hash_type(ScriptHashType::Data.into())
                .build()
                .as_slice()
                .to_owned()
                .into(),
        )
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, EXPECTED_CYCLES);
    let cycles = verify_result.expect("pass verification");
    assert_eq!(cycles, EXPECTED_CYCLES);
}
