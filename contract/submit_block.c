/* submit block action
 *
 * Aggregator send collected txs in this SubmitBlock action,
 * Aggregator calculate tx_root and accumulate into the block_root to update the
 * global state.
 * Each Tx include a secp256k1 signature that user signed.
 * Anyone can send a challenge later to peanalize Aggregator who include a
 * invalid tx.
 */

#include "common.h"

/* verify aggregator pubkey */
int check_aggregator(mol_seg_t *send_block_seg) {
  /* TODO
   * 1. verify aggregator signature according to pubkey hash
   * 2. verify aggregator exsits in aggregator_list
   */
  return OK;
}

/* verify tx_root */
int check_tx_root(mol_seg_t *send_block_seg) {
  mol_seg_t txs_seg = MolReader_SubmitBlock_get_txs(send_block_seg);
  size_t txs_len = MolReader_Txs_length(&txs_seg);
  uint8_t tx_hashes[txs_len][HASH_SIZE];
  /* calculate tx_hashes */
  blake2b_state blake2b_ctx;
  for (int i = 0; i < txs_len; i++) {
    mol_seg_res_t tx_res = MolReader_Txs_get(&txs_seg, i);
    if (tx_res.errno != MOL_OK) {
      return ERROR_INTERNAL;
    }
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, tx_res.seg.ptr, tx_res.seg.size);
    blake2b_final(&blake2b_ctx, tx_hashes[i], HASH_SIZE);
  }
  uint8_t root[HASH_SIZE];
  int ret = calculate_root(root, tx_hashes, txs_len);
  if (ret != OK) {
    return ERROR_INTERNAL;
  }
  mol_seg_t block_seg = MolReader_SubmitBlock_get_block(send_block_seg);
  mol_seg_t tx_root_seg = MolReader_AggregatorBlock_get_tx_root(&block_seg);
  ret = memcmp(root, tx_root_seg.ptr, HASH_SIZE);
  if (ret != OK) {
    return ERROR_INVALID_TX_ROOT;
  }
  return OK;
}

/* verify global state block root transition */
int check_block_root_transition(mol_seg_t *old_global_state_seg,
                                mol_seg_t *new_global_state_seg,
                                mol_seg_t *send_block_seg) {
  /* extract data */
  mol_seg_t mmr_size_seg =
      MolReader_SubmitBlock_get_block_mmr_size(send_block_seg);
  mol_seg_t proof_seg = MolReader_SubmitBlock_get_block_proof(send_block_seg);
  mol_seg_t block_seg = MolReader_SubmitBlock_get_block(send_block_seg);
  mol_seg_t block_number_seg = MolReader_AggregatorBlock_get_number(&block_seg);
  mol_seg_t last_block_hash_seg =
      MolReader_SubmitBlock_get_last_block_hash(send_block_seg);

  /* verify account root transition */
  mol_seg_t block_old_account_root_seg =
      MolReader_AggregatorBlock_get_old_account_root(&block_seg);
  mol_seg_t old_account_root_seg =
      MolReader_GlobalState_get_account_root(old_global_state_seg);
  int ret = memcmp(block_old_account_root_seg.ptr, old_account_root_seg.ptr,
                   block_old_account_root_seg.size);
  if (ret != OK) {
    return ERROR_INVALID_STATE_TRANSITION;
  }
  mol_seg_t block_new_account_root_seg =
      MolReader_AggregatorBlock_get_new_account_root(&block_seg);
  mol_seg_t new_account_root_seg =
      MolReader_GlobalState_get_account_root(new_global_state_seg);
  ret = memcmp(block_new_account_root_seg.ptr, new_account_root_seg.ptr,
               block_new_account_root_seg.size);
  if (ret != OK) {
    return ERROR_INVALID_STATE_TRANSITION;
  }

  /* verify old global state */
  uint64_t mmr_size = *(uint64_t *)mmr_size_seg.ptr;
  uint32_t block_number = *(uint32_t *)block_number_seg.ptr;
  size_t proof_len = MolReader_Byte32Vec_length(&proof_seg);
  uint8_t proof[proof_len][HASH_SIZE];
  ret = extract_merkle_proof(proof, &proof_seg, proof_len);
  if (ret != OK) {
    return ERROR_INTERNAL;
  }

  MMRSizePos last_block_pos = mmr_compute_pos_by_leaf_index(block_number - 1);
  MMRVerifyContext ctx;
  mmr_initialize_verify_context(&ctx, merge_hash);
  uint8_t block_root[HASH_SIZE];
  mmr_compute_proof_root(&ctx, block_root, mmr_size, last_block_hash_seg.ptr,
                         last_block_pos.pos, proof, proof_len);

  /* calculate old block_root: H(count | account entries root) */
  blake2b_state blake2b_ctx;
  uint8_t root_hash[HASH_SIZE];
  if (block_number == 0) {
    memset(root_hash, 0, HASH_SIZE);
  } else {
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, &block_number, sizeof(uint32_t));
    blake2b_update(&blake2b_ctx, block_root, HASH_SIZE);
    blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
  }

  mol_seg_t old_block_root_seg =
      MolReader_GlobalState_get_block_root(old_global_state_seg);
  ret = memcmp(old_block_root_seg.ptr, root_hash, HASH_SIZE);
  if (ret != 0) {
    return ERROR_INVALID_STATE_TRANSITION;
  }

  /* verify new block root */
  uint8_t block_hash[HASH_SIZE];
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, block_seg.ptr, block_seg.size);
  blake2b_final(&blake2b_ctx, block_hash, HASH_SIZE);

  MMRSizePos block_pos = mmr_compute_pos_by_leaf_index(block_number);
  mmr_compute_new_root_from_last_leaf_proof(
      &ctx, block_root, mmr_size, last_block_hash_seg.ptr, last_block_pos.pos,
      proof, proof_len, block_hash, block_pos);

  uint32_t block_count = block_number + 1;
  /* calculate new block_root: H(count | account entries root) */
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, &block_count, sizeof(uint32_t));
  blake2b_update(&blake2b_ctx, block_root, HASH_SIZE);
  blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);

  mol_seg_t new_block_root_seg =
      MolReader_GlobalState_get_block_root(new_global_state_seg);
  ret = memcmp(root_hash, new_block_root_seg.ptr, new_block_root_seg.size);
  if (ret != 0) {
    return ERROR_INVALID_STATE_TRANSITION;
  }

  return OK;
}

int verify_submit_block(mol_seg_t *old_global_state_seg,
                        mol_seg_t *new_global_state_seg,
                        mol_seg_t *send_block_seg) {
  /* check contract coins */
  uint64_t old_capacity, new_capacity;
  int ret = fetch_contract_capacities(&old_capacity, &new_capacity);
  if (ret != OK)
    return ret;

  if (old_capacity != new_capacity)
    return ERROR_INCORRECT_CAPACITY;

  /* check aggregator */
  ret = check_aggregator(send_block_seg);
  if (ret != OK) {
    return ret;
  }

  /* check tx root */
  ret = check_tx_root(send_block_seg);
  if (ret != OK) {
    return ret;
  }

  /* check block_root transition */
  ret = check_block_root_transition(old_global_state_seg, new_global_state_seg,
                                    send_block_seg);
  if (ret != OK) {
    return ret;
  }

  return OK;
}
