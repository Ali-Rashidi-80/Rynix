#include <stdint.h>
#include <stdio.h>
#include "bench.h"

int main(void) {
  int64_t a[4][4];
  int64_t b[4][4];
  int64_t c[4][4];
  for (int i = 0; i < 4; i++) {
    for (int j = 0; j < 4; j++) {
      a[i][j] = (int64_t)(i + j);
      b[i][j] = (int64_t)(i * j + 1);
      c[i][j] = 0;
    }
  }
  const int64_t reps = 900000;
  int64_t trace = 0;
  for (int64_t r = 0; r < reps; r++) {
    for (int i = 0; i < 4; i++) {
      for (int j = 0; j < 4; j++) {
        int64_t s = 0;
        for (int k = 0; k < 4; k++) {
          s += a[i][k] * b[k][j];
        }
        c[i][j] = s;
      }
    }
    trace += c[r & 3][r & 3];
  }
  suite5_print_i64(trace);
  return 0;
}
