use crate::{alloc::collections::BTreeMap, alloc::vec::Vec, hash::new_blake2b};
use blake2b_ref::Blake2b;
use sparse_merkle_tree::{
    default_store::DefaultStore,
    error::Error,
    traits::{Hasher, Value},
    tree::{MerkleProof, SparseMerkleTree},
};

pub use sparse_merkle_tree::H256;

pub type SMT = SparseMerkleTree<Blake2bHasher, U64Wrapper, DefaultStore<U64Wrapper>>;

pub fn compute_root(leaves: Vec<([u8; 32], u64)>) -> Result<[u8; 32], Error> {
    let mut tree = SMT::default();
    for (k, v) in leaves {
        tree.update(k.into(), v.into())?;
    }
    Ok((*tree.root()).into())
}

pub fn compute_root_with_proof(
    leaves: BTreeMap<[u8; 32], u64>,
    leaves_path: Vec<Vec<u8>>,
    proof: Vec<([u8; 32], u8)>,
) -> Result<[u8; 32], Error> {
    let proof = MerkleProof::new(
        leaves_path,
        proof
            .into_iter()
            .map(|(item, height)| (item.into(), height))
            .collect(),
    );
    proof
        .compute_root::<Blake2bHasher>(
            leaves
                .into_iter()
                .map(|(k, v)| (k.into(), U64Wrapper(v).to_h256()))
                .collect(),
        )
        .map(|root| root.into())
}

pub struct Blake2bHasher(Blake2b);

impl Default for Blake2bHasher {
    fn default() -> Self {
        Blake2bHasher(new_blake2b())
    }
}

impl Hasher for Blake2bHasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }
    fn finish(self) -> H256 {
        let mut hash = [0u8; 32];
        self.0.finalize(&mut hash);
        hash.into()
    }
}

#[derive(Default, Clone, Debug)]
pub struct U64Wrapper(u64);

impl From<u64> for U64Wrapper {
    fn from(v: u64) -> Self {
        U64Wrapper(v)
    }
}

impl Into<u64> for U64Wrapper {
    fn into(self) -> u64 {
        self.0
    }
}

impl Value for U64Wrapper {
    fn to_h256(&self) -> H256 {
        let mut buf = [0u8; 32];
        buf[..8].copy_from_slice(&self.0.to_le_bytes());
        buf.into()
    }

    fn zero() -> Self {
        U64Wrapper(0)
    }
}
