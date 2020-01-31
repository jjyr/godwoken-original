#[derive(Debug)]
#[repr(i8)]
pub enum Error {
    InvalidOutputTypeHash = -6,
    InvalidWitness = -11,
    IncorrectCapacity = -12,
    InvalidAccountIndex = -13,
    InvalidAccountScript = -14,
    InvalidAccountNonce = -15,
    InvalidAccountBalance = -16,
    InvalidGlobalState = -17,
    InvalidAccountMerkleProof = -18,
    InvalidBlockMerkleProof = -19,
    InvalidAggregator = -20,
    InvalidTxRoot = -21,
    InvalidAccountRoot = -22,
    InvalidSignature = -23,
    InvalidSignatureMessage = -24,
    InvalidSignatureRecoverId = -25,
    RecoveryPubkey = -26,
    WrongPubkeyHash = -27,
    IncorrectNumberOfCheckpoints = -28,
}
