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

#include "common.h"

/* Actions */

enum ActionItem {
  Register,
  Deposit,
};

#include "deposit.c"
#include "registration.c"

/* End Actions */


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
