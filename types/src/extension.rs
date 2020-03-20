use crate::{core::Index, packed::*, prelude::*};

impl AgBlock {
    pub fn new_reverted_block(
        invalid_block: AgBlockReader<'_>,
        account_root: [u8; 32],
        challenger_index: Index,
    ) -> Self {
        let number: u64 = invalid_block.number().unpack();
        let prev_account_root: [u8; 32] = invalid_block.prev_account_root().unpack();
        let prev_account_count: u64 = invalid_block.prev_account_count().unpack();
        AgBlock::new_builder()
            .number(number.pack())
            .tx_root([0u8; 32].pack())
            .txs_count(0u32.pack())
            .prev_account_root(prev_account_root.pack())
            .prev_account_count(prev_account_count.pack())
            .account_root(account_root.pack())
            .ag_sig([0u8; 65].pack())
            .ag_index(challenger_index.pack())
            .build()
    }
}

impl<'a> AgBlockReader<'a> {
    pub fn is_reverted_block(&self) -> bool {
        self.tx_root().raw_data() == &[0u8; 32][..] && self.ag_sig().raw_data() == &[0u8; 65][..]
    }
}
