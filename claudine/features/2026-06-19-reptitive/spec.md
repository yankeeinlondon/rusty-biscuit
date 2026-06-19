---
status: draft
created: 2026-06-19
area: claudine
phases: ~
packages:
    - claudine
    - claudine-cli
---

# Runaway-Output Guards + Ctrl+C Hardening

## Problem

A non-interactive `claudine compose` run (OpenCode provider, Kimi K2.7 model)
entered a tight, fast-scrolling loop that could not be stopped with Ctrl+C. The
repeating text was **model-generated** degenerate output:

```
This is the final listening.
Done.
No more.
End.
STOP.
OK.
Bye.
```

The wrapped model got stuck in a token-level repetition loop and streamed the
same short lines continuously. Claudine has no defense against this failure mode:

1. **Wall-clock `timeout`** is disabled by default (`built_in: None` in
   `composition/mod.rs::resolve_timeouts`) and the prompt opted in to neither it
   nor anything else — so there was no hard upper bound.
2. **Stream-silence `step_timeout`** (default `30m`) can only fire on *silence*.
   Every streamed line refreshes both the semantic clock (`last_event_at`) and
   the raw byte heartbeat (`last_byte_at`, refreshed in
   `exec/spawn.rs` before parsing). A continuously flooding child never goes
   silent, so the silence rule's `silence >= budget` test is never reached.
3. There is **no output-volume, output-rate, or repetition circuit breaker**
   anywhere in `claudine/lib` or `claudine/cli`.

So Claudine faithfully streamed the garbage indefinitely. Separately, Ctrl+C
*appeared* to do nothing because the child runs in its own process group and the
forwarded-SIGINT path has gaps (see Part 3).

This spec proposes two new content-driven termination guards plus a hardening
audit of the Ctrl+C / signal-forwarding path.

## Goals

1. **Exit expressions** — a user-definable set of patterns that, when detected
   in the agent's response stream, terminate the agent with an error. Scopable
   to: global (all providers/models), a single provider, or a provider+model
   combination.
2. **Repetition heuristic** — detect a line (or a group of lines) repeating over
   and over and, past a `MAX_REPETITION_ALLOWED` threshold, terminate the agent
   with an error.
3. **Ctrl+C correctness** — guarantee that there is *no* spawn/wait path where a
   user's Ctrl+C fails to terminate the wrapped child.

All three converge on the **same termination plumbing** already used by the
timeout watchdog: a signal that escalates SIGTERM → SIGKILL against the child
process group, plus a synthesized `error_kind` on the summary so the standard
failure handlers run.

> **NOTE (drafting):** Open questions are flagged inline with **❓Q#** markers and
> consolidated in [Open Questions](#open-questions). They are intentionally
> unresolved pending the brainstorming pass. Resolved decisions are recorded in
> the [Decisions Log](#decisions-log).

## Decisions Log

Decisions made during the brainstorming pass, newest last. Each supersedes the
corresponding inline **❓Q#** marker.

### Cluster A — detector placement, match unit, and surfaces (resolved 2026-06-19)

- **A1 — Hybrid placement (resolves Q4).** The detection algorithm is a **pure,
  stateful line-assembling detector in `claudine/lib`** with a `feed(&str) ->
  Option<Trip>` style surface and a `flush()` for end-of-stream. It reassembles
  arbitrary streamed chunks into canonical prose lines (split on `\n`) before
  running either guard, and keeps a bounded ring buffer of recent lines for
  cycle detection. It is **driven from the CLI wiring point** that observes the
  text (the sink / output callbacks), which supplies the active provider + model
  for scope selection and owns sending the trip into the termination channel.
  Rationale: lib-side purity gives string-in/trip-out unit tests (including a
  captured copy of the real runaway) while the caller keeps provider/model
  context and the existing SIGTERM→SIGKILL plumbing.
- **A2 — Scan `OutputText` + `Reasoning`, never tool payloads (resolves Q5).**
  Both assistant response text and thinking/reasoning text are scanned (models
  can loop in either). Tool input/result payloads are **never** scanned to avoid
  wrongful kills when the model legitimately reads files or runs commands with
  repetitive output.
- **False-positive posture (informs Q8/Q12).** A wrongful kill is a **very
  negative** outcome. `MAX_REPETITION_ALLOWED` must therefore be set
  **conservatively high** so that only unmistakable runaways trip the guard.
  Exact threshold still TBD in Cluster B, but the bias is explicit: prefer
  missing a marginal runaway over killing a healthy run.

### Cluster B — repetition algorithm (resolved 2026-06-19)

- **B1 — Group-cycle detection (resolves part of Q10).** Detect a repeating
  block of length `L` where `1 ≤ L ≤ K`. Maintain a ring buffer of the last
  `~2K` normalized lines; after each line, find the smallest period `L` such
  that the last `2L` lines are two identical halves. Count **full cycles**
  (`consecutive_matching_lines / L`) so the threshold means the same thing for
  any `L`. Single-line spam is just the `L = 1` case. Chosen over a single-line
  run counter (would miss the reproduced multi-line cycle) and over an
  entropy/compression heuristic (opaque, hard to tune false-positive-safe).
- **B2 — Exact equality on normalized lines (resolves Q11).** Trim trailing
  whitespace, then compare for exact equality. Fuzzy/near-match is explicitly
  deferred — it raises false-positive risk against legitimate numbered lists,
  tables, and step output. Known limitation: runaways with a moving part
  (incrementing counters, timestamps) will not form an exact cycle and may slip
  through; documented, revisit later if it bites.
- **B3 — Constants (resolves Q8, Q9, Q10).**
  - `MAX_REPETITION_ALLOWED ≈ 30` full cycles (high, per the false-positive
    posture; a flooding model still trips in well under a second).
  - `K ≈ 16` max detected cycle length (must be `≥ 6` to catch the incident;
    headroom is cheap).
  - Blank lines are kept as normal empty normalized lines (a blank-line flood is
    therefore an `L = 1` cycle of `""` and still trips); not skipped.
- **Volume-cap backstop — deferred to Cluster F.** A coarse per-turn
  output-volume cap (catch-all for runaways with period `> K` or no tidy cycle)
  overlaps the default-wall-clock-timeout question (Q17) and is decided there,
  not here.

### Cluster E — exit-expression config shape and scoping (partially resolved 2026-06-19)

- **E2-scope — single `scope` string field, `{agent}` | `{agent/model}` syntax
  (resolved; resolves the provider/model field question).** Each exit-expression
  entry carries an optional `scope` string:
  - absent / null → **global** (all agents, all models);
  - `"opencode"` → that agent, all models;
  - `"opencode/kimi-for-coding/k2p7"` → that agent + that model.
  Parsing: **split on the first `/`** — first segment is the agent
  (a `Provider`), the remainder is the model string verbatim (models may contain
  `/`). Scoping stays **additive**: a run is checked against the union of every
  entry whose `scope` matches (global ∪ agent ∪ agent/model); an absent scope is
  a wildcard. "Agent" here means Claudine's `Provider` (config already uses
  "agent" loosely, e.g. `preferred_agent`); the value matches a provider id.
  - *Deliberate limitation:* `{agent/model}` cannot express "this model under
    any agent" (no `*/model`). Acceptable for v1; addable later if needed.
  - *Validation:* unknown agent in a `scope` must fail at config-load
    (`ClaudineConfig::validate()`), not silently no-op.
- **E2-precision — exact model-string match in v1.** Prefix/glob deferred.
- **E1 — config surface: three layers with per-layer combine mode (resolved;
  resolves Q3).** Global config alone isn't portable to everyone working in a
  repo, so all three layers contribute:
  1. **User scope** — `~/.claudine/config.json` (`ClaudineConfig`). The base set.
  2. **Repo scope** — `{repo}/.claudine/config.json`. Combine mode
     **defaults to `override`** (repo set replaces the user set) so a repo is
     deterministic and every contributor gets identical guard behavior; may opt
     into `merge` (additive on top of user).
  3. **Markdown frontmatter** — `exit_expressions:` on the composition doc.
     Combine mode **defaults to `merge`** (additive) because the usual reason to
     scope to one prompt is handling an edge case *that prompt* exhibits while
     the general safeguards still apply; may opt into `override`.

  **Resolution pipeline** (each layer combines with the accumulated result
  below it):
  ```text
  effective = user_rules
  repo present?        override(default) → effective = repo_rules
                       merge             → effective = effective ∪ repo_rules
  frontmatter present? merge(default)    → effective = effective ∪ frontmatter_rules
                       override          → effective = frontmatter_rules
  ```
  Notes:
  - **Combine-mode expression.** `exit_expressions` accepts either an array
    (uses the layer's default mode) or an object
    `{ mode: "merge"|"override", rules: [...] }` (explicit mode). Array-or-object
    untagged deserialization matches existing house style (`TtsValue`,
    `ProtectConfig` are bool-or-object).
  - **Footgun acknowledged:** a frontmatter `override` discards repo + user
    rules (including the repo's deterministic safeguards). That is why `merge`
    is the frontmatter default; `override` is opt-in and the prompt author owns
    the consequence.
  - **Divergence from precedent (intentional):** `RepoOverrideConfig` uses
    per-event *replacement* for `actions`/`matchers`; here the **default** is
    repo-override but a per-layer **mode** is offered because these are safety
    tripwires where additive merge is often the safer intent.
  - **Scalar guard settings** (repetition `enabled`/threshold — Cluster F) do
    **not** use merge/override; they follow simple last-writer precedence
    (frontmatter > repo > user > built-in), like `timeout`/`step_timeout`. Only
    the list-typed `exit_expressions` carries a combine mode.
- **E3 — pattern kind + case (resolved; resolves Q2, Q6).**
  - **E3a — both kinds, default `literal`.** Per-entry `kind`: `"literal"`
    (default) | `"regex"`. Literal is the default because the feature is biased
    hard against false positives and literal avoids metacharacter surprises
    (e.g. regex `STOP.` matching `STOPS`); `regex` is the opt-in for anchors /
    alternation / inline flags. (Chosen over defaulting to `regex` despite the
    `protect`-uses-regex precedent.) Invalid regex fails at config-load
    (`ClaudineConfig::validate()`), never mid-stream.
  - **E3b — accept `pattern` or `patterns`.** A single `pattern: "…"` or
    `patterns: ["…", …]` sharing one `scope`/`kind`.
  - **E3c — literal `ignore_case`.** Optional `ignore_case` (default `false`)
    for literal entries; regex defers to inline flags (`(?i)`).
  - **E3d — per-line match target (v1).** Patterns are tested against each
    reassembled completed line (intuitive `^…$` anchors, cheap). Multi-line
    patterns are unsupported in v1 — documented known limitation; rolling-window
    matching is a possible later enhancement.
- **Wiring note (not a decision).** The detector receives the **compiled**
  in-scope pattern set at construction; scope is resolved and regexes compiled
  **once** before streaming, never per line.

### Cluster D — Ctrl+C correctness (resolved 2026-06-19)

Grounding (verified call sites): both production `run_child`
(`wrapper_stages.rs`, `harness_orch/attempt.rs`) and `run_child_capture`
(`harness_orch/attempt.rs`) pass `timeout_config.timeout` as their timeout arg.
When a wall-clock `timeout` is configured, that routes to
`exec/timeouts.rs::wait_with_timeout`, which (a) installs **no SIGINT handler**
and (b) sends its timeout-kill to the bare child PID, not the process group.
`run_child_capture` always puts the child in its own process group, so on that
path a configured `timeout` makes **Ctrl+C fully ineffective** and can orphan
the child if claudine dies on its default SIGINT disposition. The structured
`compose` path (`run_child_stream_semantic`) is correct because it uses the
watchdog + `wait_with_signal_and_early_termination`.

- **Invariant to guarantee.** Every wait path installs the SIGINT-forwarding
  handler, and every owned-process-group child receives **group-targeted**
  (`-pid`) signals — for both user interrupts and timeout/guard kills.
- **D — Unify the wait paths (resolves Q13; Option 2).** Fold wall-clock
  `timeout` enforcement into the unified watchdog so **all** spawn paths wait via
  the single `wait_with_signal_and_early_termination` loop; retire
  `wait_with_timeout`. One loop then owns signal handling, group-targeting,
  escalation, *and* the new content-trip channel from Parts 1–2 — making "a path
  that forgot Ctrl+C" structurally impossible. Must preserve the interactive-TUI
  passthrough case (`child_in_own_pgroup = false`, shared pgroup + TTY
  inheritance) so interactive providers (Claude/Codex) still receive terminal
  SIGINT naturally and don't hang on `SIGTTIN`.
- **Q14 — Visible interrupt feedback (accepted).** On each counted SIGINT, emit
  a visible stderr line (e.g. `⚠ interrupt received — press again to force-kill
  (n/3)`) so a user during an output flood knows the press registered. *Open
  sub-item (Q14b):* whether non-interactive runs should use a shortened ladder
  (press 1 → SIGTERM directly). Leaning yes for non-interactive, but not yet
  locked.
- **Q15 — Windows must be equally robust (resolves Q15).** Not best-effort. The
  unified wait loop needs a real Windows implementation with parity to the Unix
  group-signal/escalation behavior:
  - Spawn the child in a new process group via the `CREATE_NEW_PROCESS_GROUP`
    creation flag, and/or assign it to a **Job Object** so the whole tree can be
    terminated as a unit (`TerminateJobObject`) — the Windows analog of killing
    `-pid`.
  - Handle console Ctrl+C via `SetConsoleCtrlHandler`, and deliver interrupts to
    the child group via `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pgid)`
    (Ctrl+C events cannot be sent to a specific group; Ctrl+Break can).
  - Map the escalation ladder: Ctrl+Break (graceful) → `TerminateJobObject` /
    `TerminateProcess` (forceful), since Windows has no SIGTERM/SIGKILL split.
  - Likely pulls in the `windows`/`windows-sys` crate (confirm against existing
    workspace deps). **Testing caveat:** the dev host is macOS; Windows behavior
    must be designed carefully and validated in CI or on a Windows host —
    flagged as a known verification risk per the monorepo all-OS rule.

### Cluster C — termination labeling, reporting, and routing (resolved 2026-06-19)

Grounding (current state, verified): a child run yields an `AttemptOutcome`
carrying `exit_code` + `termination: ProcessTermination` + `final_response` +
`stderr_text` — but **not** `error_kind` (that lives only on the stream
summary, used for logging/metrics). Routing is done by
`classify_failure(&AttemptOutcome) -> Option<FailureEvent>`
(`harness/handlers.rs`): `TimedOut → Timeout`, `Interrupted → None` (suppresses
all failure handling — the Ctrl+C path), `LaunchFailed → AgentFailure`,
`Completed → AgentFailure if exit_code != 0 else None`. `FailureEvent`
(`AgentFailure`, `Timeout`, `Validation(..)`, `ShellAuditDenied`) selects the
handler block, the lifecycle message (`failure:` vs `blocked:`), and loop
fail-fast.

- **C1 — Two distinct `error_kind`s (resolves Q1).** `exit_expression` and
  `runaway_repetition`. Independent of routing; appears honestly in JSONL
  logs / metrics regardless of the `ProcessTermination` chosen.
- **C2 — Two new `EarlyTermination` variants.**
  `ExitExpression { pattern, scope }` and
  `RunawayRepetition { cycle_len, repeats }`. Each carries the structured detail
  its message and the handler payload need; two clean arms in
  `apply_early_termination_to_summary`.
- **C3 — New `ProcessTermination::Aborted` (resolves Q7).** Add one variant
  (serde-persisted; forward-compatible) shared by both guards;
  `classify_failure` maps `Aborted → FailureEvent::AgentFailure` (no new
  `FailureEvent` needed). This routes correctly (fires `failure:`, fail-fast,
  and **not** the `handle_timeout:` retry path — which would otherwise reproduce
  the runaway), and labels honestly at every layer (`Display = "aborted"`,
  `CLAUDINE_TERMINATION = "aborted"`, distinct metrics category for threshold
  tuning). Rejected: reuse `TimedOut` (wrong, triggers timeout-retry) and reuse
  `Completed`+`error_kind` (routes correctly but mislabels a killed process as
  "completed").
- **C3a — Thread context into the failure handler payload (required).**
  Principle: *error handling cannot make good decisions without context.*
  - Thread `error_kind` into both the programmatic-handler env
    (`CLAUDINE_ERROR_KIND`) and the JSON payload — at minimum.
  - Also surface the structured guard detail (matched `pattern` + `scope` for
    exit-expressions; `cycle_len` + `repeats` for repetition) in the JSON
    payload so a handler can distinguish a runaway from a genuine crash and act
    accordingly. (Mechanically this likely means carrying `error_kind` — and
    optionally a small guard-context blob — onto `AttemptOutcome`, since the
    handler payload is built from it.)
- **C4 — Fail-fast in the composition loop (accepted).** A content trip stops
  the loop. Re-running the identical prompt against the same model reproduces
  the runaway, so iterating would only burn `MAX_ITERATIONS`. The existing
  `fail_fast` default (`true`) already does this once the trip surfaces as an
  iteration error with `is_error = true`; the summary override
  (`apply_early_termination_to_summary`) provides that, mirroring the timeout
  path. Requirement: the trip must never be classified `Interrupted` (which
  would suppress failure handling) — it arrives via the early-termination
  channel, not a counted SIGINT, so this holds.

## Background — current architecture (where this hooks in)

- **Stream text** arrives as `SemanticEvent::OutputText { text }` (and
  `Reasoning { text }`) parsed in the stdout reader thread of
  `run_child_stream_semantic` (`exec/spawn.rs`). The `OutputTextCallback` is
  invoked per chunk; today it only feeds the terminal renderer.
- **Termination** is driven by `spawn_timeout_watchdog_ticker`
  (`exec/watchdog.rs`), which sends a `WatchdogTermination` over an mpsc channel
  to `wait_with_signal_and_early_termination` (`exec/termination.rs`). That wait
  loop sends `SIGTERM` to `-child_pid` (the child's process group) and escalates
  to `SIGKILL` after `kill_grace`.
- **Outcome mapping**: `EarlyTermination` (`stream/logs/opencode/reasoning.rs`)
  has `RateLimit`, `Timeout`, `StepTimeout`. `apply_early_termination_to_summary`
  sets `error_kind` (`"timeout"`, `"step_timeout"`, `"usage_limit_reached"`),
  `is_error`, and `error_message`.
- **Config**: global user config is `ClaudineConfig`
  (`config/claudine_config.rs`, `~/.claudine/config.json`). The closest existing
  precedent for a scoped pattern catalog is the **protect** service
  (`protect/`), a regex deny catalog with rule groups and toggles.
- **Provider/model identity** at runtime: provider is known from the wrap
  subcommand / `TimeoutConfig.provider`; model is known from `--model` and from
  `SemanticEvent::SessionStart { model }`.

## Part 1 — Exit Expressions

### Concept

A list of **exit expression** entries. Each entry carries one or more patterns
and an optional scope. As assistant output streams, each new unit of text is
tested against every in-scope entry; the first match terminates the run with
`error_kind = "exit_expression"` (**❓Q1** — final kind name).

### Configuration shape (straw man)

Global, in `ClaudineConfig`:

```jsonc
{
  "exit_expressions": [
    // Global (all providers, all models)
    { "pattern": "(?i)\\bI cannot continue\\b" },

    // Scoped to a provider
    { "pattern": "FATAL: context exhausted", "provider": "opencode" },

    // Scoped to a provider + model
    {
      "patterns": ["^STOP\\.$", "^Bye\\.$"],
      "provider": "opencode",
      "model": "kimi-for-coding/k2p7",
      "kind": "literal"          // ❓Q2 — literal vs regex
    }
  ]
}
```

Scoping precedence is **additive**, not override: a run with provider `opencode`
+ model `k2p7` is checked against (global entries) ∪ (provider `opencode`
entries) ∪ (provider+model entries). An entry matches a run when every field it
declares matches the run; absent fields are wildcards.

**❓Q3 — per-prompt scope.** Should exit expressions also be declarable in
composition frontmatter (per-prompt), in addition to global config? The protect
service supports repo-level overlay; this could too.

### Matching semantics

- **Pattern kind** — regex (default) vs literal substring. **❓Q2.**
- **Match unit** — what text is each pattern tested against? Options in
  [Open Questions](#open-questions) **❓Q4**: per-streamed-chunk, per-line, or a
  rolling window of the last *N* characters of assistant text. Chunk boundaries
  are arbitrary (a pattern can be split across two chunks), so a rolling
  accumulated buffer is almost certainly required.
- **Surfaces** — assistant `OutputText` only, or also `Reasoning` (thinking)
  text, or also tool output? **❓Q5.** Default proposal: `OutputText` +
  `Reasoning`, never tool *input/result* payloads (those are the model reading
  files and would cause false positives).
- **Case sensitivity / anchoring** — left to the regex author; provide
  `(?i)` etc. For literal mode, **❓Q6** case-insensitive toggle.

### Behavior on match

1. Stop feeding output to the renderer (avoid emitting more of the runaway).
2. Send a termination request through the existing watchdog/early-terminate
   channel → SIGTERM → SIGKILL escalation against the child process group.
3. Synthesize summary: `is_error = true`, `error_kind = "exit_expression"`,
   `error_message` naming the matched pattern and scope.
4. Map to a `ProcessTermination` variant so the standard failure handler runs
   (**❓Q7** — reuse `TimedOut`, or introduce a new `Terminated`/`Tripwire`
   variant for clearer reporting and to keep loop-engine `fail_fast` honest).

## Part 2 — Repetition Heuristic

### Concept

Detect when the model emits the same content repeatedly. After
`MAX_REPETITION_ALLOWED` (const, **❓Q8** — default value, proposal `≈ 20`)
consecutive repetitions of the same unit, terminate with
`error_kind = "runaway_repetition"`.

### Detection model (straw man)

Operate on **completed lines** of assistant output:

- Maintain a small ring buffer of recent normalized lines (trimmed; blank lines
  ignored or counted? **❓Q9**).
- Track a **cycle**: the repeating unit may be a single line *or* a group of
  lines (the reported failure cycled a 7-line group:
  `Done. / No more. / End. / STOP. / OK. / Bye. / Done. …`). Detect a repeating
  block of length `1..=K` (**❓Q10** — max cycle length `K`, proposal `8`).
- Increment a counter each time the next lines match the established cycle;
  reset when the cycle breaks. Trip when the counter ≥ `MAX_REPETITION_ALLOWED`.

**❓Q11 — exactness.** Exact-equality on normalized lines, or fuzzy/near-match
(e.g. counter that re-emits `1.`, `2.`, `3.` is technically not identical but is
still runaway)? Proposal for v1: exact normalized equality only — simpler, fewer
false positives — and revisit fuzzy later.

**❓Q12 — legitimate repetition.** Some real output repeats (tables, ASCII art,
generated lists, progress bars). A high threshold plus line-group cycle
detection mitigates this, but we should decide whether the guard is **on by
default** or **opt-in**. Proposal: on by default with a conservative threshold,
and a kill-switch in config.

### Behavior on trip

Identical to Part 1's termination path; only the `error_kind` and message
differ.

## Part 3 — Ctrl+C Correctness Audit

### Known gaps found during investigation

1. **`wait_with_timeout` installs no SIGINT handler.** When the legacy
   `run_child` / `run_child_capture` paths are called with `timeout: Some(_)`,
   they wait via `exec/timeouts.rs::wait_with_timeout`, which only polls
   `try_wait` and the deadline. It never registers a SIGINT forwarder. If the
   child is in its own process group (`isolate_process_group == true`, which
   happens whenever streams are piped or a stdin seed is used), terminal SIGINT
   reaches **neither** the child (it's in a background group) **nor** a
   forwarding handler (there is none). Ctrl+C is silently ineffective on that
   path. **❓Q13** — confirm exhaustively which production code paths reach this.
2. **Escalation requires multiple presses.** On the streaming path the handler
   maps press 1 → SIGINT, 2 → SIGTERM, 3 → SIGKILL. A child that ignores SIGINT
   (or whose model loop swallows it) survives the first press; during an output
   flood the user gets no feedback that presses are registering. **❓Q14** —
   should we (a) print a visible "interrupt received (n) — press again to force
   kill" line to stderr on each press, and/or (b) shorten the ladder for
   non-interactive runs?

### Goal of the audit

Enumerate every spawn path (`run_child`, `run_child_capture`,
`run_child_stream_semantic`) × every wait path (`wait_with_signal_handling`,
`wait_with_timeout`, `wait_with_signal_and_early_termination`) and prove that in
each, a user Ctrl+C results in child-group termination. Produce a matrix and
close any gap (most likely: give `wait_with_timeout` the same SIGINT-forwarding
registration the other two have, or fold timeout enforcement into the unified
watchdog so a single wait path handles all cases).

### Cross-platform note

Signal handling here is `#[cfg(unix)]`. Windows uses different mechanics
(`CTRL_C_EVENT`, no process groups in the POSIX sense). The audit must state the
Windows contract explicitly (**❓Q15**) — even if v1 only guarantees Unix and
documents Windows as best-effort, that must be a deliberate, written decision
per the monorepo cross-platform rule.

## Scope

### In scope

- New global config: `exit_expressions` (and repetition toggle/threshold
  overrides) on `ClaudineConfig`.
- A streaming content detector wired into the `OutputText`/`Reasoning` path of
  `run_child_stream_semantic`.
- Reuse of the existing watchdog → wait-loop SIGTERM/SIGKILL termination channel.
- New `EarlyTermination` variant(s) + `error_kind` value(s) + summary mapping.
- Ctrl+C audit across all spawn/wait paths and closing of identified gaps.
- Tests at the detector level (unit), the termination-mapping level (unit), and
  the signal level (the existing harness patterns).
- Docs: `claudine/docs/topics/timeouts.md` (or a new sibling) and the claudine
  skill.

### Out of scope (proposed — confirm)

- Detecting runaway behavior in *non-streaming* / captured-only paths
  (`run_child_capture`) beyond what the Ctrl+C audit requires. **❓Q16.**
- Repairing the underlying model behavior (this is a harness backstop, not a
  model fix).
- Changing the default wall-clock `timeout` (a separate, arguably-related
  decision — **❓Q17** could ride along or stay out).

## Open Questions

| # | Topic | Summary |
|---|---|---|
| Q1 | naming | ✅ RESOLVED (C1) — `exit_expression` and `runaway_repetition`. |
| Q2 | pattern kind | ✅ RESOLVED (E3a) — per-entry `kind`, default `literal`, `regex` opt-in. |
| Q3 | scope surface | ✅ RESOLVED (E1) — 3 layers: user → repo(override default) → frontmatter(merge default), per-layer mode. |
| Q4 | match unit | ✅ RESOLVED (A1) — line-assembling detector, lib-side pure algorithm. |
| Q5 | surfaces | ✅ RESOLVED (A2) — OutputText + Reasoning, never tool payloads. |
| Q6 | literal case | ✅ RESOLVED (E3c) — `ignore_case` (default false) for literal; regex uses inline flags. |
| Q7 | termination variant | ✅ RESOLVED (C3) — new `ProcessTermination::Aborted` → `AgentFailure`; thread `error_kind`+context into handler payload. |
| Q8 | threshold | ✅ RESOLVED (B3) — `MAX_REPETITION_ALLOWED ≈ 30` full cycles. |
| Q9 | blank lines | ✅ RESOLVED (B3) — kept as empty normalized lines, not skipped. |
| Q10 | cycle length | ✅ RESOLVED (B1/B3) — group-cycle detection, `K ≈ 16`. |
| Q11 | exactness | ✅ RESOLVED (B2) — exact normalized; fuzzy deferred. |
| Q12 | default-on | Repetition guard on-by-default vs opt-in. |
| Q13 | path coverage | ✅ RESOLVED (D) — unify all spawn paths onto one signal-aware wait loop; retire `wait_with_timeout`. |
| Q14 | feedback | ✅ RESOLVED — visible interrupt feedback line. Q14b (shortened non-interactive ladder) still open, leaning yes. |
| Q15 | Windows | ✅ RESOLVED — equally robust Windows path (Job Objects + console control events); not best-effort. |
| Q16 | capture paths | Guard captured-only paths too? |
| Q17 | default timeout | Ship a default wall-clock timeout alongside? |

## Success Criteria

- A configured exit expression matching streamed output terminates the child and
  reports `error_kind = "exit_expression"` with an honest message.
- A synthetic runaway stream (the reproduced 7-line cycle) trips the repetition
  guard at the threshold and terminates the child.
- A matrix test (or documented proof) shows Ctrl+C terminates the child on every
  spawn/wait path combination on Unix; Windows behavior is explicitly documented.
- All new behavior is covered by unit tests; signal behavior uses the existing
  real-process harness patterns.
- `just test` and `just lint` pass in the claudine package area.
