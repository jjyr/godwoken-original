#include "common.h"

/* register action
 * 1. verify merkle proof of last address
 * 2. verify new entry's index is last index plus one
 * 3. verify new global state
 */
int verify_register(mol_seg_t *old_global_state_seg,
                    mol_seg_t *new_global_state_seg, mol_seg_t *register_seg) {
  uint64_t old_capacity, new_capacity;
  int ret = fetch_contract_capacities(&old_capacity, &new_capacity);
  if (ret != OK) {
    return ret;
  }
  if (old_capacity != new_capacity) {
    return ERROR_INCORRECT_CAPACITY;
  }
  mol_seg_t mmr_size_seg = MolReader_Register_get_mmr_size(register_seg);
  uint64_t mmr_size = *(uint64_t *)mmr_size_seg.ptr;
  mol_seg_t new_entry_seg = MolReader_Register_get_entry(register_seg);
  mol_seg_t new_index_seg = MolReader_AddressEntry_get_index(&new_entry_seg);
  uint32_t new_index = *(uint32_t *)new_index_seg.ptr;
  mol_seg_t leaf_hash_seg =
      MolReader_Register_get_last_entry_hash(register_seg);

  mol_seg_t old_address_root_seg =
      MolReader_GlobalState_get_address_root(old_global_state_seg);

  /* load merkle proof */
  mol_seg_t proof_seg = MolReader_Register_get_proof(register_seg);
  size_t proof_len = MolReader_Byte32Vec_length(&proof_seg);
  uint8_t proof[proof_len][HASH_SIZE];
  for (int i = 0; i < proof_len; i++) {
    mol_seg_res_t bytes_res = MolReader_Byte32Vec_get(&proof_seg, i);
    if (bytes_res.errno != MOL_OK) {
      return bytes_res.errno;
    }
    memcpy(proof[i], bytes_res.seg.ptr, bytes_res.seg.size);
  }

  /* verify merkle proof for last address entry */
  uint8_t root_hash[HASH_SIZE];
  MMRSizePos last_entry_pos = {0, 0};
  blake2b_state blake2b_ctx;
  VerifyContext proof_ctx;
  initialize_verify_context(&proof_ctx, merge_hash);

  /* verify old global state address root */
  if (new_index == 0) {
    /* address entry is the first registered entry */
    memset(root_hash, 0, HASH_SIZE);
    ret = memcmp(root_hash, old_address_root_seg.ptr, HASH_SIZE);
    if (ret != OK || proof_len != 0) {
      return ERROR_INVALID_MERKLE_PROOF;
    }
  } else {
    /* calculate address entries merkle root */
    last_entry_pos = compute_pos_by_leaf_index(new_index - 1);
    compute_proof_root(&proof_ctx, root_hash, mmr_size, leaf_hash_seg.ptr,
                       last_entry_pos.pos, proof, proof_len);
    /* calculate old address_root: H(count | address entries root) */
    uint32_t count = new_index;
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, &count, sizeof(uint32_t));
    blake2b_update(&blake2b_ctx, root_hash, HASH_SIZE);
    blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);

    ret = memcmp(root_hash, old_address_root_seg.ptr, HASH_SIZE);
    if (ret != OK) {
      return ERROR_INVALID_MERKLE_PROOF;
    }
  }

  /* verify new global state */
  uint32_t new_count = new_index + 1;
  /* calculate new entries MMR root */
  if (new_index == 0) {
    /* since address entry is the first registered entry
     * the merkle root is equals to leaf_hash
     */
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, new_entry_seg.ptr, new_entry_seg.size);
    blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
  } else {
    uint8_t new_leaf_hash[HASH_SIZE];
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, new_entry_seg.ptr, new_entry_seg.size);
    blake2b_final(&blake2b_ctx, new_leaf_hash, HASH_SIZE);
    MMRSizePos new_entry_pos = compute_pos_by_leaf_index(new_index);
    compute_new_root_from_last_leaf_proof(
        &proof_ctx, root_hash, mmr_size, leaf_hash_seg.ptr, last_entry_pos.pos,
        proof, proof_len, new_leaf_hash, new_entry_pos);
  }

  /* calculate new global state address root */
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, &new_count, sizeof(uint32_t));
  blake2b_update(&blake2b_ctx, root_hash, HASH_SIZE);
  blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
  ret = memcmp(root_hash, old_address_root_seg.ptr, HASH_SIZE);
  memcpy(old_address_root_seg.ptr, root_hash, HASH_SIZE);
  /* compare global state transition */
  ret = memcmp(old_global_state_seg->ptr, new_global_state_seg->ptr,
               GLOBAL_STATE_SIZE);
  if (ret != OK) {
    return ERROR_INVALID_NEW_ROOT;
  }
  return OK;
}
