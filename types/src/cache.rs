use crate::packed::*;

pub struct TxWithHash<'a> {
    pub raw: TxReader<'a>,
    pub tx_hash: [u8; 32],
}
