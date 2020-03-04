use crate::{collections::BTreeMap, packed::*};

pub type KVMap = BTreeMap<[u8; 32], u64>;

pub struct TxWithHash<'a> {
    pub raw: TxReader<'a>,
    pub tx_hash: [u8; 32],
}
