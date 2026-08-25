# Deferred: Raft / consensus product (out of Niche-10 path)

Date: 2026-08-25  
Status: **Deferred** — not acceptance-gated for Phases 16–20

## Context

Distributed consensus (Raft, Paxos-class) is a multi-year systems product with
Jepsen-class testing, membership changes, log compaction, and failure domains.
Peer marketing sometimes lists “Raft client” as a checkbox. Shipping a stub
client or Stable ROADMAP row without real gates would violate AGENTS.md honesty.

## Decision

- **No** Raft / consensus product, client library, or Stable claim in Phases 16–20.
- Niche-10 ([ADR-0013](0013-niche-10-scorecard.md)) explicitly excludes Raft.
- Revisit only with a dedicated ADR naming: log storage, membership API, and
  in-tree chaos/Jepsen-class harnesses.

## Consequences

- END_PEER_GAP / VERDICT may note End or others lead on distributed kits — honest.
- Agents must refuse “just add Raft” theater requests.
- HTTP/TLS/fibers deepen (Phases 16/18) does **not** imply consensus readiness.
