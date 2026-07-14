# Rendezvous Data Model — CRDT Documents and the Analytics Projection

> **Status: RATIFIED (2026-07-12)** — this document is the authoritative shared
> data-model vocabulary for the rendezvous feature specs (logging-refactor,
> host-capability-broadcast, process-monitor, rendezvous-dashboard). Specs reference
> this document rather than re-deriving storage patterns per feature. The riskiest
> assumptions were validated by the register-compaction spike before ratification.

## The Two Stores

The rendezvous daemon uses two durable stores with strictly separated roles:

| Store | Role | Contents |
|-------|------|----------|
| **redb** | Transactional system of record | Loro CRDT document snapshots, sync cursors, pairings |
| **DuckDB** | Analytical projection | Columnar mirror of CRDT contents, shaped for reporting |

**Invariant: DuckDB is disposable.** Every row in DuckDB must be rebuildable from the
CRDT documents in redb (the Phase-2 implementation already does this — see
`daemon/src/projection.rs`, which truncates and re-projects at startup). Nothing may be
written to DuckDB that does not originate from a CRDT document. If a report needs a new
piece of data, the data model question is always *"which CRDT document does this fact
live in?"* — never *"which DuckDB table do I insert into?"*.

## Core Principles

1. **Single-writer, many-reader is the default.** Every CRDT document has exactly one
   writing node (the `owner_node_id` embedded in its identifier). All other nodes are
   read-only replicas that receive the document through mesh sync. With one writer,
   concurrent-edit conflicts cannot occur — Loro serves as a causally-consistent
   replication protocol (state-vector delta sync), not a collaborative editor. This is
   what keeps CRDT modeling simple; treat any proposal for a multi-writer document as a
   design smell requiring justification (see [Kind 3](#kind-3--multi-writer-documents-avoid)).

2. **CRDT documents hold facts and current state — never derived data.** Aggregates,
   rollups, correlations, and metrics are computed in DuckDB (usually as views). Storing
   a computed metric in a CRDT document couples the mesh protocol to a reporting
   decision and creates two sources of truth.

3. **CRDT documents hold data the mesh *originates*.** Commits, PRs, CI runs, and kanban
   cards already have an authoritative home (git, GitHub, Trello, …). They enter the
   mesh as *observations* recorded in fact logs — we never model external state as
   CRDT-managed state. A git SHA is already a globally consistent identity; it needs
   transport, not conflict resolution.

4. **Every fact carries an idempotency key.** Projection into DuckDB must be safely
   repeatable (re-sync, restart, rebuild). The established pattern is a
   `UNIQUE(chunk_id, sequence)` constraint with `INSERT OR IGNORE`; every new fact table
   follows the same shape.

5. **Documents are bounded.** Loro documents carry their full operation history, so an
   unbounded document grows forever. Fact logs are bounded by chunk rotation; state
   registers are bounded by low write cadence (and, when necessary, shallow-snapshot
   re-basing). "How does this document stay small?" is a required question for every new
   document type.

## Document Taxonomy

Every CRDT document in the system is one of three kinds. New features should identify
which kind each piece of data belongs to before writing any schema.

### Kind 1 — Fact Logs (append-only)

Immutable events, appended in order by the single writer, never edited or deleted.

- **Loro shape:** one `LoroList` of entries plus a `LoroMap` of chunk metadata
  (the existing session-log layout in `core/src/session_log.rs`).
- **Bounding:** chunk rotation on entry-count / byte caps; a chunk links to its
  predecessor via `previous_chunk_id`, so a session's history is a chain of small
  documents.
- **Identity:** deterministic path — `session/{owner_node_id}/{session_id}/part/{idx}`
  today; new log domains follow the same grammar (see
  [Document Addressing](#document-addressing)).
- **Examples:** session log entries, lifecycle/hook events, tool-call records, presence
  transitions (`host_online` / `host_offline`), git observations (commit seen, PR
  opened, CI run finished), process-monitor observations.

Fact logs are the bread and butter of the system. When in doubt, model data as an
event in a fact log — event-shaped data projects cleanly into DuckDB and answers
"over time" questions for free.

### Kind 2 — State Registers (last-writer-wins)

The *current* value of something, overwritten in place by the single writer.

- **Loro shape:** a `LoroMap` keyed by stable field names. Because there is exactly one
  writer, last-writer-wins semantics are trivially correct.
- **Bounding:** registers must have a genuinely low write cadence (minutes-to-days, not
  seconds). Loro retains op history, so a register rewritten every few seconds
  accumulates real weight over months. If a register's cadence grows, either move the
  hot field to the ephemeral layer (below) or periodically re-base the document via
  Loro's shallow-snapshot export.
- **Write discipline:** only write on *change*. A capability refresh that detects no
  diff must not touch the document.
- **Examples:** host capabilities (`capability/{node_id}`), per-host active-session
  registry (`sessions-active/{node_id}`: session_id → status), sequence registry
  (`sequence/{node_id}/{sequence_id}`: member sessions + status).

### Kind 3 — Multi-Writer Documents (avoid)

Documents where more than one node writes. **We currently have none, and that is
deliberate.** The first legitimate candidate is the future mesh job queue
(host-capability-broadcast's "choose an appropriate host" flow).

When one becomes unavoidable, the rules are:

- Use a `LoroMap` keyed by globally unique IDs (e.g. `{node_id}-{local_counter}`).
  Concurrent inserts of *different* keys merge cleanly; that is the only concurrency we
  rely on.
- Never use a CRDT list for multi-writer data — interleaving semantics are the hard
  part of CRDT theory and we do not need them.
- Resolve contention at the application layer, not the CRDT layer: e.g. a job is a key
  written once by its submitter; a claim is a *separate* field written by the claiming
  node; ties are broken deterministically (lowest `node_id` wins) after convergence.
- Each multi-writer document requires its own design review; do not generalize from
  one to the next.

### The Ephemeral Layer (presence — not a persistent document)

Liveness (`online`, heartbeats, "typing"-class signals) is qualitatively different from
state: it is high-cadence, it is an *observation* rather than self-declared state (a
dead host cannot write `online: false`), and persisting it bloats history.

Presence therefore never lives in a persistent CRDT document. Two sanctioned mechanisms:

1. **Derived presence (default):** each daemon already knows the last successful sync /
   QUIC connection per peer. `online` is computed locally as
   `now - last_seen < threshold`. Zero new machinery.
2. **Loro ephemeral store (if richer presence is needed):** Loro's ephemeral store
   (formerly Awareness) is purpose-built — synced between peers, never persisted into
   document history, entries expire on timeout.

If presence *history* matters (uptime reporting), the daemon additionally records
presence **transitions** as facts (Kind 1): `host_online` / `host_offline` events at the
moment the derived presence flips. History lives in the fact log; the live boolean lives
in the ephemeral layer. See [Worked Answers](#worked-answers-to-open-spec-questions).

## Document Addressing

All documents follow one path grammar, extending the established session-log scheme:

```
{domain}/{owner_node_id}[/{entity_id}][/part/{chunk_index}]
```

| Document | Kind | Path |
|----------|------|------|
| Session log chunk | 1 | `session/{node_id}/{session_id}/part/{n}` *(existing)* |
| Git/CI observations | 1 | `git/{node_id}/part/{n}` |
| Presence transitions | 1 | `presence-log/{node_id}/part/{n}` |
| Host capabilities | 2 | `capability/{node_id}` |
| Active-session registry | 2 | `sessions-active/{node_id}` |
| Sequence registry | 2 | `sequence/{node_id}/{sequence_id}` |

Notes:

- `owner_node_id` in the path *is* the single-writer declaration: only that node may
  write the document; every daemon enforces this on import (reject a delta whose ops
  originate from a foreign peer ID).
- The host-capability spec's `capability-${id}` naming is superseded by
  `capability/{node_id}` for grammar uniformity.
- The path is the redb key, the gRPC identifier, and (for fact logs) a DuckDB column —
  one identity everywhere, parseable without consulting the daemon.

## The DuckDB Projection

DuckDB is organized as a lightweight **star schema**: append-only *fact tables*
projected from Kind-1 documents, plus *dimension tables* projected from Kind-2
registers.

### Fact tables

One table per observation family, all following the established shape:

- Idempotency: `UNIQUE(chunk_id, sequence)` + `INSERT OR IGNORE`.
- A `created_at_unix_ms BIGINT` event time.
- Denormalized context columns (`owner_node_id`, `session_id`, `agent`, `model`,
  `repo`, …) repeated on every row. DuckDB is columnar — repetition compresses well and
  avoids join complexity. Do **not** over-normalize.
- A `metadata_json TEXT` catch-all for long-tail fields. Promote a field to a typed
  column only when a report actually filters or aggregates on it.

Initial fact tables implied by the current specs:

| Table | Source document | Grain |
|-------|-----------------|-------|
| `session_entries` *(existing)* | `session/...` chunks | one log entry |
| `lifecycle_events` | `session/...` chunks (typed subset) | one hook/lifecycle event |
| `presence_events` | `presence-log/...` chunks | one online/offline transition |
| `git_events` | `git/...` chunks | one commit/PR/CI observation |
| `capability_changes` | `capability/...` register writes | one capability diff |

### Dimension tables

Latest-state mirrors of the Kind-2 registers — replaced wholesale on projection, no
history semantics of their own:

| Table | Source | Row |
|-------|--------|-----|
| `hosts` | `capability/{node_id}` | one row per mesh host, current capabilities |
| `sessions` | `sessions-active/*` + session-end facts | one row per known session |
| `sequences` | `sequence/*` registers | one row per sequence |

Agents, models, and repos are **attributes on facts**, not registers — their dimension
tables (if reports want them) are `SELECT DISTINCT` views over the fact tables, not
independently maintained state.

### Metrics are views

Uptime-over-24h, per-agent usage trends, interactive-vs-non-interactive splits — all are
SQL views (or ad-hoc queries) over fact tables. Precompute and store a metric only when
a dashboard is measurably too slow, and then as a scheduled materialization that remains
derivable, never as a new source of truth.

### Write path

Row-by-row `INSERT OR IGNORE` is acceptable at current volume. When the agent-tail
monitor produces real throughput, switch the projection writer to DuckDB's Appender API
with periodic batch flushes — an optimization, not a schema change.

## Entity Mapping (logging-refactor)

Where each reporting entity from the logging-refactor spec lives:

| Entity | CRDT representation | DuckDB representation |
|--------|--------------------|-----------------------|
| Session | Kind-1 log chunks + row in `sessions-active` register | `session_entries` facts + `sessions` dimension |
| Sequence | Kind-2 `sequence/{node}/{id}` register (member sessions, status) | `sequences` dimension |
| Agent | attribute on facts | column on facts; `DISTINCT` view |
| Model | attribute on facts | column on facts; `DISTINCT` view |
| Repo | attribute on facts (canonical remote-URL form) | column on facts; `DISTINCT` view |
| Commit / PR / CI | Kind-1 `git/...` observations (external truth, transported not managed) | `git_events` facts |
| Project / kanban (v2) | Kind-1 observations from listener clients | future fact table |

The dashboard's "what is happening right now" question is answered from the Kind-2
registers (every daemon holds a synced replica of every host's `sessions-active`
register) plus local gRPC streaming — **not** from DuckDB. DuckDB answers "what
happened."

## Worked Answers to Open Spec Questions

These resolve the inline `QUESTION:` blocks in the host-capability-broadcast spec.

1. **Should `online` live in the capabilities document?** No. Its cadence violates the
   Kind-2 bounding rule, and liveness is an observation, not self-declared state (a
   dead host cannot write `online: false`). Use derived presence
   (`now - last_seen < threshold`) or the Loro ephemeral store; record transitions in
   `presence-log/...` if history is needed.

2. **Is there value in moving capabilities from redb to DuckDB?** Project, don't move.
   redb keeps the living `capability/{node_id}` register (authoritative); DuckDB
   mirrors it as the `hosts` dimension. If capabilities-over-time reporting is wanted,
   also emit a `capability_changes` fact on each register write.

3. **Is a 24-hour uptime metric easy from this structure?** Not from the register alone
   (a register only knows its latest value) — but trivially once presence transitions
   are Kind-1 facts. Uptime is then a windowed SQL aggregation over `presence_events`
   (sum of online intervals intersected with the window). This generalizes: every
   "metric over time" question is answered by *event-shaped facts in, window queries
   out* — never by storing computed metrics in CRDT state.

## Checklist for New Data

Before adding a document type or DuckDB table, answer:

1. **Does the mesh originate this data?** If an external system is authoritative,
   model observations (Kind 1), not state.
2. **Is it an event or a current value?** Event → Kind-1 fact log. Current value →
   Kind-2 register.
3. **Who is the single writer?** Name the `owner_node_id`. If the answer is "several
   nodes," stop and design-review a Kind-3 document.
4. **How is the document bounded?** Chunk rotation (Kind 1) or low write cadence /
   shallow-snapshot re-basing (Kind 2).
5. **What is the idempotency key** for its DuckDB projection?
6. **Is any part of it derived?** Derived data moves to a DuckDB view.
7. **Is any part of it high-cadence liveness?** That part moves to the ephemeral layer.

## Open Questions

- **Register history compaction:** ✅ answered by the
  [compaction spike](../../features/2026-07-11-host-capability-broadcast/spike-register-compaction.md)
  (2026-07-12). Growth is modest (~3–42 B/write depending on churn shape), re-basing via
  shallow snapshot preserves state/peer-id/persistence, and lazy thresholds
  (~256 KiB / 10k ops) suffice. One hard rule fell out: a reader behind the re-base
  point **silently** stops converging on delta sync, so the sync engine must gate
  updates-since requests on `shallow_since_vv` and respond with a snapshot-replace.
  **The gate is implemented** (2026-07-12, sync protocol v2): `PayloadKind::SnapshotReplace`
  + `SyncDelta.replace` on the wire, replica-swap semantics on receive
  (`stage_remote_replace` / `commit_staged_replace`), and a pending-ops import guard
  as defense-in-depth. It composes with foreign-writer enforcement (next bullet).
- **Foreign-writer enforcement:** ✅ implemented for registers (2026-07-12). A
  register's Loro peer id is derived deterministically from its owner's node id
  (`register::owner_peer_id`), and the import path rejects any staged update carrying
  ops from a different peer id — the op-level half of single-writer enforcement, on top
  of the sync layer's existing namespace check. Session-log *chunks* remain
  namespace-level only: existing chunk documents were created with random peer ids, so
  op-level binding there needs a migration story first (open).
- **Projection scheduling:** today the projection rebuilds at startup and appends on
  sync; a long-lived daemon may want periodic reconciliation (cheap `row_count` vs
  redb-entry-count comparison) to detect drift.
