use crate::{packed::*, prelude::*};

impl AgBlock {
    pub fn new_penalized_block<'a>(
        &invalid_block: &AgBlockReader<'a>,
        account_root: [u8; 32],
        challenger_index: u32,
    ) -> Self {
        let number: u32 = invalid_block.number().unpack();
        let previous_account_root: [u8; 32] = invalid_block.previous_account_root().unpack();
        AgBlock::new_builder()
            .number(number.pack())
            .tx_root([0u8; 32].pack())
            .previous_account_root(previous_account_root.pack())
            .current_account_root(account_root.pack())
            .ag_sig([0u8; 65].pack())
            .ag_index(challenger_index.pack())
            .build()
    }
}

impl<'a> AgBlockReader<'a> {
    pub fn is_penalized_block(&self) -> bool {
        self.tx_root().raw_data() == &[0u8; 32][..] && self.ag_sig().raw_data() == &[0u8; 65][..]
    }
}
