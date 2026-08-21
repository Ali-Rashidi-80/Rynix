/* TCP helpers with non-blocking accept/connect loops (fiber-safe). */

#include "rynix_rt.h"

#include <stdio.h>
#include <string.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <ws2tcpip.h>
static int rynix_net_inited;
static void rynix_net_init(void) {
  if (rynix_net_inited) return;
  WSADATA wsa;
  if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) rynix_rt_panic("WSAStartup failed");
  rynix_net_inited = 1;
}
typedef SOCKET rynix_sock_t;
#define RYNIX_INVALID_SOCK INVALID_SOCKET
static int rynix_sock_close(rynix_sock_t s) { return closesocket(s); }
static void rynix_set_nonblock(rynix_sock_t s) {
  u_long mode = 1;
  ioctlsocket(s, FIONBIO, &mode);
}
static int rynix_would_block(void) {
  int e = WSAGetLastError();
  return e == WSAEWOULDBLOCK || e == WSAEINPROGRESS;
}
#else
#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
static void rynix_net_init(void) {}
typedef int rynix_sock_t;
#define RYNIX_INVALID_SOCK (-1)
static int rynix_sock_close(rynix_sock_t s) { return close(s); }
static void rynix_set_nonblock(rynix_sock_t s) {
  int fl = fcntl(s, F_GETFL, 0);
  fcntl(s, F_SETFL, fl | O_NONBLOCK);
}
static int rynix_would_block(void) {
  return errno == EAGAIN || errno == EWOULDBLOCK || errno == EINPROGRESS;
}
#endif

int64_t rynix_rt_tcp_listen(int64_t port) {
  rynix_net_init();
  rynix_sock_t s = socket(AF_INET, SOCK_STREAM, 0);
  if (s == RYNIX_INVALID_SOCK) return -1;
  int yes = 1;
#ifdef _WIN32
  setsockopt(s, SOL_SOCKET, SO_REUSEADDR, (const char *)&yes, sizeof(yes));
#else
  setsockopt(s, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));
#endif
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  addr.sin_port = htons((uint16_t)port);
  if (bind(s, (struct sockaddr *)&addr, sizeof(addr)) != 0) {
    rynix_sock_close(s);
    return -1;
  }
  if (listen(s, 128) != 0) {
    rynix_sock_close(s);
    return -1;
  }
  rynix_set_nonblock(s);
  return (int64_t)(intptr_t)s;
}

int64_t rynix_rt_tcp_accept(int64_t listen_fd) {
  rynix_sock_t s = (rynix_sock_t)(intptr_t)listen_fd;
  for (;;) {
    rynix_rt_yield();
    rynix_sock_t c = accept(s, NULL, NULL);
    if (c != RYNIX_INVALID_SOCK) {
      rynix_set_nonblock(c);
      return (int64_t)(intptr_t)c;
    }
    if (!rynix_would_block()) return -1;
  }
}

int64_t rynix_rt_tcp_connect(const char *host, int64_t port) {
  rynix_net_init();
  rynix_sock_t s = socket(AF_INET, SOCK_STREAM, 0);
  if (s == RYNIX_INVALID_SOCK) return -1;
  rynix_set_nonblock(s);
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_port = htons((uint16_t)port);
  if (!host) host = "127.0.0.1";
#ifdef _WIN32
  addr.sin_addr.s_addr = inet_addr(host);
  if (addr.sin_addr.s_addr == INADDR_NONE) {
    rynix_sock_close(s);
    return -1;
  }
#else
  if (inet_pton(AF_INET, host, &addr.sin_addr) != 1) {
    rynix_sock_close(s);
    return -1;
  }
#endif
  for (;;) {
    rynix_rt_yield();
    int rc = connect(s, (struct sockaddr *)&addr, sizeof(addr));
    if (rc == 0) return (int64_t)(intptr_t)s;
#ifdef _WIN32
    int e = WSAGetLastError();
    if (e == WSAEISCONN) return (int64_t)(intptr_t)s;
    if (e != WSAEWOULDBLOCK && e != WSAEINPROGRESS && e != WSAEALREADY) {
      rynix_sock_close(s);
      return -1;
    }
#else
    if (errno == EISCONN) return (int64_t)(intptr_t)s;
    if (!rynix_would_block() && errno != EALREADY) {
      rynix_sock_close(s);
      return -1;
    }
#endif
  }
}

void rynix_rt_tcp_close(int64_t fd) {
  if (fd < 0) return;
  rynix_sock_close((rynix_sock_t)(intptr_t)fd);
}

int64_t rynix_rt_tcp_recv(int64_t fd, void *buf, int64_t n) {
  if (!buf || n <= 0) return 0;
  rynix_sock_t s = (rynix_sock_t)(intptr_t)fd;
  for (;;) {
    rynix_rt_yield();
#ifdef _WIN32
    int r = recv(s, (char *)buf, (int)n, 0);
    if (r >= 0) return r;
    if (!rynix_would_block()) return -1;
#else
    ssize_t r = recv(s, buf, (size_t)n, 0);
    if (r >= 0) return (int64_t)r;
    if (!rynix_would_block()) return -1;
#endif
  }
}

int64_t rynix_rt_tcp_send(int64_t fd, const void *buf, int64_t n) {
  if (!buf || n <= 0) return 0;
  rynix_sock_t s = (rynix_sock_t)(intptr_t)fd;
  for (;;) {
    rynix_rt_yield();
#ifdef _WIN32
    int r = send(s, (const char *)buf, (int)n, 0);
    if (r >= 0) return r;
    if (!rynix_would_block()) return -1;
#else
    ssize_t r = send(s, buf, (size_t)n, 0);
    if (r >= 0) return (int64_t)r;
    if (!rynix_would_block()) return -1;
#endif
  }
}
