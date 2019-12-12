use super::{DummyDataLoader, DUMMY_LOCK_BIN, MAX_CYCLES};
use ckb_error::Error as CKBError;
use ckb_script::TransactionScriptsVerifier;
use ckb_types::{
    bytes::Bytes,
    core::{
        cell::{CellMetaBuilder, ResolvedTransaction},
        Capacity, Cycle, DepType, ScriptHashType, TransactionBuilder, TransactionView,
    },
    packed::{CellDep, CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};
use rand::{thread_rng, Rng};

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
            CellMetaBuilder::from_cell_output(dep_output.to_owned(), dep_data.to_vec().into())
                .out_point(deps_out_point.out_point().clone())
                .build()
        })
        .collect();
    let (input_output, input_data) = data_loader.cells.get(&previous_out_point).unwrap();
    let input_cell =
        CellMetaBuilder::from_cell_output(input_output.to_owned(), input_data.to_vec().into())
            .out_point(previous_out_point)
            .build();
    ResolvedTransaction {
        transaction: tx.clone(),
        resolved_cell_deps,
        resolved_inputs: vec![input_cell],
        resolved_dep_groups: vec![],
    }
}

pub struct ContractCallTxBuilder {
    lock_bin: Vec<u8>,
    type_bin: Option<Vec<u8>>,
    previous_output_data: Vec<u8>,
    input_capacity: u64,
    output_capacity: u64,
    witnesses: Vec<Vec<u8>>,
    outputs_data: Vec<Vec<u8>>,
}

impl Default for ContractCallTxBuilder {
    fn default() -> Self {
        ContractCallTxBuilder {
            lock_bin: DUMMY_LOCK_BIN.to_vec(),
            type_bin: None,
            previous_output_data: Vec::new(),
            input_capacity: 42,
            output_capacity: 42,
            witnesses: Vec::new(),
            outputs_data: Vec::new(),
        }
    }
}

impl ContractCallTxBuilder {
    pub fn lock_bin(mut self, lock_bin: Vec<u8>) -> Self {
        self.lock_bin = lock_bin;
        self
    }

    pub fn type_bin(mut self, type_bin: Vec<u8>) -> Self {
        self.type_bin = Some(type_bin);
        self
    }

    pub fn previous_output_data(mut self, data: Vec<u8>) -> Self {
        self.previous_output_data = data;
        self
    }

    pub fn input_capacity(mut self, capacity: u64) -> Self {
        self.input_capacity = capacity;
        self
    }

    pub fn output_capacity(mut self, capacity: u64) -> Self {
        self.output_capacity = capacity;
        self
    }

    pub fn witnesses(mut self, witnesses: Vec<Vec<u8>>) -> Self {
        self.witnesses = witnesses;
        self
    }

    pub fn outputs_data(mut self, outputs_data: Vec<Vec<u8>>) -> Self {
        self.outputs_data = outputs_data;
        self
    }

    pub fn build(self, dummy: &mut DummyDataLoader) -> TransactionView {
        let lock_bin = self.lock_bin;
        let type_bin = self.type_bin;
        let previous_output_data = self.previous_output_data;
        let input_capacity = Capacity::shannons(self.input_capacity);
        let output_capacity = Capacity::shannons(self.output_capacity);
        let tx = gen_tx(
            dummy,
            lock_bin,
            type_bin,
            previous_output_data,
            input_capacity,
            output_capacity,
        );
        let witnesses = self
            .witnesses
            .into_iter()
            .map(|wit| Bytes::from(wit).pack())
            .collect();
        let outputs_data = self
            .outputs_data
            .into_iter()
            .map(|data| Bytes::from(data).pack())
            .collect();
        tx.as_advanced_builder()
            .set_witnesses(witnesses)
            .set_outputs_data(outputs_data)
            .build()
    }
}

fn gen_tx(
    dummy: &mut DummyDataLoader,
    lock_bin: Vec<u8>,
    type_bin: Option<Vec<u8>>,
    previous_output_data: Vec<u8>,
    input_capacity: Capacity,
    output_capacity: Capacity,
) -> TransactionView {
    let mut rng = thread_rng();
    let previous_tx_hash = {
        let mut buf = [0u8; 32];
        rng.fill(&mut buf);
        buf.pack()
    };
    let previous_index = 0;
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
            .insert(lock_out_point.clone(), (dep_cell, lock_bin.into()));
    }
    // setup unlock script
    let lock_script = Script::new_builder()
        .code_hash(lock_data_hash)
        .hash_type(ScriptHashType::Data.into())
        .build();
    let cell_to_spent = CellOutput::new_builder()
        .capacity(input_capacity.pack())
        .lock(lock_script)
        .build();
    let mut output_cell = CellOutput::new_builder()
        .capacity(output_capacity.pack())
        .build();

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
                .insert(type_out_point.clone(), (dep_cell, type_bin.into()));
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
        (cell_to_spent, previous_output_data.into()),
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

pub fn verify_tx(data_loader: &DummyDataLoader, tx: &TransactionView) -> Result<Cycle, CKBError> {
    let resolved_tx = build_resolved_tx(data_loader, tx);
    let mut verifier = TransactionScriptsVerifier::new(&resolved_tx, data_loader);
    verifier.set_debug_printer(|_id, msg| {
        println!("[contract debug] {}", msg);
    });
    verifier.verify(MAX_CYCLES)
}
