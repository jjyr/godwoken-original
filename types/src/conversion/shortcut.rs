use crate::packed;

impl<'a> packed::AccountReader<'a> {
    pub fn is_aggregator(&self) -> bool {
        self.is_ag().as_slice() == [1]
    }
}
