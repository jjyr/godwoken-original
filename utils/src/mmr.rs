use crate::hash::new_blake2b;
use alloc::vec;
use alloc::vec::Vec;
use ckb_merkle_mountain_range::{
    leaf_index_to_mmr_size, leaf_index_to_pos, util::MemMMR, Merge, MerkleProof,
};

pub use ckb_merkle_mountain_range::Error;

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
    accounts: Vec<(usize, [u8; 32])>,
    accounts_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<[u8; 32], Error> {
    let mmr_size = leaf_index_to_mmr_size((accounts_count - 1) as u64);
    let proof = MerkleProof::<_, HashMerge>::new(mmr_size, proof_items);
    let root = proof.calculate_root(
        accounts
            .into_iter()
            .map(|(i, hash)| {
                let pos = leaf_index_to_pos(i as u64);
                (pos, hash)
            })
            .collect(),
    )?;
    // calculate account_root: H(count | account entries root)
    let mut account_root = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&accounts_count.to_le_bytes());
    hasher.update(&root);
    hasher.finalize(&mut account_root);
    Ok(account_root)
}

/// Compute new account root from merkle proof
pub fn compute_new_account_root(
    entry_hash: [u8; 32],
    entry_index: u32,
    new_entry_hash: [u8; 32],
    new_entry_index: u32,
    accounts_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<[u8; 32], Error> {
    let root = if new_entry_index == 0 {
        new_entry_hash
    } else {
        let mmr_size = leaf_index_to_mmr_size((accounts_count - 2) as u64);
        let new_mmr_size = leaf_index_to_mmr_size(new_entry_index as u64);
        let entry_pos = leaf_index_to_pos(entry_index as u64);
        let new_entry_pos = leaf_index_to_pos(new_entry_index as u64);
        let proof = MerkleProof::<_, HashMerge>::new(mmr_size, proof_items);
        proof.calculate_root_with_new_leaf(
            vec![(entry_pos, entry_hash)],
            new_entry_pos,
            new_entry_hash,
            new_mmr_size,
        )?
    };
    // calculate account_root: H(count | account entries root)
    let mut account_root = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&accounts_count.to_le_bytes());
    hasher.update(&root);
    hasher.finalize(&mut account_root);
    Ok(account_root)
}

/// Compute block root from merkle proof
pub fn compute_block_root(
    blocks: Vec<(usize, [u8; 32])>,
    blocks_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<[u8; 32], Error> {
    let mmr_size = leaf_index_to_mmr_size((blocks_count - 1) as u64);
    let proof = MerkleProof::<_, HashMerge>::new(mmr_size, proof_items);
    let root = proof.calculate_root(
        blocks
            .into_iter()
            .map(|(i, hash)| {
                let pos = leaf_index_to_pos(i as u64);
                (pos, hash)
            })
            .collect(),
    )?;
    // calculate block_root: H(count | root)
    let mut block_root = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&blocks_count.to_le_bytes());
    hasher.update(&root);
    hasher.finalize(&mut block_root);
    Ok(block_root)
}

/// Compute new block root from merkle proof
pub fn compute_new_block_root(
    block_hash: [u8; 32],
    block_index: u32,
    new_block_hash: [u8; 32],
    new_block_index: u32,
    blocks_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<[u8; 32], Error> {
    let root = if new_block_index == 0 {
        new_block_hash
    } else {
        let mmr_size = leaf_index_to_mmr_size((blocks_count - 2) as u64);
        let new_mmr_size = leaf_index_to_mmr_size(new_block_index as u64);
        let entry_pos = leaf_index_to_pos(block_index as u64);
        let new_entry_pos = leaf_index_to_pos(new_block_index as u64);
        let proof = MerkleProof::<_, HashMerge>::new(mmr_size, proof_items);
        proof.calculate_root_with_new_leaf(
            vec![(entry_pos, block_hash)],
            new_entry_pos,
            new_block_hash,
            new_mmr_size,
        )?
    };
    // calculate account_root: H(count | account entries root)
    let mut account_root = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&blocks_count.to_le_bytes());
    hasher.update(&root);
    hasher.finalize(&mut account_root);
    Ok(account_root)
}

/// txs root
pub fn compute_tx_root(
    txs: Vec<(usize, [u8; 32])>,
    txs_count: u32,
    proof_items: Vec<[u8; 32]>,
) -> Result<[u8; 32], Error> {
    let mmr_size = leaf_index_to_mmr_size((txs_count - 1) as u64);
    let proof = MerkleProof::<_, HashMerge>::new(mmr_size, proof_items);
    let leaves = txs
        .into_iter()
        .map(|(i, tx_hash)| {
            let pos = leaf_index_to_pos(i as u64);
            (pos, tx_hash)
        })
        .collect();
    proof.calculate_root(leaves)
}

/// Compute txs root from merkle proof
pub fn merkle_root(leaves: Vec<[u8; 32]>) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut mmr: MemMMR<[u8; 32], HashMerge> = MemMMR::default();
    for leaf in leaves {
        mmr.push(leaf).expect("push leaf");
    }
    mmr.get_root().expect("root")
}
