---
implemented: true
design_for: dashboard-review-1.md (Findings 1, 2, 4; Design conclusions; Recommendation)
created: 2026-07-13
status: approved
decisions_locked: 2026-07-13
---
# Session-State Foundation — Design

**APPROVED for implementation (2026-07-13).** Execute the [phased plan](#phased-implementation-plan).

## Decisions locked (2026-07-13) — all four ⚠ confirms resolved as recommended

1. **Revision source = producer-captured unix-ns wall clock** (not a daemon Lamport
   counter). Section 2.
2. **Wire status as typed proto enums** (`SessionStatusState` / `StatusProducer` /
   `SessionStatusBasis`, each with `*_UNSPECIFIED = 0` → `Unknown`), not open strings.
   Section 3.
3. **Compute + store `permission_signal: supported|unsupported` at STARTED** from the
   `claudine hooks --support` matrix for the launched provider. Section 4.
4. **Split a distinct `ProducerId::PermissionHook` slot from `IdleHook`.** The permission-ask
   hook writes the `PermissionHook` slot; the idle/turn-complete hook writes the `IdleHook`
   slot. So `ProducerId = { Lifecycle, Sink, PermissionHook, IdleHook, ProcessMonitor }` and
   `StatusProducer` enum gains a `PERMISSION_HOOK` variant. Section 4's "if cleaner, split"
   is now the chosen path — do not have one slot double for both.

## Why this exists

`architecture review dashboard-review-1.md` requested a causal/atomic session-state
foundation **before** Trigger 2 adds a second status producer. Today every producer
writes one unversioned `status` string on the `sessions-active/{node}` register with a
non-atomic read-modify-write and arrival-order last-writer-wins. That is correct for a
single fire-and-forget producer and unsound the moment a second one appears. This design
closes four holes the review flagged as blocking or foundational:

1. **Atomicity** (Finding 1) — the transition's read and write do not share a lock, so
   UPDATE-after-END resurrects a session and END-with-a-stale-snapshot lost-updates a
   just-STARTED one.
2. **Ordering** (Finding 2) — detached UPDATE tasks can deliver `active` before
   `waiting_on_user`, leaving a live session stuck. Must survive *separate* hook
   processes, not just the current in-process sink.
3. **Precedence** (Finding 4) — a weaker `idle` must not clobber an unresolved stronger
   `waiting_on_user` merely because its hook arrived later. Trigger 1 should move to the
   hook/lifecycle boundary as primary (covering PTY), with the sink as fallback, carrying
   producer/basis/support metadata.
4. **Daemon-owned clock keys** — an UPDATED payload can currently overwrite
   `started_at_unix_ms`; the daemon must reserve the keys it owns.

The five components below are ordered by dependency: atomicity is the substrate,
the typed model + reducer is the logic, the wire carries it, the producers feed it, the
consumer reads it.

---

## 1. Atomic per-document transitions

### The two races (restated precisely)

Both live in `apply_session_event` (`daemon/src/service.rs:558-625`), which reads with
`deep_value` (locks `inner`, reads, **unlocks**) and later writes with
`upsert_local_fields` / `remove_local_fields` (locks `inner` again, writes, unlocks). The
`existing.is_none()` guard at `service.rs:591` sits *between* two independent critical
sections, so a concurrent transition can slip in:

- **UPDATE-after-END resurrection.** UPDATE reads entry A → END removes A → UPDATE writes
  the entry it read. A is resurrected despite the guard.
- **START/END lost-update.** END snapshots a register containing A → START-B upserts B →
  END's replace path writes back its stale snapshot (A removed) → B is deleted.

`remove_local_fields` (`register.rs:205-221`) has the same shape internally: `deep_value`
then `replace_local_fields`, two separate locks.

### Mechanism: a single-lock read-modify-write primitive on `RegisterStore`

Add one atomic primitive to `register.rs` that holds `inner` across the **entire**
read → transform → diff → persist sequence. All three session transitions
(STARTED / UPDATED / ENDED) become transformations of the `session_id → entry` map
computed inside that one critical section, so no concurrent transition can interleave
between the read and the write.

```rust
/// Atomically read-modify-write a locally-owned register under a single held
/// lock. `mutate` receives the register's current flat field map (already
/// inflated from Loro) and returns the COMPLETE desired field set plus an
/// arbitrary result `T`. The store diffs `desired` against the live document
/// and persists exactly as `replace_local_fields` does (delete-missing +
/// write-changed + compaction), all while `inner` is held — so the read the
/// closure saw and the write it produces are one indivisible operation.
///
/// The closure runs under the store mutex: it MUST be pure/synchronous and
/// MUST NOT call back into `RegisterStore` (parking_lot is not reentrant).
pub fn mutate_local_fields<F, T>(
    &self,
    doc_id: &DocumentId,
    mutate: F,
) -> Result<(bool, T), RegisterError>
where
    F: FnOnce(&serde_json::Map<String, JsonValue>)
        -> Result<(serde_json::Map<String, JsonValue>, T), RegisterError>;
```

Implementation shape (refactor, not rewrite):

- Split today's `write_local_fields` into the public `&self` entry points (unchanged
  signatures for capability/repos callers) and a private `write_locked(inner: &mut
  HashMap<String, LoroDoc>, doc_id, desired, delete_missing=true)` that assumes the lock
  is already held. `upsert_local_fields` / `replace_local_fields` / `remove_local_fields`
  become thin wrappers that lock then call `write_locked`.
- `mutate_local_fields` locks `inner` once, reads the current field map via the same
  `get_map(FIELDS_CONTAINER).get_deep_value()` path `deep_value` uses, hands it to the
  closure, and calls `write_locked(..., delete_missing = true)` with the returned map. The
  `bool` reports whether anything actually changed (drives the write-on-change budget).

Holding a `parking_lot::Mutex` across a synchronous serde/Loro closure is sound: no
`.await` occurs inside, and `report_session_event` already runs the whole thing in
`spawn_blocking` (`service.rs:513`). The critical section is a few small-map operations
plus one Loro export + redb save — the existing write path's cost, now covering the read
too.

### How `apply_session_event` uses it

`apply_session_event` collapses to one `mutate_local_fields` call. The closure holds the
full transition logic; because it sees the authoritative current map, both races vanish:

```rust
let (_changed, active_count) = registers.mutate_local_fields(&doc, |current| {
    // `current` is session_id -> JSON-string entry (flat scalars).
    let mut next = current.clone();
    match kind {
        Kind::Ended => { next.remove(session_id); }               // atomic remove
        Kind::Started => { /* insert/merge, run reducer, stamp clocks */ }
        Kind::Updated => {
            if !next.contains_key(session_id) {
                // Atomic no-create: the absence check and the (non-)write are
                // in the SAME critical section as any concurrent END/START.
            } else { /* merge, run reducer, stamp clocks */ }
        }
        Kind::Unspecified => unreachable!(),
    }
    let count = next.len() as u64;
    Ok((next, count))
})?;
```

- UPDATE-after-END: END's `remove` and UPDATE's `contains_key` cannot straddle a lock
  boundary; whichever runs second sees the other's result. No resurrection.
- START/END lost-update: END recomputes `current − {A}` from the live map; if START-B ran
  first, END's `current` already contains B and preserves it; if START-B runs after, its
  own atomic mutate re-reads and re-adds B. No stale-snapshot overwrite.

**Trade-off considered.** A dedicated per-document actor (an `mpsc` queue drained by one
task owning the doc) would also serialize transitions and could batch. It is more moving
parts, introduces an async hop on a best-effort path, and buys nothing over the mutex
here: transitions are low-cadence and the critical section is short. **Recommend the
single-lock primitive** — it matches the store's existing `Arc<Mutex<HashMap>>` model and
is the smallest correct change.

---

## 2. Causal ordering / revision model

Atomicity alone does **not** fix Finding 2: two well-formed UPDATEs that *arrive*
swapped (`active` applied, then a late `waiting_on_user`) still land the session in the
wrong terminal state. Ordering needs an explicit revision the daemon can reject when
stale.

### Two distinct ordering problems — do not conflate them

- **A. Same-producer reordering** (exactly Finding 2): the sink emits `waiting_on_user`
  then `active` from the *same* logical producer, but the two detached tasks race on the
  wire. Fixed by a **monotonic per-producer revision** + daemon rejection of stale ones.
- **B. Cross-producer conflict** (Finding 4): the sink says `waiting_on_user`, the idle
  hook says `idle`. Different producers, independent revision spaces — revisions are
  *incomparable* across producers. Fixed by **precedence** (Section 3), not by revision.

Keeping these separate is what lets a simple scalar revision work: revision only ever
orders one producer's own opinions.

### The revision is a host-local wall-clock timestamp captured by the producer

`revision = SystemTime::now()` in **unix nanoseconds**, captured by the producer at the
moment the event is observed (sink) or the hook fires (`claudine handle`). The daemon
stores, per session per producer slot, the revision it last applied, and **rejects any
contribution whose `revision ≤` the stored slot revision** (the write for that slot is a
no-op; other fields of the same RPC still apply).

Why wall-clock ns rather than a counter:

- **It survives separate processes.** Trigger 2's producer is a fresh `claudine handle`
  process per hook invocation — it cannot carry an in-process counter across invocations.
  Every producer for a given session runs on the **same host that owns the register**
  (the session's origin), so they share one system clock. A later event has a later
  timestamp regardless of which process emits it.
- **Same-producer strict monotonicity holds where it matters.** The sink captures
  `now()` for `waiting_on_user` then `active` in program order on one thread → strictly
  increasing ns. The reordered late `waiting_on_user` carries the *lower* revision and is
  rejected; `active` wins. The idle hook's two events (`turn_complete`, then
  `before_prompt`) are seconds apart (human in loop) — no tie risk.

⚠ **confirm — revision source.** Recommended: producer-captured unix-ns wall clock,
accepting that a backward NTP step (rare, sub-second, same host) could momentarily
mis-order two same-producer events on a best-effort telemetry field. The alternative is a
daemon-assigned Lamport counter per (session, producer) — but the daemon only sees
*arrival* order, which is the very thing being reordered, so it cannot repair intent
order. A third option (monotonic `Instant` shared via the daemon) needs an extra RPC per
event. **I recommend wall-clock ns**; flagging because it is the one place a subtle clock
assumption is baked in.

### Why not the simpler in-process serialized writer

An in-process bounded channel drained by one task (awaiting each RPC in submission order)
would guarantee the *sink's* ordering. Rejected as the foundation because:

1. **It orders one process only.** The Trigger-2 hook is a separate process; its ordering
   relative to the sink is still uncoordinated. The review says this explicitly: "a single
   in-process queue fixes only the current sink producer."
2. **It does nothing for precedence** (Problem B) — `idle` vs `waiting_on_user` is a
   cross-producer decision no in-process queue can make.
3. It adds a long-lived task + channel + shutdown-drain concern to the wrapper for a
   best-effort path.

The daemon-side revision guard is producer-count-agnostic: sink, idle hook, and the
future process monitor are all ordered by the same rule with zero shared state between
them. An in-process ordered submitter for the sink MAY still be added later purely to
avoid wasted rejected RPCs, but it is an optimization, not a correctness mechanism.

---

## 3. Typed status model with explicit precedence

Replace the open `status` string with a typed model that carries **producer identity**,
**basis**, and **strength**, so precedence is defined and a weaker re-report cannot hide a
stronger unresolved one.

### Model (new module `rendezvous/core/src/session_status.rs`)

```rust
/// The displayable state of a session, ordered by intervention strength.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Active,          // running, no intervention
    Idle,            // interactive turn complete, waiting on user (weak)
    WaitingOnUser,   // permission ask / blocked, unresolved (strong)
    Unknown,         // forward-compat: an unrecognized wire value
}

impl SessionState {
    /// Precedence rank. Higher wins in the reducer.
    pub fn strength(self) -> u8 {
        match self {
            SessionState::WaitingOnUser => 30,
            SessionState::Idle          => 20,
            SessionState::Active         => 10,
            SessionState::Unknown        => 0,
        }
    }
}

/// Which producer authored a contribution — its slot key in the entry.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProducerId { Lifecycle, Sink, IdleHook, ProcessMonitor }

/// Why the producer believes the state — surfaced to the UI for honesty.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Basis { PermissionAsk, InteractiveTurnComplete, ProcessHeuristic, Lifecycle }

/// One producer's current opinion about a session. Exactly ONE slot per
/// producer lives in an entry; a producer's newer opinion overwrites its own
/// slot (revision-guarded), never another producer's.
pub struct StatusContribution {
    pub state: SessionState,
    pub producer: ProducerId,
    pub basis: Basis,
    pub revision: i64,   // producer unix-ns; daemon LWW-guards per slot
}
```

### The reducer

```rust
/// Fold every producer's live contribution into the effective displayed
/// state. Highest strength wins; ties break by newest revision. Empty (no
/// producer has spoken beyond Lifecycle's initial Active) folds to Active.
pub fn effective(slots: &BTreeMap<ProducerId, StatusContribution>) -> EffectiveStatus {
    slots.values()
        .max_by_key(|c| (c.state.strength(), c.revision))
        .map(|c| EffectiveStatus { state: c.state, basis: c.basis, producer: c.producer })
        .unwrap_or(EffectiveStatus::active())
}
```

Retraction is implicit: a producer clears its own signal by writing a *lower-strength*
state into its slot (e.g. the sink writing `Active` after `WaitingOnUser`, or the idle
hook writing `Active` on `before_prompt`). Because each producer owns exactly one slot,
"clearing" is just its latest opinion — there is no separate delete protocol, and a
producer can never clear another's slot. ENDED removes the whole entry (all slots).

**Precedence in action.** Sink slot = `WaitingOnUser`(30), idle-hook slot = `Idle`(20) →
effective `WaitingOnUser`. The idle hook arriving *after* the permission ask cannot lower
the displayed state, which is the exact Finding-4 requirement. When the sink later clears
to `Active`(10), effective drops to `Idle` — correct: the permission was answered but the
turn is still idle.

### Wire representation (daemon-internal storage)

Register fields are flat scalars, but each session's entry is already stored as a
JSON-**string** under its `session_id` key (`service.rs:606-611`), so the daemon may nest
freely inside that string. The entry gains a `status_slots` object and daemon-projected
effective fields:

```jsonc
// value stored under sessions_json["<session_id>"] (a JSON string in the register)
{
  // descriptive (from STARTED details_json)
  "agent": "claude", "model": "opus", "interactive": true,
  "repo_root": "/…", "claudine_pid": 4242,
  "permission_signal": "supported",          // see Section 4

  // daemon-owned clocks (Section 5)
  "started_at_unix_ms": 1_752_…, "updated_at_unix_ms": 1_752_…,

  // per-producer opinions (one slot each)
  "status_slots": {
    "sink":      { "state": "waiting_on_user", "basis": "permission_ask",            "revision": 1752…001 },
    "idle_hook": { "state": "idle",            "basis": "interactive_turn_complete",  "revision": 1752…000 }
  },

  // daemon-projected reducer output (what plain consumers read)
  "status": "waiting_on_user",
  "status_basis": "permission_ask",
  "status_producer": "sink"
}
```

The daemon recomputes `status` / `status_basis` / `status_producer` from `status_slots`
via `effective()` on **every** mutation, inside the same atomic closure from Section 1.

### How the consumer reads it

- **Backward-compatible path unchanged.** The dashboard's `parse_sessions`
  (`dashboard/model.rs:247-275`) keeps reading the flat `status` field; the daemon
  projects the effective state there, so existing rendering (and Trigger 2's planned
  `idle → PossiblyIdle` mapping) works with no consumer change.
- **Richer path (optional, Section 4).** A consumer that wants to render basis or the
  "supported / no-intervention" vs "signal-unavailable" distinction reads `status_basis`,
  `status_producer`, and `permission_signal` instead of inferring from the bare string.

### Wire representation (producer → daemon RPC)

Promote the ordering/producer metadata to typed proto fields so the daemon enforces
without parsing free-form JSON. `details_json` reverts to *descriptive attributes only*;
the status opinion moves to a typed optional sub-message:

```proto
message ReportSessionEventRequest {
    string session_id = 1;
    SessionEventKind kind = 2;
    string details_json = 3;          // descriptive attrs ONLY (agent/model/repo/…)
    StatusContribution status = 4;    // present when this event carries a status opinion
}

message StatusContribution {
    SessionStatusState state = 1;     // enum (ACTIVE / IDLE / WAITING_ON_USER / …)
    StatusProducer      producer = 2; // enum (LIFECYCLE / SINK / IDLE_HOOK / PROCESS_MONITOR)
    SessionStatusBasis  basis = 3;    // enum
    int64               revision = 4; // producer unix-ns; daemon LWW-guards per (session, producer)
}

enum SessionStatusState  { SESSION_STATUS_STATE_UNSPECIFIED = 0; ACTIVE = 1; IDLE = 2; WAITING_ON_USER = 3; }
enum StatusProducer      { STATUS_PRODUCER_UNSPECIFIED = 0; LIFECYCLE = 1; SINK = 2; IDLE_HOOK = 3; PROCESS_MONITOR = 4; }
enum SessionStatusBasis  { SESSION_STATUS_BASIS_UNSPECIFIED = 0; LIFECYCLE = 1; PERMISSION_ASK = 2; INTERACTIVE_TURN_COMPLETE = 3; PROCESS_HEURISTIC = 4; }
```

STARTED carries descriptive `details_json` **and** an initial `status` contribution
`{state: ACTIVE, producer: LIFECYCLE, basis: LIFECYCLE, revision: now}`. This keeps the
`status:"active"` behavior the current STARTED sets (`session_report.rs:66`) but routes it
through the typed slot so the reducer owns it.

⚠ **confirm — enum vs open string on the wire.** Recommended: typed enums with an
`UNSPECIFIED = 0` / `Unknown` fallback, because the review asks for an *explicit*
precedence model and enums make `strength()` total and compile-checked. Cost: adding a new
state (e.g. a future `Blocked`) is a proto change rather than a new string literal. Given
only 3-4 states are foreseeable and precedence must be authoritative, **recommend enums**.
(`blocked`, which today maps to needs-input in `is_waiting`, folds into `WaitingOnUser`.)

---

## 4. Trigger-1 producer boundary

### Move the primary producer to the hook/lifecycle boundary

Trigger 1 lives today in `LiveSemanticSink::on_semantic_event`
(`live_semantic_sink/event_sink.rs:114-133`), which exists only on the **structured** and
**harness** paths. Interactive PTY sessions run `run_child` with no sink, so the
highest-value permission signal is silently missing exactly where a human is watching. The
review's direction (Finding 4, Design conclusions): the **normalized hook/lifecycle
boundary should be primary**, with the sink as fallback.

Recommendation — two producers, one reducer:

- **Primary — `ProducerId::Lifecycle` / `IdleHook` in `claudine handle`
  (`cli/src/commands/handle.rs`).** The same hook path Trigger 2 already targets. When a
  provider fires a permission-ask-class hook, `handle` reports
  `{state: WaitingOnUser, producer: IdleHook, basis: PermissionAsk, revision: now}`; the
  clearing hook reports `Active`. This covers PTY **and** structured launches because
  hooks fire out-of-band regardless of execution path. (Producer slot `IdleHook` doubles
  for both idle and permission hook signals since both originate from `handle`; the
  `basis` distinguishes them. If cleaner, split into a distinct `ProducerId::PermissionHook`
  slot — ⚠ confirm, minor.)
- **Fallback — `ProducerId::Sink`.** Keep the existing sink `waiting_on_user`/`active`
  reports for providers whose structured protocol exposes a permission request but whose
  hooks do not. Now stamped with `producer: SINK` and a `revision`.

Because each producer owns its own slot and the reducer takes the max strength, the two
compose safely:

- Both report `WaitingOnUser` (strength 30 each) → effective `WaitingOnUser`; redundant,
  never conflicting.
- On the interactive path the sink never runs, so it contributes **no slot** — there is no
  stuck sink `WaitingOnUser` to clear. The hook is the sole author; its `Active` clear
  fully resolves the state.
- Trigger 2's `IdleHook` `Idle` composes under the same reducer: `Idle`(20) is dominated
  by any live `WaitingOnUser`(30) from either producer, satisfying Finding 4's
  "weaker-idle-must-not-clobber-stronger-waiting."

### Producer/basis/support metadata

Two consumer-visible distinctions the review demands ("no signal" ≠ "no intervention"):

- **Which producer/basis** produced the current state — carried by `status_producer` /
  `status_basis` (Section 3), always present.
- **Whether the signal is even available for this provider.** At STARTED, the wrapper
  records `permission_signal: "supported" | "unsupported"` into descriptive
  `details_json`, computed from the `claudine hooks --support` matrix for the launched
  provider (spec S1). The consumer then renders:
  - live `WaitingOnUser` → "needs input" (strong),
  - `permission_signal:"supported"` + no waiting slot → "supported, no intervention
    needed,"
  - `permission_signal:"unsupported"` → "permission signal unavailable for agent X."

⚠ **confirm — compute+store `permission_signal` at STARTED.** Recommended; it is the only
place the launched provider's support is cheaply known, and it makes the UI honest per S1
without a second lookup path. Cost: one extra descriptive field per entry.

---

## 5. Daemon-owned clock keys

The daemon owns `started_at_unix_ms` and `updated_at_unix_ms` (and now the projected
`status` / `status_basis` / `status_producer` / `status_slots`, which the reducer writes).
Producer `details_json` must not set any of them. The current merge (`service.rs:594-604`)
inserts producer details *first*, so a payload carrying `started_at_unix_ms` slips in and
the `!entry.contains_key(...)` guard then preserves the producer's value — the bug the
review names.

Fix: a reserved-key denylist stripped from `details_json` **before** merge, inside the
atomic closure:

```rust
const DAEMON_OWNED_KEYS: &[&str] = &[
    "started_at_unix_ms", "updated_at_unix_ms",
    "status", "status_basis", "status_producer", "status_slots",
];
// after parsing details_json, before merging into the entry:
for k in DAEMON_OWNED_KEYS { details.remove(*k); }
```

The daemon then sets clocks authoritatively: `started_at_unix_ms` on STARTED only (never
mutated afterward), `updated_at_unix_ms` on every accepted mutation. Status fields are
written solely by the reducer. Add a test asserting a STARTED/UPDATED payload that tries
to set `started_at_unix_ms` / `status` is ignored.

---

## 6. Crashed-producer note (Finding 3 context)

This foundation **does not** solve phantom sessions from a SIGKILLed producer that never
reports ENDED — that needs process-monitor reconciliation keyed on
`(claudine_pid, process-start-time)` identity, which is out of scope here (spec S4). The
requirement is only that this design **not make it worse**, and it does not:

- It changes no liveness assumption — entries still persist until an explicit ENDED, and
  `updated_at_unix_ms` remains the honest "last reported" clock a future reconciler reads.
- It *helps* the eventual reconciler: `claudine_pid` is already carried, and the new
  `status_producer` / `status_slots` record which producer last spoke, so the process
  monitor can attribute and expire a stale entry (emit ENDED, or a
  `ProcessMonitor`-authored terminal slot) with provenance.
- Revisions are per-(session, producer); a crashed producer simply stops advancing its
  slot — it cannot wedge another producer or the reducer.

No presented-as-live guarantee is claimed here; the v1 UI honesty fix (label "last
reported" + age) remains the review's separate ask.

---

## Phased implementation plan

Each phase is independently testable and leaves the tree green. Tests use **nextest**
(`just test` / `just sanity` in `rendezvous` and `claudine-cli`), never `cargo test`;
never run `cargo fmt`. Concurrency tests use explicit barriers (`std::sync::Barrier` /
`tokio::sync::Barrier`) to force the harmful interleavings deterministically.

### Phase 1 — Atomic mutation primitive
**Goal (success):** `RegisterStore::mutate_local_fields` exists; `apply_session_event`
routes all three transitions through one atomic closure; barrier-driven concurrent
UPDATE/END and START/END tests prove no resurrection and no lost-update, repeatably.
**Files:** `rendezvous/daemon/src/register.rs` (split `write_local_fields` into locked
inner + wrappers; add `mutate_local_fields`), `rendezvous/daemon/src/service.rs`
(`apply_session_event` → single `mutate_local_fields` call; drop the separate
`deep_value`/`remove_local_fields` sequence).
**Tests (`register.rs`, `service.rs`):**
- barrier: START-A committed, then UPDATE-A and END-A released simultaneously ×100 → A
  never present afterward (no resurrection).
- barrier: START-A committed, then END-A and START-B released simultaneously ×100 → B
  always present, A always absent (no lost-update).
- `mutate_local_fields` returns `changed=false` and touches nothing when the closure
  returns an unchanged map (write-on-change budget preserved).
- existing `updated_on_missing_session_does_not_resurrect_it` still passes.

### Phase 2 — Typed status model + reducer (pure `rendezvous_core`)
**Goal:** `session_status` module with `SessionState::strength`, `StatusContribution`,
`ProducerId`, `Basis`, and `effective()`; precedence + per-slot LWW + retract are unit
proven with no daemon.
**Files:** new `rendezvous/core/src/session_status.rs`; `rendezvous/core/src/lib.rs`
(`pub mod` + re-exports); serde (de)serialization for the `status_slots` JSON shape.
**Tests (`session_status.rs`):**
- `waiting_on_user`(sink) + `idle`(hook) → effective `waiting_on_user`; then sink→`active`
  → effective `idle`; then hook→`active` → effective `active`.
- stale revision rejected: applying `waiting_on_user`@rev1 after `active`@rev2 (rev2>rev1)
  to the same slot leaves the slot `active`.
- empty slots fold to `active`; `Unknown` wire value folds to strength 0.

### Phase 3 — Wire + daemon enforcement
**Goal:** proto carries `StatusContribution`/`revision`/producer enum; the daemon applies
the reducer, enforces per-slot revision rejection, and strips daemon-owned keys — all
inside the Phase-1 atomic closure. Back-to-back reorder and cross-producer precedence
resolve correctly against a live in-process daemon.
**Files:** `rendezvous/core/proto/rendezvous.proto` (new message + enums; `details_json`
narrowed to descriptive); `rendezvous/core/src/lib.rs` (re-export generated types);
`rendezvous/daemon/src/service.rs` (`apply_session_event`: parse typed status, strip
`DAEMON_OWNED_KEYS`, revision-guard the slot, run `effective()`, project status fields).
**Tests (`service.rs`):**
- reorder: apply `waiting_on_user`@rev1 then `active`@rev2, then re-deliver
  `waiting_on_user`@rev1 (simulating the late detached task) → final effective `active`.
  Repeat ×50 to prove it is causal, not lucky.
- precedence: idle-hook `idle`@revN after sink `waiting_on_user`@revM (revN>revM) →
  effective still `waiting_on_user`.
- reserved keys: STARTED/UPDATED payload setting `started_at_unix_ms` and `status` in
  `details_json` → both ignored; daemon clocks/status authoritative.
- STARTED projects `status:"active"` from the `LIFECYCLE` slot (backward compat).

### Phase 4 — Producer migration (Trigger 1 → hook/lifecycle boundary + sink fallback)
**Goal:** interactive PTY sessions report `waiting_on_user` via `claudine handle`; the
sink remains a stamped fallback; both feed the reducer; a permission ask on an interactive
session now surfaces in `list_active_sessions`.
**Files:** `cli/src/commands/handle.rs` (permission-hook → `WaitingOnUser`, clearing hook
→ `Active`, `producer=IdleHook/PermissionHook`, `revision=now`); `cli/src/commands/wrap/
session_report.rs` (`StatusReporter::report` stamps `producer=SINK` + `revision`; STARTED
sends the `LIFECYCLE` `active` slot + `permission_signal`); `live_semantic_sink/
event_sink.rs` (unchanged logic; the reporter it calls now carries producer/revision).
**Tests:**
- `handle` driven with a permission hook + interactive env against a live daemon →
  register shows effective `waiting_on_user`; clearing hook → `active`.
- sink + hook both report `waiting_on_user` for one session → single effective
  `waiting_on_user`, two slots recorded.
- sink duplicate permission requests debounce to one edge (existing `awaiting_user` guard);
  each intended progress class clears; unrelated events do not clear.
- `permission_signal` recorded at STARTED for a supported vs unsupported provider.

### Phase 5 — Consumer honesty (basis / support rendering)
**Goal:** `claudine dashboard` reads the projected `status` as before, and additionally
renders basis and the "supported/no-intervention" vs "signal-unavailable" distinction from
`status_basis` / `permission_signal`.
**Files:** `cli/src/commands/dashboard/model.rs` (thread `status_basis`, `status_producer`,
`permission_signal` into `SessionRow`); `cli/src/commands/dashboard/report.rs` (render the
distinction); `cli/src/commands/dashboard/tests.rs`.
**Tests (`dashboard/tests.rs`):** synthetic entry with `permission_signal:"unsupported"`
renders "unavailable," not "no intervention"; `waiting_on_user` still maps to
`NeedsInput`; a fresh entry with support+no-waiting renders "no intervention needed."

### Phase 6 — Drift + verification sweep
**Goal:** every doc/artifact describing session status is current, and the full suite is
green cross-platform.
**Files/actions:** update `spec.md` D5 wording (status is now typed slots + reducer, not a
bare string); update the `claudine` skill `cli-reference.md` dashboard section and refresh
`md hash`; note in `trigger2-plan.md` that its D2 ("reuse `status` string") is superseded
by the typed model (Trigger 2's `idle` becomes an `IdleHook` `Idle` contribution — the
plan's Phase 2/3/4 land on this foundation unchanged in spirit). Regenerate dispatch
inventory if `handle.rs` line offsets shift (`CLAUDINE_UPDATE_INVENTORY=1`).
**Verification:** `just test` + `just lint` green for `rendezvous` and `claudine-cli`;
`just test-l2` (claudine) unaffected; the barrier and reorder tests are the acceptance gate
the review's test-gap section demands.

---

## Decision summary for Ken

Recommendations are decisive; the ⚠ items are the only genuine forks.

- **Atomicity:** single-lock `mutate_local_fields` read-modify-write in `RegisterStore`
  (over a per-document actor). No confirm needed.
- **Ordering:** daemon-side per-(session, producer) revision guard, revision =
  producer-captured unix-ns wall clock. Chosen over an in-process queue because it orders
  *all* producers (sink, hooks, process monitor) uniformly across separate processes.
  ⚠ confirm the wall-clock-ns revision source (vs a Lamport counter, which cannot repair
  arrival reordering).
- **Precedence:** typed `SessionState` with a total `strength()` ranking
  (`waiting_on_user` > `idle` > `active`); one slot per producer; `effective()` reducer
  runs daemon-side and projects a backward-compatible `status`. ⚠ confirm typed enums on
  the wire (vs open strings).
- **Trigger 1 boundary:** primary producer moves to `claudine handle`
  (covers PTY + structured), sink demoted to fallback, both feeding one reducer. ⚠ confirm
  storing `permission_signal` support at STARTED; ⚠ (minor) whether to split a distinct
  `PermissionHook` producer slot from `IdleHook`.
- **Clock keys:** `DAEMON_OWNED_KEYS` denylist stripped before merge. No confirm needed.
- **Crashed producers:** explicitly out of scope; foundation carries `claudine_pid` +
  `status_producer` so a later process-monitor reconciler can attribute and expire — no
  regression.

Six implementation phases; Phase 1 (atomicity) and Phase 3 (reorder/precedence
enforcement) carry the review's blocking acceptance tests.
