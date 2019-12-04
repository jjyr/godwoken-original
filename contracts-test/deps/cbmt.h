/* Complete Binary Merkle Tree
   https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0006-merkle-tree/0006-merkle-tree.md
*/

#include <ckb_syscalls.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#define HASH_SIZE 32

static int is_left(size_t index) { return (index & 1) == 1; }

/* calculate root from hashes */
int calculate_root(uint8_t root[HASH_SIZE], uint8_t hashes[][HASH_SIZE],
                   size_t len) {
  if (len == 0) {
    memset(root, 0, HASH_SIZE);
    return 0;
  }
  size_t tree_size = len * 2 - 1;
  uint8_t *tree[HASH_SIZE];
  size_t first_leaf_index = tree_size - len;
  /* put leaves into tree */
  for (int i = 0; i < len; i++) {
    tree[first_leaf_index + i] = hashes[i];
  }
  /* calculate CBMT */
  blake2b_state blake2b_ctx;
  for (int i = first_leaf_index - 1; i >= 0; i--) {
    int left = (i + 1) * 2 - 1;
    int right = left + 1;
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, tree[left], HASH_SIZE);
    blake2b_update(&blake2b_ctx, tree[right], HASH_SIZE);
    blake2b_final(&blake2b_ctx, tree[i], HASH_SIZE);
  }
  memcpy(root, tree[0], HASH_SIZE);
  return 0;
}

/* verify merkle proof */
int merkle_proof(size_t leaf_index, uint8_t *leaf_hash,
                 uint8_t root_hash[HASH_SIZE], uint8_t hashes[][HASH_SIZE],
                 size_t len) {

  /* At least should have one item
     because the tx_root is merge(raw_root, witness_root)
  */
  if (len < 1) {
    return -1;
  }
  uint8_t root[HASH_SIZE];
  uint8_t *left, *right;
  blake2b_state blake2b_ctx;
  memcpy(root, leaf_hash, HASH_SIZE);
  for (int i = 0; i < len - 1; i++) {
    if (is_left(leaf_index)) {
      left = root;
      right = hashes[i];
    } else {
      left = hashes[i];
      right = root;
    }
    blake2b_init(&blake2b_ctx, HASH_SIZE);
    blake2b_update(&blake2b_ctx, left, HASH_SIZE);
    blake2b_update(&blake2b_ctx, right, HASH_SIZE);
    blake2b_final(&blake2b_ctx, root, HASH_SIZE);
  }
  /* calculate merge(raw_root, witness_root) */
  blake2b_init(&blake2b_ctx, HASH_SIZE);
  blake2b_update(&blake2b_ctx, hashes[len - 1], HASH_SIZE);
  blake2b_update(&blake2b_ctx, root, HASH_SIZE);
  blake2b_final(&blake2b_ctx, root, HASH_SIZE);
  int ret = memcmp(root, root_hash, HASH_SIZE);
  return ret;
}
