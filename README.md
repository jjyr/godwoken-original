# Godwoken

An experimental optimistic rollup implementation, to provide a generalized account-based programming layer upon Nervos CKB.

This project is still WIP.

## Details

Godwoken contract maintains a global state cell, allows following actions to modify the state:

* `Register` - register a new account on Godwoken.
* `Deposit` - deposit layer 1 assets to a Godwoken account.
* `SubmitBlock` - submit a aggregated block.
* `InvalidBlock` - invalid a aggregated block.
* `Withdraw` - withdraw assets from Godwoken account to layer1.

## LICENSE

Please see [LICENSE](https://github.com/jjyr/godwoken/blob/master/LICENSE.txt) for details.

## Author

[Jiang Jinyang](jjyruby@gmail.com)
