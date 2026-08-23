/* Windows IOCP backend: WSARecv/WSASend + AcceptEx/ConnectEx + fiber park.
 *
 * Enabled with -DRYNIX_RT_IOCP on _WIN32. Real completion-port I/O — not a
 * fake ✅. Accept/connect use mswsock extensions when available.
 */

#include "rynix_rt.h"

#if defined(RYNIX_RT_IOCP) && defined(_WIN32)

#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <ws2tcpip.h>
#include <mswsock.h>
#include <windows.h>

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SO_UPDATE_CONNECT_CONTEXT
#define SO_UPDATE_CONNECT_CONTEXT 0x7010
#endif

enum {
  IOP_BYTES = 0,
  IOP_ACCEPT = 1,
  IOP_CONNECT = 2
};

typedef struct {
  OVERLAPPED ov; /* must be first */
  void *fiber;
  int kind;
  SOCKET accept_sock;
  char *addr_buf;
} RynixIocpOp;

static HANDLE g_iocp;
static int g_ready;
static int g_wsa_inited;
static LPFN_ACCEPTEX g_acceptex;
static LPFN_CONNECTEX g_connectex;
static int g_ext_loaded;

static void iocp_net_init(void) {
  if (g_wsa_inited) {
    return;
  }
  WSADATA wsa;
  if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) {
    return;
  }
  g_wsa_inited = 1;
}

static int load_extensions(void) {
  SOCKET s;
  GUID guid_accept = WSAID_ACCEPTEX;
  GUID guid_connect = WSAID_CONNECTEX;
  DWORD bytes = 0;
  if (g_ext_loaded) {
    return g_acceptex != NULL && g_connectex != NULL;
  }
  g_ext_loaded = 1;
  s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
  if (s == INVALID_SOCKET) {
    return 0;
  }
  if (WSAIoctl(s, SIO_GET_EXTENSION_FUNCTION_POINTER, &guid_accept, sizeof(guid_accept),
               &g_acceptex, sizeof(g_acceptex), &bytes, NULL, NULL) != 0) {
    g_acceptex = NULL;
  }
  bytes = 0;
  if (WSAIoctl(s, SIO_GET_EXTENSION_FUNCTION_POINTER, &guid_connect, sizeof(guid_connect),
               &g_connectex, sizeof(g_connectex), &bytes, NULL, NULL) != 0) {
    g_connectex = NULL;
  }
  closesocket(s);
  return g_acceptex != NULL && g_connectex != NULL;
}

void rynix_rt_iocp_init(void) {
  if (g_ready) {
    return;
  }
  iocp_net_init();
  if (!g_wsa_inited) {
    return;
  }
  g_iocp = CreateIoCompletionPort(INVALID_HANDLE_VALUE, NULL, 0, 1);
  if (!g_iocp) {
    return;
  }
  (void)load_extensions();
  g_ready = 1;
}

void rynix_rt_iocp_shutdown(void) {
  if (!g_ready) {
    return;
  }
  CloseHandle(g_iocp);
  g_iocp = NULL;
  g_ready = 0;
  g_acceptex = NULL;
  g_connectex = NULL;
  g_ext_loaded = 0;
}

int rynix_rt_iocp_ready(void) { return g_ready; }

int64_t rynix_rt_iocp_ext_ready(void) {
  return (g_ready && g_acceptex && g_connectex) ? 0 : -1;
}

int64_t rynix_rt_iocp_associate(int64_t fd) {
  if (!g_ready || fd < 0) {
    return -1;
  }
  SOCKET s = (SOCKET)(intptr_t)fd;
  if (!CreateIoCompletionPort((HANDLE)s, g_iocp, (ULONG_PTR)s, 0)) {
    /* Already associated is fine. */
  }
  return 0;
}

static void harvest_one(RynixIocpOp *op, DWORD bytes, BOOL ok) {
  void *fiber = op->fiber;
  int64_t res = -1;
  if (ok) {
    if (op->kind == IOP_ACCEPT) {
      SOCKET *listen_ptr = (SOCKET *)(op->addr_buf + 2 * (sizeof(SOCKADDR_STORAGE) + 16));
      SOCKET listen_sock = *listen_ptr;
      if (setsockopt(op->accept_sock, SOL_SOCKET, SO_UPDATE_ACCEPT_CONTEXT,
                     (char *)&listen_sock, sizeof(listen_sock)) != 0) {
        closesocket(op->accept_sock);
        res = -1;
      } else {
        u_long mode = 1;
        ioctlsocket(op->accept_sock, FIONBIO, &mode);
        (void)rynix_rt_iocp_associate((int64_t)(intptr_t)op->accept_sock);
        res = (int64_t)(intptr_t)op->accept_sock;
        op->accept_sock = INVALID_SOCKET;
      }
    } else if (op->kind == IOP_CONNECT) {
      SOCKET s = (SOCKET)(ULONG_PTR)op->accept_sock; /* reuse field as connect sock */
      (void)setsockopt(s, SOL_SOCKET, SO_UPDATE_CONNECT_CONTEXT, NULL, 0);
      res = 0;
    } else {
      res = (int64_t)bytes;
    }
  } else {
    if (op->kind == IOP_ACCEPT && op->accept_sock != INVALID_SOCKET) {
      closesocket(op->accept_sock);
    }
    res = -1;
  }
  free(op->addr_buf);
  free(op);
  if (fiber) {
    rynix_rt_fiber_set_result(fiber, res);
    rynix_rt_fiber_unpark(fiber);
  }
}

static int drain_completions(DWORD timeout_ms) {
  if (!g_ready) {
    return 0;
  }
  int got = 0;
  for (;;) {
    DWORD bytes = 0;
    ULONG_PTR key = 0;
    OVERLAPPED *ov = NULL;
    BOOL ok = GetQueuedCompletionStatus(g_iocp, &bytes, &key, &ov, timeout_ms);
    if (!ov) {
      break;
    }
    harvest_one((RynixIocpOp *)ov, bytes, ok);
    got = 1;
    timeout_ms = 0;
  }
  return got;
}

void rynix_rt_iocp_poll(void) { (void)drain_completions(0); }

void rynix_rt_iocp_wait(void) { (void)drain_completions(INFINITE); }

static int64_t wait_op(RynixIocpOp *op) {
  void *self = op->fiber;
  if (!self) {
    for (;;) {
      DWORD bytes = 0;
      ULONG_PTR key = 0;
      OVERLAPPED *ov = NULL;
      BOOL ok = GetQueuedCompletionStatus(g_iocp, &bytes, &key, &ov, INFINITE);
      if (!ov) {
        free(op->addr_buf);
        if (op->kind == IOP_ACCEPT && op->accept_sock != INVALID_SOCKET) {
          closesocket(op->accept_sock);
        }
        free(op);
        return -1;
      }
      if (ov == &op->ov) {
        /* Inline harvest without fiber. */
        int64_t res = -1;
        if (ok) {
          if (op->kind == IOP_ACCEPT) {
            SOCKET *listen_ptr =
                (SOCKET *)(op->addr_buf + 2 * (sizeof(SOCKADDR_STORAGE) + 16));
            SOCKET listen_sock = *listen_ptr;
            if (setsockopt(op->accept_sock, SOL_SOCKET, SO_UPDATE_ACCEPT_CONTEXT,
                           (char *)&listen_sock, sizeof(listen_sock)) == 0) {
              u_long mode = 1;
              ioctlsocket(op->accept_sock, FIONBIO, &mode);
              (void)rynix_rt_iocp_associate((int64_t)(intptr_t)op->accept_sock);
              res = (int64_t)(intptr_t)op->accept_sock;
              op->accept_sock = INVALID_SOCKET;
            } else {
              closesocket(op->accept_sock);
            }
          } else if (op->kind == IOP_CONNECT) {
            SOCKET s = (SOCKET)(ULONG_PTR)op->accept_sock;
            (void)setsockopt(s, SOL_SOCKET, SO_UPDATE_CONNECT_CONTEXT, NULL, 0);
            res = 0;
          } else {
            res = (int64_t)bytes;
          }
        } else if (op->kind == IOP_ACCEPT && op->accept_sock != INVALID_SOCKET) {
          closesocket(op->accept_sock);
        }
        free(op->addr_buf);
        free(op);
        return res;
      }
      harvest_one((RynixIocpOp *)ov, bytes, ok);
    }
  }
  rynix_rt_fiber_park();
  return rynix_rt_fiber_get_result(self);
}

static int64_t submit_wsa(SOCKET s, WSABUF *buf, int recv) {
  RynixIocpOp *op = (RynixIocpOp *)calloc(1, sizeof(RynixIocpOp));
  if (!op) {
    return -1;
  }
  op->fiber = rynix_rt_fiber_current();
  op->kind = IOP_BYTES;
  op->accept_sock = INVALID_SOCKET;
  DWORD flags = 0;
  DWORD transferred = 0;
  int rc = recv ? WSARecv(s, buf, 1, &transferred, &flags, &op->ov, NULL)
                : WSASend(s, buf, 1, &transferred, 0, &op->ov, NULL);
  if (rc == 0 || WSAGetLastError() == WSA_IO_PENDING) {
    return wait_op(op);
  }
  free(op);
  return -1;
}

int64_t rynix_rt_iocp_recv(int64_t fd, void *buf, int64_t n) {
  if (!g_ready || !buf || n <= 0 || fd < 0) {
    return -1;
  }
  (void)rynix_rt_iocp_associate(fd);
  WSABUF wsabuf;
  wsabuf.buf = (char *)buf;
  wsabuf.len = (ULONG)n;
  return submit_wsa((SOCKET)(intptr_t)fd, &wsabuf, 1);
}

int64_t rynix_rt_iocp_send(int64_t fd, const void *buf, int64_t n) {
  if (!g_ready || !buf || n <= 0 || fd < 0) {
    return -1;
  }
  (void)rynix_rt_iocp_associate(fd);
  WSABUF wsabuf;
  wsabuf.buf = (char *)(uintptr_t)buf;
  wsabuf.len = (ULONG)n;
  return submit_wsa((SOCKET)(intptr_t)fd, &wsabuf, 0);
}

int64_t rynix_rt_iocp_accept(int64_t listen_fd) {
  const size_t addr_bytes = 2 * (sizeof(SOCKADDR_STORAGE) + 16);
  RynixIocpOp *op;
  SOCKET listen_sock;
  SOCKET accept_sock;
  DWORD bytes = 0;
  if (!g_ready || !g_acceptex || listen_fd < 0) {
    return -1;
  }
  listen_sock = (SOCKET)(intptr_t)listen_fd;
  (void)rynix_rt_iocp_associate(listen_fd);
  accept_sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
  if (accept_sock == INVALID_SOCKET) {
    return -1;
  }
  op = (RynixIocpOp *)calloc(1, sizeof(RynixIocpOp));
  if (!op) {
    closesocket(accept_sock);
    return -1;
  }
  op->fiber = rynix_rt_fiber_current();
  op->kind = IOP_ACCEPT;
  op->accept_sock = accept_sock;
  /* addr buffer + trailing listen SOCKET for UpdateAcceptContext */
  op->addr_buf = (char *)calloc(1, addr_bytes + sizeof(SOCKET));
  if (!op->addr_buf) {
    closesocket(accept_sock);
    free(op);
    return -1;
  }
  *(SOCKET *)(op->addr_buf + addr_bytes) = listen_sock;
  if (!g_acceptex(listen_sock, accept_sock, op->addr_buf, 0,
                  (DWORD)(sizeof(SOCKADDR_STORAGE) + 16),
                  (DWORD)(sizeof(SOCKADDR_STORAGE) + 16), &bytes, &op->ov)) {
    int err = WSAGetLastError();
    if (err != WSA_IO_PENDING && err != ERROR_IO_PENDING) {
      closesocket(accept_sock);
      free(op->addr_buf);
      free(op);
      return -1;
    }
  }
  return wait_op(op);
}

int64_t rynix_rt_iocp_connect(int64_t fd, const void *addr, int64_t addrlen) {
  RynixIocpOp *op;
  SOCKET s;
  DWORD bytes = 0;
  if (!g_ready || !g_connectex || fd < 0 || !addr || addrlen <= 0) {
    return -1;
  }
  s = (SOCKET)(intptr_t)fd;
  (void)rynix_rt_iocp_associate(fd);
  op = (RynixIocpOp *)calloc(1, sizeof(RynixIocpOp));
  if (!op) {
    return -1;
  }
  op->fiber = rynix_rt_fiber_current();
  op->kind = IOP_CONNECT;
  op->accept_sock = s; /* store connect socket */
  if (!g_connectex(s, (const struct sockaddr *)addr, (int)addrlen, NULL, 0, &bytes,
                   &op->ov)) {
    int err = WSAGetLastError();
    if (err != WSA_IO_PENDING && err != ERROR_IO_PENDING) {
      free(op);
      return -1;
    }
  }
  return wait_op(op);
}

#else

void rynix_rt_iocp_init(void) {}
void rynix_rt_iocp_shutdown(void) {}
int rynix_rt_iocp_ready(void) { return 0; }
void rynix_rt_iocp_poll(void) {}
void rynix_rt_iocp_wait(void) {}
int64_t rynix_rt_iocp_ext_ready(void) { return -1; }
int64_t rynix_rt_iocp_associate(int64_t fd) {
  (void)fd;
  return -1;
}
int64_t rynix_rt_iocp_recv(int64_t fd, void *buf, int64_t n) {
  (void)fd;
  (void)buf;
  (void)n;
  return -1;
}
int64_t rynix_rt_iocp_send(int64_t fd, const void *buf, int64_t n) {
  (void)fd;
  (void)buf;
  (void)n;
  return -1;
}
int64_t rynix_rt_iocp_accept(int64_t listen_fd) {
  (void)listen_fd;
  return -1;
}
int64_t rynix_rt_iocp_connect(int64_t fd, const void *addr, int64_t addrlen) {
  (void)fd;
  (void)addr;
  (void)addrlen;
  return -1;
}

#endif
