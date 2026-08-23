/* Suite5 timing sink: skip stdout when SUITE5_BENCH=1 (harness sets this). */
#ifndef SUITE5_BENCH_H
#define SUITE5_BENCH_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static inline void suite5_print_i64(int64_t n) {
  if (getenv("SUITE5_BENCH")) {
    static volatile int64_t suite5_sink;
    suite5_sink = n;
    return;
  }
  printf("%lld\n", (long long)n);
}

/* Keep trip counts out of the optimizer so Suite5 times real loop work. */
static inline int64_t suite5_opaque_i64(int64_t x) {
  volatile int64_t v = x;
  return v;
}

#endif
