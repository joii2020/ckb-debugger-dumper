#define CKB_C_STDLIB_PRINTF

#ifndef MOL2_EXIT
#define MOL2_EXIT(err) \
  { ckb_exit(err); }
#endif

#include "blake2b.h"
#include "ckb_consts.h"
#include "ckb_syscalls.h"

#include "blockchain-api2.h"
#include "blockchain.h"

#include "output.h"

#define TMP_BUFFER_SIZE 1024 * 500
uint8_t g_mol_data_source[DEFAULT_DATA_SOURCE_LENGTH];

void dump_cur_cell_tx_hash() {
  unsigned char tx_hash[32] = {0};
  uint64_t len = 32;
  int err = ckb_load_tx_hash(tx_hash, &len, 0);
  if (err != 0 || len == 0) {
    printf("load tx hash filed, err:%d, len:%d", err, len);
    return;
  }
  printf("%s--", "CurCell TxHash");

  print_byte32(tx_hash);
  printf("\n");
}

void dump_cur_cell_script_hash() {
  unsigned char tx_hash[32] = {0};
  uint64_t len = 32;
  int err = ckb_load_script_hash(tx_hash, &len, 0);
  if (err != 0 || len == 0) {
    printf("load cell script hash filed, err:%d, len:%d", err, len);
    return;
  }
  printf("%s--", "CurCell ScriptHash");

  print_byte32(tx_hash);
  printf("\n");
}

void dump_cur_cell_script_data() {
  unsigned char temp[TMP_BUFFER_SIZE] = {0};
  uint64_t len = TMP_BUFFER_SIZE;
  int err = ckb_load_script(temp, &len, 0);
  if (err != 0 || len == 0) {
    printf("load cell script filed, err:%d, len:%d", err, len);
    return;
  }
  print_data(temp, len, "CurCell Script");
  printf("\n");
}

static uint32_t _read_transaction_data(uintptr_t arg[],
                                       uint8_t* ptr,
                                       uint32_t len,
                                       uint32_t offset) {
  int err;
  uint64_t output_len = len;
  err = ckb_load_transaction(ptr, &output_len, offset);
  if (err != 0) {
    return 0;
  }
  if (output_len > len) {
    return len;
  } else {
    return (uint32_t)output_len;
  }
}
static int _make_cursor(size_t len, mol2_cursor_t* cur) {
  int err = 0;

  cur->offset = 0;
  cur->size = len;

  memset(g_mol_data_source, 0, sizeof(g_mol_data_source));
  mol2_data_source_t* ptr = (mol2_data_source_t*)g_mol_data_source;

  ptr->read = _read_transaction_data;
  ptr->total_size = len;

  ptr->cache_size = 0;
  ptr->start_point = 0;
  ptr->max_cache_size = MAX_CACHE_SIZE;
  cur->data_source = ptr;

  return err;
}

void dump_transaction() {
  unsigned char temp[TMP_BUFFER_SIZE] = {0};
  uint64_t len = TMP_BUFFER_SIZE;
  int err = ckb_load_transaction(temp, &len, 0);
  if (err != 0 || len == 0) {
    printf("ckb_load_transaction filed, err:%d, len:%d", err, len);
    return;
  }
  print_data(temp, len, "Transaction");
  printf("\n");

  mol2_cursor_t cur;
  err = _make_cursor(len, &cur);

  TransactionType mol_transaction = make_Transaction(&cur);

  RawTransactionType mol_raw = mol_transaction.t->raw(&mol_transaction);

  uint32_t ver = RawTransaction_get_version_impl(&mol_raw);
  printf("transaction version is\n: %d", ver);

}

bool load_cell_data(size_t index, size_t source, const char* des) {
  uint8_t buf[TMP_BUFFER_SIZE] = {0};
  uint64_t buf_len = sizeof(buf);
  int rc_code = ckb_load_cell_data(buf, &buf_len, 0, index, source);
  if (rc_code == CKB_INDEX_OUT_OF_BOUND) {
    return false;
  }
  if (rc_code != 0) {
    printf("ckb_load_cell_data return %d, %s\n", rc_code, des);
    return false;
  }
  print_data(buf, buf_len, des);

  printf("\n");
  return true;
}

bool load_witness_data(size_t index, size_t source, const char* des) {
  uint8_t buf[TMP_BUFFER_SIZE] = {0};
  uint64_t buf_len = sizeof(buf);
  int rc_code = ckb_load_witness(buf, &buf_len, 0, index, source);
  if (rc_code == CKB_INDEX_OUT_OF_BOUND) {
    return false;
  }
  if (rc_code != 0) {
    printf("ckb_load_witness return %d, %s\n", rc_code, des);
    return false;
  }
  print_data(buf, buf_len, des);

  printf("\n");
  return true;
}

void dump_all_cell_info() {
  for (size_t i = 0; i < 10000; i++) {
    if (!load_cell_data(i, CKB_SOURCE_INPUT, "Input cell data"))
      break;
    if (!load_cell_data(i, CKB_SOURCE_OUTPUT, "Output cell data"))
      break;
    if (!load_witness_data(i, CKB_SOURCE_INPUT, "Input witness data"))
      break;
    if (!load_witness_data(i, CKB_SOURCE_OUTPUT, "Output witness data"))
      break;
  }
}

void dump_group_cell_info() {
  for (size_t i = 0; i < 10000; i++) {
    if (!load_cell_data(i, CKB_SOURCE_GROUP_INPUT, "InputGroup cell data"))
      break;
    if (!load_witness_data(i, CKB_SOURCE_GROUP_INPUT,
                           "InputGroup witness data"))
      break;
  }
}

void dump_deps_data() {
  for (size_t i = 0; i < 10000; i++) {
    if (!load_cell_data(i, CKB_SOURCE_CELL_DEP, "Deps cell data"))
      break;
  }
}

int main(int argc, char* argv[]) {
  printf("\n----------------------begin----------------------\n");
  dump_cur_cell_tx_hash();
  dump_cur_cell_script_hash();
  dump_cur_cell_script_data();
  dump_transaction();

  dump_all_cell_info();
  dump_group_cell_info();
  
  // Because the current bin file depends on being passed in as a file, this item is empty
  // dump_deps_data();

  printf("\n-----------------------end-----------------------\n");
  return 0;
}
