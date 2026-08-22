#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  const int64_t n = 5000000;
  int64_t a = 0, b = 1;
  for (int64_t i = 0; i < n; i++) {
    int64_t c = a + b;
    a = b;
    b = c;
  }
  suite5_print_i64(a);
  return 0;
}
