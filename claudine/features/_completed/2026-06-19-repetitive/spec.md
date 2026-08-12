---
status: draft
created: 2026-06-19
area: claudine
phases: ~
review_iterations: 6
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
forwarded-SIGINT path has gaps (see Part 4).

This spec proposes **three** content-driven termination guards (exit
expressions, a repetition heuristic, and a volume-cap backstop) plus a hardening
of the Ctrl+C / signal-forwarding path on both Unix and Windows.

## Goals

1. **Exit expressions** — a user-definable set of patterns that, when detected
   in the agent's response stream, terminate the agent with an error. Scopable
   to: global (all providers/models), a single provider, or a provider+model
   combination.
2. **Repetition heuristic** — detect a line (or a group of lines) repeating over
   and over and, past a `MAX_REPETITION_ALLOWED` threshold, terminate the agent
   with an error.
3. **Volume-cap backstop** (added during brainstorming — see F2) — a per-turn
   output-volume cap that terminates the agent when a single turn emits an
   unmistakably pathological amount of output, catching runaways that are
   acyclic or whose period exceeds the cycle detector's `K`, and bounding the
   capture path's unbounded buffer.
4. **Ctrl+C correctness** — guarantee that there is *no* spawn/wait path where a
   user's Ctrl+C fails to terminate the wrapped child, on **both** Unix and
   Windows (see Cluster D / Q15).

All three content guards (1–3) converge on the **same termination plumbing**
already used by the timeout watchdog: a signal that escalates SIGTERM → SIGKILL
(graceful → forceful on Windows) against the child process group, producing a
distinct `error_kind` per guard (`exit_expression`, `runaway_repetition`,
`runaway_volume`) and a synthesized summary so the standard failure handlers
run.

> **NOTE (status):** The brainstorming pass is complete — every open question is
> resolved and recorded in the [Decisions Log](#decisions-log), which is the
> authoritative source. Parts 1–3 below have been reconciled to match it; the
> [Open Questions](#open-questions) table is retained as a resolution index.

## Decisions Log

Decisions made during the brainstorming pass, grouped by cluster (A, B, then
E, F, D, C as they were inserted). This log is the authoritative source; the
[Open Questions](#open-questions) table maps each `Q#` to the cluster that
resolved it.

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
  negative** outcome. `MAX_REPETITION_ALLOWED` is therefore set **conservatively
  high** (`≈ 30`, settled in Cluster B) so only unmistakable runaways trip the
  guard. The bias is explicit: prefer missing a marginal runaway over killing a
  healthy run.

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

### Cluster F — defaults and blast radius (resolved 2026-06-19)

- **F1 — Repetition guard on by default + kill-switch (resolves Q12).** Runs for
  every structured streaming run with the conservative Cluster B constants; a
  config flag disables it. (A default-off guard would sit unused until the next
  incident — the original failure opted into *no* protection.) Exit-expressions
  ship **empty** by default (purely user-authored).
- **F2 — Volume-cap backstop IN SCOPE (resolves the parked Cluster B item).** A
  per-turn output-volume cap (lines and/or bytes; high default, kill-switch)
  trips regardless of structure. Catches runaways the cycle detector misses
  (acyclic, or period > `K`) and, unlike a wall-clock timeout, does **not**
  punish slow-but-legitimate runs (volume ≠ time). This makes it a **third
  guard**, with ripple effects:
  - third `error_kind`: `runaway_volume` (alongside `exit_expression`,
    `runaway_repetition`);
  - third `EarlyTermination` variant: `RunawayVolume { lines, bytes }`;
  - measurement surface differs per path: **streaming** = assistant
    `OutputText` + `Reasoning` volume, counted **per turn** (reset on
    `TurnComplete`); **capture** = total captured-stdout bytes, **per run**
    (see F3).
  - exact unit(s) + default threshold are tunable consts, TBD like the Cluster B
    constants (straw man: trip at e.g. ~50k lines or ~32 MB per turn — high
    enough no honest single turn reaches it).
- **F3 — Streaming path gets the full detector; capture path gets Ctrl+C +
  volume cap (resolves Q16).** The line-assembling content detector (exit
  expressions + repetition) wires into `run_child_stream_semantic` only — where
  the incident occurred and where text is parsed live. `run_child_capture` is
  not given the full content detector in v1 (its lines are raw provider stdout,
  not cleanly assistant prose), but it **does** get: the unified Ctrl+C fix
  (Cluster D) and the **volume cap** applied to its growing capture `String`
  (which today accumulates unbounded — a real memory exposure on a runaway).
  Full capture-path content detection deferred.
- **F4 — Wall-clock `timeout` stays opt-in (resolves Q17).** No default
  wall-clock kill is shipped. The new guards (repetition + exit-expressions +
  volume cap) plus the Ctrl+C fix cover the incident class without a blanket
  time limit that would create a new false-positive class (slow-but-legitimate
  long runs). Volume is the better pathology signal than time.
- **F5 — Shortened non-interactive interrupt ladder (resolves Q14b): YES.**
  Non-interactive runs compress the ladder to **press 1 → SIGTERM** (still
  escalating to SIGKILL on a repeat); interactive runs keep the full
  SIGINT→SIGTERM→SIGKILL ladder (a human mid-session is protected from an
  accidental single press). On Windows the analog is press 1 → Ctrl+Break /
  graceful, repeat → `TerminateJobObject`/`TerminateProcess`.

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
  escalation, *and* the new content-trip channel from Parts 1–3 — making "a path
  that forgot Ctrl+C" structurally impossible. Must preserve the interactive-TUI
  passthrough case (`child_in_own_pgroup = false`, shared pgroup + TTY
  inheritance) so interactive providers (Claude/Codex) still receive terminal
  SIGINT naturally and don't hang on `SIGTTIN`.
- **Q14 — Visible interrupt feedback (accepted).** On each counted SIGINT, emit
  a visible stderr line (e.g. `⚠ interrupt received — press again to force-kill`)
  so a user during an output flood knows the press registered. The Q14b
  sub-item (shortened non-interactive ladder) is now resolved — see **F5**.
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

- **C1 — Distinct `error_kind` per guard (resolves Q1).** `exit_expression` and
  `runaway_repetition` (and later `runaway_volume`, added by **F2**).
  Independent of routing; appears honestly in JSONL logs / metrics regardless of
  the `ProcessTermination` chosen.
- **C2 — New `EarlyTermination` variant per guard.**
  `ExitExpression { pattern, scope }`, `RunawayRepetition { cycle_len, repeats }`
  (and later `RunawayVolume { lines, bytes }`, added by **F2**). Each carries the
  structured detail its message and the handler payload need; one clean arm each
  in `apply_early_termination_to_summary`.
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
and an optional `scope`. As assistant output streams, each reassembled completed
line is tested against every in-scope entry (per-line match target, E3d); the
first match terminates the run with `error_kind = "exit_expression"`.

### Configuration shape

Entries are declared across three layers (user config, repo config, frontmatter
— see [Layering](#layering-and-scope) below). A single entry looks like:

```jsonc
{
  // `pattern` (single) or `patterns` (array sharing this entry's scope/kind)
  "patterns": ["STOP.", "Bye."],
  "kind": "literal",        // "literal" (default) | "regex"; E3a
  "ignore_case": false,     // literal only; default false; E3c
  "scope": "opencode/kimi-for-coding/k2p7"  // optional; absent = global
}
```

User-config example (`~/.claudine/config.json`):

```jsonc
{
  "exit_expressions": [
    { "pattern": "I cannot continue", "kind": "literal" },          // global
    { "pattern": "FATAL: context exhausted", "scope": "opencode" }, // agent
    {
      "patterns": ["^(STOP|Bye)\\.$"],
      "kind": "regex",
      "scope": "opencode/kimi-for-coding/k2p7"                      // agent/model
    }
  ]
}
```

### Layering and scope

- **Three config layers** (E1): user → repo → frontmatter. Repo combines with a
  default mode of `override` (deterministic for all contributors); frontmatter
  combines with a default mode of `merge` (additive). Either layer may set its
  mode explicitly via the `{ mode, rules }` object form. Full resolution
  pipeline in the [Decisions Log → Cluster E](#cluster-e--exit-expression-config-shape-and-scoping-partially-resolved-2026-06-19).
- **`scope` syntax** (E2): `{agent}` | `{agent/model}`; absent = global. Parsed
  by splitting on the first `/` (agent = first segment, model = remainder
  verbatim). Within the resolved set, scoping is **additive**: a run is checked
  against the union of every entry whose `scope` matches (global ∪ agent ∪
  agent/model). Unknown agents fail at config-load validation. Model match is
  exact-string in v1.

### Matching semantics

- **Pattern kind** (E3a) — `literal` substring (default) or `regex`. Literal is
  the default to avoid metacharacter surprises (e.g. regex `STOP.` matching
  `STOPS`), in keeping with the feature's strong false-positive aversion.
- **Match unit** (A1/E3d) — each pattern is tested against a **completed,
  reassembled line**. The detector accumulates arbitrary streamed chunks into
  canonical lines (split on `\n`) before matching, so a pattern is never missed
  because the provider split it across chunks. Multi-line patterns are
  unsupported in v1.
- **Surfaces** (A2) — assistant `OutputText` + `Reasoning` text. Tool
  input/result payloads are **never** scanned (the model legitimately reads
  files / runs commands with repetitive output).
- **Case** (E3c) — regex uses inline flags (`(?i)`); literal honors the entry's
  `ignore_case` (default `false`).
- **Compilation** — the in-scope set is resolved and compiled **once** before
  streaming, never per line.

### Behavior on match

1. Stop feeding output to the renderer (avoid emitting more of the runaway).
2. Send `EarlyTermination::ExitExpression { pattern, scope }` through the unified
   wait loop → SIGTERM → SIGKILL escalation against the child process group
   (Windows: graceful → forceful).
3. Synthesize summary: `is_error = true`, `error_kind = "exit_expression"`,
   `error_message` naming the matched pattern and scope.
4. Map to `ProcessTermination::Aborted` (C3) → `FailureEvent::AgentFailure`, so
   the standard `failure:` handler runs and the composition loop is fail-fast —
   *without* triggering the `handle_timeout:` retry path. `error_kind` and the
   matched `pattern`/`scope` are threaded into the failure-handler payload (C3a).

## Part 2 — Repetition Heuristic

### Concept

Detect when the model emits the same content repeatedly. After
`MAX_REPETITION_ALLOWED` (`≈ 30`, B3) full cycles of the same block, terminate
with `error_kind = "runaway_repetition"`. On by default with a config
kill-switch (F1).

### Detection model (B1)

Operate on **completed, normalized lines** of assistant output (shares the
Cluster A detector with Part 1):

- Maintain a ring buffer of the last `~2K` normalized lines (`K ≈ 16`, B3).
  Lines are trimmed of trailing whitespace; blank lines are kept as normal empty
  lines (a blank-line flood is an `L = 1` cycle of `""` and still trips).
- **Group-cycle detection:** the repeating unit may be a single line *or* a
  block. The reproduced failure cycled a **6-line** block
  (`Done. / No more. / End. / STOP. / OK. / Bye.`, after a one-time
  `This is the final listening.` preamble). After each line, find the smallest
  period `L ∈ 1..=K` such that the last `2L` lines are two identical halves.
- Count **full cycles** (`consecutive_matching_lines / L`) so the threshold
  means the same thing for any `L`; reset when the cycle breaks. Trip when the
  full-cycle count ≥ `MAX_REPETITION_ALLOWED`.

**Matching is exact** on normalized lines (B2). Fuzzy/near-match is deferred —
known limitation: runaways with a moving part (incrementing counters,
timestamps) won't form an exact cycle and may slip through (the volume cap in
Part 3 is the backstop for those).

The high threshold is deliberate: a wrongful kill is a very negative outcome, so
the guard only trips on unmistakable runaways (a flooding model still hits 30
cycles in well under a second).

### Behavior on trip

Same termination path as Part 1, with
`EarlyTermination::RunawayRepetition { cycle_len, repeats }`,
`error_kind = "runaway_repetition"`, and `cycle_len`/`repeats` threaded into the
failure-handler payload (C3a).

## Part 3 — Volume-Cap Backstop

### Concept

Cycle detection only catches *structured* repetition up to length `K`. A runaway
that is acyclic, or whose period exceeds `K`, slips through. The volume cap is
the content-agnostic catch-all: if a single assistant turn emits more than a high
threshold of output, terminate with `error_kind = "runaway_volume"`. On by
default with a config kill-switch (F2).

Unlike a wall-clock timeout, volume measures the pathology directly and does
**not** punish slow-but-legitimate runs (a slow big refactor takes time but not
excessive volume) — which is why a default wall-clock `timeout` was *not*
adopted (F4).

### Detection model (F2)

- **Streaming path** — count assistant `OutputText` + `Reasoning` volume (lines
  and/or bytes) **per turn**, reset on `TurnComplete`. Straw-man defaults: trip
  at `≈ 50k` lines or `≈ 32 MB` per turn (tunable consts, high enough that no
  honest single turn reaches them).
- **Capture path** — `run_child_capture` accumulates all stdout into a single
  `String` with no cap today (a real memory exposure on a runaway). The volume
  cap is applied to that growing buffer **per run**, bounding the memory and
  terminating the child (F3).

### Behavior on trip

Same termination path as Parts 1–2, with
`EarlyTermination::RunawayVolume { lines, bytes }`,
`error_kind = "runaway_volume"`, and the volume detail threaded into the
failure-handler payload.

## Part 4 — Ctrl+C Correctness

### Gaps found during investigation (verified)

1. **`wait_with_timeout` installs no SIGINT handler.** Both production
   `run_child` and `run_child_capture` call sites pass `timeout_config.timeout`
   as the timeout arg, so when a wall-clock `timeout` is configured they wait via
   `exec/timeouts.rs::wait_with_timeout`, which only polls `try_wait` and the
   deadline — it never registers a SIGINT forwarder, and its timeout-kill targets
   the bare child PID, not the process group. `run_child_capture` always puts the
   child in its own process group, so a configured `timeout` makes terminal
   Ctrl+C reach **neither** the child (background group) **nor** a forwarder —
   **Ctrl+C is silently ineffective**, and claudine's default SIGINT disposition
   can kill claudine while the child survives orphaned. The irony: opting into
   the safety timeout disables Ctrl+C.
2. **Escalation is invisible.** Even on the good paths, guaranteeing a kill needs
   3 presses (SIGINT→SIGTERM→SIGKILL), and during an output flood the user gets
   no feedback that presses registered — exactly why the incident *felt* like
   Ctrl+C did nothing.

### Resolution (D, Q14, Q15)

- **Unify the wait paths (D).** Fold wall-clock `timeout` enforcement into the
  unified watchdog so **all** spawn paths wait via the single
  `wait_with_signal_and_early_termination` loop; retire `wait_with_timeout`. One
  loop owns signal handling, group-targeting (`-pid`), escalation, and the
  content-trip channel from Parts 1–3 — making "a path that forgot Ctrl+C"
  structurally impossible. The interactive-TUI passthrough case
  (`child_in_own_pgroup = false`, shared pgroup + TTY inheritance) must be
  preserved so Claude/Codex still receive terminal SIGINT naturally and don't
  hang on `SIGTTIN`.
- **Invariant.** Every wait path installs the SIGINT-forwarding handler, and
  every owned-process-group child receives **group-targeted** signals — for both
  user interrupts and timeout/guard kills.
- **Visible feedback (Q14).** Emit a stderr line on each counted interrupt
  (e.g. `⚠ interrupt received — press again to force-kill`).
- **Shortened non-interactive ladder (Q14b/F5).** Non-interactive runs:
  press 1 → SIGTERM directly (→ SIGKILL on repeat). Interactive runs keep the
  full SIGINT→SIGTERM→SIGKILL ladder.

### Windows — equally robust (Q15)

Not best-effort. The unified wait loop needs a real Windows implementation with
parity to the Unix group-signal/escalation behavior:

- Spawn the child in a new process group (`CREATE_NEW_PROCESS_GROUP`) and/or
  assign it to a **Job Object** so the whole tree terminates as a unit
  (`TerminateJobObject`) — the Windows analog of killing `-pid`.
- Handle console Ctrl+C via `SetConsoleCtrlHandler`; deliver interrupts to the
  child group via `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pgid)` (Ctrl+C
  cannot target a specific group; Ctrl+Break can).
- Map the ladder: Ctrl+Break (graceful) → `TerminateJobObject` /
  `TerminateProcess` (forceful), since Windows has no SIGTERM/SIGKILL split.
- Likely pulls in `windows`/`windows-sys` (confirm against workspace deps).
  **Testing caveat:** the dev host is macOS; Windows behavior must be designed
  carefully and validated in CI or on a Windows host — a known verification risk
  per the monorepo all-OS rule.

## Scope

### In scope

- **Config:** `exit_expressions` across three layers (user / repo / frontmatter)
  with per-layer combine mode and `scope` strings; repetition + volume
  kill-switches/threshold overrides. Validation (unknown agent, invalid regex)
  at config-load.
- **Pure detector** in `claudine/lib`: line-assembling, exit-expression matching
  + group-cycle repetition detection, with string-in/trip-out unit tests.
- **Wiring** of the detector into the `OutputText`/`Reasoning` path of
  `run_child_stream_semantic`, supplying resolved provider/model scope.
- **Volume cap** on the streaming path (per turn) and the `run_child_capture`
  buffer (per run).
- **Termination plumbing:** three new `EarlyTermination` variants, three new
  `error_kind`s, new `ProcessTermination::Aborted` (+ `classify_failure`
  mapping), and `error_kind`+guard-context threaded into the failure-handler
  payload (`AttemptOutcome`, env, JSON).
- **Ctrl+C hardening:** unify all spawn paths onto one signal-aware wait loop
  (retire `wait_with_timeout`); visible interrupt feedback; shortened
  non-interactive ladder.
- **Windows:** an equally robust Ctrl+C / group-kill implementation (Job Objects
  + console control events).
- **Tests:** detector unit tests, termination-mapping unit tests, and signal/
  Ctrl+C behavior via the existing real-process harness patterns; a documented
  spawn×wait matrix proof.
- **Docs:** `claudine/docs/topics/timeouts.md` (or a new sibling) + the claudine
  skill.

### Out of scope

- Full content detection (exit-expressions + repetition) on the captured-only
  path (`run_child_capture`) — it gets Ctrl+C + the volume cap only (F3).
- Multi-line exit-expression patterns; fuzzy/near-match repetition;
  model-only-any-agent scope (`*/model`); prefix/glob model matching. (All noted
  as deferred enhancements.)
- A default wall-clock `timeout` — stays opt-in; the volume cap is the backstop
  (F4).
- Repairing the underlying model behavior (this is a harness backstop, not a
  model fix).

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
| Q12 | default-on | ✅ RESOLVED (F1) — repetition on by default + kill-switch. |
| Q13 | path coverage | ✅ RESOLVED (D) — unify all spawn paths onto one signal-aware wait loop; retire `wait_with_timeout`. |
| Q14 | feedback | ✅ RESOLVED — visible interrupt feedback line (Q14b tracked separately below). |
| Q15 | Windows | ✅ RESOLVED — equally robust Windows path (Job Objects + console control events); not best-effort. |
| Q16 | capture paths | ✅ RESOLVED (F3) — streaming gets full detector; capture gets Ctrl+C + volume cap. |
| Q17 | default timeout | ✅ RESOLVED (F4) — wall-clock stays opt-in; volume cap is the backstop. |
| Q14b | nonint. ladder | ✅ RESOLVED (F5) — non-interactive ladder is SIGTERM-first; interactive keeps full ladder. |

## Success Criteria

- A configured exit expression (literal or regex, at the correct `scope` across
  all three config layers) matching streamed output terminates the child and
  reports `error_kind = "exit_expression"` with an honest message naming the
  pattern and scope.
- A synthetic runaway stream (the reproduced **6-line** cycle) trips the
  repetition guard at the threshold and terminates the child with
  `error_kind = "runaway_repetition"`; the high threshold means realistic
  repetitive-but-legitimate output does not trip it.
- An acyclic / over-`K` flood, and an unbounded capture-path buffer, trip the
  volume cap (`error_kind = "runaway_volume"`).
- All three trips map to `ProcessTermination::Aborted` → `FailureEvent::AgentFailure`
  (fail-fast, never the timeout-retry path, never `Interrupted`), and thread
  `error_kind` + guard context into the failure-handler payload.
- A matrix test (or documented proof) shows Ctrl+C terminates the child on every
  spawn/wait path combination on **both Unix and Windows**, including when a
  wall-clock `timeout` is configured.
- All new behavior is covered by unit tests; signal/Ctrl+C behavior uses the
  existing real-process harness patterns.
- `just test` and `just lint` pass in the claudine package area.
