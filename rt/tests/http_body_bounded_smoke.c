/* Bounded POST echo: body over max_body → 400; else echo field. */

#include "../include/rynix_rt.h"

#include <stdio.h>
#include <string.h>

static int g_ok;
static int64_t g_serve = -99;
static int64_t g_got_a = -1;
static int g_rejected;
static int64_t g_got_b = -1;

static int post_raw_status_has(const char *body, const char *needle) {
  int64_t fd = rynix_rt_tcp_connect("127.0.0.1", 40133);
  if (fd < 0) {
    return -1;
  }
  int body_len = (int)strlen(body);
  char req[2048];
  int n = snprintf(req, sizeof(req),
                   "POST /echo HTTP/1.1\r\n"
                   "Host: 127.0.0.1\r\n"
                   "Content-Type: application/json\r\n"
                   "Content-Length: %d\r\n"
                   "Connection: close\r\n"
                   "\r\n"
                   "%s",
                   body_len, body);
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
  return strstr(buf, needle) != NULL ? 1 : 0;
}

static void server_fiber(void *arg) {
  (void)arg;
  if (rynix_rt_http_serve_loop_post_echo_json_i64(40133, "/echo", "n", 0, 32) != -1) {
    fprintf(stderr, "expected max_reqs<=0 → -1\n");
    return;
  }
  /* max_body=16 rejects oversized JSON; two successful echoes end the loop. */
  g_serve = rynix_rt_http_serve_loop_post_echo_json_i64(40133, "/echo", "n", 2, 16);
}

static void client_fiber(void *arg) {
  (void)arg;
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    g_got_a = rynix_rt_http_post_json_i64("127.0.0.1", 40133, "/echo", "{\"n\": 7}", "value");
    if (g_got_a >= 0) {
      break;
    }
  }
  /* Body longer than max_body=16 → 400 body_too_large (does not count). */
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    int rc = post_raw_status_has("{\"n\": 1, \"pad\": \"xxxxxxxxxxxxxxxx\"}",
                                 "body_too_large");
    if (rc == 1) {
      g_rejected = 1;
      break;
    }
  }
  for (int attempt = 0; attempt < 256; attempt++) {
    rynix_rt_yield();
    g_got_b = rynix_rt_http_post_json_i64("127.0.0.1", 40133, "/echo", "{\"n\": 99}", "value");
    if (g_got_b >= 0) {
      break;
    }
  }
  g_ok = (g_serve == 0 && g_got_a == 7 && g_rejected && g_got_b == 99);
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
            "http_body_bounded failed serve=%lld a=%lld rejected=%d b=%lld\n",
            (long long)g_serve, (long long)g_got_a, g_rejected, (long long)g_got_b);
    return 1;
  }
  puts("http_body_bounded ok");
  return 0;
}
