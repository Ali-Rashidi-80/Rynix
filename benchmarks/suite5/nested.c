#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  const int64_t n = 450;
  int64_t s = 0;
  for (int64_t i = 0; i < n; i++) {
    for (int64_t j = 0; j < n; j++) {
      s = s + (i * j + i) % 97;
    }
  }
  suite5_print_i64(s);
  return 0;
}
