#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  const int64_t n = 3000000;
  int64_t h = 0;
  for (int64_t i = 0; i < n; i++) {
    h = (h * 31 + i) % 1000000007;
  }
  suite5_print_i64(h);
  return 0;
}
