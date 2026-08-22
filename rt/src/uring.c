/* Linux io_uring backend (syscall path, no liburing required).
 *
 * Fiber-aware: submit SQE, park the calling fiber, harvest CQEs in the
 * scheduler (`rynix_rt_uring_poll` / `rynix_rt_uring_wait`). Sync
 * `enter(min_complete=1)` only runs when the ready queue is empty and fibers
 * are parked — never while sibling fibers still have work.
 */

#include "rynix_rt.h"

#if defined(RYNIX_RT_URING) && defined(__linux__)

#define _GNU_SOURCE
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

#include <linux/io_uring.h>

#ifndef __NR_io_uring_setup
#define __NR_io_uring_setup 425
#endif
#ifndef __NR_io_uring_enter
#define __NR_io_uring_enter 426
#endif

struct rynix_uring {
  int fd;
  struct io_uring_sqe *sqes;
  unsigned *sq_head;
  unsigned *sq_tail;
  unsigned *sq_ring_mask;
  unsigned *sq_array;
  unsigned *cq_head;
  unsigned *cq_tail;
  unsigned *cq_ring_mask;
  struct io_uring_cqe *cqes;
  unsigned sq_entries;
  int ready;
};

static struct rynix_uring g_uring;

static int io_uring_setup(unsigned entries, struct io_uring_params *p) {
  return (int)syscall(__NR_io_uring_setup, entries, p);
}

static int io_uring_enter(int fd, unsigned to_submit, unsigned min_complete, unsigned flags) {
  return (int)syscall(__NR_io_uring_enter, fd, to_submit, min_complete, flags, NULL, 0);
}

void rynix_rt_uring_init(void) {
  if (g_uring.ready) return;
  struct io_uring_params p;
  memset(&p, 0, sizeof(p));
  int fd = io_uring_setup(64, &p);
  if (fd < 0) return;
  size_t sq_sz = p.sq_off.array + p.sq_entries * sizeof(unsigned);
  size_t cq_sz = p.cq_off.cqes + p.cq_entries * sizeof(struct io_uring_cqe);
  void *sq_ptr = mmap(NULL, sq_sz, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_POPULATE, fd,
                      IORING_OFF_SQ_RING);
  void *cq_ptr = mmap(NULL, cq_sz, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_POPULATE, fd,
                      IORING_OFF_CQ_RING);
  void *sqes = mmap(NULL, p.sq_entries * sizeof(struct io_uring_sqe), PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_POPULATE, fd, IORING_OFF_SQES);
  if (sq_ptr == MAP_FAILED || cq_ptr == MAP_FAILED || sqes == MAP_FAILED) {
    close(fd);
    return;
  }
  g_uring.fd = fd;
  g_uring.sqes = (struct io_uring_sqe *)sqes;
  g_uring.sq_head = (unsigned *)((char *)sq_ptr + p.sq_off.head);
  g_uring.sq_tail = (unsigned *)((char *)sq_ptr + p.sq_off.tail);
  g_uring.sq_ring_mask = (unsigned *)((char *)sq_ptr + p.sq_off.ring_mask);
  g_uring.sq_array = (unsigned *)((char *)sq_ptr + p.sq_off.array);
  g_uring.cq_head = (unsigned *)((char *)cq_ptr + p.cq_off.head);
  g_uring.cq_tail = (unsigned *)((char *)cq_ptr + p.cq_off.tail);
  g_uring.cq_ring_mask = (unsigned *)((char *)cq_ptr + p.cq_off.ring_mask);
  g_uring.cqes = (struct io_uring_cqe *)((char *)cq_ptr + p.cq_off.cqes);
  g_uring.sq_entries = p.sq_entries;
  g_uring.ready = 1;
}

void rynix_rt_uring_shutdown(void) {
  if (!g_uring.ready) return;
  close(g_uring.fd);
  memset(&g_uring, 0, sizeof(g_uring));
}

int rynix_rt_uring_ready(void) { return g_uring.ready; }

static void harvest_cqes(void) {
  if (!g_uring.ready) return;
  for (;;) {
    unsigned head = *g_uring.cq_head;
    unsigned tail = *g_uring.cq_tail;
    if (head == tail) break;
    struct io_uring_cqe *cqe = &g_uring.cqes[head & *g_uring.cq_ring_mask];
    void *fiber = (void *)(uintptr_t)cqe->user_data;
    int64_t res = cqe->res;
    *g_uring.cq_head = head + 1;
    if (fiber) {
      rynix_rt_fiber_set_result(fiber, res < 0 ? -1 : res);
      rynix_rt_fiber_unpark(fiber);
    }
  }
}

void rynix_rt_uring_poll(void) {
  if (!g_uring.ready) return;
  /* Push any pending SQEs without waiting for completions. */
  (void)io_uring_enter(g_uring.fd, 0, 0, 0);
  harvest_cqes();
}

void rynix_rt_uring_wait(void) {
  if (!g_uring.ready) return;
  (void)io_uring_enter(g_uring.fd, 0, 1, IORING_ENTER_GETEVENTS);
  harvest_cqes();
}

static int64_t submit_and_park(struct io_uring_sqe *sqe) {
  void *self = rynix_rt_fiber_current();
  unsigned tail = *g_uring.sq_tail;
  unsigned index = tail & *g_uring.sq_ring_mask;
  g_uring.sq_array[index] = (unsigned)(sqe - g_uring.sqes);
  *g_uring.sq_tail = tail + 1;

  if (!self) {
    /* No fiber context: sync wait (smoke tests / bare main). */
    sqe->user_data = 0;
    if (io_uring_enter(g_uring.fd, 1, 1, IORING_ENTER_GETEVENTS) < 0) return -1;
    unsigned head = *g_uring.cq_head;
    if (head == *g_uring.cq_tail) return -1;
    struct io_uring_cqe *cqe = &g_uring.cqes[head & *g_uring.cq_ring_mask];
    int64_t res = cqe->res;
    *g_uring.cq_head = head + 1;
    return res < 0 ? -1 : res;
  }

  sqe->user_data = (unsigned long)(uintptr_t)self;
  if (io_uring_enter(g_uring.fd, 1, 0, 0) < 0) return -1;
  rynix_rt_fiber_park();
  return rynix_rt_fiber_get_result(self);
}

static struct io_uring_sqe *next_sqe(void) {
  unsigned head = *g_uring.sq_head;
  unsigned tail = *g_uring.sq_tail;
  if ((tail - head) >= g_uring.sq_entries) return NULL;
  unsigned index = tail & *g_uring.sq_ring_mask;
  struct io_uring_sqe *sqe = &g_uring.sqes[index];
  memset(sqe, 0, sizeof(*sqe));
  return sqe;
}

int64_t rynix_rt_uring_read(int64_t fd, void *buf, int64_t n) {
  if (!g_uring.ready || !buf || n <= 0) return -1;
  struct io_uring_sqe *sqe = next_sqe();
  if (!sqe) return -1;
  sqe->opcode = IORING_OP_READ;
  sqe->fd = (int)fd;
  sqe->addr = (unsigned long)buf;
  sqe->len = (unsigned)n;
  return submit_and_park(sqe);
}

int64_t rynix_rt_uring_write(int64_t fd, const void *buf, int64_t n) {
  if (!g_uring.ready || !buf || n <= 0) return -1;
  struct io_uring_sqe *sqe = next_sqe();
  if (!sqe) return -1;
  sqe->opcode = IORING_OP_WRITE;
  sqe->fd = (int)fd;
  sqe->addr = (unsigned long)buf;
  sqe->len = (unsigned)n;
  return submit_and_park(sqe);
}

int64_t rynix_rt_uring_accept(int64_t listen_fd) {
  if (!g_uring.ready || listen_fd < 0) return -1;
  struct sockaddr_storage addr;
  socklen_t addrlen = sizeof(addr);
  struct io_uring_sqe *sqe = next_sqe();
  if (!sqe) return -1;
  sqe->opcode = IORING_OP_ACCEPT;
  sqe->fd = (int)listen_fd;
  sqe->addr = (unsigned long)&addr;
  sqe->addr2 = (unsigned long)&addrlen;
  sqe->accept_flags = 0;
  return submit_and_park(sqe);
}

int64_t rynix_rt_uring_connect(int64_t fd, const void *addr, int64_t addrlen) {
  if (!g_uring.ready || fd < 0 || !addr || addrlen <= 0) return -1;
  struct io_uring_sqe *sqe = next_sqe();
  if (!sqe) return -1;
  sqe->opcode = IORING_OP_CONNECT;
  sqe->fd = (int)fd;
  sqe->addr = (unsigned long)addr;
  sqe->off = (unsigned long)addrlen;
  int64_t res = submit_and_park(sqe);
  return res < 0 ? -1 : 0;
}

#else

void rynix_rt_uring_init(void) {}
void rynix_rt_uring_shutdown(void) {}
int rynix_rt_uring_ready(void) { return 0; }
void rynix_rt_uring_poll(void) {}
void rynix_rt_uring_wait(void) {}

int64_t rynix_rt_uring_read(int64_t fd, void *buf, int64_t n) {
  (void)fd;
  (void)buf;
  (void)n;
  return -1;
}

int64_t rynix_rt_uring_write(int64_t fd, const void *buf, int64_t n) {
  (void)fd;
  (void)buf;
  (void)n;
  return -1;
}

int64_t rynix_rt_uring_accept(int64_t listen_fd) {
  (void)listen_fd;
  return -1;
}

int64_t rynix_rt_uring_connect(int64_t fd, const void *addr, int64_t addrlen) {
  (void)fd;
  (void)addr;
  (void)addrlen;
  return -1;
}

#endif
