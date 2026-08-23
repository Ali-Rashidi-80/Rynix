/* RFC 6455: frame round-trip + HTTP upgrade echo (fibers). */

#include "../include/rynix_rt.h"

#include <stdio.h>

static int g_ok;
static int64_t g_client = -99;
static int64_t g_serve = -99;

static void server_fiber(void *arg) {
  (void)arg;
  g_serve = rynix_rt_ws_serve_once_echo(40131);
}

static void client_fiber(void *arg) {
  (void)arg;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    g_client = rynix_rt_ws_client_echo("127.0.0.1", 40131, "rynix-ws");
    if (g_client == 0) {
      break;
    }
  }
  g_ok = (g_client == 0 && g_serve == 0);
}

int main(void) {
  if (rynix_rt_ws_frame_roundtrip_ok() != 0) {
    fprintf(stderr, "ws frame roundtrip failed\n");
    return 1;
  }
  /* RFC sample accept key still required. */
  if (rynix_rt_ws_accept_key_eq("dGhlIHNhbXBsZSBub25jZQ==",
                                "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=") != 0) {
    fprintf(stderr, "ws accept KAT failed\n");
    return 1;
  }
  if (!rynix_rt_spawn(server_fiber, NULL) || !rynix_rt_spawn(client_fiber, NULL)) {
    return 1;
  }
  rynix_rt_run();
  if (!g_ok) {
    fprintf(stderr, "ws echo failed client=%lld serve=%lld\n", (long long)g_client,
            (long long)g_serve);
    return 1;
  }
  puts("ws_frames ok");
  return 0;
}
