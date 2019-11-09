/* Mountain merkle range
 * Reference implementation:
 * https://github.com/nervosnetwork/merkle-mountain-range
 *
 * Copyright 2019 Jiang Jinyang <jjyruby@gmail.com>
 * under MIT license
 */

#ifndef MMR_H
#define HHR_H

#include "blake2b.h"
#include "string.h"
#include "stddef.h"

#define HASH_SIZE 32

/* types */
typedef struct Peaks {
  uint64_t *peaks;
  size_t len;
} Peaks;

typedef struct HeightPos {
  uint32_t height;
  uint64_t pos;
} HeightPos;

/* helper functions */

uint64_t parent_offset(uint32_t height) { return 2 << height; }

uint64_t sibling_offset(uint32_t height) { return (2 << height) - 1; }

/* return height 0 pos 0 if can't find a right peak */
HeightPos get_right_peak(uint32_t height, uint64_t pos, uint64_t mmr_size) {
  // move to right sibling pos
  pos += sibling_offset(height);
  // loop until we find a pos in mmr
  while (pos > mmr_size - 1) {
    if (height == 0) {
      HeightPos ret = {0, 0};
      return ret;
    }
    // move to left child
    pos -= parent_offset(height - 1);
    height -= 1;
  }
  HeightPos peak = {height ,pos};
  return peak;
}

uint64_t peak_pos_by_height(uint32_t height) { return (1 << (height + 1)) - 2; }

HeightPos left_peak_height_pos(uint64_t mmr_size) {
  uint32_t height = 1;
  uint64_t prev_pos = 0;
  uint64_t pos = peak_pos_by_height(height);
  while (pos < mmr_size) {
    height += 1;
    prev_pos = pos;
    pos = peak_pos_by_height(height);
  }
  HeightPos p = {height - 1, prev_pos};
  return p;
}

Peaks get_peaks(uint64_t mmr_size) {
  /* After a little thought we can figure out the number of peaks will never
   * greater than MMR height
   * https://github.com/nervosnetwork/merkle-mountain-range#construct
   */
  HeightPos left_peak = left_peak_height_pos(mmr_size);
  uint32_t height = left_peak.height;
  uint64_t pos = left_peak.pos;
  uint64_t poss[height];
  size_t i = 0;
  while (height > 0) {
    HeightPos peak = get_right_peak(height, pos, mmr_size);
    /* no more right peak */
    if (peak.height == 0 && peak.pos == 0) {
      break;
    }
    height = peak.height;
    pos = peak.pos;
    poss[i++] = pos;
  }
  struct Peaks peaks = {poss, i};
  return peaks;
}


/* binary search, arr must be a sorted array
 * return -1 if binary search failed, otherwise return index
 */
int binary_search(uint64_t *arr, size_t len, uint64_t target) {
  int b = 0;
  int e = len - 1;
  while (b != e) {
    int i = (b + e) / 2;
    if (arr[i] < target) {
      e = i;
    } else if (arr[i] > target) {
      b = i;
    } else {
      return i;
    }
  }
  return -1;
}

/* return number of zeros */
size_t count_zeros(uint64_t n, int only_count_leading) {
  size_t num_zeros = 0;

  for (size_t i = sizeof(n) - 1; i >= 0; --i) {
    if ((n & (1 << i)) == 0) {
      ++num_zeros;
    } else if (only_count_leading) {
      break;
    }
  }
  return num_zeros;
}

int is_all_one_bits(uint64_t n) {
  return n != 0 && count_zeros(n, 0) == count_zeros(n, 1);
}

uint64_t jump_left(uint64_t pos) {
  size_t bit_length = 64 - count_zeros(pos, 1);
  size_t most_significant_bits = 1 << (bit_length - 1);
  return pos - (most_significant_bits - 1);
}

uint32_t pos_height_in_tree(uint64_t pos) {
  pos += 1;

  while (!is_all_one_bits(pos)) {
    pos = jump_left(pos);
  }

  return 64 - count_zeros(pos, 1) - 1;
}

void merge_hash(blake2b_state *blake2b_ctx, uint8_t dst[HASH_SIZE],
                uint8_t left_hash[HASH_SIZE], uint8_t right_hash[HASH_SIZE]) {
  blake2b_init(blake2b_ctx, HASH_SIZE);
  blake2b_update(blake2b_ctx, left_hash, HASH_SIZE);
  blake2b_update(blake2b_ctx, right_hash, HASH_SIZE);
  blake2b_final(blake2b_ctx, dst, HASH_SIZE);
  return;
}

/* MMR API */

/* compute root from merkle proof */
void compute_proof_root(uint8_t root_hash[HASH_SIZE], uint64_t mmr_size,
                        uint8_t leaf_hash[HASH_SIZE], uint64_t pos,
                        uint8_t proof[][HASH_SIZE], size_t proof_len) {
  struct Peaks peaks = get_peaks(mmr_size);
  memcpy(root_hash, leaf_hash, HASH_SIZE);
  size_t i = 0;
  uint32_t height = 0;
  blake2b_state blake2b_ctx;
  // calculate peak's merkle root
  // start bagging peaks if pos reach a peak pos
  while (1) {
    int idx = binary_search(peaks.peaks, peaks.len, pos);
    /* end loop if reach a peak or consume all the proof items */
    if (idx >= 0 || i >= proof_len) {
      break;
    }
    uint8_t *pitem = proof[i++];
    // verify merkle path
    uint32_t pos_height = pos_height_in_tree(pos);
    uint32_t next_height = pos_height_in_tree(pos + 1);
    if (next_height > pos_height) {
      // we are on right branch
      pos += 1;
      merge_hash(&blake2b_ctx, root_hash, pitem, root_hash);
    } else {
      // we are on left branch
      pos += parent_offset(height);
      merge_hash(&blake2b_ctx, root_hash, root_hash, pitem);
    }
    height += 1;
  }

  // bagging peaks
  // bagging with left peaks if pos is last peak(last pos)
  int bagging_left = pos == mmr_size - 1;
  while (i < proof_len) {
    uint8_t *pitem = proof[i++];
    if (bagging_left) {
      merge_hash(&blake2b_ctx, root_hash, root_hash, pitem);
    } else {
      // we are not in the last peak, so bag with right peaks first
      // notice the right peaks is already bagging into one hash in proof,
      // so after this merge, the remain proofs are always left peaks.
      bagging_left = 1;
      merge_hash(&blake2b_ctx, root_hash, pitem, root_hash);
    }
  }
  return;
}

#endif
