/* register action
 * 1. verify merkle proof of last account
 * 2. verify new entry's index is last index plus one
 * 3. verify new global state
 */

#include "common.h"

int verify_register(mol_seg_t *old_global_state_seg,
                    mol_seg_t *new_global_state_seg, mol_seg_t *register_seg) {
  /* check contract coins */
  uint64_t old_capacity, new_capacity;
  int ret = fetch_contract_capacities(&old_capacity, &new_capacity);
  if (ret != OK)
    return ret;

  if (old_capacity >= new_capacity)
    return ERROR_INCORRECT_CAPACITY;

  uint64_t deposit_capacity = new_capacity - old_capacity;

  /* extract data */
  mol_seg_t mmr_size_seg = MolReader_Register_get_mmr_size(register_seg);
  uint64_t mmr_size = *(uint64_t *)mmr_size_seg.ptr;
  mol_seg_t account_seg = MolReader_Register_get_entry(register_seg);
  mol_seg_t new_index_seg = MolReader_AccountEntry_get_index(&account_seg);
  uint32_t new_index = *(uint32_t *)new_index_seg.ptr;
  mol_seg_t leaf_hash_seg =
      MolReader_Register_get_last_entry_hash(register_seg);
  mol_seg_t old_account_root_seg =
      MolReader_GlobalState_get_account_root(old_global_state_seg);

  /* check account */
  mol_seg_t is_ag_seg = MolReader_AccountEntry_get_is_aggregator(&account_seg);
  int is_ag = *(uint8_t *)is_ag_seg.ptr;
  if (is_ag) {
    ret = verify_aggregator(&account_seg);
    if (ret != OK) {
      return ERROR_INVALID_AGGREGATOR;
    }
  }

  mol_seg_t balance_seg = MolReader_AccountEntry_get_balance(&account_seg);
  uint64_t balance = *(uint64_t *)balance_seg.ptr;
  if (balance != deposit_capacity || balance < NEW_ACCOUNT_REQUIRED_BALANCE) {
    return ERROR_INCORRECT_CAPACITY;
  }

  /* load merkle proof */
  mol_seg_t proof_seg = MolReader_Register_get_proof(register_seg);
  size_t proof_len = MolReader_Byte32Vec_length(&proof_seg);
  uint8_t proof[proof_len][HASH_SIZE];
  ret = extract_merkle_proof(proof, &proof_seg, proof_len);
  if (ret != OK)
    return ret;

  /* verify merkle proof for last account entry */
  uint8_t root_hash[HASH_SIZE];
  blake2b_state blake2b_ctx;
  MMRVerifyContext proof_ctx;
  mmr_initialize_verify_context(&proof_ctx, merge_hash);

  /* verify old global state account root */
  if (new_index == 0) {
    /* account entry is the first registered entry */
    memset(root_hash, 0, HASH_SIZE);
    ret = memcmp(root_hash, old_account_root_seg.ptr, HASH_SIZE);
    if (ret != OK || proof_len != 0)
      return ERROR_INVALID_MERKLE_PROOF;
  } else {
    /* calculate account entries merkle root */
    struct compute_account_root_context ctx = {
        &proof_ctx, &blake2b_ctx, leaf_hash_seg.ptr, new_index - 1,
        new_index,  proof_len,    mmr_size,          proof};
    compute_account_root(&ctx, root_hash);
    ret = memcmp(root_hash, old_account_root_seg.ptr, HASH_SIZE);
    if (ret != OK)
      return ERROR_INVALID_MERKLE_PROOF;
  }

  /* verify new global state */
  /* calculate new global state account root */
  uint8_t new_leaf_hash[HASH_SIZE];
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, account_seg.ptr, account_seg.size);
  blake2b_final(&blake2b_ctx, new_leaf_hash, HASH_SIZE);
  struct compute_new_account_root_context new_ctx = {
      &proof_ctx, &blake2b_ctx, leaf_hash_seg.ptr, new_leaf_hash,
      new_index,  proof_len,    mmr_size,          proof};
  compute_new_account_root(&new_ctx, root_hash);

  /* compare global state transition */
  memcpy(old_account_root_seg.ptr, root_hash, HASH_SIZE);
  ret = memcmp(old_global_state_seg->ptr, new_global_state_seg->ptr,
               GLOBAL_STATE_SIZE);
  if (ret != OK)
    return ERROR_INVALID_NEW_ROOT;

  return OK;
}
