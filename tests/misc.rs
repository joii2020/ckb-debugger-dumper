use ckb_chain_spec::consensus::{Consensus, ConsensusBuilder};
use ckb_script::TxVerifyEnv;
use ckb_traits::{CellDataProvider, HeaderProvider};
use ckb_types::{
    bytes::{BufMut, Bytes, BytesMut},
    core::{
        cell::CellMeta,
        cell::{CellMetaBuilder, ResolvedTransaction},
        hardfork::HardForkSwitch,
        Capacity, DepType, EpochNumberWithFraction, HeaderView, ScriptHashType, TransactionBuilder,
    },
    packed::{Byte, Byte32, CellDep, CellInput, CellOutput, OutPoint, Script, WitnessArgsBuilder},
    prelude::*,
};
use rand::{thread_rng, Rng};
use std::{collections::HashMap, convert::TryInto, io::Read, process::Command, str::from_utf8};

#[derive(Default)]
pub struct DummyDataLoader {
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

pub fn gen_rand_array32() -> [u8; 32] {
    let mut buf = [0u8; 32];
    let mut rng = thread_rng();
    rng.fill(&mut buf);
    buf
}

pub fn gen_rand_byte32() -> Byte32 {
    Byte32::new(gen_rand_array32())
}

pub fn gen_rand_bytes(capacity: usize) -> Bytes {
    let mut ret = BytesMut::with_capacity(capacity);

    let mut rnd = thread_rng();
    for _i in 0..capacity {
        ret.put_u8(rnd.gen_range(0, 255));
    }

    ret.freeze()
}

pub struct CkbDepsData {
    pub data: Bytes,
    pub data_type: ScriptHashType,
    pub tx_hash: Byte32,
    pub tx_index: u32,

    pub out_point: Option<OutPoint>,
    pub type_hash: Option<Byte32>,
}

#[derive(Clone)]
pub struct CkbScriptData {
    pub script_id: u32,
    pub args: Bytes,
    pub witness: Bytes,
}

#[derive(Clone)]
pub struct CkbCellScritp {
    pub lock: CkbScriptData,
    pub type_: Option<CkbScriptData>,
}

#[derive(Clone)]
pub struct CkbCellData {
    pub input_tx_hash: Byte32,

    pub input_data: Bytes,
    pub output_data: Bytes,

    pub input_script: CkbCellScritp,
    pub output_script: CkbCellScritp,
}

pub fn gen_consensus() -> Consensus {
    let hardfork_switch = HardForkSwitch::new_without_any_enabled()
        .as_builder()
        .rfc_0032(200)
        .build()
        .unwrap();
    ConsensusBuilder::default()
        .hardfork_switch(hardfork_switch)
        .build()
}

pub fn gen_tx_env() -> TxVerifyEnv {
    let epoch = EpochNumberWithFraction::new(300, 0, 1);
    let header = HeaderView::new_advanced_builder()
        .epoch(epoch.pack())
        .build();
    TxVerifyEnv::new_commit(&header)
}

pub fn load_bin(path: &String) -> Bytes {
    let mut f =
        std::fs::File::open(path).expect(format!("open bin file failed: {}", path).as_str());

    let mut buf: Vec<u8> = Vec::new();
    f.read_to_end(&mut buf)
        .expect(format!("read bin file failed: {}", path).as_str());
    Bytes::from(buf)
}

pub fn gen_cell_script(script: CkbScriptData, deps: &HashMap<u32, CkbDepsData>) -> Script {
    let code = deps.get(&script.script_id).unwrap();
    let code_hash = {
        if code.data_type == ScriptHashType::Type {
            code.type_hash.clone().unwrap()
        } else {
            CellOutput::calc_data_hash(&code.data)
        }
    };
    let ret = Script::new_builder()
        .args(script.args.pack())
        .code_hash(code_hash)
        .hash_type(code.data_type.into())
        .build();
    ret
}

pub fn gen_cell_output(
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

pub fn gen_ckb_tx(
    cells: Vec<CkbCellData>,
    deps: HashMap<u32, CkbDepsData>,
    header_dep: Vec<HeaderView>,
) -> (ResolvedTransaction, DummyDataLoader) {
    let mut tx_builder = TransactionBuilder::default();
    let mut dummy = DummyDataLoader::new();

    let mut deps_max_count: u32 = 0;
    for (id, _) in &deps {
        if *id > deps_max_count {
            deps_max_count = *id;
        }
    }
    let mut deps = deps;
    for i in 0..deps_max_count {
        let dep = deps.get_mut(&i);
        if dep.is_none() {
            continue;
        }

        let dep = dep.unwrap();

        let out_point = OutPoint::new(dep.tx_hash.clone(), dep.tx_index.clone());

        let mut output_builder =
            CellOutput::new_builder().capacity(Capacity::bytes(dep.data.len()).unwrap().pack());

        if dep.data_type == ScriptHashType::Type {
            let type_sc: Script = Script::new_builder()
                .args(Bytes::from([0xFF; 32].to_vec()).pack())
                .code_hash(Byte32::new([0xFE; 32]))
                .hash_type(ScriptHashType::Data1.into())
                .build();
            dep.type_hash = Some(type_sc.calc_script_hash());
            output_builder = output_builder.type_(Some(type_sc).pack());
        }

        // dep contract code
        let sighash_all_cell = output_builder.build();
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

    for header in header_dep {
        tx_builder = tx_builder.header_dep(header.hash());
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

pub fn vec_to_slice<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

pub fn u32_to_uint32(d: u32) -> ckb_types::packed::Uint32 {
    let b = ckb_types::packed::Uint32::new_builder();
    let d: Vec<Byte> = d
        .to_le_bytes()
        .to_vec()
        .iter()
        .map(|f| f.clone().into())
        .collect();
    let d: [Byte; 4] = vec_to_slice(d);
    b.set(d).build()
}

pub fn u64_to_uint64(d: u64) -> ckb_types::packed::Uint64 {
    let b = ckb_types::packed::Uint64::new_builder();
    let d: Vec<Byte> = d
        .to_le_bytes()
        .to_vec()
        .iter()
        .map(|f| f.clone().into())
        .collect();
    let d: [Byte; 8] = vec_to_slice(d);
    b.set(d).build()
}

pub fn u128_to_uint128(d: u128) -> ckb_types::packed::Uint128 {
    let b = ckb_types::packed::Uint128::new_builder();
    let d: Vec<Byte> = d
        .to_le_bytes()
        .to_vec()
        .iter()
        .map(|f| f.clone().into())
        .collect();
    let d: [Byte; 16] = vec_to_slice(d);
    b.set(d).build()
}

pub fn run_ckb_debugger(cmd_line: &str) -> Result<String, i32> {
    let i = cmd_line.find(" ").unwrap();
    let cmd_line: String = String::from(cmd_line.split_at(i).1);
    let cmd_line = cmd_line.trim();

    let output = Command::new("c/build/ckb-debugger-bins")
        .args(cmd_line.split(" "))
        .output()
        .expect("run ckb debugger");

    let output = from_utf8(output.stdout.as_slice()).unwrap();
    let i = output.rfind("----").unwrap();
    let output = output.split_at(i + 4).0;
    //println!("{}", output);
    Ok(String::from(output))
}
