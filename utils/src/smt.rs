use crate::{alloc::vec::Vec, hash::new_blake2b};
use blake2b_ref::Blake2b;
use godwoken_types::{core::TokenID, packed::*, prelude::*};
use sparse_merkle_tree::{
    default_store::DefaultStore,
    error::Error,
    traits::Hasher,
    tree::{MerkleProof, SparseMerkleTree},
};
pub use sparse_merkle_tree::{traits::Value as ValueTrait, H256};

#[repr(u8)]
pub enum SMTPrefix {
    Count = 0,
    Index,
    Token,
    Withdraw,
    Store,
}

pub type SMT = SparseMerkleTree<Blake2bHasher, Value, DefaultStore<Value>>;

pub fn account_index_key(index: u64) -> H256 {
    let mut key = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&[SMTPrefix::Index as u8]);
    hasher.update(&index.to_le_bytes());
    hasher.finalize(&mut key);
    key.into()
}

pub fn token_id_key(index: u64, token_id: &TokenID) -> H256 {
    let mut key = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(&[SMTPrefix::Token as u8]);
    hasher.update(&index.to_le_bytes());
    hasher.update(token_id);
    hasher.finalize(&mut key);
    key.into()
}

// shortcut

pub fn compute_root(leaves: Vec<(H256, Value)>) -> Result<H256, Error> {
    let mut tree = SMT::default();
    for (k, v) in leaves {
        tree.update(k, v)?;
    }
    Ok(*tree.root())
}

pub fn compute_root_with_proof(
    leaves: Vec<(H256, H256)>,
    leaves_path: Vec<Vec<u8>>,
    proof: Vec<(H256, u8)>,
) -> Result<H256, Error> {
    let proof = MerkleProof::new(leaves_path, proof);
    proof.compute_root::<Blake2bHasher>(leaves)
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
pub struct Value(Vec<u8>);

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        if v == 0 {
            return Self::zero();
        }
        Value(v.to_le_bytes().to_vec())
    }
}

impl Into<u64> for Value {
    fn into(self) -> u64 {
        if self.0.is_empty() {
            return 0;
        }
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.0);
        u64::from_le_bytes(buf)
    }
}

impl From<Account> for Value {
    fn from(v: Account) -> Self {
        Value(v.as_bytes().to_vec())
    }
}

impl Into<Account> for Value {
    fn into(self) -> Account {
        Account::new_unchecked(self.0.into())
    }
}

impl Value {
    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }
}

impl ValueTrait for Value {
    fn to_h256(&self) -> H256 {
        let mut buf = [0u8; 32];
        // use self as digest if value is less or equals to buf
        // this is safe since our SMT use hash(key | value) as node's hash
        if self.0.len() <= buf.len() {
            buf[..self.0.len()].copy_from_slice(&self.0);
            return buf.into();
        }
        let mut hasher = new_blake2b();
        hasher.update(&self.0);
        hasher.finalize(&mut buf);
        buf.into()
    }

    fn zero() -> Self {
        Value(Vec::new())
    }
}
