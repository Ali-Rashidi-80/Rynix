/* IOCP smoke: backend ready + TCP echo via WSARecv/WSASend path. */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <string.h>

#ifndef _WIN32
int main(void) {
  puts("skip: IOCP is Windows-only");
  return 77;
}
#else

static int64_t g_port = 40141;
static int g_ok;

static void server_fiber(void *arg) {
  (void)arg;
  int64_t listen_fd = rynix_rt_tcp_listen(g_port);
  if (listen_fd < 0) {
    return;
  }
  (void)rynix_rt_iocp_associate(listen_fd);
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return;
  }
  (void)rynix_rt_iocp_associate(client);
  char buf[64];
  int64_t n = rynix_rt_tcp_recv(client, buf, (int64_t)sizeof(buf));
  if (n > 0) {
    (void)rynix_rt_tcp_send(client, buf, n);
  }
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
}

static void client_fiber(void *arg) {
  (void)arg;
  const char *msg = "iocp-ping";
  int64_t c = -1;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    c = rynix_rt_tcp_connect("127.0.0.1", g_port);
    if (c >= 0) {
      break;
    }
  }
  if (c < 0) {
    return;
  }
  (void)rynix_rt_iocp_associate(c);
  if (rynix_rt_tcp_send(c, msg, (int64_t)strlen(msg)) < 0) {
    rynix_rt_tcp_close(c);
    return;
  }
  char got[64];
  int64_t n = rynix_rt_tcp_recv(c, got, (int64_t)sizeof(got));
  rynix_rt_tcp_close(c);
  if (n == (int64_t)strlen(msg) && memcmp(got, msg, (size_t)n) == 0) {
    g_ok = 1;
  }
}

int main(void) {
  rynix_rt_iocp_init();
  if (!rynix_rt_iocp_ready()) {
    fprintf(stderr, "iocp not ready\n");
    return 1;
  }
  if (rynix_rt_iocp_ext_ready() != 0) {
    fprintf(stderr, "AcceptEx/ConnectEx not loaded\n");
    return 1;
  }
  if (rynix_rt_iocp_recv(-1, NULL, 0) != -1) {
    fprintf(stderr, "iocp_recv should reject bad args\n");
    return 1;
  }
  if (!rynix_rt_spawn(server_fiber, NULL) || !rynix_rt_spawn(client_fiber, NULL)) {
    return 1;
  }
  rynix_rt_run();
  rynix_rt_iocp_shutdown();
  if (!g_ok) {
    fprintf(stderr, "iocp echo failed\n");
    return 1;
  }
  puts("iocp_echo ok");
  return 0;
}
#endif
