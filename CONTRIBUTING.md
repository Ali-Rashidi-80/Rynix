# Contributing

1. Keep changes atomic and tested (`cargo test --workspace`, clippy `-D warnings`).
2. Docs are English; do not invent language features in README without SPEC + tests.
3. Prefer fixing the compiler over weakening a test.
4. Runtime changes: exercise `rt/tests` / `size_echo_gates`.
5. See [AGENTS.md](AGENTS.md) for AI-oriented workflows and [docs/COMPARE.md](docs/COMPARE.md)
   for honest peer positioning.

## Useful commands

```sh
cargo test --workspace
cargo clippy -p rynixc -p rynix-rir -p rynix-codegen -- -D warnings
python benchmarks/suite5/run_suite5.py --langs c,rynix
rynixc arch check
cd editors/vscode && npm install && npm run compile
```
