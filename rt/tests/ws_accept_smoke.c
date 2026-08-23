/* RFC 6455 Sec-WebSocket-Accept known answer. */

#include "../include/rynix_rt.h"

#include <stdio.h>

int main(void) {
  /* From RFC 6455 §1.3 */
  const char *key = "dGhlIHNhbXBsZSBub25jZQ==";
  const char *want = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
  if (rynix_rt_ws_accept_key_eq(key, want) != 0) {
    fprintf(stderr, "ws accept key mismatch\n");
    return 1;
  }
  if (rynix_rt_ws_accept_sha1_first_i64(key) == 0) {
    fprintf(stderr, "ws sha1 first unexpectedly 0\n");
    return 1;
  }
  puts("ws_accept ok");
  return 0;
}
