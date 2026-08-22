#include <stdint.h>
#include <stdio.h>
#include "bench.h"

static int64_t gcd64(int64_t a, int64_t b) {
  while (b != 0) {
    int64_t t = a % b;
    a = b;
    b = t;
  }
  return a;
}

int main(void) {
  const int64_t n = 2500000;
  int64_t acc = 0;
  for (int64_t i = 1; i <= n; i++) {
    int64_t a = i * 9973;
    int64_t b = i * 1237 + 42;
    acc += gcd64(a, b);
  }
  suite5_print_i64(acc);
  return 0;
}
