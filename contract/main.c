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

#include "ckb_syscalls.h"
#include "godwoken.h"
#include "molecule_reader.h"

#define HASH_SIZE 32
#define MAX_WITNESS_SIZE 32768

/* error codes */
#define OK 0
#define ERROR_INVALID_OUTPUT_TYPE_HASH -10
#define ERROR_INVALID_WITNESS -11
#define ERROR_UNKNOWN_ACTION -12

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
  *action_seg = MolReader_Action_unpack(&raw_bytes_seg);
  return OK;
}

/* actions verification */
int verify_register() { return OK; }

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
  mol_union_t action_seg;
  ret = load_action(&action_seg);
  if (ret != OK) {
    return ERROR_INVALID_WITNESS;
  }
  switch (action_seg.item_id) {
  case Register:
    ret = verify_register();
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
