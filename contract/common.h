/* common defines */

#ifndef COMMON_H
#define COMMON_H

#include "blake2b.h"
#include "blockchain.h"
#include "cbmt.h"
#include "ckb_syscalls.h"
#include "godwoken.h"
#include "mmr.h"

// constants
#define HASH_SIZE 32
#define MAX_WITNESS_SIZE 32768
#define BUF_SIZE 32768
#define GLOBAL_STATE_SIZE 64

// configs
#define AGGREGATOR_REQUIRED_BALANCE 1000

/* error codes */
#define OK 0
#define ERROR_INTERNAL -1
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
#define ERROR_INVALID_STATE_TRANSITION -15
#define ERROR_INVALID_TX_ROOT -16
#define ERROR_INVALID_AGGREGATOR -17
#define ERROR_INVALID_BLOCK_SIGNATURE -18

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

/* extract proof array from proof_seg */
int extract_merkle_proof(uint8_t proof[][HASH_SIZE], mol_seg_t *proof_seg,
                         size_t proof_len) {
  for (int i = 0; i < proof_len; i++) {
    mol_seg_res_t bytes_res = MolReader_Byte32Vec_get(proof_seg, i);
    if (bytes_res.errno != MOL_OK) {
      return bytes_res.errno;
    }
    memcpy(proof[i], bytes_res.seg.ptr, bytes_res.seg.size);
  }
  return OK;
}

struct compute_account_root_context {
  MMRVerifyContext *proof_ctx;
  blake2b_state *blake2b_ctx;
  uint8_t *leaf_hash;
  uint64_t leaf_index;
  uint32_t leaves_count;
  size_t proof_len;
  uint64_t mmr_size;
  void *proof;
};

/* compute account root */
void compute_account_root(struct compute_account_root_context *ctx,
                          uint8_t root_hash[HASH_SIZE]) {
  MMRSizePos entry_pos = mmr_compute_pos_by_leaf_index(ctx->leaf_index);
  mmr_compute_proof_root(ctx->proof_ctx, root_hash, ctx->mmr_size,
                         ctx->leaf_hash, entry_pos.pos, ctx->proof,
                         ctx->proof_len);
  /* calculate old account_root: H(count | account entries root) */
  blake2b_init(ctx->blake2b_ctx, HASH_SIZE);
  blake2b_update(ctx->blake2b_ctx, &ctx->leaves_count, sizeof(uint32_t));
  blake2b_update(ctx->blake2b_ctx, root_hash, HASH_SIZE);
  blake2b_final(ctx->blake2b_ctx, root_hash, HASH_SIZE);
}

struct compute_new_account_root_context {
  MMRVerifyContext *proof_ctx;
  blake2b_state *blake2b_ctx;
  uint8_t *leaf_hash;
  uint8_t *new_leaf_hash;
  uint64_t new_leaf_index;
  size_t proof_len;
  uint64_t mmr_size;
  void *proof;
};

/* compute new account root from last merkle proof */
void compute_new_account_root(struct compute_new_account_root_context *ctx,
                              uint8_t root_hash[HASH_SIZE]) {
  /* calculate new entries MMR root */
  if (ctx->new_leaf_index == 0) {
    /* since account entry is the first registered entry
     * the merkle root is equals to leaf_hash
     */
    memcpy(root_hash, ctx->new_leaf_hash, HASH_SIZE);
  } else {
    MMRSizePos new_entry_pos =
        mmr_compute_pos_by_leaf_index(ctx->new_leaf_index);
    MMRSizePos last_entry_pos =
        mmr_compute_pos_by_leaf_index(ctx->new_leaf_index - 1);
    mmr_compute_new_root_from_last_leaf_proof(
        ctx->proof_ctx, root_hash, ctx->mmr_size, ctx->leaf_hash,
        last_entry_pos.pos, ctx->proof, ctx->proof_len, ctx->new_leaf_hash,
        new_entry_pos);
  }

  /* calculate new global state account root */
  uint32_t new_count = ctx->new_leaf_index + 1;
  blake2b_init(ctx->blake2b_ctx, HASH_SIZE);
  blake2b_update(ctx->blake2b_ctx, &new_count, sizeof(uint32_t));
  blake2b_update(ctx->blake2b_ctx, root_hash, HASH_SIZE);
  blake2b_final(ctx->blake2b_ctx, root_hash, HASH_SIZE);
}

/* verify aggregator */
int verify_aggregator(mol_seg_t *ag_seg) {
  mol_seg_t is_ag_seg = MolReader_AccountEntry_get_is_aggregator(ag_seg);
  int is_ag = *(uint8_t *)is_ag_seg.ptr;
  if (!is_ag) {
    return ERROR_INVALID_AGGREGATOR;
  }
  mol_seg_t balance_seg = MolReader_AccountEntry_get_balance(ag_seg);
  uint64_t balance = *(uint64_t *)balance_seg.ptr;
  if (balance < AGGREGATOR_REQUIRED_BALANCE) {
    return ERROR_INVALID_AGGREGATOR;
  }
  return OK;
}

int verify_signature(uint8_t signature[65], uint8_t message[HASH_SIZE],
                     uint8_t pubkey_hash[20]) {
  // TODO
  return OK;
}

#endif /* COMMON_H */
