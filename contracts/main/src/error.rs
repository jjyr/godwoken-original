#[derive(Debug)]
#[repr(i8)]
pub enum Error {
    InvalidOutputTypeHash = -6,
    InvalidWitness = -11,
    IncorrectCapacity = -12,
    InvalidEntryIndex = -13,
    InvalidEntryPubkeyHash = -14,
    InvalidEntryNonce = -15,
    InvalidEntryBalance = -16,
    InvalidGlobalState = -17,
    InvalidAccountMerkleProof = -18,
    InvalidBlockMerkleProof = -19,
    InvalidAggregator = -20,
}
