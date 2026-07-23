# Claudine Timeouts

## Contents

- Overview
- timeout (wall-clock)
- steptimeout (stream-silence)
- Configuration sources and precedence
- Configuration knobs
- Defaults and rationale
- Frontmatter syntax
- Termination path
- Subagent diagnostics in error reports
- Disabling
- Content guards (runaway-output)
- OpenCode stalled-generation backstop
- Provider-specific stream variants
- Worked example: the OpenCode hang class
- Loop iteration failures: honest classification
- Loop iteration rate-limit handling

Use heading search to jump to the listed subsystem.


This is the canonical reference for the two timeout rules Claudine enforces
when wrapping a provider CLI (`claudine claude` / `codex` / `opencode` / ...
and the composition pipelines `compose`, `inline-compose`, `sequence`).
Other docs link here rather than duplicating definitions.

## Overview

Claudine enforces **exactly two timeouts**, both evaluated by a single
timeout ticker in the wrapper process:

| Name | What it measures | Built-in default |
|---|---|---|
| `timeout` | Wall-clock budget from child spawn | none (opt-in) |
| `step_timeout` | Stream-silence budget since the last parent-stream event | `30m` |

Both rules feed the same termination path: the ticker sends a
termination request into the wait loop, which sends `SIGTERM` to the child
process group, waits a 10-second grace period, then escalates to
`SIGKILL`. The synthesized `session_end` JSONL event records the breach as
`error_kind: "timeout"` or `"step_timeout"`. There is no third "subagent"
or "stream-idle" rule; any silence kill is a `step_timeout`, and
stuck-subagent detail is surfaced in the rendered error block, not in a
separate exit reason.

The two-rule design replaces the previous matrix of
`CLAUDINE_SUBAGENT_*` and `CLAUDINE_STREAM_IDLE_*` knobs with one
vocabulary that flows uniformly through CLI flags, markdown frontmatter,
and env-var defaults.

## `timeout` (wall-clock)

`timeout` is a hard ceiling on how long the wrapped child may run, measured
from the moment Claudine spawns it.

- **Formula.** The ticker fires when `now - started_at >= timeout`.
- **Resets.** Nothing resets it. Tool calls, subagent activity, and
  reasoning chunks all leave the wall-clock alone; the ceiling is absolute.
- **Default.** None. The rule is opt-in: if neither CLI, frontmatter, nor
  env supplies a value, no wall-clock kill ever fires.
- **Exit reason on breach.** `timeout`.

**Worked example.**

```
12:00:00  child spawned                   started_at = 12:00:00
12:00:30  --timeout 5m supplied           ceiling    = 12:05:00
12:04:59  any activity                    ceiling unchanged
12:05:00  ticker observes breach          SIGTERM sent
12:05:10  grace period elapsed            SIGKILL sent
12:05:10  session_end recorded            error_kind = "timeout"
```

## `step_timeout` (stream-silence)

`step_timeout` is a silence deadline on the parent-stream channel. As long
as the child keeps emitting structured events at a healthy cadence, the
silence clock keeps resetting and the ticker stays asleep.

- **Formula.** The ticker fires when `now - last_activity_at >= step_timeout`,
  the in-flight gate does not suppress, and one full activity signal has
  been observed (first-event grace). `last_activity_at` is the more
  recent of two clocks — the structured-event clock `last_event_at` and
  the raw-byte clock `last_byte_at` — so even a provider whose
  structured events lag behind real progress refreshes silence whenever
  bytes flow.
- **Resets.** Every parent-stream **activity event** advances
  `last_event_at`; every non-empty chunk of stdout/stderr bytes from the
  wrapped child advances `last_byte_at`. See
  [Activity vocabulary](#activity-vocabulary) below for the exact
  taxonomy.
- **First-event grace.** Until at least one activity signal has been
  observed on either clock, the rule cannot fire. Slow provider startup
  or a long first model response will not be killed.
- **In-flight gate.** When the structured stream reports in-flight tool
  calls (`in_flight`) or active subagents (`in_flight_subagents`), the
  rule is suppressed *unless* the in-flight item itself is stuck. See
  [Stuck-aware suppression](#stuck-aware-suppression).
- **Default.** `30m`. The default is intentionally large so that legitimate
  long-running tool calls (deep research, large test suites) finish without
  spurious kills. Overriding with a smaller value is the right choice when
  a session legitimately should not go silent for that long.
- **Exit reason on breach.** `step_timeout`.

### Activity vocabulary

Two independent clocks feed the silence reference. The ticker uses the
more recent of the two on every evaluation.

#### Structured-event clock (`last_event_at`)

A parent-stream event refreshes `last_event_at` if and only if it lands as
one of the following `SemanticEvent`
variants (the `SemanticEvent::is_activity()` predicate):

| Variant | Typical source |
|---|---|
| `OutputText` | Assistant text / `text` / `text_delta` / `assistant_text` |
| `Reasoning` | Provider reasoning chunks |
| `ToolCall` | A tool start (where the provider emits one) |
| `ToolResult` | A tool completion |
| `SubagentStart` | A subagent dispatch (where the provider emits one) |
| `SubagentStop` | A subagent completion |
| `FileChange` | A workspace edit notification |
| `PlanUpdate` | A todo/plan revision |
| `Info` | Step boundaries, tool progress markers, generic info lines |
| `Warning` | Provider-emitted warnings or malformed-line surfaces |
| `Error` | Terminal or non-terminal stream errors |
| `ProviderExtension` | Any unrecognised event the parser preserves verbatim |

The following variants are **envelopes** and do not reset the silence
clock: `SessionStart`, `TurnStart`, `TurnComplete`, `PermissionRequest`.

#### Raw-byte clock (`last_byte_at`)

Every non-empty chunk of stdout (and stderr for providers that emit
structured events on stderr) read from the wrapped child refreshes
`last_byte_at` **before** the bytes are handed to the semantic parser.
This is a provider-agnostic activity signal that protects against false
silence kills on providers whose structured stream is sparse enough that
`last_event_at` can lag behind real progress (notably OpenCode — see
[Provider-specific stream variants](#provider-specific-stream-variants)).

Bytes that contain no non-whitespace characters are ignored so a child
that flushes blank lines cannot look infinitely active. The heartbeat
fires at the byte-stream layer, so partially buffered output (provider
mid-flush) also refreshes the clock. A child that is truly stuck
producing zero bytes still allows `step_timeout` to fire normally; the
byte heartbeat does not mask genuine hangs.

### Stuck-aware suppression

The in-flight gate is **not** an absolute suppression. The ticker
classifies each in-flight tool or subagent as *active* or *stuck*:

- **Active.** `now - last_progress_at < step_timeout`. The tool/subagent
  has produced a recent progress event (or was started recently) and is
  presumed healthy.
- **Stuck.** `now - last_progress_at >= step_timeout`. The tool/subagent
  has not produced any progress within the silence budget.

Suppression rules:

1. If at least one in-flight item exists and **all** in-flight items are
   active, the silence rule is suppressed.
2. If any in-flight item is stuck, the silence rule is allowed to fire
   and the breach message enumerates the stuck items.
3. If no in-flight items exist, the silence rule is evaluated normally
   against `last_event_at` (this is the path most provider runs follow).

This suppression depends on the provider populating `in_flight` /
`in_flight_subagents`. See
[Provider-specific stream variants](#provider-specific-stream-variants)
for cases where a provider's stream does not feed this state and the
gate is effectively bypassed.

**Worked example (Claude-style provider with rich start/stop events).**

```txt
12:00:00  child spawned                                         last_event_at = 12:00:00
12:00:01  ToolCall: Task (rust-developer)                       last_event_at = 12:00:01
                                                                in_flight_subagents={B}
12:00:02  SubagentStart id=B                                    last_event_at = 12:00:02
                                                                B.last_progress_at = 12:00:02
12:00:03  ...stream goes silent while subagent works...
12:30:03  step_timeout budget met (30m)                         in_flight_subagents non-empty
                                                                B.last_progress_at age = 30m
                                                                B classified STUCK
                                                                step_timeout fires with stuck list
```

If the subagent had emitted any progress event during that window
(e.g., a `task_progress` Info line), `B.last_progress_at` would have
advanced and the gate would have continued to suppress.

## Configuration sources and precedence

The same value can be supplied at four layers. Resolution is **strict
top-down**: the first source that supplies a value wins.

| Priority | Source | Example |
|---|---|---|
| 1 (highest) | CLI flag | `--timeout 2h` / `--step-timeout 45s` |
| 2 | Markdown frontmatter | `timeout: 2h` / `step_timeout: 45s` |
| 3 | Env-var default | `CLAUDINE_TIMEOUT=2h` / `CLAUDINE_STEP_TIMEOUT=45s` |
| 4 (lowest) | Built-in default | `timeout=None`, `step_timeout=30m` |

All values use the existing duration grammar (`30s`, `5m`, `2h`).
Setting an env var (or CLI flag, or frontmatter) to `0s` **disables** the
rule for this run.

**Worked example: same prompt, different layers.**

```sh
# 1. Frontmatter only — kill after 30s of silence.
$ cat prompt.md
---
prompt: ...
step_timeout: 30s
---

$ claudine compose prompt.md

# 2. Env override — without changing prompt.md, raise to 5m.
$ CLAUDINE_STEP_TIMEOUT=5m claudine compose prompt.md
# Frontmatter `step_timeout: 30s` still wins. Env only fills in when
# frontmatter is absent.

# 3. CLI override — wins over both.
$ CLAUDINE_STEP_TIMEOUT=5m claudine compose --step-timeout 10s prompt.md
# Effective step_timeout = 10s.

# 4. Disable — env value of 0s disables the rule when no higher source sets it.
$ CLAUDINE_STEP_TIMEOUT=0s claudine compose prompt-without-frontmatter.md
# Effective step_timeout = None. No silence kill ever fires.
```

## Configuration knobs

### User-facing timeouts

| Env var | Default | Purpose |
|---|---|---|
| `CLAUDINE_TIMEOUT` | none | Wall-clock budget when neither CLI nor frontmatter supplies one. Duration string (`30s`, `5m`, `2h`). |
| `CLAUDINE_STEP_TIMEOUT` | `30m` | Stream-silence budget when neither CLI nor frontmatter supplies one. Duration string. |

`CLAUDINE_TIMEOUT` and `CLAUDINE_STEP_TIMEOUT` are the **only two**
user-facing timeout env vars. They map 1-for-1 to the `timeout` and
`step_timeout` frontmatter properties (with environment-style
capitalisation). There is no third user-facing knob in this namespace.

Both values use the same `parse_timeout` grammar as frontmatter, so
`30s`, `5m`, `2h`, and `0s` all parse the same way.

### Internal cadence

The ticker evaluates both rules at a 5-second cadence. After a breach,
the wait loop sends SIGTERM and then waits 10 seconds before escalating
to SIGKILL. These internals are intentionally not part of the
user-facing surface; they are constants from the user's perspective and
must not be promoted to a third public timeout concept.

## Defaults and rationale

- **Why `timeout` has no built-in default.** The wall-clock ceiling is a
  policy decision specific to the workload (interactive shell, batch CI
  job, overnight refactor). Choosing one as a default would either cut
  off legitimate long-runs or feel arbitrary; opt-in is safer.
- **Why `step_timeout` defaults to `30m`.** A modern agent commonly runs
  long single tool calls (deep research, large file edits, test suites
  that take many minutes). The point of `step_timeout` is to catch *true*
  silence — the provider is no longer producing any stream events at all,
  not the absence of assistant text in particular. The in-flight gate
  ensures that long-running subagents (which produce parent-stream silence
  by design) are never killed by this rule; the wall-clock `timeout` is
  the correct tool for bounding total session duration.

## Frontmatter syntax

Both timeouts may be set in markdown frontmatter using the same names and
grammar as the CLI flags and env vars.

```yaml
---
timeout: 2h           # opt-in wall-clock ceiling
step_timeout: 30m     # silence ceiling (matches the built-in default)
---
```

In `sequence` documents, the same fields apply to each composition step.

### Warning variants

Two companion frontmatter-only fields raise non-fatal warnings *before*
the corresponding hard threshold fires:

- **`timeout_warn`** — wall-clock warning. Fires once when
  `now - started_at >= timeout_warn`.
- **`step_timeout_warn`** — stream-silence warning. Fires once per stall
  episode when `now - last_event_at >= step_timeout_warn`.

Each `*_warn` value must be strictly less than its corresponding hard
threshold when both are present; `timeout_warn >= timeout` and
`step_timeout_warn >= step_timeout` are rejected at parse time. A `*_warn`
set without its corresponding hard threshold is legal — Claudine prints
the "no hard threshold" message variant and never terminates the run on
that warning alone.

```yaml
---
timeout: 2h
timeout_warn: 1h            # warn at 1h, kill at 2h
step_timeout: 30m
step_timeout_warn: 10m      # warn at 10m of silence, kill at 30m
---
```

See the [Composition](composition.md#timing-surface) Timing Surface
section for the exact wording of the warning lines.

## Termination path

When either rule fires, the ticker sends a termination request
into the same channel that the wait loop already monitors for early
termination signals (rate limits, completed-but-hung detection). The wait
loop then runs the standard escalation:

1. **SIGTERM** to the child process group.
2. **Grace period** (10 s).
3. **SIGKILL** to the child process group if it has not exited.

This reuses the existing `wait_with_signal_and_early_termination` plumbing
in `claudine/cli/src/commands/wrap/exec.rs`, so user-driven SIGINT
handling, rate-limit termination, and timeout-initiated termination all
share one path. User SIGINT (Ctrl+C) continues to surface as the existing
`exit_code: 130` flow — it is not a timeout event.

After termination, `apply_early_termination_to_summary` rewrites the
synthesised `session_end` JSONL event with the corresponding
`error_kind`:

| Reason | `error_kind` in summary |
|---|---|
| Wall-clock breach | `timeout` |
| Stream-silence breach | `step_timeout` |

A double-fire guard ensures only the **first** breach wins, even if both
rules would expire on the same tick.

## Subagent diagnostics in error reports

When `step_timeout` fires, the ticker snapshots any active subagents
from shared tracker state (populated by `SubagentStart` /
`SubagentStop` events on the structured stream) and includes them in the
rendered error block on stderr. The block is a colored `BlockQuote`
labelled `Step Timeout` (rendered through `SemanticEvent::Error` with
`SemanticErrorKind::AgentNative` so it shares the same red border as
other agent-native errors), enumerating each outstanding subagent's id,
name, and elapsed time since its last progress event.

In addition, the 30-second `flush_if_idle` ticker (which already exists
to flush dangling assistant prose during long silences) emits at most
**one** diagnostic line per active subagent per silence window:

```
 ⏳ Awaiting subagent: <name-or-id> (<elapsed-since-start>)
```

These lines route through the same `SectionTracker` and Tool Use & Events
section as tool-call rendering so the spacing matches the rest of the
live stderr surface. The diagnostic emission is gated on `step_timeout`
being enabled — disabling the silence rule
(`CLAUDINE_STEP_TIMEOUT=0s`) also suppresses the awaiting-subagent
diagnostic.

## Disabling

Both rules are individually disablable by supplying `0s` at any layer of
the precedence chain:

```sh
# Disable the silence rule for this run only.
claudine compose --step-timeout 0s prompt.md

# Disable the wall-clock rule globally for the current shell.
export CLAUDINE_TIMEOUT=0s
```

Frontmatter can also disable a rule explicitly:

```yaml
---
timeout: 0s          # disable wall-clock kill
step_timeout: 0s     # disable silence kill
---
```

Omitting a field falls through to the next layer (frontmatter → env →
built-in default), so simply leaving `timeout` out of frontmatter does
**not** disable it — that just means "use whatever lower-priority source
is configured."

## Content guards (runaway-output)

The two timeout rules above are *time*-driven: they catch a child that has
gone **silent**. They cannot catch the opposite failure — a child that
floods the stream with degenerate output and never stops. A
non-interactive run that enters a tight token-level repetition loop keeps
the byte heartbeat alive (bytes are flowing), so `step_timeout` never
fires, and `timeout` is opt-in. The **content guards** are the
*volume*-driven backstop for exactly this class.

Three guards scan the typed semantic stream — `OutputText` and
`Reasoning` text only, **never** tool-call / tool-result payloads — as it
flows:

| Guard | `error_kind` | Trips when | Built-in default | Kill-switch |
|---|---|---|---|---|
| Exit expression | `exit_expression` | A completed line matches a user-authored literal/regex pattern in scope | none declared (off) | omit / empty list |
| Runaway repetition | `runaway_repetition` | A group-cycle of length `L ≤ 16` repeats `≥ 30` full times | on (30 cycles) | `guard_settings.repetition.enabled: false` |
| Runaway volume | `runaway_volume` | A turn exceeds `50_000` lines **or** `32 MiB` | on | `guard_settings.volume.enabled: false` |

The repetition guard counts the smallest repeating group, so single-line
spam is just the `L = 1` case. Volume is counted **per turn** on the
streaming path (reset on `TurnComplete`) and **per run** on the
capture path; the capture path gets *only* the volume cap plus Ctrl+C,
not exit-expression or repetition detection.

### Aborted, not timed out

All three guards converge on the same SIGTERM → SIGKILL plumbing the
timeouts use (see [Termination path](#termination-path)), but they
synthesize a distinct termination:
`ProcessTermination::Aborted`, **not**
`TimedOut`. This is deliberate — `Aborted` maps to
`FailureEvent::AgentFailure` (fail-fast), so a guard trip **never** takes
a `failure`-stack `Retry` path that would re-run the provider and
reproduce the runaway. It is also never `Interrupted` (which would
suppress failure handling like a user cancel). The honest per-guard
`error_kind` is carried through the synthesized `session_end` summary, so
a lifecycle `failure`/`finalize` stack can branch on the `err` global
(`runaway_repetition` versus a generic `agent_failure`).

| Termination | `error_kind` | Failure event | Recovery path |
|---|---|---|---|
| `TimedOut` | `timeout` / `step_timeout` | `Timeout` | `failure` stack `Retry`/`Resume` |
| `Aborted` | `exit_expression` / `runaway_repetition` / `runaway_volume` / `repeated_stream_error` / `stalled_generation` | `AgentFailure` | none (fail-fast) |

`repeated_stream_error` is an OpenCode-specific stderr backstop: consecutive
`message="stream error"` records crossing `MAX_CONSECUTIVE_STREAM_ERRORS` (5)
with no step advance abort the run, bounding OpenCode's unbounded backoff
retries when the provider fails every attempt. It is fail-fast (`Aborted`),
never a `failure`-stack `Retry` that would reproduce the failure loop.

`stalled_generation` is the OpenCode **live-but-dead** backstop — see
[OpenCode stalled-generation backstop](#opencode-stalled-generation-backstop)
below. It is likewise fail-fast (`Aborted`), never a `failure`-stack `Retry`.

### Configuration surface

`exit_expressions` is declared per layer (user config, repo
`.claudine` config, and document frontmatter) and accepts either a bare
array or an explicit `{ mode, rules }` object. Each entry takes a single
`pattern:` or a `patterns:` array, an optional `kind: literal | regex`
(default `literal`, to avoid metacharacter surprises like the regex
`STOP.` matching `STOPS`), an optional `ignore_case` (literal-only), and
an optional `scope` (`{agent}` or `{agent}/{model}`; absent = global).
Scopes are **additive** — a run is checked against the union of every
matching entry. Invalid regex, an unknown agent in a `scope`, and an
empty `patterns` are rejected at config-load, never mid-stream.

```yaml
---
exit_expressions:
    - pattern: "I have completed the task"     # literal, global
    - patterns: ["FATAL", "unrecoverable"]
      kind: regex
      scope: opencode/kimi-for-coding/k2p7     # only this agent+model
guard_settings:
    repetition:
        enabled: true
        max_repeats: 30        # full cycles before trip
        max_cycle_length: 16   # largest group L the detector recognizes
    volume:
        enabled: true
        max_lines: 50000       # per-turn line cap
        max_bytes: 33554432    # per-turn byte cap (32 MiB)
---
```

The repetition/volume scalar settings use last-writer precedence
(frontmatter > repo > user > built-in), like `timeout` / `step_timeout`.
Only the list-typed `exit_expressions` carries a per-layer combine mode
(repo defaults to `override`, frontmatter to `merge`).

### False-positive posture

The thresholds are deliberately conservative — a *wrongful* kill of an
honest run is the worst outcome, far worse than letting a real runaway
stream a few thousand extra lines before the cap. Repetition uses
**exact** line equality (no fuzzy matching) and requires 30 full cycles;
the volume cap sits at 50k lines / 32 MiB. Do not tune these down
without a real false-positive incident. The wall-clock `timeout` remains
opt-in and unchanged; the volume cap is the always-on content backstop
that bounds the unbounded capture buffer even when no timeout is set.

## OpenCode stalled-generation backstop

This is **not** a third general timeout rule — `timeout` and `step_timeout`
remain the only two (see [Overview](#overview)). The stalled-generation
backstop is an **OpenCode-scoped** guard for a failure shape neither timeout
catches: a *live-but-dead* run where the provider keeps retrying a dropped
generation. OpenCode re-emits a `service=llm ... stream` (`llm_call_start`)
record on every retry, and those retries keep the byte heartbeat alive, so
`step_timeout` never fires — yet no assistant text, reasoning, tool call, or
step advance is ever produced. The run is "alive" on the wire but
producing nothing.

The guard keys on a **retry-churn fingerprint** and trips only when **both**
conditions hold on the same `llm_call_start`:

1. **Retry churn.** The count of streamed `LlmCall` (`is_stream == true`)
   records since the last progress event is
   `>= MAX_GENERATIONS_WITHOUT_PROGRESS` (a constant, `4`).
2. **Progress silence.** `now - last_progress_at >= stall_timeout`
   (built-in default `10m`).

Either condition alone never fires. The count condition is what makes the
guard safe against a single legitimately-slow generation: one slow first
call past the silence budget does not trip, because the retry count has not
accumulated. The two conditions are also **anti-correlated** with healthy
output — a run streaming assistant text or advancing steps resets the count,
so it cannot reach the churn threshold. A long tool producing **no**
`llm_call_start` records at all never trips this guard, even past
`stall_timeout`.

Progress events that reset the count and advance `last_progress_at` come from
**both** producers, which share one progress cell. On the stderr bridge: a
genuine `StepLoop` advance, `StepExit`, and subagent lifecycle (`SubagentStart`
/ `SubagentStop`). On the stdout NDJSON stream (a separate reader thread): every
progress-class semantic event — `OutputText`, `Reasoning`, `ToolCall`,
`ToolResult`, `SubagentStart`, `SubagentStop`, `FileChange`, `PlanUpdate` — via
a stdout progress observer wired to the same cell. This matters because the
stderr bridge never sees stdout events; without the stdout-side reset a run that
made real stdout progress could still trip on a later `llm_call_start`.
Liveness-only events do **not** reset on either producer: another
`llm_call_start`, a deduped/repeated `StepLoop` for the same `(session_id,
step)`, `http_response`, `permission_evaluated`, `service=bus` lines, raw bytes,
and the stdout `Info`/`Warning`/`Error`/session-turn-envelope events.

On trip the guard emits a terminal `SemanticEvent::Error`
(`SemanticErrorKind::AgentNative`, label **"Stalled Generation"**) and routes
to `ProcessTermination::Aborted` with `error_kind = "stalled_generation"`. It
is **fail-fast** — never `TimedOut`, so it never takes a `handle_timeout:` /
`failure`-stack `Retry` path that would re-launch the provider and reproduce
the stall. The synthesized `session_end` summary carries `generation_count`
and `stall_duration_ms` plus available OpenCode metadata (session id, step,
agent, provider id, model id, mode); it never stores prompt text or tool
payloads.

### Configuration

| Layer | Surface | Notes |
|---|---|---|
| CLI flag | `--stall-timeout <DURATION>` | wrapper and compose; highest priority |
| Env default | `CLAUDINE_OPENCODE_STALL_TIMEOUT` | duration string |
| Built-in | `10m` | `Duration::from_secs(10 * 60)` |

Precedence is CLI > env > built-in, and the same duration grammar as the two
general timeouts applies. `0s` from the CLI or environment **disables** the
guard for that run. The OpenCode-specific guard is intentionally not exposed
in Markdown frontmatter. There is no `stall_timeout_warn` companion.

## Provider-specific stream variants

Claudine's `step_timeout` rule and its in-flight gate both depend on the
wrapped provider emitting structured events at a useful cadence. Provider
streams differ substantially in event richness, and those differences
shape how the silence rule behaves in practice.

The table below summarises which event surfaces each provider's parser
populates today (entries marked **(parsed)** route into the activity
clock and the in-flight gate; entries marked **(silent)** mean the
provider never emits the event so the corresponding code path stays
inert).

| Provider | Tool start (`tool_start` → `ToolCall`) | Tool finish (`tool_end` / `tool_use` → `ToolResult`) | Subagent start (`task_started` → `SubagentStart`) | Subagent finish (`task_completed` → `SubagentStop`) | Text deltas (`OutputText`) | Reasoning chunks (`Reasoning`) |
|---|---|---|---|---|---|---|
| Claude Code | parsed | parsed | parsed | parsed | parsed | parsed |
| Codex | parsed | parsed | parsed | parsed | parsed | parsed |
| Gemini | parsed | parsed | (n/a — Gemini does not expose subagents) | (n/a) | parsed | parsed |
| Goose | parsed | parsed | (n/a) | (n/a) | parsed | parsed |
| Kimi Code | parsed | parsed | (n/a) | (n/a) | parsed | parsed |
| OpenCode | **silent** | parsed | **stderr-promoted** | **stderr-promoted** | parsed | parsed |
| Qwen Code | parsed | parsed | (n/a) | (n/a) | parsed | parsed |

The OpenCode row is the structurally important one and is detailed
below. "stderr-promoted" entries mean the event is not on stdout NDJSON
but is reconstructed from OpenCode's structured stderr stream — see
[OpenCode Event Sources](../../../.claude/skills/claudine/opencode-event-sources.md)
for the full signal-to-event mapping.

### OpenCode

OpenCode's CLI streams its `tool_use` and `task_completed` events
**only after the tool or subagent has reached `completed` / `error`**.
There is no paired request-side event on the wire; per the parser docs,
"OpenCode emits `tool_use` only after the tool has reached `completed`
/ `error`. OpenCode does not emit a paired request-side event, so we
emit only a `ToolResult` (no synthesized `ToolCall`)." The
`task_started` event variant is recognised in the parser but is not
emitted in practice by current OpenCode releases.

The functional consequence: during OpenCode runs,
`LiveMetricsState.in_flight` and `in_flight_subagents` are **never
populated from stdout NDJSON**. The in-flight gate from
[Stuck-aware suppression](#stuck-aware-suppression) is a no-op for the
stdout source on OpenCode. **Four** complementary mechanisms compensate
so legitimate work is not misclassified as a hang and the silence-rule
diagnostic still has signal to report. Mechanisms 1–3 keep the silence
clock honest; mechanism 4 reconstructs lifecycle visibility from a
second source:

1. **Raw-byte heartbeat.** The byte-stream clock `last_byte_at` (see
   [Raw-byte clock](#raw-byte-clock-last_byte_at)) refreshes whenever
   the wrapped child writes any non-whitespace bytes, including
   partially-buffered output that has not yet parsed into a structured
   `SemanticEvent`. This is the provider-agnostic protection that
   covers OpenCode-style sparse streams.
2. **Per-step `provider_status` grace.** The silence rule is
   suppressed **while an OpenCode step is in flight** — from
   `step_start` until the matching `step_finish` — because mid-step
   silence is expected on this provider. The grace resets for every
   new step, so multi-step flows that dispatch subagents mid-stream
   are protected throughout the entire session, not just during the
   first step. The wall-clock `timeout` rule is **not** suppressed; it
   remains the unconditional backstop. The guard fires only for
   OpenCode; richer-stream providers do not need it.
3. **Synthesized subagent lifecycle (legacy path, removed).** Earlier
   releases synthesized `SubagentStart` → `SubagentStop` from the
   `task` `tool_use` payload at completion time. That path was removed
   on 2026-05-12 (fix: `2026-05-12-opencode-stderr-returns`) in favor
   of the stderr-promoted lifecycle below; it is documented here only
   to clarify why the NDJSON parser no longer emits subagent events
   for `task` completions.
4. **Stderr-promoted activity and lifecycle.** Claudine opts the
   OpenCode wrapper into the structured stderr stream
   (`--print-logs --log-level INFO`) and classifies INFO log records
   through `OpenCodeLogBridge`.
   Boot, `service=session` (parent + child), `service=llm` LLM calls,
   `service=session.prompt` step loops and exits, `service=permission`
   evaluations, and `service=default` HTTP responses are promoted to
   `SemanticEvent` variants. Every promoted event is in
   `SemanticEvent::is_activity()` so the structured-event clock
   (`last_event_at`) advances throughout long NDJSON silences, and
   subagent lifecycle is reconstructed by emitting `SubagentStart`
   when a `parentID`-bearing session is created and `SubagentStop`
   when its `service=session.prompt ... exiting loop` closure
   arrives. `service=bus` lines are filtered before classification —
   they refresh the byte heartbeat only. See
   [OpenCode Event Sources](../../../.claude/skills/claudine/opencode-event-sources.md)
   for the full mapping table and dedup rules.

This matters most for two flow shapes:

1. **A long-running tool (e.g. a `bash` command running a test suite)
   inside a single OpenCode turn.** OpenCode does not stream a
   `tool_start` for the bash invocation, so claudine cannot register
   the tool as in-flight. The byte heartbeat refreshes whenever the
   bash tool produces output; the silence clock fires only if the
   tool itself goes truly quiet for `step_timeout`.
2. **A `task`-tool fan-out followed by a long synthesis turn.** Once
   the parallel `task_completed` events stop arriving, the parent
   model may spend significant wall-clock time composing its closing
   response. The byte heartbeat refreshes whenever the model streams
   any reasoning or text bytes — even before they parse into a
   `SemanticEvent` — so post-fan-out synthesis turns are no longer
   misclassified as silence.

Recommendations for OpenCode workloads:

- Set `step_timeout` more generously than for Claude/Codex on
  comparable workloads. A rule of thumb is 2–3× the value you would
  use for a richer-stream provider performing the same task.
- Prefer the wall-clock `timeout` rule over a tight `step_timeout` for
  bounding total session duration; the wall-clock rule is independent
  of stream richness.
- For tool-heavy commit/synthesis flows, `step_timeout: 30m` (the
  built-in default) is usually appropriate even when shorter values
  would be safe on Claude or Codex.

## Worked example: the OpenCode hang class

The reference incident that motivated the unified `step_timeout` was an
OpenCode session that issued multiple parallel `task` subagents,
observed each `task_completed` event, and then went completely silent
during the closing synthesis. The wrapped process never exited and
Claudine had no diagnostic surface to terminate the session.

With the unified `step_timeout`, OpenCode's actual stream surface, the
byte heartbeat, and the per-step `provider_status` grace:

```
12:00:00  claudine compose ... (step_timeout = 30m)
12:00:01  step_start observed                       step_in_flight = true
                                                     silence rule suppressed
12:00:02  parent text: "Launching N subagents..."   last_event_at = 12:00:02
                                                     last_byte_at  = 12:00:02
                                                     in_flight_subagents = {}  (no task_started)
12:14:32  N × task_completed observed               last_event_at = 12:14:32
                                                     last_byte_at  = 12:14:32
                                                     subagent_done_count = N
12:14:33  step_finish observed                      step_in_flight = false
                                                     silence rule re-enabled
12:14:34  parent text: "All N succeeded..."         last_event_at = 12:14:34
                                                     last_byte_at  = 12:14:34
12:14:35  ...closing synthesis streams reasoning bytes (no parsed event)...
                                                     last_byte_at  advances on each chunk
                                                     last_event_at remains stale
                                                     silence = now - last_activity_at < 30m
                                                     → no kill

# A genuinely stuck child (zero bytes for 30m) would fire normally:
13:14:35  no bytes, no events for 30m              step_timeout fires
13:14:35  SIGTERM → SIGKILL (10s grace)            session_end → error_kind: "step_timeout"
```

Note the contrast with the Claude-style worked example earlier: the
breach message for OpenCode includes the count of subagents observed
in the current step and the elapsed time since the last completion,
so a timeout during a long subagent fan-out is no longer misreported
as "no outstanding subagents". The byte heartbeat distinguishes
between *the model is silently composing* (bytes still flowing) and
*the wrapped process is genuinely stuck* (no bytes at all).

To kill the same hang faster during incident triage, drop the silence
budget for the next run:

```sh
CLAUDINE_STEP_TIMEOUT=2m claudine compose ...
```

## Loop iteration failures: honest classification

When `claudine compose --loop` runs across multiple iterations and the
**inner provider exits non-zero**, the top-level error reports the
*actual* cause — `step_timeout`, `wall-clock timeout`, signal, or a
plain provider exit — pulled from the iteration's session_end JSONL
row (`extra.exit_reason`). The phrase `invalid loop definition` is
reserved for malformed `loop:` frontmatter and **never** appears for
runtime failures.

```text
Error: loop iteration 2 of fixes/.../plan.md: step_timeout (exit code 1)
       ↳ no stream activity for 30m 0s; terminating due to step_timeout
```

Practically: if a long `cargo test` (or any tool) inside an iteration
trips the silence watchdog, the resulting error names the silence rule
that fired — not the loop frontmatter. Loop-config validation errors
still surface as `LoopInvalid` as before.

## Loop iteration rate-limit handling

When a completed iteration emits a provider rate-limit trailer
(`summary.rate_limit.is_throttled = true`), the loop engine consults
the **`on_rate_limit` policy** between iterations:

| Policy     | Behavior                                                                                                                  |
|------------|---------------------------------------------------------------------------------------------------------------------------|
| `pause`    | Default. Sleep until `reset_at` + 5s safety margin, then run the next iteration. If `reset_at` is missing or already past, falls back to `abort` to avoid an unbounded sleep. The pause is interruptible by Ctrl+C. |
| `abort`    | Halt the loop with a structured `LoopRateLimited` error. Exits with code `75` (`EX_TEMPFAIL` from `sysexits.h`) so shell wrappers can distinguish a transient rate-limit halt from a generic non-zero exit. |
| `continue` | Run the next iteration immediately, ignoring the trailer. Reserved for soft per-request limits that won't recur; not recommended as a default. |

Set the policy at the document level via `loop.on_rate_limit:` in
frontmatter, or override per run with `--on-rate-limit <pause|abort|continue>`.
CLI > frontmatter > default precedence applies, identical to
`--max-iterations` and `--step-timeout`.

The `pause` safety margin defaults to `5s` but can be overridden with the
`CLAUDINE_PAUSE_RESET_MARGIN` environment variable (duration string, e.g.
`10s`, `0.5s`); invalid values fall back to the built-in default.

```yaml
---
loop:
    until: "phase > total_phases"
    action: increment(phase)
    on_rate_limit: pause   # default; values: pause | abort | continue
---
```

The check runs against every completed iteration regardless of exit
code — including successful ones — because providers commonly attach
rate-limit trailers after a successful completion summary. On the very
last iteration the policy is skipped (the loop is about to exit
anyway, so pausing or aborting would be a false positive).
