#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  const int64_t n = 8000000;
  int64_t acc = 0;
  for (int64_t i = 0; i < n; i++) {
    if (i % 3 == 0 || i % 7 == 0) {
      acc++;
    }
  }
  suite5_print_i64(acc);
  return 0;
}
