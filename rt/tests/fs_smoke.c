/* Smoke: write + read_eq round-trip. */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#define TEST_PATH "rynix_fs_smoke.tmp"
#else
#define TEST_PATH "/tmp/rynix_fs_smoke.tmp"
#endif

int main(void) {
  const char *msg = "rynix-fs-ok";
  if (rynix_rt_fs_write_file(TEST_PATH, msg) != 0) {
    fprintf(stderr, "fs_write_file failed\n");
    return 1;
  }
  if (rynix_rt_fs_read_file_eq(TEST_PATH, msg) != 0) {
    fprintf(stderr, "fs_read_file_eq mismatch\n");
    return 1;
  }
  {
    char *got = rynix_rt_fs_read_file(TEST_PATH);
    if (!got || strcmp(got, msg) != 0) {
      fprintf(stderr, "fs_read_file failed\n");
      free(got);
      return 1;
    }
    free(got);
  }
  remove(TEST_PATH);
  puts("fs_smoke ok");
  return 0;
}
