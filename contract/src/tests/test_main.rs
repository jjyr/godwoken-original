use super::utils::{build_resolved_tx, TxBuilder};
use super::{DummyDataLoader, MAIN_CONTRACT_BIN, MAX_CYCLES};
use ckb_error::Error as CKBError;
use ckb_hash::{blake2b_256, new_blake2b};
use ckb_merkle_mountain_range::{leaf_index_to_pos, util::MemMMR, Merge};
use ckb_script::TransactionScriptsVerifier;
use ckb_types::{
    core::{Cycle, TransactionView},
    packed::WitnessArgs,
    prelude::*,
};
use godwoken_types::packed::{AccountEntry, Action, Byte20, Deposit, GlobalState, Register};
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
}

impl GlobalStateContext {
    fn new() -> Self {
        Default::default()
    }

    fn get_global_state(&self) -> GlobalState {
        GlobalState::new_builder()
            .account_root(self.account_root.pack())
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
        let tx = TxBuilder::default()
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
    let tx = TxBuilder::default()
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
