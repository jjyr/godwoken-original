use crate::error::Error;

pub fn verify_ag_signature(
    ag_sig: [u8; 65],
    sig_message: [u8; 32],
    ag_pubkey_hash: [u8; 20],
) -> Result<(), Error> {
    // TODO
    Ok(())
}
