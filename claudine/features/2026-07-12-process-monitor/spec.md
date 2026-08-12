# Process Monitor

The **process monitor** gives Rendezvous visibility into agent activity that Claudine does *not* wrap. When a user launches an Agentic CLI directly (or uses an agent inside an app like an IDE), Claudine's wrapper never runs, no hook events fire, and without this feature that session is invisible. The monitor closes the gap by scanning the host's process table on an interval and reporting what it finds.

It is the third of three observation producers feeding the logging pipeline (alongside the Claudine CLI and the log monitor — see the [logging-refactor spec](../2026-07-10-logging-refactor/spec.md)), and it is the primary data source for the "what is happening right now" half of the [dashboard](../2026-07-12-rendezvous-dashboard/spec.md).

## What It Does

On a configurable interval the monitor:

1. Scans the host process table (via the **sniff** library where its detection already covers the need, with the **sysinfo** crate — a cross-platform Rust crate for process/CPU/memory inspection — underneath for the raw process data)
2. Classifies matches into:
    - **Claudine-wrapped sessions** — a `claudine` process with an agent child; these are already richly observed via hooks, so the monitor only confirms liveness
    - **Direct CLI sessions** — a known agent binary (`claude`, `codex`, `gemini`, …) running without a Claudine parent; identified using the provider metadata catalog (binary names per provider are already part of our generated provider data)
    - **Agent applications** — GUI apps or IDE-embedded agents (e.g. an agent running inside an editor); hardest to classify, see S2
3. Extracts what it can from each match: PID, parent PID, binary path, start time, CPU/memory, and — where the OS allows — command line and working directory (the working directory is how we attribute a session to a **repo**)
4. Derives a per-session status: **active** (producing output / consuming CPU), **idle** (alive but quiescent — typically an interactive session waiting for the user), or **ended**

## Data Model

Storage follows the shared [rendezvous data-model doc](../../rendezvous/docs/crdt.md). The key subtlety: process polling is *high-cadence* data, and the data-model rules forbid writing high-cadence data into a persistent CRDT document (its edit history grows forever). So the monitor's output is split by cadence:

| Data | Cadence | Where it lives |
|------|---------|----------------|
| Live process table ("right now") | every poll | **ephemeral only** — in-memory daemon state, served over gRPC and shared via the ephemeral (non-persisted) sync layer; never written to a CRDT document |
| Session transitions (started / ended / status changed) | low — only when something changes | Kind-1 **fact log** entries through the canonical log envelope, plus an update to the host's `sessions-active/{node_id}` **register** |
| Historical reporting (sessions per day, unwrapped-vs-wrapped ratio, …) | derived | DuckDB views over the transition facts |

This mirrors how presence works (live boolean is ephemeral, transitions are facts) and keeps the registers within their write-cadence budget: a poll that observes *no change* writes *nothing*.

## Decisions (RATIFIED 2026-07-12)

All decisions below were ratified as recommended; the option discussion is kept for
context.

- **D1 — Where the monitor runs.** In-process inside the daemon (a tokio task) vs a separate spawned client like `agent-tail`. *Recommendation:* in-process. Unlike log tailing (per-source parsers, likely to churn), process scanning is one small loop; a separate process buys isolation we don't need and costs supervision we do.
- **D2 — Poll interval.** Fast enough that the dashboard's "now" view feels live, slow enough to be invisible in Activity Monitor. *Recommendation:* 5s for the process scan with change-driven emission; make it configurable and consider slowing when no agent processes exist at all.
- **D3 — Scan scope.** Only binaries known to the provider metadata catalog vs a broader heuristic net (anything whose cmdline mentions a model name, etc.). *Recommendation:* catalog-known binaries only for v1 — the broad net produces false positives and the catalog is already maintained; revisit when we tackle agent *applications* (S2).
- **D4 — Idle vs active definition.** Candidates: CPU over a window, child-process activity, TTY foreground state, recent writes to the provider's log file (the log monitor knows this). *Recommendation:* start with CPU-over-window as the v1 heuristic, but design the status field as an enum with a `confidence`/`basis` note — the dashboard's "needs human intervention" feature will want to refine this, and cross-referencing the log monitor's "last log write" is likely the strongest signal. Needs spike S3.
- **D5 — PID ↔ session correlation.** For wrapped sessions Claudine already reports both PIDs, so linkage is exact. For direct sessions the monitor only has a PID; the log monitor only has the provider's session id. Correlation candidates: cwd + start-time proximity, or per-provider tricks (some providers put a session id in the cmdline or env). *Recommendation:* record both observations independently with whatever identifiers each has (per logging-refactor D3) and correlate in the projection; run the correlation spike (logging-refactor S3) before promising session-level linkage for unwrapped sessions.

## Risk Areas & Spikes

- **S1 — Per-OS process introspection (spike required).** What we can read varies sharply by platform: on Linux `/proc` gives cmdline and cwd freely; on macOS cwd needs `proc_pidinfo` and may be blocked by SIP/sandboxing for processes we don't own; on Windows cmdline/cwd of another user's process needs elevated rights. Spike: a matrix of {field × OS × same-user/other-user} of what's actually readable, since **cwd is our repo-attribution mechanism** and losing it degrades a headline feature. All three OSes must be first-class.
- **S2 — Agent applications.** Detecting "an agent inside VS Code" from the process table alone is unreliable (the interesting activity is a thread/subprocess of a generic app). Treat app detection as best-effort in v1 and be explicit in the dashboard about the confidence level, rather than over-promising.
- **S3 — Idle detection quality (feeds D4).** An interactive session waiting for user input and a hung session look identical to a CPU heuristic. Spike alongside the log monitor: does "provider log last written at" beat CPU sampling in practice?
- **S4 — PID reuse.** OSes recycle PIDs; a naive "PID disappeared then reappeared" reading can stitch two different sessions together. Always key observations on (PID, process start time), never PID alone.
- **S5 — Privacy of command lines.** Cmdlines can contain prompts, file paths, or tokens, and these observations sync across the mesh. Decide a redaction policy *before* cmdline capture ships (e.g. store the binary + a hash of the full cmdline, or an allowlist of flags worth keeping).
