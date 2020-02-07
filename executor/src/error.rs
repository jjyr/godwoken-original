#[derive(Debug)]
pub enum Error {
    ContractCall(u8),
    MissingAccount(u32),
    /// balance, required_amount
    BalanceNotEnough(u64, u64),
    BalanceOverflow,
    InvalidSignature,
}
