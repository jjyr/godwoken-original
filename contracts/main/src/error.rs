#[derive(Debug)]
#[repr(i8)]
pub enum Error {
    InvalidOutputTypeHash = -6,
    InvalidWitness = -11,
    IncorrectCapacity = -12,
    InvalidEntryIndex = -13,
    InvalidEntryScript = -14,
    InvalidEntryNonce = -15,
    InvalidEntryBalance = -16,
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
}
