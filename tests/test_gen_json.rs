use ckb_script::{TransactionScriptsVerifier, TxVerifyEnv};
use ckb_types::{
    bytes::Bytes,
    core::{HeaderBuilder, HeaderView, ScriptHashType},
    packed::Byte32,
    prelude::*,
};
use lazy_static::lazy_static;
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

mod misc;
use misc::*;

lazy_static! {
    pub static ref DUMP_BIN_PATH: String = String::from("build/dump");
    pub static ref ALWAY_SUCCESS_BIN_PATH: String = String::from("build/always_success");
    pub static ref ALWAY_FAILED_BIN_PATH: String = String::from("build/always_failed");
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

    // print!("{}", msg);
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
    let verifier = TransactionScriptsVerifier::new(
        Arc::new(tx.clone()),
        dummy.clone(),
        Arc::new(consensus),
        Arc::new(TxVerifyEnv::new_commit(
            &HeaderView::new_advanced_builder()
                .epoch(ckb_types::core::EpochNumberWithFraction::new(5, 0, 1).pack())
                .build(),
        )),
    );
    // verifier.set_debug_printer(debug_printer);
    verifier.verify(0xFFFFFFFF).expect("run failed");

    ckb_debugger_dumper::dump_tx_to_file(&tx, Vec::new(), "test_multi.json", Some("./build/"))
        .expect("msg");
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
            .version(1u32.pack())
            .compact_target(1u32.pack())
            .timestamp(1231231231.pack())
            .number(0.pack())
            .epoch(0.pack())
            .parent_hash(Byte32::new([0; 32]))
            .transactions_root(Byte32::new([1; 32]))
            .proposals_hash(Byte32::new([2; 32]))
            .extra_hash(Byte32::new([3; 32]))
            .dao(Byte32::new([4; 32]))
            .nonce(123123132123132.pack())
            .build()
    });

    let (tx, dummy) = gen_ckb_tx(cells, deps, header_dep.clone());

    let consensus = gen_consensus();
    let verifier = TransactionScriptsVerifier::new(
        Arc::new(tx.clone()),
        dummy.clone(),
        Arc::new(consensus),
        Arc::new(TxVerifyEnv::new_commit(
            &HeaderView::new_advanced_builder()
                .epoch(ckb_types::core::EpochNumberWithFraction::new(5, 0, 1).pack())
                .build(),
        )),
    );

    verifier.verify(0xFFFFFFFF).expect("run script failed");

    ckb_debugger_dumper::dump_tx_to_file(&tx, header_dep, "test_sign.json", Some("./build/"))
        .expect("");
}
