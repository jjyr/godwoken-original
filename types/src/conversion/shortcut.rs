use crate::packed;

impl<'a> packed::AccountEntryReader<'a> {
    pub fn is_ag(&self) -> bool {
        self.is_aggregator().as_slice() == [1]
    }
}
