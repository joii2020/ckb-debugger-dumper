# Directions
* Used to export json from the test case of the contract for debugging of the [ckb-debugger](https://github.com/nervosnetwork/ckb-standalone-debugger)

```rust
pub fn gen_json<'a, DL: CellDataProvider + HeaderProvider>(
    verifier: &TransactionScriptsVerifier<'a, DL>,
    resolved_tx: &ResolvedTransaction,
    group_index: usize,
    bin_path: &str,
    json_file_name: &str,
) -> String {...}
```
By calling this function, you can generate transaction data for ckb-debugger

### verifier
[CKB VM to verify transaction inputs](https://docs.rs/ckb-script/0.100.0-rc2/ckb_script/struct.TransactionScriptsVerifier.html).
Get grouping information through this function.

### resolved_tx
[This Library provides the essential types for CKB](https://docs.rs/ckb-types/0.100.0-rc2/ckb_types/index.html).
Get cell information, including inputs, outputs and dependencies.
* If the dep cell is ```bin_path``` data, the data filled with ```0x```.

### group_index
Transaction group index, the index of ```verifier.groups()```, if it out of bounds, an exception will be thrown. and if the group script is not bin_path data, also throw an exception.

### bin_path
Contract path to be executed

### json_file_name
File for exporting transaction data

### return value
According to the ckb-debugger command generated from these data, some of the file paths will be replaced with absolute paths.

## For example

```rust
pub fn dumper(&self, bin_path: &str, dumper_name: &str) -> String {
    let consensus = TX::gen_consensus();
    let tx_env = TX::gen_tx_env();
    let verifier = TransactionScriptsVerifier::new(
        &self.resolved_tx,
        &consensus,
        &self.data_loader,
        &tx_env,
    );
    ckb_debugger_dumper::gen_json(
        &verifier,
        &self.resolved_tx,
        Option::None,
        0,
        bin_path,
        dumper_name,
        Option::None,
    )
}
```
This code is [here](https://github.com/joii2020/ckb-production-scripts/blob/compact_udt_lock_debugger/tests/compact_udt_rust/src/lib.rs#L1144), 
you can also refer to [this commit](https://github.com/nervosnetwork/ckb-production-scripts/pull/42/files).
