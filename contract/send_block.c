/* send block action
 *
 * Aggregator send collected txs in this SendBlock action,
 * Aggregator calculate tx_root and accumulate into the block_root to update the
 * global state.
 * Each Tx include a secp256k1 signature that user signed.
 * Anyone can send a challenge later to peanalize Aggregator who include a
 * invalid tx.
 */

int verify_send_block(mol_seg_t *old_global_state_seg,
                      mol_seg_t *new_global_state_seg,
                      mol_seg_t *send_block_seg) {
  return OK;
}
