mod test_deposit;
mod test_invalid_block;
mod test_register;
mod test_submit_block;

#[derive(Debug)]
#[repr(i8)]
pub enum Error {
    InvalidAggregator = -20,
    InvalidSignature = -23,
}
