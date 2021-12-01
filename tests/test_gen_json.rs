use ckb_chain_spec::consensus::{Consensus, ConsensusBuilder};
use ckb_script::{TransactionScriptsVerifier, TxVerifyEnv};
use ckb_traits::{CellDataProvider, HeaderProvider};
use ckb_types::{
    bytes::{BufMut, Bytes, BytesMut},
    core::{
        cell::{CellMeta, CellMetaBuilder, ResolvedTransaction},
        hardfork::HardForkSwitch,
        Capacity, DepType, EpochNumberWithFraction, HeaderView, ScriptHashType, TransactionBuilder,
    },
    packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint, Script, WitnessArgsBuilder},
    prelude::*,
};
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use std::{collections::HashMap, vec};

lazy_static! {
    pub static ref DUMP_BIN_PATH: String = String::from("c/build/dump");
    pub static ref DUMP_BIN: Bytes = Bytes::from(&include_bytes!("../c/build/dump")[..]);
}

#[derive(Default)]
struct DummyDataLoader {
    pub cells: HashMap<OutPoint, (CellOutput, ckb_types::bytes::Bytes)>,
}

impl DummyDataLoader {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CellDataProvider for DummyDataLoader {
    // load Cell Data
    fn load_cell_data(&self, cell: &CellMeta) -> Option<ckb_types::bytes::Bytes> {
        cell.mem_cell_data.clone().or_else(|| {
            self.cells
                .get(&cell.out_point)
                .map(|(_, data)| data.clone())
        })
    }

    fn load_cell_data_hash(&self, cell: &CellMeta) -> Option<Byte32> {
        self.load_cell_data(cell)
            .map(|e| CellOutput::calc_data_hash(&e))
    }

    fn get_cell_data(&self, _out_point: &OutPoint) -> Option<ckb_types::bytes::Bytes> {
        None
    }

    fn get_cell_data_hash(&self, _out_point: &OutPoint) -> Option<Byte32> {
        None
    }
}

impl HeaderProvider for DummyDataLoader {
    fn get_header(&self, _hash: &Byte32) -> Option<HeaderView> {
        None
    }
}

fn gen_rand_array32() -> [u8; 32] {
    let mut buf = [0u8; 32];
    let mut rng = thread_rng();
    rng.fill(&mut buf);
    buf
}

fn gen_rand_byte32() -> Byte32 {
    Byte32::new(gen_rand_array32())
}

fn gen_rand_bytes(capacity: usize) -> Bytes {
    let mut ret = BytesMut::with_capacity(capacity);

    let mut rnd = thread_rng();
    for _i in 0..capacity {
        ret.put_u8(rnd.gen_range(0, 255));
    }

    ret.freeze()
}

struct CkbDepsData {
    data: Bytes,
    data_type: ScriptHashType,
    tx_hash: Byte32,
    tx_index: u32,

    out_point: Option<OutPoint>,
}

#[derive(Clone)]
struct CkbScriptData {
    script_id: u32,
    args: Bytes,
    witness: Bytes,
}

#[derive(Clone)]
struct CkbCellScritp {
    lock: CkbScriptData,
    type_: Option<CkbScriptData>,
}

#[derive(Clone)]
struct CkbCellData {
    input_tx_hash: Byte32,

    input_data: Bytes,
    output_data: Bytes,

    input_script: CkbCellScritp,
    output_script: CkbCellScritp,
}

fn gen_consensus() -> Consensus {
    let hardfork_switch = HardForkSwitch::new_without_any_enabled()
        .as_builder()
        .rfc_0232(200)
        .build()
        .unwrap();
    ConsensusBuilder::default()
        .hardfork_switch(hardfork_switch)
        .build()
}

fn gen_tx_env() -> TxVerifyEnv {
    let epoch = EpochNumberWithFraction::new(300, 0, 1);
    let header = HeaderView::new_advanced_builder()
        .epoch(epoch.pack())
        .build();
    TxVerifyEnv::new_commit(&header)
}

fn gen_cell_script(script: CkbScriptData, deps: &HashMap<u32, CkbDepsData>) -> Script {
    let code = deps.get(&script.script_id).unwrap();
    let code_hash = CellOutput::calc_data_hash(&code.data);
    let ret = Script::new_builder()
        .args(script.args.pack())
        .code_hash(code_hash)
        .hash_type(code.data_type.into())
        .build();
    ret
}

fn gen_cell_output(
    cell_script: CkbCellScritp,
    cell_data: Bytes,
    deps: &HashMap<u32, CkbDepsData>,
) -> CellOutput {
    let cell_capacity = Capacity::bytes(cell_data.len()).unwrap();
    let mut input_cell = CellOutput::new_builder()
        .capacity(cell_capacity.pack())
        .lock(gen_cell_script(cell_script.lock.clone(), &deps));
    if cell_script.type_.is_some() {
        input_cell = input_cell
            .type_(Some(gen_cell_script(cell_script.type_.clone().unwrap(), &deps)).pack());
    }
    input_cell.build()
}

fn gen_ckb_tx(
    cells: Vec<CkbCellData>,
    deps: HashMap<u32, CkbDepsData>,
) -> (ResolvedTransaction, DummyDataLoader) {
    let mut tx_builder = TransactionBuilder::default();
    let mut dummy = DummyDataLoader::new();

    let mut deps = deps;

    for (_id, dep) in &mut deps {
        let out_point = OutPoint::new(dep.tx_hash.clone(), dep.tx_index.clone());

        // dep contract code
        let sighash_all_cell = CellOutput::new_builder()
            .capacity(Capacity::bytes(dep.data.len()).unwrap().pack())
            .build();
        dummy
            .cells
            .insert(out_point.clone(), (sighash_all_cell, dep.data.clone()));
        dep.out_point = Option::Some(out_point.clone());

        tx_builder = tx_builder.cell_dep(
            CellDep::new_builder()
                .out_point(out_point)
                .dep_type(DepType::Code.into())
                .build(),
        );
    }

    for cell in cells {
        let input_cell = gen_cell_output(cell.input_script.clone(), cell.input_data.clone(), &deps);
        let input_out_point = OutPoint::new(cell.input_tx_hash.clone(), 0);

        dummy
            .cells
            .insert(input_out_point.clone(), (input_cell, cell.input_data));

        tx_builder = tx_builder.input(CellInput::new(input_out_point, 0));
        tx_builder = tx_builder
            .output(gen_cell_output(
                cell.output_script.clone(),
                cell.output_data.clone(),
                &deps,
            ))
            .output_data(cell.output_data.pack());

        let mut witness = WitnessArgsBuilder::default();
        if !cell.input_script.lock.witness.is_empty() {
            witness = witness.lock(Some(cell.input_script.lock.witness).pack());
        }

        if cell.input_script.type_.is_some() {
            let t = cell.input_script.type_.unwrap();
            if !t.witness.is_empty() {
                witness = witness.input_type(Some(t.witness).pack());
            }
        }

        if cell.output_script.type_.is_some() {
            let t = cell.output_script.type_.unwrap();
            if !t.witness.is_empty() {
                witness = witness.output_type(Some(t.witness).pack());
            }
        }

        tx_builder = tx_builder.witness(witness.build().as_bytes().pack());
    }

    let tx_builder = tx_builder.build();

    let resolved_cell_deps = tx_builder
        .cell_deps()
        .into_iter()
        .map(|deps_out_point| {
            let (dep_output, dep_data) = dummy.cells.get(&deps_out_point.out_point()).unwrap();
            CellMetaBuilder::from_cell_output(dep_output.to_owned(), dep_data.to_owned())
                .out_point(deps_out_point.out_point())
                .build()
        })
        .collect();

    let mut resolved_inputs = Vec::new();
    for i in 0..tx_builder.inputs().len() {
        let previous_out_point = tx_builder.inputs().get(i).unwrap().previous_output();
        let (input_output, input_data) = dummy.cells.get(&previous_out_point).unwrap();
        resolved_inputs.push(
            CellMetaBuilder::from_cell_output(input_output.to_owned(), input_data.to_owned())
                .out_point(previous_out_point)
                .build(),
        );
    }

    let tx = ResolvedTransaction {
        transaction: tx_builder.clone(),
        resolved_cell_deps,
        resolved_inputs,
        resolved_dep_groups: vec![],
    };
    (tx, dummy)
}

#[test]
fn test_multiple() {
    let mut deps: HashMap<u32, CkbDepsData> = HashMap::new();

    deps.insert(
        0,
        CkbDepsData {
            data: DUMP_BIN.clone(),
            data_type: ScriptHashType::Data1,
            tx_hash: gen_rand_byte32(),
            tx_index: 0,
            out_point: Option::None,
        },
    );
    let mut cells: Vec<CkbCellData> = Vec::new();

    let lock_script1 = CkbScriptData {
        script_id: 0,
        args: gen_rand_bytes(32),
        witness: gen_rand_bytes(100),
    };
    let lock_script2 = CkbScriptData {
        script_id: 0,
        args: gen_rand_bytes(32),
        witness: gen_rand_bytes(100),
    };

    let type_script1 = CkbScriptData {
        script_id: 0,
        args: gen_rand_bytes(32),
        witness: gen_rand_bytes(100),
    };
    let type_script2 = CkbScriptData {
        script_id: 0,
        args: gen_rand_bytes(32),
        witness: gen_rand_bytes(100),
    };

    cells.push(CkbCellData {
        input_tx_hash: gen_rand_byte32(),
        input_data: gen_rand_bytes(123),
        output_data: gen_rand_bytes(123),

        input_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Option::None,
        },
        output_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Option::None,
        },
    });

    cells.push(CkbCellData {
        input_tx_hash: gen_rand_byte32(),
        input_data: gen_rand_bytes(456),
        output_data: gen_rand_bytes(456),

        input_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Some(type_script1.clone()),
        },
        output_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Some(type_script1.clone()),
        },
    });

    cells.push(CkbCellData {
        input_tx_hash: gen_rand_byte32(),
        input_data: gen_rand_bytes(12),
        output_data: gen_rand_bytes(12),

        input_script: CkbCellScritp {
            lock: lock_script2.clone(),
            type_: Some(type_script1.clone()),
        },
        output_script: CkbCellScritp {
            lock: lock_script2.clone(),
            type_: Some(type_script1.clone()),
        },
    });

    cells.push(CkbCellData {
        input_tx_hash: gen_rand_byte32(),
        input_data: gen_rand_bytes(99),
        output_data: gen_rand_bytes(99),

        input_script: CkbCellScritp {
            lock: lock_script2.clone(),
            type_: Some(type_script2.clone()),
        },
        output_script: CkbCellScritp {
            lock: lock_script2.clone(),
            type_: Some(type_script2.clone()),
        },
    });

    let (tx, dummy) = gen_ckb_tx(cells, deps);

    let consensus = gen_consensus();
    let env = gen_tx_env();
    let verifier = TransactionScriptsVerifier::new(&tx, &consensus, &dummy, &env);
    verifier.verify(10000000).expect("run failed");

    let cmd_line =
        ckb_transaction_dumper_rs::gen_json(&verifier, &tx, 0, DUMP_BIN_PATH.as_str(), "test_multi.json");
    println!("debugger command is: \n{}", cmd_line);
}

#[test]
fn test_single() {
    let mut deps: HashMap<u32, CkbDepsData> = HashMap::new();

    deps.insert(
        0,
        CkbDepsData {
            data: DUMP_BIN.clone(),
            data_type: ScriptHashType::Data1,
            tx_hash: Byte32::new([0; 32]),
            tx_index: 0,
            out_point: Option::None,
        },
    );
    let mut cells: Vec<CkbCellData> = Vec::new();

    let lock_script1 = CkbScriptData {
        script_id: 0,
        args: Bytes::from([1; 32].to_vec()),
        witness: Bytes::from([2; 100].to_vec()),
    };

    let type_script1 = CkbScriptData {
        script_id: 0,
        args: Bytes::from([3; 64].to_vec()),
        witness: Bytes::from([4; 100].to_vec()),
    };

    cells.push(CkbCellData {
        input_tx_hash: gen_rand_byte32(),
        input_data: Bytes::from([5; 123].to_vec()),
        output_data: Bytes::from([6; 123].to_vec()),

        input_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Some(type_script1.clone()),
        },
        output_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Some(type_script1.clone()),
        },
    });

    let (tx, dummy) = gen_ckb_tx(cells, deps);

    let consensus = gen_consensus();
    let env = gen_tx_env();
    let verifier = TransactionScriptsVerifier::new(&tx, &consensus, &dummy, &env);

    let cmd_line =
        ckb_transaction_dumper_rs::gen_json(&verifier, &tx, 0, DUMP_BIN_PATH.as_str(), "test.json");

    println!("debugger command is: \n{}", cmd_line);
}
