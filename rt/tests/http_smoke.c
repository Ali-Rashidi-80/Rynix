/* HTTP soft builtin smoke — connect failure must return -1 (no fake server). */
#include "../include/rynix_rt.h"

#include <stdio.h>

int main(void) {
  int64_t r = rynix_rt_http_get_json_i64("127.0.0.1", 1, "/", "value");
  if (r != -1) {
    fprintf(stderr, "expected -1 on failed connect, got %lld\n", (long long)r);
    return 1;
  }
  puts("http_smoke ok");
  return 0;
}
