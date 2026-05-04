# Claudine Timeouts

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

- **Formula.** The ticker fires when `now - last_event_at >= step_timeout`
  **and** no tool calls or subagents are currently in-flight.
- **In-flight gate.** When the structured stream reports in-flight tool
  calls (`in_flight`) or active subagents (`in_flight_subagents`), the
  rule is suppressed entirely. A long-running Task/subagent call produces
  parent-stream silence by design while the child works; the wall-clock
  `timeout` rule serves as the backstop for truly stuck tool calls.
- **Resets.** Every parent-stream event resets `last_event_at`: tool
  calls, tool results, reasoning chunks, info/warning lines, assistant
  text deltas, and recognized subagent progress events.
- **Default.** `30m`. The default is intentionally large so that legitimate
  long-running tool calls (deep research, large test suites) finish without
  spurious kills. Overriding with a smaller value is the right choice when
  a session legitimately should not go silent for that long.
- **Exit reason on breach.** `step_timeout`.

**Worked example.**

```txt
12:00:00  child spawned                                         last_event_at = 12:00:00
12:00:01  tool_use: Task (starts rust-developer subagent)     last_event_at = 12:00:01
12:00:02  task_started: id=B                                   last_event_at = 12:00:02
12:00:03  ...stream goes silent while subagent works...
12:30:03  step_timeout budget met (30m)                        IN-FLIGHT GATE: suppressed
                                                                     (in_flight_subagents non-empty)
...subagent continues working...
13:00:00  task_completed: id=B                                  last_event_at = 13:00:00
13:00:01  tool_result: Task                                     in_flight cleared
13:00:01  parent agent continues with subagent output
```

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

Both values use the same `parse_timeout` grammar as frontmatter, so
`30s`, `5m`, `2h`, and `0s` all parse the same way.

The ticker evaluates both rules at a 5-second cadence. After a breach,
the wait loop sends SIGTERM and then waits 10 seconds before escalating
to SIGKILL. These internals are not configurable.

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

## Worked example: the OpenCode hang class

The reference incident that motivated this design was an OpenCode session
that issued nine parallel `task` subagents, observed seven
`task_completed` events, and then went completely silent. The wrapped
process never exited and Claudine had no diagnostic surface to terminate
the session.

With the unified `step_timeout`:

```
12:00:00  claudine compose ... (default step_timeout = 30m)
12:00:01  9 × task_started observed                last_event_at = 12:00:01
12:14:32  7 × task_completed observed              last_event_at = 12:14:32
12:14:33  ...silence...

# At 12:30:00 (silence_window default 30s) the flush_if_idle ticker emits:
 ⏳ Awaiting subagent: <id-A> (16m)
 ⏳ Awaiting subagent: <id-B> (14m)

# At 12:44:33 (last_event_at + 30m) the ticker fires:
> Step Timeout
> No stream activity for 30m. Outstanding subagents:
>   - <id-A> (<name-A>) — 30m since last progress
>   - <id-B> (<name-B>) — 30m since last progress

# Termination path:
12:44:33  SIGTERM
12:44:43  SIGKILL (10s grace period)
12:44:43  session_end → error_kind: "step_timeout"
```

The synthesized summary records the breach so downstream reporting (`claudine
logs`) can attribute the hang to a stream-silence kill rather than a
generic non-zero exit.

To kill the same hang faster during incident triage, drop the silence
budget for the next run:

```sh
CLAUDINE_STEP_TIMEOUT=2m claudine compose ...
```
