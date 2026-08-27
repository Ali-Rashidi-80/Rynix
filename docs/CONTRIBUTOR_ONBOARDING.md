# Contributor onboarding (E-14)

Process checklist — not a fake “N contributors” KPI gate.

## First hour

1. Read [README.md](../README.md) + [INSTALL.md](../INSTALL.md) (one-path clang).
2. `cargo test -p rynixc --test agent_cli golden_remaining_sot`
3. Skim [docs/SPEC.md](SPEC.md) and [AGENTS.md](../AGENTS.md).
4. Run `rynixc run examples/tutorial_01_hello.ryx`.

## First contribution

1. Pick a **good-first-issue** label (docs typo, example polish, gate naming).
2. Follow [CONTRIBUTING.md](../CONTRIBUTING.md) — English docs canonical; no Cursor
   co-author trailer.
3. Prefer fixing the compiler over loosening a test.
4. For language surface: RFC → ADR → gate (see [rfcs/README.md](../rfcs/README.md)).

## Honesty

Do not invent stub domains or mark Quality-10 axes without named gates
([GOLDEN_PATH.md](GOLDEN_PATH.md), [GOLDEN_REMAINING.md](GOLDEN_REMAINING.md)).
