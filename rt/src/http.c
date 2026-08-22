#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../include/rynix_rt.h"

/* HTTP/1.1 GET with JSON body field extraction (soft std/http surface). */
int64_t rynix_rt_http_get_json_i64(const char *host, int64_t port, const char *path,
                                   const char *field) {
  if (!host || !path || !field) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    return -1;
  }
  char req[512];
  int n = snprintf(req, sizeof(req),
                   "GET %s HTTP/1.1\r\nHost: %s\r\nConnection: close\r\n\r\n", path,
                   host);
  if (n <= 0 || n >= (int)sizeof(req)) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  if (rynix_rt_tcp_send(fd, req, (int64_t)n) != (int64_t)n) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  char buf[4096];
  int64_t total = 0;
  for (;;) {
    int64_t got = rynix_rt_tcp_recv(fd, buf + total, (int64_t)sizeof(buf) - 1 - total);
    if (got <= 0) {
      break;
    }
    total += got;
    if (total >= (int64_t)sizeof(buf) - 1) {
      break;
    }
  }
  rynix_rt_tcp_close(fd);
  buf[total] = '\0';
  const char *body = strstr(buf, "\r\n\r\n");
  if (!body) {
    return -1;
  }
  body += 4;
  return rynix_rt_json_get_i64(body, field);
}
