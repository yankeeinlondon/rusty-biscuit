# Rendezvous Dashboard

We want to be able to provide users of the Claudine ecosystem with "situational awareness" on what is:

- happening right now
    - what sessions are running?
    - what sessions need human intervention?
    - on which hosts (the mesh means "right now" spans machines, not just the one you're sitting at)
- happening over time (the historical complement — trends by agent, model, repo, and host)

## Two Views, Two Data Paths

The [rendezvous data-model doc](../../rendezvous/docs/crdt.md) splits the system into live state (CRDT state registers in **redb**, the key/value system of record) and historical facts (projected into **DuckDB**, the embedded analytics database). The dashboard has exactly one view on each side of that split:

### The NOW view

Answers "what is running, where, and does anything need me?" — served from **live state, never DuckDB**:

- each host's daemon maintains a `sessions-active/{node_id}` register (a small CRDT document listing that host's live sessions and their status); mesh sync means *every* daemon holds a replica of *every* host's register, so any single daemon can render the whole mesh. ✅ **Daemon side IMPLEMENTED (2026-07-12):** the register domain, a `ReportSessionEvent` RPC (STARTED/UPDATED/ENDED transitions with daemon-stamped `started_at`/`updated_at`; ENDED removes the entry — the register holds only live sessions), and the mesh-wide `ListActiveSessions` read RPC. ✅ **Producer side IMPLEMENTED (2026-07-13):** Claudine's wrapper brackets every provider child with a `SessionPresence` guard (`cli/src/commands/wrap/session_report.rs`) — construction reports STARTED (agent, model, interactive, repo_root, claudine_pid), drop reports ENDED on *every* exit path. Strictly best-effort: 250ms cap per call, absent daemon degrades to a debug log, kill switch `CLAUDINE_RENDEZVOUS_REPORT=false`. Covers all three execution paths (harness attempts, direct structured stream, direct capture/interactive). Still pending: stuck-entry hygiene for crashed producers (the process monitor's reconciliation job; until then consumers judge staleness from `updated_at_unix_ms`)
- sub-second freshness for the local host comes from gRPC streaming (a live subscription to the daemon rather than polling)
- host liveness comes from the ephemeral presence layer (derived from last successful peer sync — see the data-model doc's presence section)

Data sources feeding session status: Claudine wrapper hook events (richest — includes permission asks), the log monitor, and the [process monitor](../2026-07-12-process-monitor/spec.md) (which is what makes *unwrapped* sessions visible at all).

### The HISTORY view

Answers "what happened?" — served from DuckDB views over the fact tables (sessions per day, agent/model mix, repo focus over time, uptime). These are SQL queries, cheap to add; the hard work is the ingestion pipeline owned by the [logging refactor](../2026-07-10-logging-refactor/spec.md), not this feature.

## "Needs Human Intervention"

This is the dashboard's highest-value signal and needs a concrete definition. Candidate triggers, strongest first:

1. a wrapped session fired a permission-ask / blocked hook event and no subsequent event has cleared it (exact — we see these events directly)
2. a wrapped interactive session has been idle since an assistant turn completed (the agent is waiting on the user)
3. an unwrapped session is idle per the process monitor's heuristic (weakest — see process-monitor D4/S3)

The signal should carry its *basis* (which trigger and how confident) so the UI can render certainty honestly.

## Decisions (RATIFIED 2026-07-12)

All decisions below were ratified as recommended; specifics are stamped per item.

- **D1 — Delivery surface.** Options: (a) a `claudine` CLI report (one-shot render), (b) a live TUI (terminal UI that refreshes in place), (c) a browser view. *Recommendation:* start with (a) — our render components are dual-target (`TerminalRenderable` + `BrowserRenderable`, as `MetricsReport` already proves), so a CLI report gets us a browser artifact almost for free; graduate to a live TUI once the NOW plumbing is proven. **Ratified:** a new top-level `claudine dashboard` command; the live TUI later graduates under the same name.
- **D2 — Scope toggle.** Local host only vs whole mesh, and which is the default. *Recommendation:* mesh-wide default with a `--local` flag; the mesh view is the feature's differentiator.
- **D3 — Refresh model for the NOW view.** Poll the registers on interval vs subscribe over gRPC push. *Recommendation:* subscription for the local daemon; remote hosts are inherently "as of last sync" so polling the local replicas is fine — which makes D4 the real question.
- **D4 — Staleness presentation.** With eventual consistency, a remote host's data is always *slightly* old — and if a peer is unreachable, possibly *very* old. *Recommendation:* every remote host row renders its last-sync age ("as of 3s ago" / "unreachable for 2h"), and past a threshold its sessions render as unknown rather than as their last-known status. **Ratified:** the threshold is **60 seconds** of sync silence — tight enough that a user won't act on a dead session, loose enough for normal sync jitter.
- **D5 — Intervention definition (see above).** Which triggers ship in v1, and is the signal binary or tiered (needs-input vs possibly-stuck)? *Recommendation:* v1 ships trigger 1 (exact) and trigger 2 (good), tiered; trigger 3 arrives with the process monitor's idle spike.

## Risk Areas & Spikes

- **S1 — Intervention coverage varies by provider.** Trigger 1 depends on permission-ask hook events, and hook support differs across the 10 providers (`claudine hooks --support` is the matrix). Spike: audit which providers can actually produce the signal, so the dashboard can say "not supported for agent X" instead of silently showing nothing.
- **S2 — Stale data mistaken for live data.** The failure mode of every eventually-consistent dashboard: a user acts on a session that ended an hour ago, or misses one that needs them, because a peer stopped syncing. D4's staleness rendering is the mitigation; treat it as a launch requirement, not polish.
- **S3 — Register churn vs write-cadence budget.** A parallel-heavy user (many short non-interactive sessions) makes `sessions-active` a hot register, straining the data-model rule that registers stay low-cadence. Shares the register-compaction spike already flagged in the data-model doc; measure with a realistic parallel workload before the design is considered stable.
- **S4 — Cross-feature dependency risk.** The NOW view is only as good as its producers: unwrapped-session visibility needs the process monitor, and status quality needs the logging refactor's canonical envelope (its D1). Sequence the milestones so the dashboard lands *after* those two have their v1 shapes — or scope dashboard-v1 to wrapped sessions only. **Ratified:** dashboard v1 ships early, **wrapped sessions only**; unwrapped sessions appear when the process monitor lands (the dashboard is its consumer, not its blocker).
