/* Keep-alive HTTP: one TCP connection, multiple GETs on the same socket. */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <string.h>

static int g_ok;
static int64_t g_serve = -99;
static int64_t g_got[3] = {-1, -1, -1};

static int64_t read_one_json_value(int64_t fd) {
  char buf[4096];
  int64_t total = 0;
  for (;;) {
    if (total >= (int64_t)sizeof(buf) - 1) {
      return -1;
    }
    int64_t got = rynix_rt_tcp_recv(fd, buf + total, (int64_t)sizeof(buf) - 1 - total);
    if (got <= 0) {
      return -1;
    }
    total += got;
    buf[total] = '\0';
    const char *hdr_end = strstr(buf, "\r\n\r\n");
    if (!hdr_end) {
      continue;
    }
    const char *body = hdr_end + 4;
    /* Prefer Content-Length so keep-alive framing is exact. */
    int64_t cl = -1;
    const char *p = buf;
    while (p < hdr_end) {
      if ((p[0] == 'C' || p[0] == 'c') && strncmp(p + 1, "ontent-Length:", 14) == 0) {
        cl = 0;
        const char *v = p + 15;
        while (*v == ' ') {
          v++;
        }
        while (*v >= '0' && *v <= '9') {
          cl = cl * 10 + (*v - '0');
          v++;
        }
        break;
      }
      const char *nl = strstr(p, "\r\n");
      if (!nl || nl >= hdr_end) {
        break;
      }
      p = nl + 2;
    }
    int64_t header_bytes = (int64_t)(body - buf);
    int64_t need = cl >= 0 ? header_bytes + cl : header_bytes + (int64_t)strlen(body);
    if (total < need) {
      continue;
    }
    /* Leave unread bytes for the next response on the same connection. */
    (void)need;
    return rynix_rt_json_get_i64(body, "value");
  }
}

static void server_fiber(void *arg) {
  (void)arg;
  if (rynix_rt_http_serve_loop_keepalive_json_i64(40134, "/api", 7, 0) != -1) {
    fprintf(stderr, "expected max_reqs<=0 → -1\n");
    return;
  }
  g_serve = rynix_rt_http_serve_loop_keepalive_json_i64(40134, "/api", 7, 3);
}

static void client_fiber(void *arg) {
  (void)arg;
  int64_t fd = -1;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    fd = rynix_rt_tcp_connect("127.0.0.1", 40134);
    if (fd >= 0) {
      break;
    }
  }
  if (fd < 0) {
    return;
  }
  for (int i = 0; i < 3; i++) {
    char req[256];
    int n = snprintf(req, sizeof(req),
                     "GET /api HTTP/1.1\r\n"
                     "Host: 127.0.0.1\r\n"
                     "Connection: keep-alive\r\n"
                     "\r\n");
    if (n <= 0 || n >= (int)sizeof(req) ||
        rynix_rt_tcp_send(fd, req, (int64_t)n) != (int64_t)n) {
      rynix_rt_tcp_close(fd);
      return;
    }
    g_got[i] = read_one_json_value(fd);
  }
  rynix_rt_tcp_close(fd);
  g_ok = (g_serve == 0 && g_got[0] == 7 && g_got[1] == 7 && g_got[2] == 7);
}

int main(void) {
  if (!rynix_rt_spawn(server_fiber, NULL)) {
    return 1;
  }
  if (!rynix_rt_spawn(client_fiber, NULL)) {
    return 1;
  }
  rynix_rt_run();
  if (!g_ok) {
    fprintf(stderr,
            "http_keepalive_bounded failed serve=%lld got=%lld,%lld,%lld\n",
            (long long)g_serve, (long long)g_got[0], (long long)g_got[1],
            (long long)g_got[2]);
    return 1;
  }
  puts("http_keepalive_bounded ok");
  return 0;
}
