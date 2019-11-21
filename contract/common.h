/* common defines */

#ifndef COMMON_H
#define COMMON_H

#include "blake2b.h"
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

/* merge function for MMR proof */
void merge_hash(uint8_t dst[HASH_SIZE], uint8_t left_hash[HASH_SIZE],
                uint8_t right_hash[HASH_SIZE]) {
  blake2b_state blake2b_ctx;
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, left_hash, HASH_SIZE);
  blake2b_update(&blake2b_ctx, right_hash, HASH_SIZE);
  blake2b_final(&blake2b_ctx, dst, HASH_SIZE);
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


#endif /* COMMON_H */
