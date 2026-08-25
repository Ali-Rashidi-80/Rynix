# راهنمای عامل‌ها برای Rynix (فارسی)

**زبان‌ها:** [English](AGENTS.md) (پیش‌فرض) · [فارسی](AGENTS.fa.md)

> خلاصهٔ [`AGENTS.md`](AGENTS.md). فرمان‌ها و مسیرهای فایل را از نسخهٔ انگلیسی کپی کنید.

## زنجیرهٔ ابزار

```text
.ryx → rynixc check | dump-rir | emit-ll | emit-wasm | build | run | fmt | mcp-serve | lsp-serve
 | arch check | graph | slice | impact | eval | patch | verify | precheck | context
 | security | scope | deps | dna | new
```

- تشخیص: `--error-format=json` → `rynix.diag.v1`
- پکیج: `deps --attest` → `rynix.attest.v1` (**نه** Sigstore)
- WASM: `emit-wasm` بدون WASI؛ Node می‌تواند `main` و host-import `print_i64` را اجرا کند
- Niche-10: [docs/NICHE10.md](docs/NICHE10.md)؛ Raft تعویق: [ADR-0012](docs/adr/0012-deferred-consensus.md)

## صداقت

- ROADMAP ✅ فقط با تست درختی.
- دامنه/کلیدواژهٔ End (`feature`/`skill`/`task`/`agent`) اختراع نکنید.
- Suite5: opaque trip count؛ strength reduction فقط با checksum و افشا.

Skill کامل: [`.agents/skills/rynix/SKILL.md`](.agents/skills/rynix/SKILL.md).
