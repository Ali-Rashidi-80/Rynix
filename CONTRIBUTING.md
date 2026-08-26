# Contributing

**Languages:** [English](CONTRIBUTING.md) (default) · [فارسی](CONTRIBUTING.fa.md)

1. Keep changes atomic and tested (`cargo test --workspace`, clippy `-D warnings`).
2. **English is the canonical docs language** (SPEC, ADRs, schemas). Persian
   companions (`.fa.md`) must stay fact-aligned; do not invent language features
   in any README without SPEC + tests.
3. Prefer fixing the compiler over weakening a test.
4. Runtime changes: exercise `rt/tests` / `size_echo_gates`.
5. See [AGENTS.md](AGENTS.md) for AI-oriented workflows and [docs/COMPARE.md](docs/COMPARE.md)
   for honest peer positioning.
6. Dual license: MIT OR Apache-2.0 ([LICENSE.md](LICENSE.md)). Contributions are dual-licensed
   the same way unless stated otherwise.
7. Do **not** add `Co-authored-by: Cursor <cursoragent@cursor.com>` trailers. Optional
   local hook: `git config core.hooksPath .githooks` (strips that trailer on commit).

## Useful commands

```sh
cargo test --workspace
cargo clippy -p rynixc -p rynix-rir -p rynix-codegen -- -D warnings
python benchmarks/suite5/run_suite5.py --langs c,rynix
rynixc arch check
cd editors/vscode && npm install && npm run compile
```

## Documentation

- Do not mark ROADMAP ✅ without in-tree tests.
- Suite5 Notes must disclose strength reductions (see [benchmarks/suite5/README.md](benchmarks/suite5/README.md)).
- Keep machine-local ms tables dated and marked as sample runs.

## RFCs (Track C)

Non-trivial language or process proposals use the RFC template:

1. Copy [rfcs/0000-template.md](rfcs/0000-template.md) to `rfcs/NNNN-slug.md`.
2. Discuss in a PR; Accepted RFCs may require a follow-on ADR + named gate.
3. Do not invent End-style domain theater or mark ROADMAP ✅ without evidence.

Good first issues (when labeled): docs typos, example polish, gate naming —
never Absolute-10 marketing stubs.
