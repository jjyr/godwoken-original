use crate::error::Error;
use crate::utils::hash::new_blake2b;
use alloc::vec::Vec;
use ckb_merkle_mountain_range::{leaf_index_to_mmr_size, leaf_index_to_pos, Merge, MerkleProof};

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
pub fn compute_account_root(
    entry_hash: [u8; 32],
    entry_index: u32,
    entries_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<[u8; 32], Error> {
    let mmr_size = leaf_index_to_mmr_size((entries_count - 1) as u64);
    let entry_pos = leaf_index_to_pos(entry_index as u64);
    let proof = MerkleProof::<_, HashMerge>::new(mmr_size, proof_items);
    let root = proof
        .calculate_root(entry_pos, entry_hash)
        .map_err(|_| Error::InvalidAccountMerkleProof)?;
    // calculate account_root: H(count | account entries root)
    let mut account_root = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&entries_count.to_le_bytes());
    hasher.update(&root);
    hasher.finalize(&mut account_root);
    Ok(account_root)
}
