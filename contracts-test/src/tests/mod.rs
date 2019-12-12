#[macro_use]
mod types_utils;
mod test_main;
mod utils;

use ckb_script::DataLoader;
use ckb_types::{
    bytes::Bytes,
    core::{cell::CellMeta, BlockExt, EpochExt, HeaderView},
    packed::{Byte32, CellOutput, OutPoint},
};
use lazy_static::lazy_static;
use std::collections::HashMap;
use utils::{verify_tx, ContractCallTxBuilder};

lazy_static! {
    pub static ref DUMMY_LOCK_BIN: Bytes =
        Bytes::from(&include_bytes!("../../../contracts/binaries/dummy_lock")[..]);
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
        cell.mem_cell_data
            .as_ref()
            .map(|(data, hash)| (Bytes::from(data.to_vec()), hash.to_owned()))
            .or_else(|| {
                self.cells.get(&cell.out_point).map(|(_, data)| {
                    (
                        Bytes::from(data.to_vec()),
                        CellOutput::calc_data_hash(&data),
                    )
                })
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
    const DUMMY_LOCK_CYCLES: u64 = 2155;
    let mut data_loader = DummyDataLoader::new();
    let tx = ContractCallTxBuilder::default()
        .lock_bin(DUMMY_LOCK_BIN.to_vec())
        .build(&mut data_loader);
    let verify_result = verify_tx(&data_loader, &tx);
    let cycles = verify_result.expect("pass verification");
    assert_eq!(cycles, DUMMY_LOCK_CYCLES);
}
