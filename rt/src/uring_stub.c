/* Linux io_uring backend stubs (Phase 8).
 *
 * Built only when RYNIX_RT_URING is defined (rynixc build --runtime=uring).
 * Full SQE submit + fiber park lands with liburing linkage in CI/Linux.
 * This translation unit keeps the symbol surface compiling; portable I/O
 * remains the default fallback until liburing is present.
 */

#include "rynix_rt.h"

#if defined(RYNIX_RT_URING) && defined(__linux__)

#include <stdio.h>

/* Placeholder: a real implementation would hold per-core io_uring + runqueues.
 * For now we document the hook and fall back to portable yield semantics via
 * the shared fiber scheduler. */

void rynix_rt_uring_init(void) {
  /* reserved for per-core ring setup */
}

void rynix_rt_uring_shutdown(void) {
  /* reserved */
}

#else

/* Ensure the TU is never empty on non-uring builds. */
static int rynix_rt_uring_unused;

#endif
