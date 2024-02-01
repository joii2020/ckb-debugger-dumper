use ckb_mock_tx_types::{MockCellDep, MockInfo, MockInput, MockTransaction, ReprMockTransaction};
use ckb_types::{
    core::{cell::ResolvedTransaction, DepType, HeaderView},
    packed::CellDepBuilder,
    prelude::*,
};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    EncodeJson(String),
    StdIO(String),
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::EncodeJson(format!("Encode to json failed: {:?}", value))
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::StdIO(format!("Std IO Error: {:?}", value))
    }
}

fn cmp_data_bin(meta: &std::fs::Metadata, data: &str) -> bool {
    let file_size = meta.len() as usize;
    let data_len = data.len() / 2 - 1;
    data_len >= file_size && data_len <= (file_size as f64 * 1.1) as usize
}

fn replace_bin_cell(dir: &PathBuf, tx: &mut Value) -> Result<(), Error> {
    let items = std::fs::read_dir(dir)?;

    for item in items {
        if item.is_err() {
            continue;
        }

        let item = item.unwrap();
        if item.file_type()?.is_file() {
            let celldeps = tx
                .get_mut("mock_info")
                .unwrap()
                .get_mut("cell_deps")
                .unwrap();

            for dep in celldeps.as_array_mut().unwrap() {
                let dep_data = dep.get_mut("data");
                if dep_data.is_none() {
                    continue;
                }
                if !cmp_data_bin(
                    &item.metadata()?,
                    dep_data.as_ref().unwrap().as_str().unwrap(),
                ) {
                    continue;
                }

                let dd = dep_data.unwrap();
                *dd = Value::String(format!(
                    "0x{{{{ data {} }}}}",
                    item.path().to_str().unwrap()
                ));

                // replace to bin path
            }
        } else if item.file_type()?.is_dir() {
        }
    }

    Ok(())
}

pub fn dump_tx_to_file(
    rtx: &ResolvedTransaction,
    header_deps: Vec<HeaderView>,
    output_file: &str,
    cell_bin_dir: Option<&str>,
) -> Result<(), Error> {
    let tx = dump_tx(&rtx, header_deps)?;
    let mut tx = serde_json::to_value(&tx)?;

    if cell_bin_dir.is_some() {
        //
        replace_bin_cell(&PathBuf::from(cell_bin_dir.as_ref().unwrap()), &mut tx)?;
    }

    // get
    std::fs::write(output_file, tx.to_string())?;

    Ok(())
}

pub fn dump_tx(
    rtx: &ResolvedTransaction,
    header_deps: Vec<HeaderView>,
) -> Result<ReprMockTransaction, Error> {
    let mut inputs = Vec::with_capacity(rtx.resolved_inputs.len());
    // We are doing it this way so we can keep original since value is available
    for (i, input) in rtx.resolved_inputs.iter().enumerate() {
        inputs.push(MockInput {
            input: rtx.transaction.inputs().get(i).unwrap(),
            output: input.cell_output.clone(),
            data: input.mem_cell_data.clone().unwrap(),
            header: input.transaction_info.clone().map(|info| info.block_hash),
        });
    }
    // MockTransaction keeps both types of cell deps in a single array, the order does
    // not really matter for now
    let mut cell_deps =
        Vec::with_capacity(rtx.resolved_cell_deps.len() + rtx.resolved_dep_groups.len());
    for dep in rtx.resolved_cell_deps.iter() {
        cell_deps.push(MockCellDep {
            cell_dep: CellDepBuilder::default()
                .out_point(dep.out_point.clone())
                .dep_type(DepType::Code.into())
                .build(),
            output: dep.cell_output.clone(),
            data: dep.mem_cell_data.clone().unwrap(),
            header: None,
        });
    }
    for dep in rtx.resolved_dep_groups.iter() {
        cell_deps.push(MockCellDep {
            cell_dep: CellDepBuilder::default()
                .out_point(dep.out_point.clone())
                .dep_type(DepType::DepGroup.into())
                .build(),
            output: dep.cell_output.clone(),
            data: dep.mem_cell_data.clone().unwrap(),
            header: None,
        });
    }
    let mut tx_header_deps = Vec::with_capacity(rtx.transaction.header_deps().len());
    // let mut extensions = Vec::new();

    for header_hash in rtx.transaction.header_deps_iter() {
        tx_header_deps.push(
            header_deps[header_deps
                .iter()
                .position(|it| it.hash() == header_hash)
                .unwrap()]
            .clone(),
        );

        // if let Some(extension) = self.get_block_extension(&header_hash) {
        //     extensions.push((header_hash, extension.unpack()));
        // }
    }
    Ok(MockTransaction {
        mock_info: MockInfo {
            inputs,
            cell_deps,
            header_deps: tx_header_deps,
            extensions: Vec::new(),
        },
        tx: rtx.transaction.data(),
    }
    .into())
}
