---
total_phases: 5
source_files_during_phase_2:
  - claudine/cli/src/commands/dashboard/model.rs
  - claudine/cli/src/commands/dashboard/report.rs
  - claudine/cli/src/commands/dashboard/tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/env/mod.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
  - claudine/cli/src/commands/wrap/session_report.rs
  - claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/handle.rs
  - claudine/cli/src/commands/wrap/session_report.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/features/2026-07-12-rendezvous-dashboard/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/cli-reference.md
source_code:
  - claudine/cli/src/commands/dashboard/model.rs
  - claudine/cli/src/commands/dashboard/report.rs
  - claudine/cli/src/commands/dashboard/tests.rs
  - claudine/cli/src/commands/wrap/env/mod.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
  - claudine/cli/src/commands/wrap/session_report.rs
  - claudine/cli/src/commands/handle.rs
  - claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap
  - claudine/docs/providers/dispatch-inventory.json
documentation:
  - claudine/features/2026-07-12-rendezvous-dashboard/spec.md
  - .claude/skills/claudine/cli-reference.md
packages:
  - claudine-cli
---
# Trigger 2 — Interactive-Idle Intervention Signal (Implementation Plan)

Completes the tiered "needs human intervention" signal (dashboard spec **D5**).
Trigger 1 (permission-ask, `waiting_on_user`) shipped in dashboard v1; this plan adds
**Trigger 2**: *a wrapped interactive session that has been idle since its last assistant
turn completed — the agent is waiting on the user.*

## Core design insight (read first)

Trigger 1 is wired through the **semantic stream sink** (`live_semantic_sink`), which
observes `SemanticEvent::PermissionRequest`. That sink only exists on the **structured**
and **harness** execution paths. **Interactive sessions run through the direct pty
passthrough path (`run_child`) — which has no sink and parses no stream** — so the
sink mechanism is blind to interactive turns. Trigger 2 therefore **cannot** reuse the
Trigger 1 plumbing.

The only signal source available for an interactive session's turn boundaries is the
**provider's hooks**, which fire `claudine handle <event>` out-of-band:

- an assistant turn completing → a `turn_complete`-class hook (e.g. Claude's `Stop`),
- the user submitting the next prompt → a `before_prompt`-class hook (Claude's
  `UserPromptSubmit`).

So Trigger 2 is a **hook-driven producer** living in the `claudine handle` command, a
different producer path than Trigger 1's sink. This is the central decision behind the
phasing below.

## Data model & vocabulary

> **SUPERSEDED by the session-state foundation (2026-07-13).** This section's provisional
> D2 — "reuse the flat `status` string + extend its vocabulary" — is obsolete. The
> [`session-state-design.md`](./session-state-design.md) foundation (landed 2026-07-13)
> replaced the bare last-writer-wins `status` string with **typed per-producer `status_slots`
> and a precedence reducer**. Trigger 2's `idle` is now an **`IdleHook` producer contributing
> a typed `Idle` state** (basis `interactive_turn_complete`, strength 20) into its own slot —
> not a new string literal on a shared field. The reducer's precedence (`waiting_on_user`
> (30) > `idle` (20) > `active` (10)) is exactly what makes the weaker `idle` unable to
> clobber an unresolved `waiting_on_user`, resolving what D5's Finding-4 note below flagged.
> The plan's phases below (producer wiring, consumer `PossiblyIdle` badge, gating, tests)
> land on this foundation **unchanged in spirit**; only the mechanism shifts from "write the
> `idle` string" to "contribute an `Idle` slot", and the daemon still projects a
> backward-compatible flat `status: "idle"` the consumer reads. The vocabulary table below is
> retained as the tier/consumer mapping, now read as reducer output rather than raw writes.

The `sessions-active` register entry already carries a `status` string. We extend the
vocabulary rather than adding a field:

| `status` value      | Producer            | Basis / tier        | Consumer `Intervention` |
|---------------------|---------------------|---------------------|-------------------------|
| `active`            | STARTED / progress  | —                   | `None`                  |
| `waiting_on_user`   | sink (Trigger 1)    | permission ask (strong) | `NeedsInput`        |
| `idle`              | hook (Trigger 2)    | interactive turn-complete (weak) | `PossiblyIdle` |

Idle **duration** is derived consumer-side from `updated_at_unix_ms`. **Clock-skew caveat
(review Finding 5): this is skew-free only for the LOCAL host** — its `updated_at` was
stamped by the same daemon whose clock the dashboard reads. A REMOTE session's `updated_at`
was stamped by the *remote* daemon, so comparing it to the consumer's local clock mis-reports
idle age under clock skew (a 5-min-slow remote clock shows "idle 5m" instantly; a fast one
clamps to 0). **v1 therefore shows idle duration for LOCAL sessions only**; for remote
sessions render "idle" without a duration, or derive a conservative *local observation age*
from per-document sync metadata (deferred — the daemon does not expose it yet). Both
intervention tiers are suppressed for stale hosts (reusing the existing `sessions_trusted()`
gate).

## Decisions (baked in; ⚠ = confirm before Phase 3)

- **D1 — Hook-based producer.** Trigger 2 fires from `claudine handle`, not the sink,
  because interactive sessions bypass the sink. *(Forced by the architecture; not really
  optional.)*
- **D2 — Vocabulary.** ~~Reuse `status` with a new value `idle` (vs a new `basis` field).
  Minimal, and the consumer already keys off `status`.~~ **SUPERSEDED (2026-07-13):** the
  typed session-state foundation makes `idle` an `IdleHook` `Idle` slot under the reducer
  (basis `interactive_turn_complete`), which still projects a flat `status: "idle"` for the
  consumer — so the consumer keeps keying off `status` while precedence is now authoritative.
- **D3 ⚠ — No dwell threshold in v1.** Flag `PossiblyIdle` immediately on turn-complete
  and show the elapsed duration, rather than waiting N seconds. Alternative: a small dwell
  (e.g. 10s) to avoid flagging brief think-pauses. *Recommend immediate + duration.*
- **D4 — Interactive gating via env.** The wrapper injects `CLAUDINE_INTERACTIVE=1|0`
  into the child env so the hook subprocess (`claudine handle`) can gate: non-interactive
  turn-completes are the agent auto-proceeding, **not** waiting on a human, and must not be
  flagged.
- **D5 — Clear on user resume only.** `idle → active` fires on `before_prompt`
  (user submitted). Because a provider's turn-complete hook (Claude `Stop`) fires only at
  genuine turn end — not between mid-turn tool calls — `handle` needs no cross-invocation
  state; the two edge events alternate naturally.

## Phase 1 — Grounding spike (no product code)

**Goal:** every assumption D1–D5 rests on is confirmed in writing before code is touched.

Investigate and record findings in a short `trigger2-findings.md` (this directory):

1. **Event mappings** — via `claudine hooks --support` / `--mapping` and the event model,
   confirm which providers fire a hook normalizing to `AgenticEvent::TurnComplete` (turn
   end) and `AgenticEvent::BeforePrompt` (user submit). Claude is the v1 target; note the
   coverage matrix for the rest (spec **S1**). Confirm `Stop` maps to `TurnComplete` and
   does **not** fire between mid-turn tool calls (only at true turn end).
2. **Session identity reaches hooks** — confirm a wrapped interactive session's hook
   subprocess (`claudine handle`) inherits `CLAUDINE_SESSION_ID` (env propagates
   wrapper → provider → hook child).
3. **Sink is absent on the interactive path** — confirm `run_child` (direct interactive)
   builds no `LiveSemanticSink`, so hooks are the only turn-boundary source.
4. **`handle` async + env** — confirm `commands::handle::run` is `async` and can do a
   best-effort daemon call, and reads process env.

**Verification:** findings doc lists confirmed mappings + the provider coverage matrix;
any assumption that fails is resolved (or the plan is revised) before Phase 2.

**Risks:** a provider that fires `Stop` mid-tool-loop would cause false idle flags —
this phase is where that's caught.

## Phase 2 — Consumer: `PossiblyIdle` tier (standalone, testable with synthetic data)

**Goal:** `claudine dashboard` renders a distinct, weaker idle signal (with duration) for
sessions whose `status` is `idle` on a fresh host, and suppresses it on stale hosts —
provable with synthetic register JSON, no daemon or producer needed.

**Changes** (`cli/src/commands/dashboard/`):

- `model.rs`: add `Intervention::PossiblyIdle`; split the status mapping —
  `waiting_on_user | blocked → NeedsInput`, `idle → PossiblyIdle` (fresh hosts only).
  Add `updated_at_unix_ms: Option<i64>` to `SessionRow`; thread the capture instant
  (`now_ms`) so the fold can compute idle age.
- `report.rs`: distinct badge for `PossiblyIdle` (e.g. dim "◦ idle 45s" vs the yellow
  "⚠ input"); heading keeps the strong needs-input count, optionally appends an idle count.
- `tests.rs`: synthetic `idle` → `PossiblyIdle` + duration; stale host suppresses it;
  render smoke shows the idle badge; `waiting_on_user` still maps to `NeedsInput`.

**Verification:** `just test` (claudine-cli) green; the dashboard renders an idle row when
fed a synthetic `idle`-status register.

**Risks:** low — pure consumer logic; no runtime dependency.

## Phase 3 — Producer plumbing (env + reporter helper)

**Goal:** the wrapper advertises interactiveness to hooks, and `claudine handle` has a
best-effort UPDATED helper — both provable in isolation.

**Changes:**

- `cli/src/commands/wrap/env/mod.rs`: inject `CLAUDINE_INTERACTIVE = "1" | "0"` alongside
  `CLAUDINE_SESSION_ID` / `CLAUDINE_PID` (~L307–317). Update the existing env test that
  asserts its **absence** (`env/tests.rs:445`).
- `cli/src/commands/wrap/session_report.rs`: add a standalone async best-effort
  `report_status(session_id, status)` (connect → UPDATED `{status}`, `REPORT_TIMEOUT`
  cap, kill-switch honored, degrade to `debug!`). `handle` is async, so this is a plain
  awaited call — no `block_in_place` needed. Reuses the merge-only daemon semantics that
  make a late/duplicate UPDATE safe.

**Verification:** env-injection unit test (interactive vs non-interactive); `report_status`
round-trip against a live in-process daemon flips a STARTED session's status.

**Risks:** env propagation to hook subprocesses is provider-mediated — Phase 1 confirms it;
this phase adds a test asserting the value is present in the child env map.

## Phase 4 — Producer wiring in `claudine handle`

**Goal:** a wrapped **interactive** session reports `idle` on turn-complete and `active`
on the next user prompt; non-interactive and unwrapped sessions report nothing (or safely
no-op).

**Changes** (`cli/src/commands/handle.rs`): after existing event dispatch, when
`CLAUDINE_INTERACTIVE=1`, `CLAUDINE_SESSION_ID` is set, and reporting is enabled:

- `AgenticEvent::TurnComplete` → `report_status(sid, "idle")`.
- `AgenticEvent::BeforePrompt` → `report_status(sid, "active")`.

Best-effort throughout: absent/wedged daemon never delays or fails the hook; kill switch
`CLAUDINE_RENDEZVOUS_REPORT=false` disables it. Bare (unwrapped) sessions have no register
entry, so the UPDATE **safely no-ops** (daemon merge-only rule — a Trigger 1 invariant this
plan depends on).

**Verification:** drive `handle` with a `turn_complete` event + interactive env against a
live daemon → register shows `idle`; a `before_prompt` → `active`; a non-interactive env →
no write. If `handle`'s source shifts dispatch-inventory line numbers, regenerate with
`CLAUDINE_UPDATE_INVENTORY=1`.

**Risks:** double-reporting if a provider fires `Stop` more than once per turn (idempotent —
same status). Interactive-*structured* sessions could get both a sink `waiting_on_user`
and a hook `idle` — **last-writer-wins is NOT acceptable here (review Finding 4)**: the
weaker `idle` must never overwrite an unresolved `waiting_on_user`. This is resolved by the
typed-precedence reducer in `session-state-design.md`, which this phase must build on.

## Phase 5 — End-to-end verification + drift

**Goal:** the full loop is observed live, and every doc/artifact that describes the signal
is current.

- **Live:** wrap an interactive Claude session (or replay real `handle` hook invocations
  with the interactive env) → finish a turn → `claudine dashboard` shows "idle Ns" → submit
  a prompt → the row clears to active.
- **Drift:** mark Trigger 2 implemented in `spec.md` (D5); update the skill
  `cli-reference.md` dashboard section (intervention now tiered: needs-input vs idle) and
  `SKILL.md` if wording changes; refresh `md hash` on edited skill files; regenerate the
  dispatch inventory if touched.
- **Full sweep:** `just test`, `just test-l2`, `just lint` (claudine + rendezvous) green.

**Verification:** the live drive shows both the set and the clear; full suite green; docs
match behavior.

## Cross-cutting risks

- **Provider coverage (S1).** Only providers exposing both a turn-end and a user-submit
  hook can produce Trigger 2. Claude qualifies; others degrade to **no idle signal** (never
  a wrong one). The dashboard already renders honestly when a signal is absent.
- **Write cadence (S3).** Trigger 2 writes at most twice per interactive turn (idle, then
  active) — negligible vs the register-compaction budget.
- **Clock source (review Finding 5 — CORRECTED).** `updated_at_unix_ms` is skew-free only
  for the LOCAL host (same daemon clock the dashboard reads). A remote session's `updated_at`
  is stamped by the remote daemon, so idle duration derived from it is wrong under clock skew.
  v1 shows idle duration for local sessions only; remote sessions show "idle" without a
  duration (or a conservative local observation age once the daemon exposes per-document sync
  metadata — deferred).
- **Status precedence (review Finding 4 — BLOCKER, gates this plan).** This plan cannot ship
  on the current open `status` string + arrival-order last-writer-wins: a weaker `idle` must
  not overwrite an unresolved stronger `waiting_on_user` just because its hook arrived later,
  and separate hook processes can reorder writes. Trigger 2 depends on the session-state
  foundation (atomic transitions + causal revision + typed precedence + hook-primary Trigger 1)
  being designed and landed first — see `session-state-design.md`. The D1/D2 vocabulary below
  is provisional and will be reconciled with that typed model.
- **Unwrapped sessions.** Out of scope for v1 (spec S4); they arrive with the process
  monitor. Any stray UPDATE from an unwrapped hook safely no-ops.
