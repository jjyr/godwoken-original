use super::{DummyDataLoader, DUMMY_LOCK_BIN, MAIN_CONTRACT_BIN};
use ckb_hash::new_blake2b;
use ckb_script::TransactionScriptsVerifier;
use ckb_types::{
    bytes::Bytes,
    core::{
        cell::{CellMetaBuilder, ResolvedTransaction},
        Capacity, DepType, ScriptHashType, TransactionBuilder, TransactionView,
    },
    packed::{CellDep, CellInput, CellOutput, OutPoint, Script, WitnessArgs},
    prelude::*,
};
use godwoken_types::packed::{Action, AddressEntry, GlobalState, Register};
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = std::u64::MAX;
const DUMMY_LOCK_CYCLES: u64 = 2108;

fn build_resolved_tx(data_loader: &DummyDataLoader, tx: &TransactionView) -> ResolvedTransaction {
    let previous_out_point = tx
        .inputs()
        .get(0)
        .expect("should have at least one input")
        .previous_output();
    let resolved_cell_deps = tx
        .cell_deps()
        .into_iter()
        .map(|dep| {
            let deps_out_point = dep.clone();
            let (dep_output, dep_data) =
                data_loader.cells.get(&deps_out_point.out_point()).unwrap();
            CellMetaBuilder::from_cell_output(dep_output.to_owned(), dep_data.to_owned())
                .out_point(deps_out_point.out_point().clone())
                .build()
        })
        .collect();
    let (input_output, input_data) = data_loader.cells.get(&previous_out_point).unwrap();
    let input_cell =
        CellMetaBuilder::from_cell_output(input_output.to_owned(), input_data.to_owned())
            .out_point(previous_out_point)
            .build();
    ResolvedTransaction {
        transaction: tx.clone(),
        resolved_cell_deps,
        resolved_inputs: vec![input_cell],
        resolved_dep_groups: vec![],
    }
}

fn gen_tx(
    dummy: &mut DummyDataLoader,
    lock_bin: Bytes,
    type_bin: Option<Bytes>,
    previous_output_data: Bytes,
) -> TransactionView {
    let mut rng = thread_rng();
    let previous_tx_hash = {
        let mut buf = [0u8; 32];
        rng.fill(&mut buf);
        buf.pack()
    };
    let previous_index = 0;
    let capacity = Capacity::shannons(42);
    let previous_out_point = OutPoint::new(previous_tx_hash, previous_index);
    let contract_tx_hash = {
        let mut buf = [0u8; 32];
        rng.fill(&mut buf);
        buf.pack()
    };
    let lock_out_point = OutPoint::new(contract_tx_hash.clone(), 0);
    let type_out_point = OutPoint::new(contract_tx_hash.clone(), 1);
    // deploy contract code
    let lock_data_hash = CellOutput::calc_data_hash(&lock_bin);
    {
        let dep_cell = CellOutput::new_builder()
            .capacity(
                Capacity::bytes(lock_bin.len())
                    .expect("script capacity")
                    .pack(),
            )
            .build();
        dummy
            .cells
            .insert(lock_out_point.clone(), (dep_cell, lock_bin));
    }
    // setup unlock script
    let lock_script = Script::new_builder()
        .code_hash(lock_data_hash)
        .hash_type(ScriptHashType::Data.into())
        .build();
    let cell_to_spent = CellOutput::new_builder()
        .capacity(capacity.pack())
        .lock(lock_script)
        .build();
    let mut output_cell = CellOutput::new_builder().capacity(capacity.pack()).build();

    // setup type script
    if let Some(type_bin) = type_bin.clone() {
        let type_data_hash = CellOutput::calc_data_hash(&type_bin);
        {
            let dep_cell = CellOutput::new_builder()
                .capacity(
                    Capacity::bytes(type_bin.len())
                        .expect("script capacity")
                        .pack(),
                )
                .build();
            dummy
                .cells
                .insert(type_out_point.clone(), (dep_cell, type_bin));
        }
        let type_script = Script::new_builder()
            .code_hash(type_data_hash)
            .hash_type(ScriptHashType::Data.into())
            .build();
        output_cell = output_cell
            .as_builder()
            .type_(Some(type_script).pack())
            .build();
    }
    dummy.cells.insert(
        previous_out_point.clone(),
        (cell_to_spent, previous_output_data),
    );
    let mut tx_builder = TransactionBuilder::default()
        .input(CellInput::new(previous_out_point.clone(), 0))
        .cell_dep(
            CellDep::new_builder()
                .out_point(lock_out_point)
                .dep_type(DepType::Code.into())
                .build(),
        )
        .output(output_cell)
        .output_data(Bytes::new().pack());
    if type_bin.is_some() {
        tx_builder = tx_builder.cell_dep(
            CellDep::new_builder()
                .out_point(type_out_point)
                .dep_type(DepType::Code.into())
                .build(),
        );
    }
    tx_builder.build()
}

#[test]
fn test_dummy_lock() {
    let mut data_loader = DummyDataLoader::new();
    let tx = gen_tx(&mut data_loader, DUMMY_LOCK_BIN.clone(), None, Bytes::new());
    let resolved_tx = build_resolved_tx(&data_loader, &tx);
    let verify_result =
        TransactionScriptsVerifier::new(&resolved_tx, &data_loader).verify(MAX_CYCLES);
    let cycles = verify_result.expect("pass verification");
    assert_eq!(cycles, DUMMY_LOCK_CYCLES);
}

#[test]
fn test_main() {
    let mut data_loader = DummyDataLoader::new();
    let global_state = GlobalState::new_builder().build();
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
    let new_global_state = {
        let mut address_entry_root = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(address_entry.as_slice());
        hasher.finalize(&mut address_entry_root);
        let mut address_root = [0u8; 32];
        let mut hasher = new_blake2b();
        let mut entries_count: u32 = address_entry.index().unpack();
        entries_count += 1;
        hasher.update(&entries_count.to_le_bytes());
        hasher.update(&address_entry_root);
        hasher.finalize(&mut address_root);
        GlobalState::new_builder()
            .address_root(address_root.pack())
            .build()
    };
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
