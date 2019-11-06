#ifndef POW_H
#define POW_H

#include "common.h"

#define ERROR_NOT_FOUND_TARGET_TYPE_ID -20
#define ERROR_LOAD_COMPACT_TARGET -21

/* POW helper functions */

/* load compact target from POW_TARGET_TYPE_ID */
int load_compact_target(uint32_t *compact_target) {
  /* load type_id of deps */
  uint8_t type_hash[HASH_SIZE];
  uint64_t len;
  int ret;
  int i = 0;
  while (1) {
    len = HASH_SIZE;
    ret = ckb_load_cell_by_field(&type_hash, &len, 0, i, CKB_SOURCE_CELL_DEP,
                                 CKB_CELL_FIELD_TYPE_HASH);
    if (ret == CKB_INDEX_OUT_OF_BOUND) {
      return ERROR_NOT_FOUND_TARGET_TYPE_ID;
    }
    if (ret == CKB_ITEM_MISSING) {
      i++;
      continue;
    }
    if (ret != CKB_SUCCESS) {
      return ERROR_LOAD_COMPACT_TARGET;
    }
    uint8_t pow_target_type_id[HASH_SIZE] = POW_TARGET_TYPE_ID;
    if (memcmp(type_hash, pow_target_type_id, HASH_SIZE) == OK) {
      break;
    }
    i++;
  }
  len = sizeof(uint32_t);
  ret = ckb_load_cell_data(compact_target, &len, 0, i, CKB_SOURCE_CELL_DEP);
  if (ret != CKB_SUCCESS || len != sizeof(uint32_t)) {
    return ERROR_LOAD_COMPACT_TARGET;
  }
  return OK;
}

/* calculate pow_message from pow_hash and nonce */
void pow_message(uint8_t dst[POW_MESSAGE_SIZE], uint8_t pow_hash[HASH_SIZE],
                 uint8_t nonce[NONCE_SIZE]) {
  memcpy(dst, pow_hash, HASH_SIZE);
  memcpy(dst + HASH_SIZE, nonce, NONCE_SIZE);
}

/* convert compact target to normal format,
   return 0 represent compact is valid, otherwise represent compact is overflow
   TODO make sure it works for larger target(u256) */
int compact_to_target(uint8_t dst[HASH_SIZE], uint32_t compact) {
  int exponent = compact >> 24;
  uint64_t mantissa = compact & 0x00ffffff;
  if (exponent <= 3) {
    /* TODO mantissa should be 256 bit */
    mantissa >>= 8 * (3 - exponent);
    memcpy(dst, (uint8_t *)&mantissa, sizeof(uint64_t));
  } else {
    uint64_t target = mantissa;
    /* TODO target should be 256 bit */
    target <<= 8 * (exponent - 3);
    memcpy(dst, (uint8_t *)&target, sizeof(uint64_t));
  }
  return mantissa && (exponent > 32);
}

/* verify_pow, return 0 if pow is valid, otherwise return error code */
int verify_pow(uint8_t pow_hash[HASH_SIZE], uint8_t nonce[NONCE_SIZE],
               uint8_t target[HASH_SIZE]) {
  uint8_t pow_msg[POW_MESSAGE_SIZE];
  uint8_t resolve_hash[HASH_SIZE];

  // calculate pow resolve hash
  pow_message(pow_msg, pow_hash, nonce);
  eaglesong_hash(resolve_hash, pow_msg, POW_MESSAGE_SIZE);

  // test resolve
  if (resolve_hash > target) {
    return ERROR_INVALID_POW;
  }

  return OK;
}

#endif /* POW_H */
