# Godwoken

An experimental optimistic rollup implementation, to provide a generalized account-based programming layer upon Nervos CKB.

This [design documentation](https://github.com/jjyr/godwoken/blob/master/docs/design.md) explains the design from a high-level overview.

This project is still WIP.

## Details

Godwoken contract maintains a global state cell, allows following actions to modify the state:

* `Register` - register a new account on Godwoken.
* `Deposit` - deposit layer 1 assets to a Godwoken account.
* `SubmitBlock` - submit a aggregated block.
* `InvalidBlock` - invalid a aggregated block.
* `Withdraw` - withdraw assets from Godwoken account to layer1.

## LICENSE

[MIT LICENSE](https://github.com/jjyr/godwoken/blob/master/LICENSE.txt)

## Author

[Jiang Jinyang](https://justjjy.com)
