use ckb_script::TransactionScriptsVerifier;
use ckb_types::{
    bytes::Bytes,
    core::{HeaderBuilder, HeaderView, ScriptHashType},
    packed::{Byte, Byte32},
    prelude::{Builder, Entity},
};
use lazy_static::lazy_static;
use std::{collections::HashMap, convert::TryInto};

mod misc;
use misc::*;

lazy_static! {
    pub static ref DUMP_BIN_PATH: String = String::from("c/build/dump");
    pub static ref ALWAY_SUCCESS_BIN_PATH: String = String::from("c/build/always_success");
    pub static ref ALWAY_FAILED_BIN_PATH: String = String::from("c/build/always_failed");
}

fn gen_deps() -> HashMap<u32, CkbDepsData> {
    let mut deps: HashMap<u32, CkbDepsData> = HashMap::new();
    deps.insert(
        0,
        CkbDepsData {
            data: load_bin(&DUMP_BIN_PATH),
            data_type: ScriptHashType::Type,
            tx_hash: Byte32::new([1; 32]),
            tx_index: 0,
            out_point: Option::None,
            type_hash: Option::None,
        },
    );

    deps.insert(
        1,
        CkbDepsData {
            data: load_bin(&ALWAY_SUCCESS_BIN_PATH),
            data_type: ScriptHashType::Data1,
            tx_hash: Byte32::new([2; 32]),
            tx_index: 0,
            out_point: Option::None,
            type_hash: Option::None,
        },
    );

    deps.insert(
        2,
        CkbDepsData {
            data: load_bin(&ALWAY_FAILED_BIN_PATH),
            data_type: ScriptHashType::Data1,
            tx_hash: Byte32::new([3; 32]),
            tx_index: 0,
            out_point: Option::None,
            type_hash: Option::None,
        },
    );
    deps
}

fn print_mem(d: &[u8]) {
    let mut c = 0;
    for i in 0..d.len() {
        c = i;
        print!("{:#04X}, ", d[i]);
        if i % 16 == 15 {
            print!("\n");
        }
    }
    if c % 16 != 15 {
        print!("\n");
    }
}

fn vec_to_slice<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

fn u32_to_uint32(d: u32) -> ckb_types::packed::Uint32 {
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

fn u64_to_uint64(d: u64) -> ckb_types::packed::Uint64 {
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

fn u128_to_uint128(d: u128) -> ckb_types::packed::Uint128 {
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

pub fn dbg_print_mem(d: &[u8], n: &str) {
    println!("{}, size:{}", n, d.len());
    print_mem(d);
}

#[test]
fn test_multiple() {
    let deps = gen_deps();
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

    let (tx, dummy) = gen_ckb_tx(cells, deps, Vec::new());

    let consensus = gen_consensus();
    let env = gen_tx_env();
    let verifier = TransactionScriptsVerifier::new(&tx, &consensus, &dummy, &env);
    verifier.verify(0xFFFFFFFF).expect("run failed");

    let cmd_line = ckb_debugger_dumper::gen_json(
        &verifier,
        &tx,
        Option::None,
        0,
        DUMP_BIN_PATH.as_str(),
        "test_multi.json",
        Option::None,
    );
    println!("debugger command is: \n{}", cmd_line);
}

#[test]
fn test_single() {
    let deps = gen_deps();
    let mut cells: Vec<CkbCellData> = Vec::new();

    let lock_script1 = CkbScriptData {
        script_id: 0,
        args: Bytes::from([4; 32].to_vec()),
        witness: Bytes::from([5; 100].to_vec()),
    };

    let type_script1 = CkbScriptData {
        script_id: 1,
        args: Bytes::from([6; 64].to_vec()),
        witness: Bytes::from([7; 100].to_vec()),
    };

    cells.push(CkbCellData {
        input_tx_hash: Byte32::new([8; 32]),
        input_data: Bytes::from([9; 123].to_vec()),
        output_data: Bytes::from([10; 123].to_vec()),

        input_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Some(type_script1.clone()),
        },
        output_script: CkbCellScritp {
            lock: lock_script1.clone(),
            type_: Some(type_script1.clone()),
        },
    });

    let mut header_dep: Vec<HeaderView> = Vec::new();
    header_dep.push({
        HeaderBuilder::default()
            .version(u32_to_uint32(1))
            .compact_target(u32_to_uint32(1))
            .timestamp(u64_to_uint64(1231231231))
            .number(u64_to_uint64(0))
            .epoch(u64_to_uint64(0))
            .parent_hash(Byte32::new([0; 32]))
            .transactions_root(Byte32::new([1; 32]))
            .proposals_hash(Byte32::new([2; 32]))
            .extra_hash(Byte32::new([3; 32]))
            .dao(Byte32::new([4; 32]))
            .nonce(u128_to_uint128(123123132123132))
            .build()
    });

    let (tx, dummy) = gen_ckb_tx(cells, deps, header_dep.clone());
    let data = tx.transaction.hash();
    println!("{}", data.to_string());

    let consensus = gen_consensus();
    let env = gen_tx_env();
    let mut verifier = TransactionScriptsVerifier::new(&tx, &consensus, &dummy, &env);
    verifier.set_debug_printer(debug_printer);
    verifier.verify(0xFFFFFFFF).expect("run script failed");

    let header_dep: HashMap<Byte32, HeaderView> =
        header_dep.iter().map(|f| (f.hash(), f.clone())).collect();
    let cmd_line = ckb_debugger_dumper::gen_json(
        &verifier,
        &tx,
        Option::Some(header_dep),
        0,
        DUMP_BIN_PATH.as_str(),
        "test.json",
        Option::Some("127.0.0.1:12300"),
    );

    println!("debugger command is: \n{}", cmd_line);
}
