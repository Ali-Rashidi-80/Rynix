#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  const int64_t n = suite5_opaque_i64(10000000);
  int64_t acc = 0;
  for (int64_t i = 0; i < n; i++) {
    acc = acc + i * 31 - i / 8 + i % 13;
  }
  suite5_print_i64(acc);
  return 0;
}
