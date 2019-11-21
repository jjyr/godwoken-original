/* deposit action
 * 1. verify new entry's state
 * 2. verify merkle proof of old global state
 * 3. verify new global state
 */

#include "common.h"

int verify_deposit(mol_seg_t *old_global_state_seg,
                   mol_seg_t *new_global_state_seg, mol_seg_t *deposit_seg) {
  /* check contract coins */
  uint64_t old_capacity, new_capacity;
  int ret = fetch_contract_capacities(&old_capacity, &new_capacity);
  if (ret != OK)
    return ret;

  if (old_capacity >= new_capacity)
    return ERROR_INCORRECT_CAPACITY;

  uint64_t deposit_capacity = new_capacity - old_capacity;

  /* check entry state */
  mol_seg_t old_entry_seg = MolReader_Deposit_get_old_entry(deposit_seg);
  mol_seg_t new_entry_seg = MolReader_Deposit_get_new_entry(deposit_seg);
  mol_seg_t old_index_seg = MolReader_AddressEntry_get_index(&old_entry_seg);
  mol_seg_t new_index_seg = MolReader_AddressEntry_get_index(&new_entry_seg);
  ret = memcmp(old_index_seg.ptr, new_index_seg.ptr, new_index_seg.size);
  if (ret != 0)
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;

  mol_seg_t old_pubkey_hash_seg =
      MolReader_AddressEntry_get_pubkey_hash(&old_entry_seg);
  mol_seg_t new_pubkey_hash_seg =
      MolReader_AddressEntry_get_pubkey_hash(&new_entry_seg);
  ret = memcmp(old_pubkey_hash_seg.ptr, new_pubkey_hash_seg.ptr,
               new_pubkey_hash_seg.size);
  if (ret != 0)
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;

  mol_seg_t old_nonce_seg = MolReader_AddressEntry_get_nonce(&old_entry_seg);
  mol_seg_t new_nonce_seg = MolReader_AddressEntry_get_nonce(&new_entry_seg);
  uint32_t old_nonce = *(uint32_t *)old_nonce_seg.ptr;
  uint32_t new_nonce = *(uint32_t *)new_nonce_seg.ptr;
  if (old_nonce + 1 != new_nonce)
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;

  mol_seg_t old_balance_seg =
      MolReader_AddressEntry_get_balance(&old_entry_seg);
  mol_seg_t new_balance_seg =
      MolReader_AddressEntry_get_balance(&new_entry_seg);
  uint64_t old_balance = *(uint64_t *)old_balance_seg.ptr;
  uint64_t new_balance = *(uint64_t *)new_balance_seg.ptr;
  if (old_balance + deposit_capacity != new_balance)
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;

  /* verify old state */
  mol_seg_t mmr_size_seg = MolReader_Deposit_get_mmr_size(deposit_seg);
  uint64_t mmr_size = *(uint64_t *)mmr_size_seg.ptr;

  mol_seg_t old_address_root_seg =
      MolReader_GlobalState_get_address_root(old_global_state_seg);

  /* load merkle proof */
  mol_seg_t proof_seg = MolReader_Deposit_get_proof(deposit_seg);
  size_t proof_len = MolReader_Byte32Vec_length(&proof_seg);
  uint8_t proof[proof_len][HASH_SIZE];
  ret = extract_merkle_proof(proof, &proof_seg, proof_len);
  if (ret != OK)
    return ret;

  /* verify old state merkle proof */
  mol_seg_t count_seg = MolReader_Deposit_get_count(deposit_seg);
  uint32_t count = *(uint32_t *)count_seg.ptr;

  uint8_t root_hash[HASH_SIZE];
  blake2b_state blake2b_ctx;
  MMRVerifyContext proof_ctx;
  mmr_initialize_verify_context(&proof_ctx, merge_hash);

  /* calculate address entries merkle root */
  uint8_t leaf_hash[HASH_SIZE];
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, old_entry_seg.ptr, old_entry_seg.size);
  blake2b_final(&blake2b_ctx, leaf_hash, HASH_SIZE);

  struct compute_address_root_context ctx = {
      &proof_ctx, &blake2b_ctx, leaf_hash, count - 1,
      proof_len,  mmr_size,     proof};
  compute_address_root(&ctx, root_hash);
  ret = memcmp(root_hash, old_address_root_seg.ptr, HASH_SIZE);
  if (ret != OK)
    return ERROR_INVALID_MERKLE_PROOF;

  /* verify new global state */
  /* calculate new entries MMR root */
  uint8_t updated_leaf_hash[HASH_SIZE];
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, new_entry_seg.ptr, new_entry_seg.size);
  blake2b_final(&blake2b_ctx, updated_leaf_hash, HASH_SIZE);

  ctx = (struct compute_address_root_context){
      &proof_ctx, &blake2b_ctx, updated_leaf_hash, count - 1, proof_len,
      mmr_size,   proof};
  compute_address_root(&ctx, root_hash);

  /* compare global state transition */
  memcpy(old_address_root_seg.ptr, root_hash, HASH_SIZE);
  ret = memcmp(old_global_state_seg->ptr, new_global_state_seg->ptr,
               GLOBAL_STATE_SIZE);
  if (ret != OK)
    return ERROR_INVALID_NEW_ROOT;

  return OK;
}
