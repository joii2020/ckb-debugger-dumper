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