use crate::error::Error;
use crate::utils;
use secp256k1::{recover, Message, RecoveryId, Signature};

pub fn verify_ag_signature(
    ag_sig: [u8; 65],
    sig_message: [u8; 32],
    ag_pubkey_hash: &[u8],
) -> Result<(), Error> {
    debug_assert_eq!(ag_pubkey_hash.len(), 20);
    let message = Message::parse(&sig_message);
    let sig = Signature::parse_slice(&ag_sig[..64]).map_err(|_| Error::InvalidSignature)?;
    let recover_id = RecoveryId::parse(ag_sig[64]).map_err(|_| Error::InvalidSignatureRecoverId)?;
    let pubkey = recover(&message, &sig, &recover_id).map_err(|_| Error::RecoveryPubkey)?;
    let pubkey_hash = {
        let mut hash = [0u8; 20];
        let mut hasher = utils::new_blake2b();
        let pubkey_bytes = pubkey.serialize_compressed();
        hasher.update(&pubkey_bytes);
        hasher.finalize(&mut hash);
        hash
    };
    if &pubkey_hash[..] != ag_pubkey_hash {
        return Err(Error::WrongPubkeyHash);
    }
    Ok(())
}
