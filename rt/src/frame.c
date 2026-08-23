#include <stdint.h>
#include <string.h>

#include "../include/rynix_rt.h"

static int64_t frame_send_raw(int64_t fd, const void *data, int64_t n) {
  if (fd < 0 || !data || n < 0 || n > 0xffff) {
    return -1;
  }
  uint8_t hdr[4];
  uint32_t len = (uint32_t)n;
  hdr[0] = (uint8_t)((len >> 24) & 0xff);
  hdr[1] = (uint8_t)((len >> 16) & 0xff);
  hdr[2] = (uint8_t)((len >> 8) & 0xff);
  hdr[3] = (uint8_t)(len & 0xff);
  if (rynix_rt_tcp_send(fd, hdr, 4) != 4) {
    return -1;
  }
  if (n == 0) {
    return 0;
  }
  if (rynix_rt_tcp_send(fd, data, n) != n) {
    return -1;
  }
  return n;
}

static int64_t frame_recv_raw(int64_t fd, char *buf, int64_t cap) {
  if (fd < 0 || !buf || cap <= 0) {
    return -1;
  }
  uint8_t hdr[4];
  int64_t got = 0;
  while (got < 4) {
    int64_t r = rynix_rt_tcp_recv(fd, hdr + got, 4 - got);
    if (r <= 0) {
      return -1;
    }
    got += r;
  }
  int64_t len = ((int64_t)hdr[0] << 24) | ((int64_t)hdr[1] << 16) | ((int64_t)hdr[2] << 8) |
                (int64_t)hdr[3];
  if (len < 0 || len > cap) {
    return -1;
  }
  got = 0;
  while (got < len) {
    int64_t r = rynix_rt_tcp_recv(fd, buf + got, len - got);
    if (r <= 0) {
      return -1;
    }
    got += r;
  }
  return len;
}

int64_t rynix_rt_frame_send(int64_t fd, const char *data, int64_t n) {
  return frame_send_raw(fd, data, n);
}

int64_t rynix_rt_frame_recv(int64_t fd, void *buf, int64_t cap) {
  return frame_recv_raw(fd, (char *)buf, cap);
}

/* One-shot framed echo server (binary framing — C3 / EndForge-class minimal). */
int64_t rynix_rt_frame_serve_once_echo(int64_t port) {
  if (port <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  char buf[1024];
  int64_t n = frame_recv_raw(client, buf, (int64_t)sizeof(buf));
  if (n < 0) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  if (frame_send_raw(client, buf, n) != n) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  return 0;
}

/* Client: send `msg` as one frame, expect identical echo; returns 0 on match. */
int64_t rynix_rt_frame_client_echo(const char *host, int64_t port, const char *msg) {
  if (!host || !msg || port <= 0) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    return -1;
  }
  int64_t n = (int64_t)strlen(msg);
  if (frame_send_raw(fd, msg, n) != n) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  char buf[1024];
  int64_t got = frame_recv_raw(fd, buf, (int64_t)sizeof(buf));
  rynix_rt_tcp_close(fd);
  if (got != n) {
    return -1;
  }
  if (memcmp(buf, msg, (size_t)n) != 0) {
    return -1;
  }
  return 0;
}
