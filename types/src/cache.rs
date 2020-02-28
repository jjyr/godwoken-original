use crate::{collections::BTreeMap, packed::*, vec::Vec};

pub type KVMap = BTreeMap<[u8; 32], u64>;

pub struct TxWithHash<'a> {
    pub raw: TxReader<'a>,
    pub tx_hash: [u8; 32],
}

pub struct AccountWithKV<'a> {
    pub account: AccountReader<'a>,
    pub kv: KVMap,
    pub leaves_path: Vec<Vec<u8>>,
    pub proof: Vec<([u8; 32], u8)>,
}
