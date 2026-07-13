---
implemented: false
reviewed_by: codex/gpt-5.6-high
created: 2026-07-13
---
# Architecture Review — Rendezvous Dashboard v1

## Verdict

**Request changes before building Trigger 2.** The overall layering is sound, and I agree
with keeping `DashboardReport` CLI-local, using successful sync rather than mDNS as the
host-freshness clock, and making `UPDATED` non-creating at the RPC contract level. Three
correctness properties are not actually established, though:

1. session transitions are not atomic, so the UPDATE/ENDED resurrection race still exists;
2. detached status reports can reorder `waiting_on_user` and `active` while a session remains
   live; and
3. a healthy host can advertise a crashed producer's session as live forever.

Those are blockers because Trigger 2 adds another status producer and more transitions to the
same last-writer-wins field. The current design needs a causal/atomic session-state foundation
before that producer is added.

I did not rerun the test suites, as requested. I reviewed `c17c19943^..f16dc7056`, excluding
`9106bc4e6`, and used `10253e846` as the producer-presence baseline. GitNexus's existing index
resolved the pre-feature symbols and showed `apply_session_event` has one direct production
caller (`report_session_event`, LOW blast radius), while `PeerRegistry::spawn_with` reaches the
daemon bootstrap and a broad integration-test surface (HIGH blast radius). The index was 12
commits behind; its prescribed refresh failed locally because the LadybugDB FTS extension was
unavailable, so findings about newly added symbols below are grounded in the exact source/diff
rather than treated as graph-proven.

## Correctness risks

### 1. BLOCKER — `UPDATED` can still resurrect an ended session because the merge-only check is not atomic

`apply_session_event` reads the register entry and later calls `upsert_local_fields`
(`daemon/src/service.rs:573-611`). `ENDED` similarly reaches `remove_local_fields`, which reads
the whole register and later replaces it (`daemon/src/register.rs:205-220`). Each individual
store operation locks `RegisterStore::inner`, but the transition's read/modify/write sequence
does not hold one lock.

Concrete failure:

1. Session A exists. UPDATE-A reads A and builds its merged entry.
2. END-A removes A.
3. UPDATE-A upserts the entry it read in step 1.
4. A is resurrected, despite the new `existing.is_none()` guard.

There is a second, broader lost-update race:

1. END-A snapshots a register containing A.
2. START-B upserts B.
3. END-A calls the replace/delete-missing path using its stale snapshot.
4. B is deleted even though it just started.

The sequential `updated_on_missing_session_does_not_resurrect_it` test proves only the
non-concurrent case. The invariant belongs in the daemon, but it needs to be implemented as one
atomic per-document mutation (or behind a dedicated session-register serialization primitive),
not as a service-layer check followed by separate store calls. Add deterministic barrier-driven
tests for UPDATE/END and START/END interleavings.

### 2. BLOCKER — detached UPDATE tasks do not preserve `waiting_on_user -> active` order

Every `StatusReporter::report` call spawns a separate task, opens a separate connection, and
sends independently (`cli/src/commands/wrap/session_report.rs:126-159`). The sink can observe a
permission request and immediate progress back-to-back, spawning `waiting_on_user` followed by
`active`, but the daemon can receive `active` first and `waiting_on_user` second.

Concrete failure:

1. `PermissionRequest` sets the sink's `awaiting_user = true` and spawns the waiting update.
2. An immediate `OutputText`, `ToolResult`, or `TurnComplete` sets `awaiting_user = false` and
   spawns the active update.
3. The active RPC completes first; the waiting RPC completes second.
4. The live session remains `waiting_on_user`. Because the sink already believes
   `awaiting_user == false`, later progress does not send another clear.

Merge-only UPDATE semantics protect against a late UPDATE after END only once finding 1 is
fixed; they do nothing for UPDATE-vs-UPDATE reordering. The current test deliberately waits for
the waiting state before sending active, so it cannot expose this race.

Use a bounded, per-session serialized writer or attach a monotonic session generation/revision
that the daemon rejects when stale. Trigger 2's separate hook processes make the daemon-side
revision/precedence model the more durable direction; a single in-process queue fixes only the
current sink producer.

Normal traffic is bounded somewhat by edge debounce and the 250ms timeout, so detached-task
volume is not my primary concern. A hostile alternating semantic stream can still create many
short-lived tasks because there is no backpressure. Runtime shutdown may drop detached updates,
which is acceptable for best-effort telemetry; ENDED is intended to dominate them, but only
after atomicity and ordering are made real.

### 3. BLOCKER — host freshness is not session liveness; crashed producers leave trusted phantom sessions forever

The register carries `updated_at_unix_ms`, and the feature spec says consumers judge stuck
entries from it until process-monitor reconciliation lands. The dashboard does not parse or use
that field (`dashboard/model.rs:247-275`). It trusts every local entry unconditionally and every
remote entry whenever that host's *replication* sync is fresh.

Concrete failure:

1. A wrapper reports STARTED and later reaches `waiting_on_user`.
2. The wrapper is killed with SIGKILL or crashes, so `SessionPresence::drop` never reports END.
3. Both rendezvous daemons stay healthy and continue syncing every 20 seconds.
4. The host remains `Fresh`, and the phantom session remains a trusted needs-input session
   indefinitely.

This is not solvable honestly by applying a simple session TTL: reports are transition-driven,
not heartbeats, so a legitimate long-running quiet session may also have an old `updated_at`.
Either process-monitor reconciliation (including PID/start-time identity) must become a launch
prerequisite, or the v1 UI must call these entries “last reported” and expose their age rather
than claiming they are live. At minimum, the current spec sentence about consumer-side hygiene
must not remain stamped as implemented.

### 4. HIGH — Trigger 1 misses the sessions where permission prompts matter most

Trigger 1 is implemented in `LiveSemanticSink`, but direct interactive PTY sessions use
`run_child` and have no semantic sink. The result is a silent false negative for interactive
wrapped sessions, while the dashboard and D5 wording imply wrapped-session coverage generally.
The existing provider-support uncertainty compounds this: absence of a signal currently means
either “no intervention needed” or “this execution path/provider cannot report it.”

I disagree with treating the sink as the primary architectural home for Trigger 1. The
normalized hook/lifecycle boundary should be the primary producer for providers that expose the
event, because it covers PTY and structured launches. A sink fallback can remain for providers
whose structured protocol exposes a permission request but whose hooks do not. The session
entry/UI should retain producer/basis/support metadata so lack of coverage is not rendered as a
negative observation.

Do this design pass before Trigger 2. Otherwise Trigger 2 creates a second hook-based producer
while Trigger 1 remains sink-based, and both write the same unversioned `status` field with no
defined precedence. “Last writer wins” is not sufficient: a weaker `idle` report must not
overwrite a stronger unresolved `waiting_on_user` report merely because its hook arrived later.

### 5. HIGH (Trigger 2 blocker) — remote idle duration is not clock-skew-free

`trigger2-plan.md` says `updated_at_unix_ms` is daemon-stamped and therefore can be compared with
the consumer's local clock using the same skew-free arrangement as host staleness. That is true
only for the local host. For a remote session, `updated_at_unix_ms` was stamped by the *remote*
daemon and is compared by the dashboard host, while `last_synced_unix_ms` is stamped locally.

Concrete failure: if the remote clock is five minutes slow, a newly idle session appears idle
for five minutes immediately; if it is five minutes fast, its idle age clamps to zero until the
consumer clock catches up. Do not implement remote idle duration from origin wall time without
clock-offset/provenance handling. Safer v1 options are to omit duration for remote sessions or
derive a conservative local observation age from per-document sync metadata.

### 6. HIGH — one slow peer can make unrelated healthy peers stale, and connection loss never recovers

`resync_connected_peers` iterates a `HashMap` snapshot sequentially and awaits a complete sync
for each peer (`daemon/src/peers.rs:352-380`). A sync session has a 30-second overall timeout
(`daemon/src/sync.rs:40-43, 767-769`), longer than the 20-second ticker interval. Therefore the
stated “at least two syncs inside 60 seconds” property is not guaranteed.

Concrete failure: the first two peers in one tick accept QUIC but stop responding. They consume
up to 60 seconds before the third, healthy peer is attempted, so that healthy peer crosses the
dashboard's stale threshold. `HashMap` iteration order makes which peer is starved unstable.

On connection loss, sync errors only update `last_error`; the record remains `Connected` with
the same dead `quinn::Connection`. mDNS rediscovery updates the address/last-seen time but does
not reconnect. The worker thus reports staleness honestly, but does not recover when the peer
returns unless another explicit connect path runs.

The nominal 20s/60s ratio is reasonable for a small mesh only after each peer has an independent,
bounded state machine. Run peers concurrently with a cap, add jitter/backoff, transition closed
connections out of Connected, and define reconnection ownership. Test one wedged peer alongside
one healthy peer and prove the healthy clock remains inside the threshold.

The cost also needs measurement before calling the cadence stable. Every periodic round
advertises all known session-log chunks and registers, even when nothing changed, and both ends
run the worker. The idle cost is roughly O(peers x documents / 20s) per daemon and approaches
O(hosts^2 x documents) for a full mesh. That may be fine for the intended small mesh, but it is
not a heartbeat-sized operation and should have an explicit bandwidth/CPU budget.

### 7. MEDIUM — host freshness is attributed by direct peer identity, not by the register's actual sync path

The fold uses register owners to construct host rows, then looks up each owner in `ListPeers`.
This creates two topology errors:

- A peer present in `ListPeers` but with no capability/repos/session document is omitted entirely,
  so D4's “every remote host row” guarantee is not met.
- A register for C received transitively through B is always `NeverSynced` unless this daemon also
  has a direct peer record for C, even if the C replica arrived in the immediately preceding sync
  with B.

The second behavior is conservative—unknown is safer than falsely fresh—but “never synced” is
not accurate, and a partial mesh can never produce a useful whole-mesh dashboard for transitive
hosts. Include peer IDs in the host-union fold and track freshness per replicated document/owner
(including observation provenance), rather than inferring document freshness from a same-named
direct peer. Until then, label the transitive case “freshness unknown / no direct peer.”

### 8. MEDIUM — stale last-known rows are still counted and presented as active sessions

For stale hosts, only `Status` becomes `unknown` and intervention is suppressed. Session ID,
agent, model, and repo remain visible, and `MeshSnapshot::total_sessions` includes those rows in
the headline (`dashboard/model.rs:195-197`; `dashboard/report.rs:50-57, 89-108`).

Concrete failure: a remote session ended two hours ago after the link failed. The dashboard
headline still says “1 session” and shows its agent/repo row, even while the host heading says
“sessions unknown.” That is a last-known observation, not a live session count.

Either suppress stale session rows, or separate `trusted live` from `last known` counts and label
the latter explicitly. The 60,000ms boundary itself is coherent with the ratified wording:
exactly 60 seconds is fresh and anything greater is stale. The issue is presentation and
attribution, not an off-by-one.

### 9. MEDIUM — the client transport is portable, but the daemon side is still Unix-only

`rendezvous_client::connect` correctly selects UDS on Unix and a named pipe on Windows, so the
new dashboard call site chose the right client abstraction. The daemon has no matching named-pipe
server: `daemon/src/server.rs` imports `tokio::net::UnixListener` and
`UnixListenerStream` unconditionally and only exposes `spawn_uds_server`. Consequently the
dashboard cannot work end-to-end on Windows, regardless of the portable client.

The periodic worker itself uses portable Tokio/QUIC primitives and introduces no obvious
OS-specific problem. The blocker is the missing server half of local IPC. Dashboard and
session-reporter integration tests also instantiate `rendezvous_daemon`/Unix socket paths from
test modules that are not `cfg(unix)`, while the dev-dependency is Unix-targeted. Add a Windows
named-pipe server and Windows runtime coverage; a Unix-only test gate would make compilation
honest temporarily but would not satisfy the monorepo's product portability requirement.

### 10. MEDIUM — a connected-but-wedged daemon can hang `claudine dashboard`

Only the initial connect failure receives the friendly no-daemon treatment. The five unary RPCs
are then awaited sequentially without per-call or total deadlines (`dashboard/mod.rs:68-101`).
A daemon that accepts local IPC but stalls in one handler can hang the CLI indefinitely, contrary
to the graceful-degradation posture used by the producer.

Use a bounded total snapshot deadline and decide whether partial data is useful. Parallelizing
independent reads would reduce latency, but a future aggregate `DashboardSnapshot` RPC would also
give one capture contract and centralize partial-failure semantics.

## Design and maintainability conclusions

- **UPDATE-merge-only:** agree with the API contract after atomicity is fixed. A process monitor
  discovering an unwrapped process should emit STARTED on its first `(PID, process-start-time)`
  observation, then UPDATED only for transitions. Allowing UPDATE to create state makes lifecycle
  typos and late messages indistinguishable. Document this as a producer conformance rule. Also
  reserve daemon-owned clock keys: an UPDATED payload can currently overwrite
  `started_at_unix_ms`, despite the comment that the daemon owns clocks.
- **CLI-local `DashboardReport`:** agree. The report is a CLI-over-daemon concern, its plain model
  is testable, and keeping `claudine` free of `rendezvous-*` dependencies is the cleaner boundary.
- **60-second threshold:** agree with the exact boundary and the 3:1 nominal cadence for a small
  healthy mesh. Disagree that the current sequential full-sync worker establishes that service
  level under failure.
- **Staleness fold:** local-vs-remote identification through `Status.node_id` is correct. Direct
  `owner_node_id -> PeerInfo` matching is insufficient for transitive replication; use
  per-document observation metadata.
- **Trigger architecture:** converge Trigger 1 and Trigger 2 on a shared typed session-status
  reducer with explicit basis, strength/precedence, producer identity, session generation, and
  revision. An open string plus arrival-order LWW will become harder to repair after more
  producers ship.

## Test-coverage gaps

The following tests should exist before this becomes a foundation for Trigger 2:

1. Deterministic concurrent UPDATE/END and START/END register transitions, using barriers to force
   the harmful interleavings.
2. Back-to-back `waiting_on_user`/`active` reports with no await between them, repeated enough to
   prove causal ordering rather than eventual luck.
3. Sink-level transition tests: duplicate permission requests debounce; each intended progress
   class clears; unrelated events do not clear; interactive PTY coverage is explicitly absent or
   supplied by hooks.
4. Periodic-worker tests with paused time: successful rounds advance the clock; failed rounds do
   not; one 30-second wedged peer cannot starve a healthy peer; worker shutdown cancels in-flight
   work; closed connections enter reconnect/backoff.
5. Fold tests for a peer-only host, a fresh transitive register without a direct peer, stale-row
   headline counting, malformed/duplicate register payloads, and a crashed producer's old entry.
6. Provider/path coverage for permission hooks, including the UI distinction between “supported
   and no intervention” and “signal unavailable.”
7. Remote-clock-skew tests before displaying Trigger 2 idle duration.
8. Windows compilation and a real named-pipe daemon/client/dashboard round trip.
9. Dashboard RPC deadline/partial-failure behavior against a live-but-stalled server.

## Recommendation

Do not build Trigger 2 on the current status path. First:

1. make session-register transitions atomic;
2. define causal revisions and status precedence across sink/hook/process-monitor producers;
3. serialize or version the current detached updates;
4. decide how crashed producers stop being presented as live; and
5. correct Trigger 2's remote-clock assumption.

After those changes, the dashboard's CLI-local rendering and host-level staleness presentation
are a reasonable base to extend.
