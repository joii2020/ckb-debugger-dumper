

#define DBG_PRINT_LEN
#define DBG_PRINT_HASH
//#define DBG_PRINT_DATA

char _bin_to_char(uint8_t b) {
  if (b >= 0 && b <= 9)
    return '0' + b;
  if (b >= 0xA && b <= 0xF)
    return 'A' + (b - 0xA);

  return '\0';
}

void _hash_to_str(const uint8_t* buf, char* out_buf) {
  for (int i = 0; i < 32; i++) {
    uint8_t l_c = buf[i] & 0x0F;
    uint8_t h_c = buf[i] >> 4;
    out_buf[0] = _bin_to_char(h_c);
    out_buf[1] = _bin_to_char(l_c);
    out_buf = &out_buf[2];
  }
}

void _bin_to_hash_str(const uint8_t* buf, uint64_t len, char* out_buf) {
  blake2b_state b2 = {0};
  blake2b_init(&b2, 32);
  blake2b_update(&b2, buf, len);
  uint8_t h[32] = {0};
  blake2b_final(&b2, h, 32);

  _hash_to_str(h, out_buf);
}

void _print_data_hash(const uint8_t* buf, uint64_t len) {
  char hash[32 * 3] = {0};
  _bin_to_hash_str(buf, len, hash);
  printf("size is: %d\n%s\n", len, hash);
}

void _print_data(const uint8_t* buf, uint64_t len) {
  printf("size is: %d\n", len);
  int i = 0;
  for (i = 0; i < len; i++) {
    printf("%02X ", buf[i]);
    if (i % 32 == 31)
      printf("\n");
  }
  if (i % 32 != 31)
    printf("\n");
}

void print_data(const uint8_t* buf, uint64_t len, const char* des) {
  printf("%s--", des);
#if defined(DBG_PRINT_DATA)
  _print_data(buf, len);
#elif defined(DBG_PRINT_HASH)
  _print_data_hash(buf, len);
#elif defined(DBG_PRINT_LEN)
  printf("size is: %d\n", len);
#endif  //
}

void print_byte32(const uint8_t* data) {
  char hash[32 * 3] = {0};
  _hash_to_str(data, hash);
  printf("size is: %d\n%s\n", 32, hash);
}
