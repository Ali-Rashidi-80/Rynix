#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  int64_t acc = 1;
  const int64_t base = 3;
  const int64_t n = 2500000;
  for (int64_t i = 0; i < n; i++) {
    acc = (acc * base) % 1000000007;
  }
  suite5_print_i64(acc);
  return 0;
}
