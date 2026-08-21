/* Unity build of the portable Rynix runtime (Phase 8).
 *
 * rynixc build links this file with -I rt/include. Modular sources live under
 * rt/src/ and are pulled in below.
 */

#include "src/alloc.c"
#include "src/fiber.c"
#include "src/io_portable.c"
#include "src/uring_stub.c"
