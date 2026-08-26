# Sanitizer CI scaffold (Phase 26-F)

**Status:** Scaffold documented; enforce in Phase 27-C (`msan_ubsan_rt_clean`).

Recommended Linux CI flags for `rt/` smokes (not yet hard-fail on all hosts):

```text
clang -fsanitize=address,undefined -O1 -g
# memory sanitizer: clang -fsanitize=memory (requires instrumented libc++)
```

Workspace clippy remains `-D warnings` on critical crates. Promote
`clippy::pedantic` allows carefully; do not blanket-deny without fixing noise.

Workflow tip: start with `continue-on-error: true`, flip to required in Phase 27.
