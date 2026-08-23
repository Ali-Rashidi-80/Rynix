/* Cooperative fibers — Win32 Fibers on Windows, ucontext elsewhere.
 * Stacks: 256 KiB with a leading guard page (Phase 8).
 * Supports PARKED state for fiber-aware io_uring waits.
 */

#ifndef _WIN32
#define _XOPEN_SOURCE 700
#include <time.h>
#endif

#include "rynix_rt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <sys/mman.h>
#include <ucontext.h>
#include <unistd.h>
#endif

#define RYNIX_STACK_SIZE (256u * 1024u)
#define RYNIX_MAX_FIBERS 256

typedef enum { FIB_READY, FIB_RUNNING, FIB_PARKED, FIB_DONE } FiberState;

typedef struct Fiber {
  rynix_rt_fiber_fn fn;
  void *arg;
  FiberState state;
  void *stack_base;
  size_t stack_total;
  int64_t io_result;
#ifdef _WIN32
  LPVOID win_fiber;
#else
  ucontext_t uctx;
  int uctx_live;
#endif
  struct Fiber *next;
} Fiber;

static Fiber g_fibers[RYNIX_MAX_FIBERS];
static int g_fiber_cap;
static Fiber *g_ready_head;
static Fiber *g_ready_tail;
static Fiber *g_current;
static int g_worker_converted;
static int64_t g_live_count;
static int g_parked_count;

#ifdef _WIN32
static LPVOID g_main_fiber;
#endif

#ifndef _WIN32
static ucontext_t g_main_ctx;
static int g_main_ctx_live;
#endif

static void enqueue(Fiber *f) {
  f->next = NULL;
  if (!g_ready_tail) {
    g_ready_head = g_ready_tail = f;
  } else {
    g_ready_tail->next = f;
    g_ready_tail = f;
  }
}

static Fiber *dequeue(void) {
  Fiber *f = g_ready_head;
  if (!f) return NULL;
  g_ready_head = f->next;
  if (!g_ready_head) g_ready_tail = NULL;
  f->next = NULL;
  return f;
}

static void *alloc_stack(size_t *out_total) {
  size_t page;
#ifdef _WIN32
  SYSTEM_INFO si;
  GetSystemInfo(&si);
  page = (size_t)si.dwPageSize;
#else
  long p = sysconf(_SC_PAGESIZE);
  page = p > 0 ? (size_t)p : 4096u;
#endif
  size_t total = page + RYNIX_STACK_SIZE;
  *out_total = total;

#ifdef _WIN32
  void *base = VirtualAlloc(NULL, total, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
  if (!base) rynix_rt_panic("VirtualAlloc fiber stack failed");
  DWORD old;
  if (!VirtualProtect(base, page, PAGE_NOACCESS, &old))
    rynix_rt_panic("VirtualProtect guard failed");
  return base;
#else
  void *base = mmap(NULL, total, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (base == MAP_FAILED) rynix_rt_panic("mmap fiber stack failed");
  if (mprotect((char *)base + page, RYNIX_STACK_SIZE, PROT_READ | PROT_WRITE) != 0)
    rynix_rt_panic("mprotect fiber stack failed");
  return base;
#endif
}

static void free_stack(void *base, size_t total) {
  if (!base) return;
#ifdef _WIN32
  (void)total;
  VirtualFree(base, 0, MEM_RELEASE);
#else
  munmap(base, total);
#endif
}

#ifdef _WIN32
static void WINAPI fiber_tramp(LPVOID param) {
  Fiber *f = (Fiber *)param;
  f->state = FIB_RUNNING;
  f->fn(f->arg);
  f->state = FIB_DONE;
  g_live_count--;
  g_current = NULL;
  SwitchToFiber(g_main_fiber);
}
#else
static void fiber_tramp(void) {
  Fiber *f = g_current;
  f->state = FIB_RUNNING;
  f->fn(f->arg);
  f->state = FIB_DONE;
  g_live_count--;
  g_current = NULL;
  if (g_main_ctx_live) setcontext(&g_main_ctx);
  rynix_rt_panic("fiber returned with no main context");
}
#endif

static Fiber *alloc_fiber_slot(void) {
  for (int i = 0; i < g_fiber_cap; i++) {
    if (g_fibers[i].state == FIB_DONE || g_fibers[i].fn == NULL) {
      return &g_fibers[i];
    }
  }
  if (g_fiber_cap >= RYNIX_MAX_FIBERS) rynix_rt_panic("too many fibers");
  return &g_fibers[g_fiber_cap++];
}

void *rynix_rt_spawn(rynix_rt_fiber_fn fn, void *arg) {
  if (!fn) rynix_rt_panic("spawn null fn");

#ifdef _WIN32
  if (!g_worker_converted) {
    g_main_fiber = ConvertThreadToFiber(NULL);
    if (!g_main_fiber) rynix_rt_panic("ConvertThreadToFiber failed");
    g_worker_converted = 1;
  }
#endif

  Fiber *f = alloc_fiber_slot();
  memset(f, 0, sizeof(*f));
  f->fn = fn;
  f->arg = arg;
  f->state = FIB_READY;
  f->stack_base = alloc_stack(&f->stack_total);

  size_t page;
#ifdef _WIN32
  SYSTEM_INFO si;
  GetSystemInfo(&si);
  page = (size_t)si.dwPageSize;
  f->win_fiber = CreateFiber(RYNIX_STACK_SIZE, fiber_tramp, f);
  if (!f->win_fiber) rynix_rt_panic("CreateFiber failed");
  (void)page;
#else
  page = (size_t)sysconf(_SC_PAGESIZE);
  if (page == (size_t)-1) page = 4096;
  void *stack = (char *)f->stack_base + page;
  if (getcontext(&f->uctx) != 0) rynix_rt_panic("getcontext failed");
  f->uctx.uc_stack.ss_sp = stack;
  f->uctx.uc_stack.ss_size = RYNIX_STACK_SIZE;
  f->uctx.uc_link = &g_main_ctx;
  makecontext(&f->uctx, fiber_tramp, 0);
  f->uctx_live = 1;
#endif

  g_live_count++;
  enqueue(f);
  return f;
}

static void switch_to_fiber(Fiber *f) {
  Fiber *prev = g_current;
  g_current = f;
  f->state = FIB_RUNNING;
#ifdef _WIN32
  SwitchToFiber(f->win_fiber);
  (void)prev;
#else
  if (!g_main_ctx_live) {
    if (getcontext(&g_main_ctx) != 0) rynix_rt_panic("getcontext main failed");
    g_main_ctx_live = 1;
  }
  if (prev) {
    swapcontext(&prev->uctx, &f->uctx);
  } else {
    swapcontext(&g_main_ctx, &f->uctx);
  }
#endif
}

void *rynix_rt_fiber_current(void) { return g_current; }

int rynix_rt_fiber_parked_count(void) { return g_parked_count; }

void rynix_rt_fiber_set_result(void *fiber, int64_t result) {
  Fiber *f = (Fiber *)fiber;
  if (f) f->io_result = result;
}

int64_t rynix_rt_fiber_get_result(void *fiber) {
  Fiber *f = (Fiber *)fiber;
  return f ? f->io_result : -1;
}

void rynix_rt_fiber_park(void) {
  Fiber *cur = g_current;
  if (!cur) return;
  cur->state = FIB_PARKED;
  g_parked_count++;
  g_current = NULL;
#ifdef _WIN32
  SwitchToFiber(g_main_fiber);
#else
  if (!g_main_ctx_live) {
    if (getcontext(&g_main_ctx) != 0) rynix_rt_panic("getcontext main failed");
    g_main_ctx_live = 1;
  }
  swapcontext(&cur->uctx, &g_main_ctx);
#endif
}

void rynix_rt_fiber_unpark(void *fiber) {
  Fiber *f = (Fiber *)fiber;
  if (!f || f->state != FIB_PARKED) return;
  f->state = FIB_READY;
  g_parked_count--;
  if (g_parked_count < 0) g_parked_count = 0;
  enqueue(f);
}

void rynix_rt_yield(void) {
  Fiber *cur = g_current;
  if (!cur) return;
  cur->state = FIB_READY;
  enqueue(cur);
#ifdef _WIN32
  g_current = NULL;
  SwitchToFiber(g_main_fiber);
#else
  g_current = NULL;
  if (!g_main_ctx_live) {
    if (getcontext(&g_main_ctx) != 0) rynix_rt_panic("getcontext main failed");
    g_main_ctx_live = 1;
  }
  swapcontext(&cur->uctx, &g_main_ctx);
#endif
}

static void finish_fiber(Fiber *f) {
  if (f->state != FIB_DONE) return;
#ifdef _WIN32
  if (f->win_fiber) {
    DeleteFiber(f->win_fiber);
    f->win_fiber = NULL;
  }
#endif
  free_stack(f->stack_base, f->stack_total);
  f->stack_base = NULL;
  f->fn = NULL;
#ifndef _WIN32
  f->uctx_live = 0;
#endif
}

void rynix_rt_run(void) {
  rynix_rt_uring_init();
  rynix_rt_iocp_init();
#ifdef _WIN32
  if (!g_worker_converted) {
    g_main_fiber = ConvertThreadToFiber(NULL);
    if (!g_main_fiber) rynix_rt_panic("ConvertThreadToFiber failed");
    g_worker_converted = 1;
  }
#endif
#ifndef _WIN32
  if (!g_main_ctx_live) {
    if (getcontext(&g_main_ctx) != 0) rynix_rt_panic("getcontext failed");
    g_main_ctx_live = 1;
  }
#endif
  for (;;) {
    rynix_rt_uring_poll();
    rynix_rt_iocp_poll();
    Fiber *f = dequeue();
    if (!f) {
      if (g_parked_count > 0) {
        /* Only block on CQ/IOCP when a backend can complete waits. */
        if (rynix_rt_uring_ready()) {
          rynix_rt_uring_wait();
          continue;
        }
        if (rynix_rt_iocp_ready()) {
          rynix_rt_iocp_wait();
          continue;
        }
        rynix_rt_panic("fibers parked with no completion backend to resume waits");
      }
      break;
    }
    if (f->state == FIB_DONE) {
      finish_fiber(f);
      continue;
    }
    switch_to_fiber(f);
    if (f->state == FIB_DONE) finish_fiber(f);
  }
}

int64_t rynix_rt_fiber_count(void) { return g_live_count; }

void rynix_rt_sleep_ms(int64_t ms) {
  if (ms <= 0) {
    rynix_rt_yield();
    return;
  }
#ifdef _WIN32
  Sleep((DWORD)ms);
#else
  struct timespec ts;
  ts.tv_sec = (time_t)(ms / 1000);
  ts.tv_nsec = (long)((ms % 1000) * 1000000L);
  nanosleep(&ts, NULL);
#endif
  rynix_rt_yield();
}
