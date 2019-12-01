use super::utils::{build_resolved_tx, ContractCallTxBuilder};
use super::{DummyDataLoader, MAIN_CONTRACT_BIN, MAX_CYCLES};
use ckb_error::Error as CKBError;
use ckb_hash::{blake2b_256, new_blake2b};
use ckb_merkle_mountain_range::{leaf_index_to_pos, util::MemMMR, Merge};
use ckb_script::TransactionScriptsVerifier;
use ckb_types::{
    core::{Cycle, TransactionView},
    packed::WitnessArgs,
    prelude::*,
    utilities::merkle_root,
};
use godwoken_types::packed::{
    AccountEntry, Action, AggregatorBlock, Byte20, Deposit, GlobalState, Register, SubmitBlock, Tx,
    Txs,
};
use rand::{thread_rng, Rng};

struct HashMerge;

impl Merge for HashMerge {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut merge_result = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize(&mut merge_result);
        merge_result
    }
}

type HashMMR = MemMMR<[u8; 32], HashMerge>;

#[derive(Default)]
struct GlobalStateContext {
    account_root: [u8; 32],
    account_mmr: HashMMR,
    block_root: [u8; 32],
    block_mmr: HashMMR,
}

impl GlobalStateContext {
    fn new() -> Self {
        Default::default()
    }

    fn get_global_state(&self) -> GlobalState {
        GlobalState::new_builder()
            .account_root(self.account_root.pack())
            .block_root(self.block_root.pack())
            .build()
    }

    fn add_entry(&mut self, entry: AccountEntry) {
        let entry_hash = blake2b_256(entry.as_slice());
        self.account_mmr.push(entry_hash).expect("mmr push");
        let account_mmr_root = self.account_mmr.get_root().expect("mmr root");
        let mut entries_count: u32 = entry.index().unpack();
        entries_count += 1;
        let mut hasher = new_blake2b();
        hasher.update(&entries_count.to_le_bytes());
        hasher.update(&account_mmr_root);
        hasher.finalize(&mut self.account_root);
    }

    fn gen_account_merkle_proof(&self, leaf_index: u32) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .account_mmr
            .gen_proof(leaf_index_to_pos(leaf_index.into()))
            .expect("result");
        (proof.mmr_size(), proof.proof_items().to_owned())
    }

    fn add_block(&mut self, block: AggregatorBlock, mut count: u32) {
        let block_hash = blake2b_256(block.as_slice());
        self.block_mmr.push(block_hash).expect("mmr push");
        let block_mmr_root = self.block_mmr.get_root().expect("mmr root");
        count += 1;
        let mut hasher = new_blake2b();
        hasher.update(&count.to_le_bytes());
        hasher.update(&block_mmr_root);
        hasher.finalize(&mut self.block_root);
    }

    fn gen_block_merkle_proof(&self, leaf_index: u32) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .block_mmr
            .gen_proof(leaf_index_to_pos(leaf_index.into()))
            .expect("result");
        (proof.mmr_size(), proof.proof_items().to_owned())
    }
}

fn verify_tx(data_loader: &DummyDataLoader, tx: &TransactionView) -> Result<Cycle, CKBError> {
    let resolved_tx = build_resolved_tx(data_loader, tx);
    let mut verifier = TransactionScriptsVerifier::new(&resolved_tx, data_loader);
    verifier.set_debug_printer(|_id, msg| {
        println!("[contract debug] {}", msg);
    });
    verifier.verify(MAX_CYCLES)
}

#[test]
fn test_registration() {
    let mut data_loader = DummyDataLoader::new();
    let mut global_state_context = GlobalStateContext::new();
    let global_state = global_state_context.get_global_state();
    // insert few entries
    let mut last_entry: Option<AccountEntry> = None;
    let mut global_state = global_state;
    for index in 0u32..=5u32 {
        let tx = ContractCallTxBuilder::default()
            .type_bin(MAIN_CONTRACT_BIN.clone())
            .previous_output_data(global_state.as_bytes())
            .build(&mut data_loader);
        let entry = {
            let mut pubkey = [0u8; 20];
            let mut rng = thread_rng();
            rng.fill(&mut pubkey);
            AccountEntry::new_builder()
                .index(index.pack())
                .pubkey_hash(Byte20::new_unchecked(pubkey.to_vec().into()))
                .build()
        };
        let register = match last_entry {
            None => {
                // first entry
                Register::new_builder().entry(entry.clone()).build()
            }
            Some(last_entry) => {
                let (mmr_size, proof) =
                    global_state_context.gen_account_merkle_proof(last_entry.index().unpack());
                Register::new_builder()
                    .entry(entry.clone())
                    .last_entry_hash(blake2b_256(last_entry.as_slice()).pack())
                    .mmr_size(mmr_size.pack())
                    .proof(
                        proof
                            .into_iter()
                            .map(|i| i.pack())
                            .collect::<Vec<_>>()
                            .pack(),
                    )
                    .build()
            }
        };
        let action = Action::new_builder().set(register).build();
        global_state_context.add_entry(entry.clone());
        let new_global_state = global_state_context.get_global_state();
        let witness = WitnessArgs::new_builder()
            .output_type(Some(action.as_bytes()).pack())
            .build();
        let tx = tx
            .as_advanced_builder()
            .witnesses(vec![witness.as_bytes().pack()].pack())
            .set_outputs_data(vec![new_global_state.as_bytes().pack()])
            .build();
        verify_tx(&data_loader, &tx).expect("pass verification");
        last_entry = Some(entry);
        global_state = new_global_state;
    }
}

#[test]
fn test_deposit() {
    let mut data_loader = DummyDataLoader::new();
    let mut global_state_context = GlobalStateContext::new();
    // prepare a account entry
    let entry = AccountEntry::new_builder().build();
    global_state_context.add_entry(entry.clone());
    let global_state = global_state_context.get_global_state();

    let original_amount = 12u64;
    let deposit_amount = 42u64;

    // deposit money
    let tx = ContractCallTxBuilder::default()
        .type_bin(MAIN_CONTRACT_BIN.clone())
        .previous_output_data(global_state.as_bytes())
        .input_capacity(original_amount)
        .output_capacity(original_amount + deposit_amount)
        .build(&mut data_loader);
    let new_entry = {
        let balance: u64 = entry.balance().unpack();
        entry
            .clone()
            .as_builder()
            .balance((balance + deposit_amount).pack())
            .build()
    };
    let (mmr_size, proof) = global_state_context.gen_account_merkle_proof(entry.index().unpack());
    let deposit = Deposit::new_builder()
        .old_entry(entry.clone())
        .new_entry(new_entry.clone())
        .count(1u32.pack())
        .mmr_size(mmr_size.pack())
        .proof(
            proof
                .into_iter()
                .map(|i| i.pack())
                .collect::<Vec<_>>()
                .pack(),
        )
        .build();
    let action = Action::new_builder().set(deposit).build();
    let new_global_state = {
        let mut new_global_state_context = GlobalStateContext::new();
        new_global_state_context.add_entry(new_entry.clone());
        new_global_state_context.get_global_state()
    };

    // update tx witness
    let witness = WitnessArgs::new_builder()
        .output_type(Some(action.as_bytes()).pack())
        .build();
    let tx = tx
        .as_advanced_builder()
        .witnesses(vec![witness.as_bytes().pack()].pack())
        .set_outputs_data(vec![new_global_state.as_bytes().pack()])
        .build();
    verify_tx(&data_loader, &tx).expect("pass verification");
}

#[test]
fn test_submit_block() {
    let mut data_loader = DummyDataLoader::new();
    let mut global_state_context = GlobalStateContext::new();

    // prepare account entries
    let entry_a = AccountEntry::new_builder()
        .balance(20u64.pack())
        .index(0u32.pack())
        .build();
    let entry_b = AccountEntry::new_builder()
        .balance(100u64.pack())
        .index(1u32.pack())
        .build();
    let entry_ag = AccountEntry::new_builder()
        .balance(1000u64.pack())
        .index(2u32.pack())
        .is_aggregator(1u8.into())
        .build();
    global_state_context.add_entry(entry_a.clone());
    global_state_context.add_entry(entry_b.clone());
    global_state_context.add_entry(entry_ag.clone());
    let global_state = global_state_context.get_global_state();
    let old_account_root = global_state.account_root();

    let transfer_tx = Tx::new_builder()
        .from_index(entry_a.index())
        .to_index(entry_b.index())
        .amount(15u64.pack())
        .fee(3u64.pack())
        .nonce(1u32.pack())
        .build();

    // new account root
    let new_entry_a = entry_a
        .clone()
        .as_builder()
        .balance(2u64.pack())
        .nonce(1u32.pack())
        .build();
    let new_entry_b = entry_b.clone().as_builder().balance(115u64.pack()).build();
    let new_entry_ag = entry_ag
        .clone()
        .as_builder()
        .balance(1003u64.pack())
        .build();
    let new_account_root = {
        let mut new_global_state_context = GlobalStateContext::new();
        new_global_state_context.add_entry(new_entry_a.clone());
        new_global_state_context.add_entry(new_entry_b.clone());
        new_global_state_context.add_entry(new_entry_ag.clone());
        new_global_state_context.get_global_state().account_root()
    };

    let original_amount = 120u64;

    // send money
    let tx = ContractCallTxBuilder::default()
        .type_bin(MAIN_CONTRACT_BIN.clone())
        .previous_output_data(global_state.as_bytes())
        .input_capacity(original_amount)
        .output_capacity(original_amount)
        .build(&mut data_loader);

    let tx_root = merkle_root(&[blake2b_256(transfer_tx.as_slice()).pack()]);

    let block = AggregatorBlock::new_builder()
        .number(0u32.pack())
        .tx_root(tx_root)
        .old_account_root(old_account_root.clone())
        .new_account_root(new_account_root)
        .build();

    let (block_mmr_size, block_proof) = global_state_context.gen_block_merkle_proof(0);
    let (ag_mmr_size, ag_proof) =
        global_state_context.gen_account_merkle_proof(entry_ag.index().unpack());
    let submit_block = {
        let txs = Txs::new_builder().set(vec![transfer_tx.clone()]).build();
        SubmitBlock::new_builder()
            .txs(txs)
            .block(block.clone())
            .block_proof(
                block_proof
                    .into_iter()
                    .map(|i| i.pack())
                    .collect::<Vec<_>>()
                    .pack(),
            )
            .block_mmr_size(block_mmr_size.pack())
            .aggregator(entry_ag.clone())
            .aggregator_proof(
                ag_proof
                    .into_iter()
                    .map(|i| i.pack())
                    .collect::<Vec<_>>()
                    .pack(),
            )
            .aggregator_mmr_size(ag_mmr_size.pack())
            .account_count(3u32.pack())
            .build()
    };
    let action = Action::new_builder().set(submit_block).build();
    let new_global_state = {
        let mut new_global_state_context = GlobalStateContext::new();
        new_global_state_context.add_entry(new_entry_a.clone());
        new_global_state_context.add_entry(new_entry_b.clone());
        new_global_state_context.add_entry(new_entry_ag.clone());
        new_global_state_context.add_block(block.clone(), 0);
        new_global_state_context.get_global_state()
    };

    // update tx witness
    let witness = WitnessArgs::new_builder()
        .output_type(Some(action.as_bytes()).pack())
        .build();
    let tx = tx
        .as_advanced_builder()
        .witnesses(vec![witness.as_bytes().pack()].pack())
        .set_outputs_data(vec![new_global_state.as_bytes().pack()])
        .build();
    verify_tx(&data_loader, &tx).expect("pass verification");
}
