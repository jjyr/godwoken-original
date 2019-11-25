mod test_main;
pub mod utils;

use ckb_script::{DataLoader, TransactionScriptsVerifier};
use ckb_types::{
    bytes::Bytes,
    core::{cell::CellMeta, BlockExt, EpochExt, HeaderView},
    packed::{Byte32, CellOutput, OutPoint},
};
use lazy_static::lazy_static;
use std::collections::HashMap;
use utils::{build_resolved_tx, TxBuilder};

lazy_static! {
    pub static ref DUMMY_LOCK_BIN: Bytes =
        Bytes::from(&include_bytes!("../../binary/dummy_lock")[..]);
    pub static ref MAIN_CONTRACT_BIN: Bytes = Bytes::from(&include_bytes!("../../binary/main")[..]);
}

pub const MAX_CYCLES: u64 = 500_000;

#[derive(Default)]
pub struct DummyDataLoader {
    pub cells: HashMap<OutPoint, (CellOutput, Bytes)>,
    pub headers: HashMap<Byte32, HeaderView>,
    pub epoches: HashMap<Byte32, EpochExt>,
}

impl DummyDataLoader {
    fn new() -> Self {
        Self::default()
    }
}

impl DataLoader for DummyDataLoader {
    // load Cell Data
    fn load_cell_data(&self, cell: &CellMeta) -> Option<(Bytes, Byte32)> {
        cell.mem_cell_data.clone().or_else(|| {
            self.cells
                .get(&cell.out_point)
                .map(|(_, data)| (data.clone(), CellOutput::calc_data_hash(&data)))
        })
    }
    // load BlockExt
    fn get_block_ext(&self, _hash: &Byte32) -> Option<BlockExt> {
        unreachable!()
    }

    // load header
    fn get_header(&self, block_hash: &Byte32) -> Option<HeaderView> {
        self.headers.get(block_hash).cloned()
    }

    // load EpochExt
    fn get_block_epoch(&self, block_hash: &Byte32) -> Option<EpochExt> {
        self.epoches.get(block_hash).cloned()
    }
}

#[test]
fn test_dummy_lock() {
    const DUMMY_LOCK_CYCLES: u64 = 2108;
    let mut data_loader = DummyDataLoader::new();
    let tx = TxBuilder::default()
        .lock_bin(DUMMY_LOCK_BIN.clone())
        .build(&mut data_loader);
    let resolved_tx = build_resolved_tx(&data_loader, &tx);
    let verify_result =
        TransactionScriptsVerifier::new(&resolved_tx, &data_loader).verify(MAX_CYCLES);
    let cycles = verify_result.expect("pass verification");
    assert_eq!(cycles, DUMMY_LOCK_CYCLES);
}
