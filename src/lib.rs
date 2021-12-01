use ckb_script::{ScriptGroup, ScriptGroupType, TransactionScriptsVerifier};
use ckb_traits::{CellDataProvider, HeaderProvider};
use ckb_types::{
    core::{
        cell::{CellMeta, ResolvedTransaction},
        ScriptHashType,
    },
    packed::{Byte32, CellOutput, OutPoint, Script},
    prelude::Entity,
};
use json::{self, JsonValue};
use std::{convert::{TryFrom, TryInto}, fs::File, io::Read};

fn vec_to_slice<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

fn bytes_to_u32(d: &[u8]) -> u32 {
    u32::from_le_bytes(vec_to_slice(d.to_vec()))
}

fn fmt_u32(d: &[u8]) -> String {
    assert!(d.len() == 4);
    format!("0x{:x}", bytes_to_u32(d))
}

fn bytes_to_u64(d: &[u8]) -> u64 {
    u64::from_le_bytes(vec_to_slice(d.to_vec()))
}

fn fmt_u64(d: &[u8]) -> String {
    assert!(d.len() == 8);
    format!("0x{:x}", bytes_to_u64(d))
}

fn fmt_vec(d: &[u8]) -> String {
    let mut s = String::from("0x");
    for i in 0..d.len() {
        s.push_str(&String::from(format!("{:02x}", d[i])));
    }
    s
}

fn gen_json_script(sc: &Script) -> JsonValue {
    let mut json = JsonValue::new_object();
    json["code_hash"] = fmt_vec(sc.code_hash().as_slice()).into();
    json["hash_type"] = {
        let t: u8 = sc.hash_type().into();
        let t = ScriptHashType::try_from(t).clone().unwrap();
        match t {
            ScriptHashType::Data => "data".into(),
            ScriptHashType::Type => "type".into(),
            ScriptHashType::Data1 => "data1".into(),
        }
    };
    json["args"] = fmt_vec(sc.args().raw_data().to_vec().as_slice()).into();

    json
}

fn gen_json_outpoint(sc: &OutPoint) -> JsonValue {
    let mut js = JsonValue::new_object();
    js["index"] = fmt_u32(&sc.index().as_slice()).into();
    js["tx_hash"] = fmt_vec(sc.tx_hash().as_slice()).into();
    js
}

fn gen_json_output(d: &CellOutput) -> JsonValue {
    let mut js = JsonValue::new_object();
    js["capacity"] = fmt_u64(&d.capacity().as_slice()).into();
    js["lock"] = gen_json_script(&d.lock());
    if d.type_().is_some() {
        js["type"] = gen_json_script(&d.type_().to_opt().unwrap())
    }
    js
}

fn gen_json_cell_dep(cell: &CellMeta) -> JsonValue {
    let mut js_cell = JsonValue::new_object();

    js_cell["out_point"] = gen_json_outpoint(&cell.out_point);
    js_cell["dep_type"] = "code".into(); // TODO
    js_cell
}

fn get_ckb_vm_version<'a, DL: CellDataProvider + HeaderProvider>(
    verifier: &TransactionScriptsVerifier<'a, DL>,
    group_index: usize,
) -> String {
    let cur_cell: Vec<&'_ ScriptGroup> = verifier
        .groups()
        .map(|(_type, _data, script)| script)
        .collect();
    let cur_cell = cur_cell.get(group_index).unwrap();

    let ver = verifier.select_version(&cur_cell.script).unwrap();

    format!("0x{:x}", ver as u32)
}

fn gen_json_data<'a, DL: CellDataProvider + HeaderProvider>(
    verifier: &TransactionScriptsVerifier<'a, DL>,
    resolved_tx: &ResolvedTransaction,
    group_index: usize,
    bin_hash: &Byte32,
) -> JsonValue {
    let mut js_root: JsonValue = JsonValue::new_object();
    js_root["mock_info"] = {
        let mut js = JsonValue::new_object();
        js["inputs"] = {
            let mut js_inputs: Vec<JsonValue> = Vec::new();
            for cell in &resolved_tx.resolved_inputs {
                js_inputs.push({
                    let mut js_cell = JsonValue::new_object();
                    js_cell["input"] = {
                        let mut js = JsonValue::new_object();
                        js["since"] = "0x0".into();
                        js["previous_output"] = gen_json_outpoint(&cell.out_point);
                        js
                    };
                    js_cell["output"] = gen_json_output(&cell.cell_output);
                    js_cell["data"] =
                        fmt_vec(cell.mem_cell_data.clone().unwrap().to_vec().as_slice()).into();
                    js_cell
                });
            }
            JsonValue::Array(js_inputs)
        };
        js["cell_deps"] = {
            let mut js_celldeps: Vec<JsonValue> = Vec::new();
            for cell in &resolved_tx.resolved_cell_deps {
                js_celldeps.push({
                    let mut js_cell_dep = JsonValue::new_object();
                    js_cell_dep["cell_dep"] = gen_json_cell_dep(cell);
                    js_cell_dep["output"] = gen_json_output(&cell.cell_output);
                    if *bin_hash == cell.mem_cell_data_hash.clone().unwrap() {
                        js_cell_dep["data"] = "0x".into();
                    } else {
                        js_cell_dep["data"] =
                            fmt_vec(cell.mem_cell_data.clone().unwrap().to_vec().as_slice()).into();
                    }
                    js_cell_dep
                });
            }
            JsonValue::Array(js_celldeps)
        };
        js["header_deps"] = JsonValue::new_array();
        js
    };
    js_root["tx"] = {
        let mut js_tx = JsonValue::new_object();

        js_tx["version"] = get_ckb_vm_version(verifier, group_index).into();
        js_tx["cell_deps"] = {
            let mut cell_deps: Vec<JsonValue> = Vec::new();
            for cell in &resolved_tx.resolved_cell_deps {
                let js_cell = gen_json_cell_dep(cell);
                cell_deps.push(js_cell);
            }
            JsonValue::Array(cell_deps)
        };
        js_tx["header_deps"] = JsonValue::new_array();
        js_tx["inputs"] = {
            let mut js_inputs: Vec<JsonValue> = Vec::new();
            for cell in &resolved_tx.resolved_inputs {
                js_inputs.push({
                    let mut js = JsonValue::new_object();
                    js["since"] = "0x0".into();
                    js["previous_output"] = gen_json_outpoint(&cell.out_point);
                    js
                });
            }
            JsonValue::Array(js_inputs)
        };
        js_tx["outputs"] = {
            let mut js_output: Vec<JsonValue> = Vec::new();
            for cell in &resolved_tx.resolved_inputs {
                js_output.push(gen_json_output(&cell.cell_output));
            }
            JsonValue::Array(js_output)
        };
        js_tx["outputs_data"] = {
            let mut js_data: Vec<JsonValue> = Vec::new();
            for cell in &resolved_tx.resolved_inputs {
                js_data
                    .push(fmt_vec(cell.mem_cell_data.clone().unwrap().to_vec().as_slice()).into());
            }
            JsonValue::Array(js_data)
        };
        js_tx["witnesses"] = {
            let mut js_witness: Vec<JsonValue> = Vec::new();
            for data in resolved_tx.transaction.witnesses() {
                js_witness.push(fmt_vec(data.as_slice()).into());
            }
            JsonValue::Array(js_witness)
        };

        js_tx
    };

    js_root
}

fn get_bin_hash(path: &str) -> Byte32 {
    let mut file = File::open(path).expect("open bin file failed");

    let mut file_buf: Vec<u8> = Vec::new();
    let size = file
        .read_to_end(&mut file_buf)
        .expect("read bin file failed");
    assert!(size != 0, "");

    CellOutput::calc_data_hash(file_buf.as_slice())
}

pub fn gen_json<'a, DL: CellDataProvider + HeaderProvider>(
    verifier: &TransactionScriptsVerifier<'a, DL>,
    resolved_tx: &ResolvedTransaction,
    group_index: usize,
    bin_path: &str,
    json_file_name: &str,
) -> String {
    let bin_path = std::fs::canonicalize(bin_path).expect("cannot get absolute path");

    let bin_hash = get_bin_hash(bin_path.to_str().unwrap());
    let js_root = gen_json_data(verifier, resolved_tx, group_index, &bin_hash);
    let path = String::from(json_file_name);
    let mut fs = File::create(path).expect("create json file failed");
    js_root.write_pretty(&mut fs, 2).expect("write json failed");

    let groups_info: Vec<(ScriptGroupType, &'_ ScriptGroup)> =
        verifier.groups().map(|(f1, _f2, f3)| (f1, f3)).collect();
    let (group_script_type, script_group) = {
        let (t, s) = groups_info.get(group_index).unwrap();
        (*t, *s)
    };
    assert_eq!(
        script_group.script.code_hash(),
        bin_hash,
        "group_index is not bin_path"
    );
    let group_type = {
        match group_script_type {
            ScriptGroupType::Lock => {
                "lock"
            }
            ScriptGroupType::Type => {
                "type"
            }
        }
    };
    let json_file_name = std::fs::canonicalize(json_file_name).expect("cannot get absolute path");
    let script_hash: String = {
        let i = script_group.input_indices[group_index];
        let cell = &resolved_tx.resolved_inputs[i];
        let script = match group_script_type {
            ScriptGroupType::Lock => cell.cell_output.lock(),
            ScriptGroupType::Type => cell.cell_output.type_().to_opt().unwrap(),
        };
        fmt_vec(script.calc_script_hash().as_slice())
    };
    String::from(format!(
        "ckb-debugger --bin {} --tx-file {} --script-group-type {} --script-hash {}",
        bin_path.to_str().unwrap(),
        json_file_name.to_str().unwrap(),
        group_type,
        script_hash
    ))
}
