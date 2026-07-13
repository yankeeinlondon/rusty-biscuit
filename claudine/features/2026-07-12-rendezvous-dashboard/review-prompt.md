# Architecture Review — Rendezvous Dashboard v1 (Claudine)

You are reviewing a just-landed feature in the `rusty-biscuit` monorepo (branch   `claudine`). Give it an independent, skeptical read. Full test suite (`just test`,   `just test-l2`, `just lint` across `claudine/` and `rendezvous/`) is green — I want design/correctness scrutiny, not a test run.

## What it is

`claudine dashboard` — a mesh "NOW view" over the rendezvous daemon: live agentic   sessions across hosts, per-host staleness, and a "needs human intervention" signal.   It spans three layers: rendezvous **daemon** (staleness clock), a **CLI consumer**   (the dashboard command), and a wrapper **producer** (session status transitions).

## Scope to review

- Ratified design: `claudine/features/2026-07-12-rendezvous-dashboard/spec.md`
    (see the "Decisions" section, especially the 2026-07-13 impl stamps on D4/D5/D6).

- Commit range: `c17c19943^..f16dc7056` (7 feature commits + docs).
    **Skip `9106bc4e6`** (`repo_home.rs` shadow-purge change) — unrelated, rode along.

- The producer-presence base this builds on: `10253e846`.
- Use the GitNexus MCP tools (`impact`, `context`, `explain`) freely to trace call
    graphs and blast radius.

## Please scrutinize these decisions specifically

1. **Staleness clock.** `PeerInfo.last_synced_unix_ms` is stamped *only* on a
   successful direct-sync round (not mDNS chatter), and a new ~20s periodic re-sync
   worker keeps it advancing under the dashboard's 60s staleness threshold
   (`rendezvous/daemon/src/peers.rs`). Is 20s vs 60s the right ratio? Does the worker
   handle connection loss / repeated sync failure gracefully? What's the bandwidth/CPU

2. **The UPDATE-merge-only race fix** (`rendezvous/daemon/src/service.rs`,
   `apply_session_event`). We changed `UPDATED` to merge over an *existing* session and
   never create one, so a late fire-and-forget status report can't resurrect a session
   after `ENDED`. Is this the right layer for that invariant? Does it foreclose any
   legitimate future producer (e.g. the process monitor reporting an unwrapped session
   it discovers via UPDATE without a prior STARTED)?

3. **Fire-and-forget `StatusReporter`** (`cli/src/commands/wrap/session_report.rs`).
   It captures a tokio `Handle` at build time and spawns detached, timeout-bounded
   reports so it never stalls the stream-render path (which may run off-runtime). Is
   the ordering-vs-ENDED reasoning sound (given #2)? Any unbounded-spawn or
   shutdown-drop concern? Is the `awaiting_user` edge debounce in the sink correct?

4. **Trigger-1 coverage gap (biggest question).** Trigger 1 (permission-ask →
   `waiting_on_user`) is wired through the semantic **sink**, which only exists on the
   structured/harness paths — so **interactive pty sessions get no Trigger 1 from the
   stream**. Is a sink-based Trigger 1 the right choice, or should it (and the planned
   Trigger 2) be hook-based for uniform coverage? (See `trigger2-plan.md` in the same
   feature dir — Trigger 2 is planned hook-based precisely because of this.)

5. **Staleness attribution & the fold** (`cli/src/commands/dashboard/model.rs`).
   Local host identified via a new `Status.node_id`; remotes matched to `ListPeers`
   by `owner_node_id`. A host we hold a register replica for but aren't directly peered
   with folds to `NeverSynced`. Is the local/fresh/stale/never-synced bucketing correct
   and honest? Any off-by-one at the 60s boundary?

6. **Placement & portability.** Dual-target `DashboardReport` lives CLI-local (not in
   the `claudine` lib) to keep the library free of any `rendezvous-*` dep — reasonable?
   Any Windows concern in the periodic worker or the client's UDS/named-pipe split?

## Deliverable

A prioritized findings list: correctness risks first (with a concrete failure scenario   each), then design/maintainability concerns, then test-coverage gaps. Flag anything   that should block building on this (Trigger 2 is next). Note where you disagree with a   ratified decision and why.

Save your review contents to "dashboard-review-1.md" in the same directory as the feature.
