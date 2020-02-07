use crate::hash::new_blake2b;
use secp256k1::{recover, Message, RecoveryId, Signature};

pub enum Error {
    InvalidSignature,
    InvalidRecoveryId,
    InvalidMessage,
    InvalidPubkeyHash,
    RecoveryPubkey,
    IncorrectPubkeyHash,
}

const SIGNATURE_LEN: usize = 65;
const MESSAGE_LEN: usize = 32;
const PUBKEY_HASH_LEN: usize = 20;

pub fn verify_signature(signature: &[u8], message: &[u8], pubkey_hash: &[u8]) -> Result<(), Error> {
    if signature.len() != SIGNATURE_LEN {
        return Err(Error::InvalidSignature);
    }
    if message.len() != MESSAGE_LEN {
        return Err(Error::InvalidMessage);
    }
    if pubkey_hash.len() != PUBKEY_HASH_LEN {
        return Err(Error::InvalidPubkeyHash);
    }
    let msg = Message::parse_slice(&message).map_err(|_| Error::InvalidMessage)?;
    let sig = Signature::parse_slice(&signature[..64]).map_err(|_| Error::InvalidSignature)?;
    let recover_id = RecoveryId::parse(signature[64]).map_err(|_| Error::InvalidRecoveryId)?;
    let pubkey = recover(&msg, &sig, &recover_id).map_err(|_| Error::RecoveryPubkey)?;
    let pubkey_hash = {
        let mut hash = [0u8; 20];
        let mut hasher = new_blake2b();
        let pubkey_bytes = pubkey.serialize_compressed();
        hasher.update(&pubkey_bytes);
        hasher.finalize(&mut hash);
        hash
    };
    if &pubkey_hash[..] != pubkey_hash {
        return Err(Error::IncorrectPubkeyHash);
    }
    Ok(())
}
