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

#include "blake2b.h"
#include "blockchain.h"
#include "ckb_syscalls.h"
#include "godwoken.h"
#include "mmr.h"

#define HASH_SIZE 32
#define MAX_WITNESS_SIZE 32768
#define BUF_SIZE 32768
#define GLOBAL_STATE_SIZE 32

/* error codes */
#define OK 0
#define ERROR_SYSCALL -4
/* contract state errors */
#define ERROR_INVALID_NEW_ROOT -5
#define ERROR_INVALID_OUTPUT_TYPE_HASH -6
#define ERROR_INCORRECT_CAPACITY -7
/* other errors */
#define ERROR_INVALID_WITNESS -11
#define ERROR_UNKNOWN_ACTION -12
#define ERROR_LOAD_GLOBAL_STATE -13
#define ERROR_INVALID_MERKLE_PROOF -14
#define ERROR_INVALID_ENTRY_STATE_TRANSITION -15

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

/* fetch old capacity and new capacity */
int fetch_contract_capacities(uint64_t *old_capacity, uint64_t *new_capacity) {
  uint64_t len = sizeof(uint64_t);
  int ret = ckb_checked_load_cell_by_field(
      old_capacity, &len, 0, 0, CKB_SOURCE_INPUT, CKB_CELL_FIELD_CAPACITY);
  if (ret != CKB_SUCCESS || len != sizeof(uint64_t)) {
    return ERROR_SYSCALL;
  }
  ret = ckb_checked_load_cell_by_field(
      new_capacity, &len, 0, 0, CKB_SOURCE_OUTPUT, CKB_CELL_FIELD_CAPACITY);
  if (ret != CKB_SUCCESS || len != sizeof(uint64_t)) {
    return ERROR_SYSCALL;
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

/* merge function for MMR proof */
void merge_hash(uint8_t dst[HASH_SIZE], uint8_t left_hash[HASH_SIZE],
                uint8_t right_hash[HASH_SIZE]) {
  blake2b_state blake2b_ctx;
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, left_hash, HASH_SIZE);
  blake2b_update(&blake2b_ctx, right_hash, HASH_SIZE);
  blake2b_final(&blake2b_ctx, dst, HASH_SIZE);
}

/* actions verification */

/* address register
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

/* address deposit
 * 1. verify new entry's state
 * 2. verify merkle proof of old global state
 * 3. verify new global state
 */
int verify_deposit(mol_seg_t *old_global_state_seg,
                   mol_seg_t *new_global_state_seg, mol_seg_t *deposit_seg) {
  uint64_t old_capacity, new_capacity;
  int ret = fetch_contract_capacities(&old_capacity, &new_capacity);
  if (ret != OK) {
    return ret;
  }
  if (old_capacity >= new_capacity) {
    return ERROR_INCORRECT_CAPACITY;
  }
  uint64_t deposit_capacity = new_capacity - old_capacity;
  /* check entry state */
  mol_seg_t old_entry_seg = MolReader_Deposit_get_old_entry(deposit_seg);
  mol_seg_t new_entry_seg = MolReader_Deposit_get_new_entry(deposit_seg);
  mol_seg_t old_index_seg = MolReader_AddressEntry_get_index(&old_entry_seg);
  mol_seg_t new_index_seg = MolReader_AddressEntry_get_index(&new_entry_seg);
  ret = memcmp(old_index_seg.ptr, new_index_seg.ptr, new_index_seg.size);
  if (ret != 0) {
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;
  }

  mol_seg_t old_pubkey_hash_seg =
      MolReader_AddressEntry_get_pubkey_hash(&old_entry_seg);
  mol_seg_t new_pubkey_hash_seg =
      MolReader_AddressEntry_get_pubkey_hash(&new_entry_seg);
  ret = memcmp(old_pubkey_hash_seg.ptr, new_pubkey_hash_seg.ptr,
               new_pubkey_hash_seg.size);
  if (ret != 0) {
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;
  }

  mol_seg_t old_nonce_seg = MolReader_AddressEntry_get_nonce(&old_entry_seg);
  mol_seg_t new_nonce_seg = MolReader_AddressEntry_get_nonce(&new_entry_seg);
  uint32_t old_nonce = *(uint32_t *)old_nonce_seg.ptr;
  uint32_t new_nonce = *(uint32_t *)new_nonce_seg.ptr;
  if (old_nonce + 1 != new_nonce) {
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;
  }

  mol_seg_t old_balance_seg =
      MolReader_AddressEntry_get_balance(&old_entry_seg);
  mol_seg_t new_balance_seg =
      MolReader_AddressEntry_get_balance(&new_entry_seg);
  uint64_t old_balance = *(uint64_t *)old_balance_seg.ptr;
  uint64_t new_balance = *(uint64_t *)new_balance_seg.ptr;
  if (old_balance + deposit_capacity != new_balance) {
    return ERROR_INVALID_ENTRY_STATE_TRANSITION;
  }

  /* verify old state */

  mol_seg_t mmr_size_seg = MolReader_Deposit_get_mmr_size(deposit_seg);
  uint64_t mmr_size = *(uint64_t *)mmr_size_seg.ptr;

  mol_seg_t old_address_root_seg =
      MolReader_GlobalState_get_address_root(old_global_state_seg);

  /* load merkle proof */
  mol_seg_t proof_seg = MolReader_Deposit_get_proof(deposit_seg);
  size_t proof_len = MolReader_Byte32Vec_length(&proof_seg);
  uint8_t proof[proof_len][HASH_SIZE];
  for (int i = 0; i < proof_len; i++) {
    mol_seg_res_t bytes_res = MolReader_Byte32Vec_get(&proof_seg, i);
    if (bytes_res.errno != MOL_OK) {
      return bytes_res.errno;
    }
    memcpy(proof[i], bytes_res.seg.ptr, bytes_res.seg.size);
  }

  /* verify old state merkle proof */
  mol_seg_t count_seg = MolReader_Deposit_get_count(deposit_seg);
  uint8_t root_hash[HASH_SIZE];
  blake2b_state blake2b_ctx;
  VerifyContext proof_ctx;
  initialize_verify_context(&proof_ctx, merge_hash);

  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, old_entry_seg.ptr, old_entry_seg.size);
  blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
  MMRSizePos entry_pos =
      compute_pos_by_leaf_index(*(uint64_t *)old_index_seg.ptr);
  /* verify old global state address root */
  compute_proof_root(&proof_ctx, root_hash, mmr_size, root_hash, entry_pos.pos,
                     proof, proof_len);
  /* calculate old address_root: H(count | address entries root) */
  uint32_t count = *(uint32_t *)count_seg.ptr;
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, &count, sizeof(uint32_t));
  blake2b_update(&blake2b_ctx, root_hash, HASH_SIZE);
  blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);

  ret = memcmp(root_hash, old_address_root_seg.ptr, HASH_SIZE);
  if (ret != OK) {
    return ERROR_INVALID_MERKLE_PROOF;
  }

  /* verify new global state */
  /* calculate new entries MMR root */
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, new_entry_seg.ptr, new_entry_seg.size);
  blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
  compute_proof_root(&proof_ctx, root_hash, mmr_size, root_hash, entry_pos.pos,
                     proof, proof_len);

  /* calculate new global state address root */
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  uint32_t new_count = count + 1;
  blake2b_update(&blake2b_ctx, &new_count, sizeof(uint32_t));
  blake2b_update(&blake2b_ctx, root_hash, HASH_SIZE);
  blake2b_final(&blake2b_ctx, root_hash, HASH_SIZE);
  /* compare global state transition */
  ret = memcmp(root_hash, new_global_state_seg->ptr, GLOBAL_STATE_SIZE);
  if (ret != OK) {
    return ERROR_INVALID_NEW_ROOT;
  }
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

  switch (action_seg.item_id) {
  case Register:
    ret = verify_register(&old_global_state_seg, &new_global_state_seg,
                          &action_seg.seg);
    if (ret != OK) {
      return ret;
    }
    break;
  case Deposit:
    ret = verify_deposit(&old_global_state_seg, &new_global_state_seg,
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
