/* send block action
 *
 * Aggregator send collected txs in this SendBlock action,
 * Aggregator calculate tx_root and accumulate into the block_root to update the
 * global state.
 * Each Tx include a secp256k1 signature that user signed.
 * Anyone can send a challenge later to peanalize Aggregator who include a
 * invalid tx.
 */

#include "common.h"

int verify_send_block(mol_seg_t *old_global_state_seg,
                      mol_seg_t *new_global_state_seg,
                      mol_seg_t *send_block_seg) {
  /* check contract coins */
  uint64_t old_capacity, new_capacity;
  int ret = fetch_contract_capacities(&old_capacity, &new_capacity);
  if (ret != OK)
    return ret;

  if (old_capacity != new_capacity)
    return ERROR_INCORRECT_CAPACITY;

  /* check tx root */

  /* check block_root transition */

  return OK;
}
