mod test_deposit;
mod test_register;
mod test_revert_block;
mod test_submit_block;

#[derive(Debug)]
#[repr(i8)]
pub enum Error {
    InvalidAggregator = -20,
    InvalidSignature = -23,
}
