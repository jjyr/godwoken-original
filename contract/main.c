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

#define HASH_SIZE 32
#define OK 0

/* error codes */
#define ERROR_INVALID_OUTPUT_TYPE_HASH -10

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

int main() {
  uint8_t type_hash[HASH_SIZE];
  uint64_t len = HASH_SIZE;
  /* load contract type_hash */
  int ret = ckb_checked_load_cell_by_field(
      type_hash, &len, 0, 0, CKB_SOURCE_GROUP_INPUT, CKB_CELL_FIELD_TYPE_HASH);
  if (ret == CKB_SUCCESS) {
    /* we are on input verification, just check the spending logic */
    ret = check_output_type(type_hash);
    if (ret != CKB_SUCCESS) {
      return ret;
    }
  } else {
    int action = 0;
    /* we are on output verification, check state transition */
    switch (action) {
    case 0:
      /* code */
      break;

    default:
      break;
    }
  }
  return CKB_SUCCESS;
}
