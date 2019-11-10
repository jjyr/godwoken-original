/* The main contract of Godwoken,
   This contract maintains the global state of all registered accounts,
   and allow any valid aggregator to update the global state.

   This contract must guarantee a later challenger can fetch the state
   transition `apply(S1, txs) -> S2`, the data of txs and the ID of the
   aggregator who made the transition from the chain.

   Operations:

   1. Registration
   2. Deposit
   3. Witdraw
   4. Send Tx
*/

#include "blockchain.h"
#include "ckb_syscalls.h"
#include "godwoken.h"
#include "mmr.h"

#define HASH_SIZE 32
#define MAX_WITNESS_SIZE 32768
#define BUF_SIZE 32768
#define GLOBAL_STATE_SIZE 64

/* error codes */
#define OK 0
#define ERROR_INVALID_OUTPUT_TYPE_HASH -10
#define ERROR_INVALID_WITNESS -11
#define ERROR_UNKNOWN_ACTION -12
#define ERROR_INVALID_GLOBAL_STATE -13
#define ERROR_LOAD_GLOBAL_STATE -14
#define ERROR_INVALID_MERKLE_PROOF -15

enum ActionItem {
  Register,
  Deposit,
};

/* check the first output cell must have the same type */
int check_output_type(uint8_t type_hash[HASH_SIZE]) {
  uint8_t output_type_hash[HASH_SIZE];
  uint64_t len = HASH_SIZE;
  int ret = ckb_checked_load_cell_by_field(output_type_hash, &len, 0, 0,
                                           CKB_SOURCE_OUTPUT,
                                           CKB_CELL_FIELD_TYPE_HASH);
  if (ret != CKB_SUCCESS) {
    return ret;
  }
  ret = memcmp(type_hash, output_type_hash, HASH_SIZE);
  if (ret != OK) {
    return ERROR_INVALID_OUTPUT_TYPE_HASH;
  }
  return OK;
}

/* load action from witness */
int load_action(mol_union_t *action_seg) {
  uint8_t witness[MAX_WITNESS_SIZE];
  uint64_t len = MAX_WITNESS_SIZE;
  int ret =
      ckb_checked_load_witness(witness, &len, 0, 0, CKB_SOURCE_GROUP_OUTPUT);
  if (ret != CKB_SUCCESS) {
    return ret;
  }
  mol_seg_t witness_args_seg;
  witness_args_seg.ptr = witness;
  witness_args_seg.size = len;
  mol_errno err = MolReader_WitnessArgs_verify(&witness_args_seg, 0);
  if (err != MOL_OK) {
    return ERROR_INVALID_WITNESS;
  }
  mol_seg_t output_type_seg =
      MolReader_WitnessArgs_get_output_type(&witness_args_seg);
  if (MolReader_BytesOpt_is_none(&output_type_seg)) {
    return ERROR_INVALID_WITNESS;
  }
  mol_seg_t raw_bytes_seg = MolReader_Bytes_raw_bytes(&output_type_seg);
  err = MolReader_Action_verify(&raw_bytes_seg, 0);
  if (err != MOL_OK) {
    return ERROR_INVALID_WITNESS;
  }
  *action_seg = MolReader_Action_unpack(&raw_bytes_seg);
  return OK;
}

int load_global_state(mol_seg_t *global_state_seg,
                      uint8_t buf[GLOBAL_STATE_SIZE], size_t source) {
  uint64_t len = GLOBAL_STATE_SIZE;

  int ret = ckb_checked_load_cell_data(buf, &len, 0, 0, source);
  if (ret != CKB_SUCCESS || len != GLOBAL_STATE_SIZE) {
    return ERROR_LOAD_GLOBAL_STATE;
  }
  global_state_seg->ptr = buf;
  global_state_seg->size = len;
  return OK;
}

/* actions verification */

/* address register
 * 1. verify merkle proof of last address
 * 2. verify new entry's index is last index plus one
 * 3. verify new global state
 */
int verify_register(mol_seg_t *old_global_state_seg,
                    mol_seg_t *new_global_state_seg, mol_seg_t *register_seg) {
  mol_seg_t mmr_size_seg = MolReader_Register_get_mmr_size(register_seg);
  uint64_t mmr_size = *(uint64_t *)mmr_size_seg.ptr;
  mol_seg_t address_entry_seg =
      MolReader_Register_get_address_entry(register_seg);
  mol_seg_t new_index_seg =
      MolReader_AddressEntry_get_index(&address_entry_seg);
  uint32_t new_index = *(uint32_t *)new_index_seg.ptr;
  mol_seg_t leaf_hash_seg =
      MolReader_Register_get_last_address_entry_hash(register_seg);

  mol_seg_t address_root_seg =
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
  int ret;
  blake2b_state blake2b_ctx;

  /* address entry is the first registered entry */
  if (new_index == 0) {
    memset(root_hash, 0, HASH_SIZE);
    ret = memcmp(root_hash, address_root_seg.ptr, HASH_SIZE);
    if (ret != OK || proof_len != 0) {
      return ERROR_INVALID_MERKLE_PROOF;
    }

    /* since address entry is the first registered entry
     * the merkle root is equals to leaf_hash
     */
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, address_entry_seg.ptr, address_entry_seg.size);
    blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
    /* calculate new address_root: H(count | address entries root) */
    uint32_t count = new_index + 1;
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, &count, sizeof(uint32_t));
    blake2b_update(&blake2b_ctx, root_hash, HASH_SIZE);
    blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
    memcpy(address_root_seg.ptr, root_hash, HASH_SIZE);
    /* compare global state transition */
    ret = memcmp(old_global_state_seg->ptr, new_global_state_seg->ptr,
                 GLOBAL_STATE_SIZE);
    if (ret != OK) {
      return ERROR_INVALID_GLOBAL_STATE;
    }
    return OK;
  }

  /* calculate address entries merkle root */
  uint64_t last_entry_pos = leaf_index_to_pos(new_index - 1);
  compute_proof_root(root_hash, mmr_size, leaf_hash_seg.ptr, last_entry_pos,
                     proof, proof_len);
  /* calculate old address_root: H(count | address entries root) */
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, &new_index, sizeof(uint32_t));
  blake2b_update(&blake2b_ctx, root_hash, HASH_SIZE);
  blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);

  ret = memcmp(root_hash, address_root_seg.ptr, HASH_SIZE);
  if (ret != OK) {
    return ERROR_INVALID_MERKLE_PROOF;
  }
  /* TODO verify new global state */
  return OK;
}

int main() {
  uint8_t type_hash[HASH_SIZE];
  uint64_t len = HASH_SIZE;

  /* load contract type_hash */
  int ret = ckb_checked_load_cell_by_field(
      type_hash, &len, 0, 0, CKB_SOURCE_GROUP_INPUT, CKB_CELL_FIELD_TYPE_HASH);

  if (ret == CKB_SUCCESS) {
    /* we are on input verification
     * just check the type contract still exists
     */
    ret = check_output_type(type_hash);
    if (ret != OK) {
      return ret;
    }
    return CKB_SUCCESS;
  }

  /* we are on output verification, check state transition */
  uint8_t old_global_state[GLOBAL_STATE_SIZE];
  uint8_t new_global_state[GLOBAL_STATE_SIZE];
  mol_seg_t old_global_state_seg;
  mol_seg_t new_global_state_seg;
  mol_union_t action_seg;
  ret = load_action(&action_seg);
  if (ret != OK) {
    return ERROR_INVALID_WITNESS;
  }
  switch (action_seg.item_id) {
  case Register:
    ret = load_global_state(&old_global_state_seg, old_global_state,
                            CKB_SOURCE_INPUT);
    if (ret != OK) {
      return ret;
    }

    ret = load_global_state(&new_global_state_seg, new_global_state,
                            CKB_SOURCE_OUTPUT);
    if (ret != OK) {
      return ret;
    }

    ret = verify_register(&old_global_state_seg, &new_global_state_seg,
                          &action_seg.seg);
    if (ret != OK) {
      return ret;
    }
    break;

  default:
    return ERROR_UNKNOWN_ACTION;
    break;
  }
  return CKB_SUCCESS;
}
