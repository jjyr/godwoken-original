use super::utils::{build_resolved_tx, gen_tx};
use super::{DummyDataLoader, DUMMY_LOCK_BIN, MAIN_CONTRACT_BIN, MAX_CYCLES};
use ckb_error::Error as CKBError;
use ckb_hash::{blake2b_256, new_blake2b};
use ckb_merkle_mountain_range::{leaf_index_to_pos, util::MemMMR, Merge};
use ckb_script::TransactionScriptsVerifier;
use ckb_types::{
    core::{Cycle, TransactionView},
    packed::WitnessArgs,
    prelude::*,
};
use godwoken_types::packed::{Action, AddressEntry, Byte20, GlobalState, Register};
use rand::{thread_rng, Rng};

struct HashMerge;

impl Merge for HashMerge {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        println!("merge left {:?}", left.pack());
        println!("merge right {:?}", right.pack());
        let mut merge_result = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize(&mut merge_result);
        println!("result {}\n", merge_result.pack());
        merge_result
    }
}

type HashMMR = MemMMR<[u8; 32], HashMerge>;

#[derive(Default)]
struct GlobalStateContext {
    address_root: [u8; 32],
    balance_root: [u8; 32],
    address_mmr: HashMMR,
}

impl GlobalStateContext {
    fn new() -> Self {
        Default::default()
    }

    fn get_global_state(&self) -> GlobalState {
        GlobalState::new_builder()
            .address_root(self.address_root.pack())
            .balance_root(self.balance_root.pack())
            .build()
    }

    fn add_address_entry(&mut self, entry: AddressEntry) {
        let entry_hash = blake2b_256(entry.as_slice());
        self.address_mmr.push(entry_hash).expect("mmr push");
        let address_mmr_root = self.address_mmr.get_root().expect("mmr root");
        let mut entries_count: u32 = entry.index().unpack();
        entries_count += 1;
        let mut hasher = new_blake2b();
        hasher.update(&entries_count.to_le_bytes());
        hasher.update(&address_mmr_root);
        hasher.finalize(&mut self.address_root);
    }

    fn gen_address_merkle_proof(&self, leaf_index: u32) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .address_mmr
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

    // insert a new address
    let tx = gen_tx(
        &mut data_loader,
        DUMMY_LOCK_BIN.clone(),
        Some(MAIN_CONTRACT_BIN.clone()),
        global_state.as_bytes(),
    );
    let address_entry = AddressEntry::new_builder().build();
    let register = Register::new_builder()
        .address_entry(address_entry.clone())
        .build();
    let action = Action::new_builder().set(register).build();
    global_state_context.add_address_entry(address_entry.clone());
    let new_global_state = global_state_context.get_global_state();

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

    // insert more addresses
    let mut last_address_entry = address_entry;
    let mut global_state = new_global_state;
    for index in 1u32..=5u32 {
        let tx = gen_tx(
            &mut data_loader,
            DUMMY_LOCK_BIN.clone(),
            Some(MAIN_CONTRACT_BIN.clone()),
            global_state.as_bytes(),
        );
        let address_entry = {
            let mut pubkey = [0u8; 20];
            let mut rng = thread_rng();
            rng.fill(&mut pubkey);
            AddressEntry::new_builder()
                .index(index.pack())
                .pubkey_hash(Byte20::new_unchecked(pubkey.to_vec().into()))
                .build()
        };
        let register = {
            let (mmr_size, proof) =
                global_state_context.gen_address_merkle_proof(last_address_entry.index().unpack());
            Register::new_builder()
                .address_entry(address_entry.clone())
                .last_address_entry_hash(blake2b_256(last_address_entry.as_slice()).pack())
                .mmr_size(mmr_size.pack())
                .proof(
                    proof
                        .into_iter()
                        .map(|i| i.pack())
                        .collect::<Vec<_>>()
                        .pack(),
                )
                .build()
        };
        let action = Action::new_builder().set(register).build();
        global_state_context.add_address_entry(address_entry.clone());
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
        last_address_entry = address_entry;
        global_state = new_global_state;
    }
}

#[test]
fn test_merkle() {
    let mut mmr = HashMMR::new(0, Default::default());
    for i in 0u32..=7u32 {
        let mut buf = [0u8; 32];
        buf[0..4].clone_from_slice(&i.to_le_bytes());
        mmr.push(buf).unwrap();
    }
    let i = 7u32;
    let mut buf = [0u8; 32];
    buf[0..4].clone_from_slice(&i.to_le_bytes());
    let proof = mmr.gen_proof(leaf_index_to_pos(i.into())).unwrap();
    let root = mmr.get_root().unwrap();
    let result = proof
        .verify(root, leaf_index_to_pos(i.into()), buf)
        .unwrap();
    assert!(result);
}
