use crate::utils::hash::new_blake2b;
use merkle_cbt::{merkle_tree::Merge, CBMT as ExCBMT};

pub struct MergeByte32;

impl Merge for MergeByte32 {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut ret = [0u8; 32];
        let mut blake2b = new_blake2b();

        blake2b.update(&left[..]);
        blake2b.update(&right[..]);
        blake2b.finalize(&mut ret);
        ret
    }
}

pub type CBMT = ExCBMT<[u8; 32], MergeByte32>;

pub fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    CBMT::build_merkle_root(leaves)
}
