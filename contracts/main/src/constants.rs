pub const HASH_SIZE: usize = 32;
/// required shannons to create a new account
pub const NEW_ACCOUNT_REQUIRED_BALANCE: u64 = 1000;
/// required shannons for an aggregator
pub const AGGREGATOR_REQUIRED_BALANCE: u64 = 2000;
/// reward rate for challenge, other coins will be burnt.
pub const CHALLENGE_REWARD_RATE: (u64, u64) = (8, 10);
/// CKB token id
pub const CKB_TOKEN_ID: [u8; 32] = [0u8; 32];
