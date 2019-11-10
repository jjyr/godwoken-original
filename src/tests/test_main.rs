use super::utils::{build_resolved_tx, gen_tx};
use super::{DummyDataLoader, DUMMY_LOCK_BIN, MAIN_CONTRACT_BIN, MAX_CYCLES};
use ckb_hash::{blake2b_256, new_blake2b};
use ckb_merkle_mountain_range::{util::MemMMR, Merge};
use ckb_script::TransactionScriptsVerifier;
use ckb_types::{packed::WitnessArgs, prelude::*};
use godwoken_types::packed::{Action, AddressEntry, GlobalState, Register};

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
}

#[test]
fn test_main() {
    let mut data_loader = DummyDataLoader::new();
    let mut global_state_context = GlobalStateContext::new();
    let global_state = global_state_context.get_global_state();
    let tx = gen_tx(
        &mut data_loader,
        DUMMY_LOCK_BIN.clone(),
        Some(MAIN_CONTRACT_BIN.clone()),
        global_state.as_bytes(),
    );

    // insert a new address
    let address_entry = AddressEntry::new_builder().build();
    let register = Register::new_builder()
        .address_entry(address_entry.clone())
        .build();
    let action = Action::new_builder().set(register).build();
    global_state_context.add_address_entry(address_entry);
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
    let resolved_tx = build_resolved_tx(&data_loader, &tx);
    let mut verifier = TransactionScriptsVerifier::new(&resolved_tx, &data_loader);
    verifier.set_debug_printer(|id, msg| {
        println!("[{}] {}", id, msg);
    });
    let verify_result = verifier.verify(MAX_CYCLES);
    verify_result.expect("pass verification");
}
