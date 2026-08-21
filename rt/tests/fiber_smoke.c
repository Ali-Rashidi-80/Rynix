/* Fiber smoke test — compile with:
 *   clang -O2 -I rt/include rt/portable.c rt/tests/fiber_smoke.c -o fiber_smoke
 */

#include "rynix_rt.h"

#include <stdio.h>
#include <stdlib.h>

static int g_hits;

static void worker(void *arg) {
  int id = (int)(intptr_t)arg;
  for (int i = 0; i < 3; i++) {
    g_hits++;
    printf("fiber %d step %d\n", id, i);
    rynix_rt_yield();
  }
}

int main(void) {
  rynix_rt_spawn(worker, (void *)(intptr_t)1);
  rynix_rt_spawn(worker, (void *)(intptr_t)2);
  rynix_rt_run();
  if (g_hits != 6) {
    fprintf(stderr, "expected 6 hits, got %d\n", g_hits);
    return 1;
  }
  if (rynix_rt_fiber_count() != 0) {
    fprintf(stderr, "fiber leak: count=%lld\n", (long long)rynix_rt_fiber_count());
    return 1;
  }
  puts("fiber_smoke ok");
  return 0;
}
