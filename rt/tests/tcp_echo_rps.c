/* TCP echo + RPS smoke (non-blocking sockets + fibers). */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <string.h>

static int64_t g_port;
static const char *g_msg;
static int g_ok;
static char g_got[256];
static int64_t g_got_n;

static void server_fiber(void *arg) {
  (void)arg;
  int64_t listen_fd = rynix_rt_tcp_listen(g_port);
  if (listen_fd < 0) return;
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return;
  }
  char buf[256];
  int64_t n = rynix_rt_tcp_recv(client, buf, (int64_t)sizeof(buf));
  if (n > 0) (void)rynix_rt_tcp_send(client, buf, n);
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
}

static void client_fiber(void *arg) {
  (void)arg;
  int64_t c = -1;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    c = rynix_rt_tcp_connect("127.0.0.1", g_port);
    if (c >= 0) break;
  }
  if (c < 0) return;
  if (rynix_rt_tcp_send(c, g_msg, (int64_t)strlen(g_msg)) < 0) {
    rynix_rt_tcp_close(c);
    return;
  }
  g_got_n = rynix_rt_tcp_recv(c, g_got, (int64_t)sizeof(g_got) - 1);
  if (g_got_n > 0) g_got[g_got_n] = 0;
  rynix_rt_tcp_close(c);
  if (g_got_n == (int64_t)strlen(g_msg) && memcmp(g_got, g_msg, (size_t)g_got_n) == 0) {
    g_ok = 1;
  }
}

static int one_echo(int64_t port, const char *msg) {
  g_port = port;
  g_msg = msg;
  g_ok = 0;
  g_got_n = 0;
  if (!rynix_rt_spawn(server_fiber, NULL)) return 1;
  if (!rynix_rt_spawn(client_fiber, NULL)) return 1;
  rynix_rt_run();
  return g_ok ? 0 : 1;
}

int main(void) {
  if (one_echo(39876, "ping") != 0) {
    fprintf(stderr, "single echo failed\n");
    return 1;
  }

  const int iters = 16;
  int64_t t0 = rynix_rt_now_ms();
  for (int i = 0; i < iters; i++) {
    char msg[32];
    snprintf(msg, sizeof(msg), "e%d", i);
    if (one_echo(39900 + (i % 8), msg) != 0) {
      fprintf(stderr, "echo %d failed\n", i);
      return 1;
    }
  }
  int64_t t1 = rynix_rt_now_ms();
  double sec = (t1 - t0) / 1000.0;
  if (sec < 0.001) sec = 0.001;
  double rps = iters / sec;
  printf("tcp_echo_rps ok  iters=%d  rps=%.1f\n", iters, rps);
  if (rps < 1.0) {
    fprintf(stderr, "RPS too low: %.1f\n", rps);
    return 1;
  }
  return 0;
}
