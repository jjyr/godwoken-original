#[derive(Debug)]
pub enum Error {
    ContractCall(u8),
    MissingAccount(u64),
    /// balance, required_amount
    BalanceNotEnough(u64, u64),
    /// expected nonce, tx's nonce
    InvalidNonce(u32, u32),
    BalanceOverflow,
    InvalidSignature,
    InvalidMerkleProof,
}
