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

int main() { return 0; }
