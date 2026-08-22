/* Smoke for io_uring helpers — works with or without RYNIX_RT_URING. */

#include "../include/rynix_rt.h"

#include <stdio.h>

int main(void) {
  rynix_rt_uring_init();
  rynix_rt_uring_poll();
  if (rynix_rt_uring_accept(-1) != -1) {
    fprintf(stderr, "uring_accept(-1) should fail\n");
    return 1;
  }
  if (rynix_rt_uring_connect(-1, NULL, 0) != -1) {
    fprintf(stderr, "uring_connect(-1) should fail\n");
    return 1;
  }
  if (rynix_rt_uring_read(-1, NULL, 0) != -1) {
    fprintf(stderr, "uring_read(-1) should fail\n");
    return 1;
  }
  if (rynix_rt_uring_write(-1, NULL, 0) != -1) {
    fprintf(stderr, "uring_write(-1) should fail\n");
    return 1;
  }
  rynix_rt_uring_shutdown();
  puts("uring_sqe_smoke ok");
  return 0;
}
