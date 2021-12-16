use ckb_script::TransactionScriptsVerifier;
use ckb_types::{
    bytes::Bytes,
    core::{HeaderBuilder, HeaderView, ScriptHashType},
    packed::Byte32,
};
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Mutex};

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

pub fn dbg_print_mem(d: &[u8], n: &str) {
    println!("{}, size:{}", n, d.len());
    print_mem(d);
}

lazy_static! {
    pub static ref CKB_VM_OUTPUT_DATA: Mutex<HashMap<Byte32, String>> = Mutex::new(HashMap::new());
}

pub fn debug_printer(script: &Byte32, msg: &str) {
    let mut output_data = CKB_VM_OUTPUT_DATA.lock().unwrap();
    let it = output_data.get_mut(script);
    if it.is_none() {
        output_data.insert(script.clone(), String::from(msg));
    } else {
        let it = it.unwrap();
        it.push_str(msg);
    }

    //print!("{}", msg);
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
    let mut verifier = TransactionScriptsVerifier::new(&tx, &consensus, &dummy, &env);
    verifier.set_debug_printer(debug_printer);
    verifier.verify(0xFFFFFFFF).expect("run failed");

    for group_index in 0..3 {
        let cmd_line = ckb_debugger_dumper::gen_json(
            &verifier,
            &tx,
            Option::None,
            group_index,
            DUMP_BIN_PATH.as_str(),
            "test_multi.json",
            Option::None,
        );
        let ckb_dbg_output = run_ckb_debugger(cmd_line.as_str()).unwrap();

        let groups: Vec<Byte32> = verifier
            .groups_with_type()
            .map(|(_f1, f2, _f3)| f2.clone())
            .collect();
        let script_id = groups.get(group_index).unwrap();
        let ckb_output = {
            let output_data = CKB_VM_OUTPUT_DATA.lock().unwrap();
            let data = output_data.get(script_id).unwrap().clone();

            let i = data.rfind("----").unwrap();
            String::from(data.split_at(i + 4).0)
        };
        assert_eq!(ckb_dbg_output, ckb_output);
    }
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

    let consensus = gen_consensus();
    let env = gen_tx_env();
    let mut verifier = TransactionScriptsVerifier::new(&tx, &consensus, &dummy, &env);

    verifier.set_debug_printer(debug_printer);
    verifier.verify(0xFFFFFFFF).expect("run script failed");

    let header_dep: HashMap<Byte32, HeaderView> =
        header_dep.iter().map(|f| (f.hash(), f.clone())).collect();
    let group_index = 0;
    let cmd_line = ckb_debugger_dumper::gen_json(
        &verifier,
        &tx,
        Option::Some(header_dep),
        group_index,
        DUMP_BIN_PATH.as_str(),
        "test.json",
        Option::None,
    );

    let ckb_dbg_output = run_ckb_debugger(cmd_line.as_str()).unwrap();

    let groups: Vec<Byte32> = verifier
        .groups_with_type()
        .map(|(_f1, f2, _f3)| f2.clone())
        .collect();
    let script_id = groups.get(group_index).unwrap();
    let ckb_output = {
        let output_data = CKB_VM_OUTPUT_DATA.lock().unwrap();
        let data = output_data.get(script_id).unwrap().clone();

        let i = data.rfind("----").unwrap();
        String::from(data.split_at(i + 4).0)
    };
    assert_eq!(ckb_dbg_output, ckb_output);
}
