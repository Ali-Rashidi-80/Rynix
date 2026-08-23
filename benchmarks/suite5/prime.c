#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  const int64_t limit = suite5_opaque_i64(100000);
  int64_t count = 0;
  for (int64_t i = 2; i <= limit; i++) {
    int64_t prime = 1;
    for (int64_t j = 2; j * j <= i; j++) {
      if (i % j == 0) {
        prime = 0;
        break;
      }
    }
    if (prime) {
      count++;
    }
  }
  suite5_print_i64(count);
  return 0;
}
