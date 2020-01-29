# Design

This document explains the design of Godwoken from a high-level overview.

Godwoken is an experimental optimistic rollup implementation to provide a generalized account-based programming layer upon Nervos CKB.

We mainly solve two problems: scalability and aggregation.

Godwoken applies optimistic rollup architecture to promise scalability, the godwoken transactions designed to be light-weight than the layer-1 transaction; it takes less size and does no on-chain verification.

Chains use the UTXO-like model naturally depend on off-chain aggregation. Godwoken supports aggregation by providing an account-based model; it's quite useful for many scenarios. For example: If we design a decentralized mining pool on CKB, the big problem is to split the rewards to miners every block; depends on the number of miners, the reward transaction size is enormous(think about hundred to thousands of recipients). Since CKB required at least 41 CKB to hold a cell(need 61 CKB for a cell with secp256k1 lock), the implementation is even more complicated. By building such a decentralized mining pool upon an account-based model like Godwoken, we can transfer money to thousands of miners with a small size tx, and break the 41 CKB limitation.

> Many people reference rollup as layer-1.5, layer-2, or even layer-1(by trust-level). This document references the optimistic rollup as layer-1.5 to distinguish it with layer-1.

## Global state and actions

Godwoken contract maintains a global state which constructed by two hashes:

* `account_root` - calculated from `hash(account count | account merkle root)`
* `block_root` - calculated from `hash(block count | block merkle root)`

Godwoken contract supports several actions to update the global state:

* register
* deposit
* withdraw
* submit block
* invalid block

Under optimistic rollup architecture, we define two roles: aggregator and user; all actions above can invoke by both aggregator and user except the submit-block, only an aggregator can submit a block.

Anyone layer-1 user can register themself as an aggregator or a user after deposit certain layer-1 assets to Godwoken contract. They can also deposit additional assets anytime they want.

Only a valid aggregator(with required balance) can invoke submit-block action. So others able to penalize the aggregator when the block is invalid, this also incentive aggregators to be honest.

The invalid-block action is used to invalid a block and penalize the aggregator who submits the block. The action typically invoked by an off-chain program(usually also an aggregator). The off-chain program watches the Godwoken contract; when it found a block is invalid, it automatically generates proof, then sends the invalid-block action to the Godwoken contract to grab reward.

The withdrawal action allows both user and aggregator to take back their layer-1 assets. Need to notice the withdraw is not immediately since the optimistic rollup does not on-chain verify the layer-1.5 transactions, that the withdraw action must be locked for a period to prevent the layer-1.5 block invalidation. The withdrawal lock period is relatively shorter for users and relatively long for aggregators.

## layer-1.5 account and assets transfer

(TODO)

## Generalized contract

(TODO)
