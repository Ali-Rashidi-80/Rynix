/* TLS echo smoke (fibers) — real SChannel/OpenSSL, not a simulated handshake. */

#include "../include/rynix_rt.h"

#include <stdio.h>

static int g_ok;
static int64_t g_client = -99;
static int64_t g_serve = -99;

static void server_fiber(void *arg) {
  (void)arg;
  g_serve = rynix_rt_tls_serve_once_echo(40129);
}

static void client_fiber(void *arg) {
  (void)arg;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    g_client = rynix_rt_tls_client_echo("127.0.0.1", 40129, "rynix-tls");
    if (g_client == 0 || g_client == -2) {
      break;
    }
  }
  if (g_client == -2 || g_serve == -2) {
    g_ok = 2; /* unsupported platform */
    return;
  }
  g_ok = (g_client == 0 && g_serve == 0);
}

int main(void) {
  if (!rynix_rt_spawn(server_fiber, NULL)) {
    return 1;
  }
  if (!rynix_rt_spawn(client_fiber, NULL)) {
    return 1;
  }
  rynix_rt_run();
  if (g_ok == 2) {
    puts("tls_echo skip (no TLS backend)");
    return 77;
  }
  if (!g_ok) {
    fprintf(stderr, "tls_echo smoke failed client=%lld serve=%lld\n",
            (long long)g_client, (long long)g_serve);
    return 1;
  }
  puts("tls_echo ok");
  return 0;
}
