#include <stdint.h>
#include <stdio.h>
#include "bench.h"

static int popcount64(int64_t x) {
  int c = 0;
  while (x) {
    c += (int)(x & 1);
    x >>= 1;
  }
  return c;
}

int main(void) {
  const int64_t n = suite5_opaque_i64(25000000);
  int64_t x = 1;
  int64_t acc = 0;
  for (int64_t i = 0; i < n; i++) {
    x = (x * 31 + i) % 1000000007;
    acc += popcount64(x);
  }
  suite5_print_i64(acc);
  return 0;
}
