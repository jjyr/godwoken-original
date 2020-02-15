# Design

This document explains the design of Godwoken from a high-level overview.

Godwoken is an experimental optimistic rollup implementation to provide a generalized account-based programming layer upon Nervos CKB.

We mainly solve two problems: scalability and aggregation.

Godwoken applies optimistic rollup architecture to promise scalability, the godwoken transactions designed to be light-weight than the layer-1 transaction; it takes less size and does not perform on-chain verification.

Blockchains use the UTXO-like model naturally depend on off-chain aggregation. Godwoken supports aggregation by providing an account-based model; it's quite useful for many scenarios. For example: If we design a decentralized mining pool on CKB, the big problem is to split the rewards to miners every block; depends on the number of miners, the reward transaction size is enormous(think about hundred to thousands of outputs). Since CKB required at least 41 CKB to hold a cell (61 CKB for a cell with secp256k1 lock), the implementation is even more complicated. By building such a decentralized mining pool upon an account-based model like Godwoken, we can transfer money to thousands of miners with tx size still small, and get over the 41 CKB transfer limitation.

> Many people reference rollup as layer-1.5, layer-2, or even layer-1(by trust-level). This document references the optimistic rollup as layer-1.5 to distinguish it with layer-1.

> Since we are still WIP, this document may not accurately reflect every detail in the current project. But the core idea described in this document should be stable.

## Architecture

As an optimistic rollup solution, Godwoken composite by the following parts:

* Main contract - a contract deployed on CKB, which maintains a global state.
* Aggregator - an off-chain program that packs off-chain transactions into layer 1.5 block and submits the block to the main contract regularly.
* Validator - an off-chain program that continuously watches the contract state, when found an invalided block is submitted, validator sends an invalid block request to revert the block and get a reward. Usually, an aggregator is also a validator.

## Layer 1.5 structures

### Account

```
table Account {
    index: Uint32, // address index
    script: AccountScript, // account's code
    nonce: Uint32, // nonce
    state_root: Byte32, // state merkle root
    is_ag: byte, // a flag to indicates aggregator account
}

table AccountScript {
    code_hash: Byte32, // hash code the code
    args: Bytes, // initialized args of the Account
}
```

To register an account, a user needs to send `register` action to the Godwoken contract, and deposit layer-1 assets.

`index` field used to indicates an account, for a newly registered account, `index` must equal to the `last_account.index + 1`.

`nonce` used to prevent the replay attack, each time a tx sent to an account, the nonce will increase by `1`;

`state_root` field represents a root of a [sparse merkle tree], sparse merkle tree is a tree with `N ** 2` leaves, each leaf can store a value, which is perfect to represent a key-value store.

In the sparse merkle tree, we use UDT's `type_hash` as key, the amount of token as the value to represent the UDT balance `type_hash -> amount`; specially for the CKB we use `0x00..00` as the key; for non-UDT Cell, we stores `1` under the cell's `type_hash` to indicate the cell exists.

`script` field used for account-model contract: when an account receives messages, the script code will be loaded and executed. Godwoken provides a default script `0x00..00`, which does secp256k1 verification; the `args` is pubkey hash.

`is_ag` used for indicates an account is whether an aggregator or not, an aggregator account needs more assets to register, and takes longer waiting time to withdraw.

### Block

```
table AgBlock {
    number: Uint32, // block number
    tx_root: Byte32,
    previous_account_root: Byte32, // account root before this block
    current_account_root: Byte32, // account root after this block
    ag_sig: Byte65, // Aggregator's signature
    ag_index: Uint32, // Aggregator's index
}
```

`number`, must equal to `last_block.numer + 1`.

`tx_root`, merkle root of transactions, the transactions are separated from block structure to make blocks small.

`previous_account_root`, merkle root of all accounts before this block.

`current_account_root`, merkle root of all accounts after this block.

`ag_sig`, aggregator's signature, the signed message is computed by filling zeros to the `ag_sig` field then hash the block.

`ag_index`, the index of the aggregator account.

### Tx

```
table Tx {
    from_index: Uint32,
    to_index: Uint32,
    nonce: Uint32, // nonce
    amount: Payment, // amount
    fee: Payment, // fee
    args: Bytes, // args
    witness: Bytes, // tx witness
}

union Payment {
    Uint32,
    UDTPayment,
}

struct UDTPayment {
    type_hash: Byte32,
    amount: Uint32,
}
```

`from_index` is the payer, and `to_index` is the recipient.

`nonce` must equals to `account.nonce + 1`.

`amount` can be either native token or UDT.

`fee` is transferred to the aggregator's account.

`args` is used for calling contract; it has no use when the recipient is a non-contract account.

`witness` contains the user's signature of the transaction; this field will be removed after the aggregation signature support.

## Main contract

### Global state

Godwoken contract maintains a global state which constructed by two hashes: `account_root` and `block_root`.

We use [mountain merkle range](MMR for short) to calculate those two roots. The MMR allows us efficiently accumulate new elements; it's suitable for our use case: which continuously produces new blocks and new accounts.

The `account_root` is calculated from `hash(account_count | account_mmr.root)`, we put `account_count` into the root so it's simply to detect the index for a new registered account.

The `block_root` is calculated from `hash(block_count | block_mmr.root)`, just like `account_root`, the `block_count` in the root helps us to check the new block.

When a dispute occurred, the contract may need some data to resolve the dispute on layer-1; we can generate merkle proof from MMR, and submit the actual data and the merkle proof to the contract.

### Supported actions

Godwoken contract supports several actions to update the global state:

* register
* deposit
* submit block
* invalid block
* prepare_withdraw
* withdraw

`register`, deposit layer-1 assets, and register a new account on Godwoken contract, the `index` of the new account must be `last_account.index + 1`; the `nonce` must be `0`; the `state_root` must be the merkle root of deposited assets (construct a sparse merkle tree as previous sections mentioned); `script` can be set to default script or a contract.

`deposit`, deposit layer-1 assets, and update account's `state_root`.

`submit block`, only an aggregator account with the required balance, can invoke this action. The aggregator needs to commit `block`, `transactions`, and merkle proof; the `transactions` will not verify on-chain; however other users can send an invalid block action to penalize the aggregator who committed an invalid block and take the deposited assets from the aggregator.

`invalid block`, any account can send this action to invalid a block, challenger collects invalided `block`, `transactions` and `touched_accounts`; `touched_accounts` contains all accounts involved in the transactions, plus the aggregator's account and the challenger's account. This action replace the invalided `block` with a penalized block: `Block { (untouched fields: number, previous_account_root), tx_root: 0x00..00, ag_sig: 0x00..00, ag_index: challenger_index, current_account_root: new_account_root }`, in the `new_account_root` a part of the invalided aggregator's CKB is sent to challenger's account as reward.

`prepare_withdraw`, move assets from account's `state_root` to a field called `withdraw_state_root`.

`withdraw`, after `WITHDRAW_WAIT` blocks of the `prepare_withdraw` action; a user can take assets from `withdraw_state_root` to layer-1.

## Account contract

(TODO)

[merkle mountain range]: https://github.com/nervosnetwork/merkle-mountain-range "merkle mountain range"
[sparse merkle tree]: https://github.com/jjyr/sparse-merkle-tree "sparse merkle tree"
