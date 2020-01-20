use crate::constants::Error;
use blake2b_ref::{Blake2b, Blake2bBuilder};
use ckb_merkle_mountain_range::{Merge, leaf_index_to_mmr_size, leaf_index_to_pos, MerkleProof};
use alloc::vec::Vec;

pub const CKB_HASH_PERSONALIZATION: &[u8] = b"ckb-default-hash";

pub fn new_blake2b() -> Blake2b {
    Blake2bBuilder::new(32)
        .personal(CKB_HASH_PERSONALIZATION)
        .build()
}

pub struct HashMerge;

impl Merge for HashMerge {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut merge_result = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize(&mut merge_result);
        merge_result
    }
}

/// Compute account root from merkle proof
pub fn compute_account_root(entry_hash: [u8;32], entry_index: u32,  entries_count: u32, proof_items: Vec<[u8;32]>) -> Result<[u8;32], Error> {
    let mmr_size = leaf_index_to_mmr_size((entries_count - 1) as u64);
    let entry_pos = leaf_index_to_pos(entry_index as u64);
    let proof = MerkleProof::<_, HashMerge>::new(mmr_size, proof_items);
    let root = proof.calculate_root(entry_pos, entry_hash).map_err(|_| Error::InvalidMerkleProof)?;
    // calculate account_root: H(count | account entries root)
    let mut account_root = [0u8;32];
    let mut hasher = new_blake2b();
    hasher.update(&entries_count.to_le_bytes());
    hasher.update(&root);
    hasher.finalize(&mut account_root);
    Ok(account_root)
}
