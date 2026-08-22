/* Honest fiber park/unpark smoke — no io_uring required.
 * f_wait parks; f_wake unparks it with a result. If park/run were broken,
 * this hangs or exits non-zero (no fake pass).
 */

#include "../include/rynix_rt.h"

#include <stdio.h>

static void *g_waiter;
static int g_ok;

static void f_wait(void *arg) {
  (void)arg;
  g_waiter = rynix_rt_fiber_current();
  if (!g_waiter) {
    fprintf(stderr, "no current fiber\n");
    return;
  }
  rynix_rt_fiber_park();
  if (rynix_rt_fiber_get_result(g_waiter) == 42) g_ok = 1;
}

static void f_wake(void *arg) {
  (void)arg;
  for (int i = 0; i < 1024 && !g_waiter; i++) rynix_rt_yield();
  if (!g_waiter) {
    fprintf(stderr, "waiter never appeared\n");
    return;
  }
  rynix_rt_fiber_set_result(g_waiter, 42);
  rynix_rt_fiber_unpark(g_waiter);
}

int main(void) {
  g_waiter = NULL;
  g_ok = 0;
  if (!rynix_rt_spawn(f_wait, NULL)) return 1;
  if (!rynix_rt_spawn(f_wake, NULL)) return 1;
  rynix_rt_run();
  if (!g_ok) {
    fprintf(stderr, "park/unpark failed (result not observed)\n");
    return 2;
  }
  if (rynix_rt_fiber_count() != 0) {
    fprintf(stderr, "fiber leak: %lld\n", (long long)rynix_rt_fiber_count());
    return 3;
  }
  puts("fiber_park_smoke ok");
  return 0;
}
