/* Bearer HTTP serve-loop: GET /b with Authorization: Bearer secret → {"value": 1}. */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <string.h>

static int g_ok;
static int64_t g_serve = -99;
static int64_t g_got = -1;

static int64_t get_with_bearer(const char *path, const char *token) {
  int64_t fd = rynix_rt_tcp_connect("127.0.0.1", 40133);
  if (fd < 0) {
    return -1;
  }
  char req[512];
  int n = snprintf(req, sizeof(req),
                   "GET %s HTTP/1.1\r\n"
                   "Host: 127.0.0.1\r\n"
                   "Authorization: Bearer %s\r\n"
                   "Connection: close\r\n"
                   "\r\n",
                   path, token);
  if (n <= 0 || n >= (int)sizeof(req) ||
      rynix_rt_tcp_send(fd, req, (int64_t)n) != (int64_t)n) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  char buf[4096];
  int64_t total = 0;
  for (;;) {
    if (total >= (int64_t)sizeof(buf) - 1) {
      break;
    }
    int64_t got = rynix_rt_tcp_recv(fd, buf + total, (int64_t)sizeof(buf) - 1 - total);
    if (got <= 0) {
      break;
    }
    total += got;
    buf[total] = '\0';
    if (strstr(buf, "\r\n\r\n") != NULL) {
      break;
    }
  }
  rynix_rt_tcp_close(fd);
  if (total <= 0) {
    return -1;
  }
  const char *body = strstr(buf, "\r\n\r\n");
  if (!body) {
    return -1;
  }
  body += 4;
  return rynix_rt_json_get_i64(body, "value");
}

static void server_fiber(void *arg) {
  (void)arg;
  if (rynix_rt_http_serve_loop_bearer_json_i64(40133, "/b", "secret", 0) != -1) {
    fprintf(stderr, "expected max_reqs<=0 → -1\n");
    return;
  }
  g_serve = rynix_rt_http_serve_loop_bearer_json_i64(40133, "/b", "secret", 1);
}

static void client_fiber(void *arg) {
  (void)arg;
  int64_t got = -1;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    got = get_with_bearer("/b", "secret");
    if (got >= 0) {
      break;
    }
  }
  g_got = got;
}

int main(void) {
  if (!rynix_rt_spawn(server_fiber, NULL)) {
    return 1;
  }
  if (!rynix_rt_spawn(client_fiber, NULL)) {
    return 1;
  }
  rynix_rt_run();
  g_ok = (g_serve == 0 && g_got == 1);
  if (!g_ok) {
    fprintf(stderr, "bearer smoke fail serve=%lld got=%lld\n", (long long)g_serve,
            (long long)g_got);
    return 1;
  }
  puts("http_bearer ok");
  return 0;
}
