# RFC process (Track C)

Language / ABI / process changes that widen the SPEC surface should land as an
RFC **before** coding, then an ADR when irreversible.

## Steps

1. Copy [0000-template.md](0000-template.md) → `NNNN-slug.md` (next free number).
2. Open a PR titled `rfc: NNNN slug`.
3. Discuss until Accepted / Rejected / Deferred is recorded in the RFC header.
4. **Track G language widen** (parametric `Vec[T]` / `Map[K,V]`) requires an
   Accepted RFC **and** ADR-0025 (Phase 35) before implementation waves.
5. Never mark ROADMAP ✅ without named gates.

## Out of scope for RFCs

- Absolute-10 marketing claims
- End-style domain theater
- Silent SPEC widen in README without tests
