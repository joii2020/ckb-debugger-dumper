use ckb_script::{ScriptGroup, ScriptGroupType, TransactionScriptsVerifier};
use ckb_traits::{CellDataProvider, HeaderProvider};
use ckb_types::{
    core::{
        cell::{CellMeta, ResolvedTransaction},
        HeaderView, ScriptHashType,
    },
    packed::{Byte, Byte32, CellOutput, OutPoint, Script, Uint32},
    prelude::{Builder, Entity},
};
use json::{self, JsonValue};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fs::File,
    io::Read,
};

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

fn u32_to_uint32(d: u32) -> Uint32 {
    let b = Uint32::new_builder();
    let d: Vec<Byte> = d
        .to_le_bytes()
        .to_vec()
        .iter()
        .map(|f| f.clone().into())
        .collect();
    let d: [Byte; 4] = vec_to_slice(d);
    let b = b.set(d);
    b.build()
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
    js_cell["dep_type"] = "code".into();
    js_cell
}

fn gen_json_cell_dep_group(cell: &CellMeta) -> JsonValue {
    let mut js_cell = JsonValue::new_object();

    js_cell["out_point"] = gen_json_outpoint(&cell.out_point);
    js_cell["dep_type"] = "dep_group".into();
    js_cell
}

fn gen_json_data(
    resolved_tx: &ResolvedTransaction,
    bin_hash: &Byte32,
    header_deps: &Option<HashMap<Byte32, HeaderView>>,
) -> JsonValue {
    let mut js_root: JsonValue = JsonValue::new_object();
    js_root["mock_info"] = {
        let mut js = JsonValue::new_object();
        js["inputs"] = {
            let mut js_inputs: Vec<JsonValue> = Vec::new();
            let mut index: usize = 0;
            for cell in &resolved_tx.resolved_inputs {
                js_inputs.push({
                    let mut js_cell = JsonValue::new_object();
                    js_cell["input"] = {
                        let mut js = JsonValue::new_object();
                        js["since"] = {
                            let input = resolved_tx.transaction.inputs().get(index).unwrap();
                            let since = input.since();
                            fmt_u64(since.as_slice()).into()
                        };
                        js["previous_output"] = gen_json_outpoint(&cell.out_point);
                        js
                    };
                    js_cell["output"] = gen_json_output(&cell.cell_output);
                    js_cell["data"] =
                        fmt_vec(cell.mem_cell_data.clone().unwrap().to_vec().as_slice()).into();
                    js_cell
                });
                index += 1;
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
            for cell in &resolved_tx.resolved_dep_groups {
                js_celldeps.push({
                    let mut js_cell_dep = JsonValue::new_object();
                    js_cell_dep["cell_dep"] = gen_json_cell_dep_group(cell);
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
        let mut js_header_vec: Vec<JsonValue> = Vec::new();

        for i in 0..resolved_tx.transaction.header_deps().len() {
            js_header_vec.push({
                let hash = resolved_tx.transaction.header_deps().get(i).unwrap();

                let mut js = JsonValue::new_object();
                js["hash"] = fmt_vec(hash.as_slice()).into();

                let headers = header_deps.clone().unwrap();
                let header_data = headers.get(&hash).unwrap();
                js["version"] =
                    fmt_u32(header_data.version().to_le_bytes().to_vec().as_slice()).into();
                js["compact_target"] = fmt_u32(
                    header_data
                        .compact_target()
                        .to_le_bytes()
                        .to_vec()
                        .as_slice(),
                )
                .into();
                js["timestamp"] =
                    fmt_vec(header_data.timestamp().to_le_bytes().to_vec().as_slice()).into();
                js["number"] =
                    fmt_u64(header_data.number().to_le_bytes().to_vec().as_slice()).into();
                js["epoch"] = fmt_u64(&header_data.epoch().index().to_le_bytes()).into();
                js["parent_hash"] = fmt_vec(header_data.parent_hash().as_slice()).into();
                js["transactions_root"] =
                    fmt_vec(header_data.transactions_root().as_slice()).into();
                js["proposals_hash"] = fmt_vec(header_data.proposals_hash().as_slice()).into();
                js["extra_hash"] = fmt_vec(header_data.extra_hash().as_slice()).into();
                js["dao"] = fmt_vec(header_data.dao().as_slice()).into();
                js["nonce"] = fmt_vec(header_data.nonce().to_le_bytes().to_vec().as_slice()).into();

                js
            });
        }
        js["header_deps"] = JsonValue::Array(js_header_vec);

        js
    };
    js_root["tx"] = {
        let mut js_tx = JsonValue::new_object();
        js_tx["version"] = fmt_u32(&resolved_tx.transaction.version().to_le_bytes()).into();

        resolved_tx.transaction.version();

        js_tx["cell_deps"] = {
            let mut cell_deps: Vec<JsonValue> = Vec::new();
            for cell in &resolved_tx.resolved_cell_deps {
                let js_cell = gen_json_cell_dep(cell);
                cell_deps.push(js_cell);
            }
            for cell in &resolved_tx.resolved_dep_groups {
                let js_cell = gen_json_cell_dep_group(cell);
                cell_deps.push(js_cell);
            }
            JsonValue::Array(cell_deps)
        };
        let mut js_header_vec: Vec<JsonValue> = Vec::new();
        for i in 0..resolved_tx.transaction.header_deps().len() {
            js_header_vec.push(
                fmt_vec(
                    resolved_tx
                        .transaction
                        .header_deps()
                        .get(i)
                        .unwrap()
                        .as_slice(),
                )
                .into(),
            );
        }
        js_tx["header_deps"] = JsonValue::Array(js_header_vec);

        js_tx["inputs"] = {
            let mut js_inputs: Vec<JsonValue> = Vec::new();
            let mut index: usize = 0;
            for cell in &resolved_tx.resolved_inputs {
                js_inputs.push({
                    let mut js = JsonValue::new_object();
                    js["since"] = {
                        let input = resolved_tx.transaction.inputs().get(index).unwrap();
                        let since = input.since();
                        fmt_u64(since.as_slice()).into()
                    };
                    js["previous_output"] = gen_json_outpoint(&cell.out_point);
                    js
                });
                index += 1;
            }
            JsonValue::Array(js_inputs)
        };
        js_tx["outputs"] = {
            let mut js_output: Vec<JsonValue> = Vec::new();

            let tx_hash = resolved_tx.transaction.hash();
            let outputs: Vec<CellMeta> = resolved_tx
                .transaction
                .outputs_with_data_iter()
                .enumerate()
                .map(|(output_index, (cell_output, data))| {
                    let out_point = OutPoint::new_builder()
                        .tx_hash(tx_hash.clone())
                        .index(u32_to_uint32(output_index as u32))
                        .build();
                    let data_hash = CellOutput::calc_data_hash(&data);
                    CellMeta {
                        cell_output,
                        out_point,
                        transaction_info: None,
                        data_bytes: data.len() as u64,
                        mem_cell_data: Some(data),
                        mem_cell_data_hash: Some(data_hash),
                    }
                })
                .collect();

            for cell in &outputs {
                js_output.push(gen_json_output(&cell.cell_output));
            }
            JsonValue::Array(js_output)
        };
        js_tx["outputs_data"] = {
            let mut js_data: Vec<JsonValue> = Vec::new();
            let output_datas = &resolved_tx.transaction.outputs_data();
            for i in 0..output_datas.len() {
                let data = output_datas.get(i).unwrap();
                js_data.push(fmt_vec(data.as_slice().to_vec().split_at(4).1).into());
            }
            JsonValue::Array(js_data)
        };
        js_tx["witnesses"] = {
            let mut js_witness: Vec<JsonValue> = Vec::new();
            for data in resolved_tx.transaction.witnesses() {
                let data = data.as_bytes().to_vec().split_at(4).1.to_vec();
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
    header_deps: Option<HashMap<Byte32, HeaderView>>,
    group_index: usize,
    bin_path: &str,
    json_file_name: &str,
    dbg_addr: Option<&str>,
) -> String {
    let bin_path = std::fs::canonicalize(bin_path).expect("cannot get absolute path");

    let bin_hash = get_bin_hash(bin_path.to_str().unwrap());
    let js_root = gen_json_data(resolved_tx, &bin_hash, &header_deps);
    let path = String::from(json_file_name);
    let mut fs = File::create(path).expect("create json file failed");
    js_root.write_pretty(&mut fs, 2).expect("write json failed");

    let groups_info: Vec<(ScriptGroupType, &'_ ScriptGroup)> = verifier
        .groups_with_type()
        .map(|(f1, _f2, f3)| (f1, f3))
        .collect();
    let (group_script_type, script_group) = {
        let (t, s) = groups_info.get(group_index).unwrap();
        (*t, *s)
    };
    // assert_eq!(
    //     script_group.script.code_hash(),
    //     bin_hash,
    //     "group_index is not bin_path"
    // );
    let group_type = {
        match group_script_type {
            ScriptGroupType::Lock => "lock",
            ScriptGroupType::Type => "type",
        }
    };
    let json_file_name = std::fs::canonicalize(json_file_name).expect("cannot get absolute path");
    let (cell_index, cell_type) = {
        if script_group.input_indices.len() > 0 {
            (script_group.input_indices[0], "input")
        } else {
            (script_group.output_indices[0], "output")
        }
    };
    let ckb_dbg_str: String = {
        if dbg_addr.is_none() {
            String::new()
        } else {
            format!(" --mode gdb --gdb-listen {}", dbg_addr.unwrap())
        }
    };

    String::from(format!(
        "ckb-debugger --bin {} --tx-file {} --cell-index {} --script-group-type {} --cell-type {}{}",
        bin_path.to_str().unwrap(),
        json_file_name.to_str().unwrap(),
        cell_index,
        group_type,
        cell_type,
        ckb_dbg_str,
    ))
}
