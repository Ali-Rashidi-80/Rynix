# Sanitizer CI scaffold (Phase 26-F → Phase 31 hard)

**Status:** Phase 31 enforces **ASan+UBSan** on `sanitizer-rt` CI job
(`-fsanitize=address,undefined`). Gate: `msan_ubsan_rt_enforced`.

```text
clang -fsanitize=address,undefined -O1 -g
# memory sanitizer (optional separate job): clang -fsanitize=memory
#   requires instrumented libc++ — not merged into sanitizer-rt
```

MSan remains **optional / documented** (instrumented libc++ host dependency).
Workspace clippy remains `-D warnings` on critical crates.
