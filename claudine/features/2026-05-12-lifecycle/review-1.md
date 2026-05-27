---
ready: false
agent: codex
model: ""
---

# Review: Lifecycle Formalization for Claudine Prompts

## Findings

### Critical: The formalized lifecycle event model is not implemented

**Location:** `claudine/lib/src/composition/lifecycle.rs:69`, `claudine/lib/src/composition/lifecycle.rs:83`, `claudine/lib/src/composition/lifecycle.rs:539`

The spec defines seven lifecycle events: `initialize`, `start`, `blocked`, `success`, `failure`, `loop`, and `next`. The implementation still only models the legacy four-event set: `start`, `success`, `blocked`, and `failure`. `LifecycleConfig`, `LifecycleSignal`, and `parse_lifecycle_config` all hard-code only those four events.

Consequences:

- `initialize` cannot run before pre-flight.
- `Skip`, `Proxy`, and `Error` at `initialize` cannot alter flow.
- `loop` lifecycle concerns cannot fire once per iteration.
- `next` handoff cannot be configured or executed.
- The spec's "full event set" has no runtime representation.

**Verification level:** no coverage exists for the missing events. These are mostly Level 1/CLI-observable flow requirements; `notify`/styled terminal rendering would also need Level 2 where rendered output is asserted.

### Critical: `stack` and lifecycle actions are absent

**Location:** `claudine/lib/src/composition/lifecycle.rs:34`, `claudine/lib/src/composition/lifecycle.rs:555`, `claudine/cli/src/commands/wrap/composition/mod.rs:1678`

The spec's central model is an ordered `stack:` of conditional actions, including `Stop`, `Skip`, `Error`, `Proxy`, `Retry`, `Resume`, `Requeue`, communication actions, side effects, expression functions, and shell actions. The current `LifecycleNotification` has only `say`, `say_first`, `effect`, `message`, `stderr`, and `notify`; `serde(deny_unknown_fields)` makes any `stack:` block fail parsing as an invalid lifecycle property.

Runtime execution also only emits the legacy notification fields through `LifecycleRunGuard`; there is no stack parser, no `when` evaluator, no action dispatcher, and no short-circuit behavior.

**Verification level:** no Level 1 tests cover parsing or executing stack items, lifecycle action short-circuiting, `when` conditions, shell action preflight, or per-action `no_error`.

### High: Required top-level communication properties and aliases are missing

**Location:** `claudine/lib/src/composition/lifecycle.rs:34`, `claudine/lib/src/composition/lifecycle.rs:555`

The spec says every event accepts `say`, `speak`, `effect`, `message`, `notify`, `stderr`, and `stdout`. The implementation accepts `say`, non-spec `say_first`, `effect`, `message`, `stderr`, and `notify`.

Gaps:

- `speak` alias is rejected.
- `stdout` is rejected.
- The spec says top-level properties execute "in the order written"; the implementation emits in a fixed internal order: stderr, message, notify, then audio phases.

This breaks spec-valid frontmatter and changes observable output ordering.

**Verification level:** existing tests are Level 1 parser/emitter tests only. Add Level 1 tests for accepted aliases and ordering, plus Level 2 coverage for terminal-rendered stdout/stderr behavior if styled output remains part of the user-facing contract.

### High: `loop` lifecycle concerns are rejected by the existing loop parser

**Location:** `claudine/lib/src/composition/loop_config.rs:57`, `claudine/lib/src/composition/loop_config.rs:113`

The spec repurposes `loop:` so iteration controls and lifecycle concerns share one block. The implementation's loop parser rejects every key outside `while`, `until`, `action`, `actions`, `max`, `fail_fast`, and `on_rate_limit`.

A spec-valid block like:

```yaml
loop:
  until: "phase > total_phases"
  say: "Phase {{phase}}"
  stack:
    - when: "_loop_is_first"
      action: notify
      message: "Loop started"
```

will fail before execution with an unknown `loop.say` or `loop.stack` error.

**Verification level:** loop iteration controls have Level 1 coverage elsewhere, but lifecycle concerns at the loop boundary have no coverage at any level. The per-iteration behavior should have Level 1 executor tests and CLI integration tests; any terminal rendering assertions should be Level 2.

### High: `next` handoff is not parsed or executed

**Location:** `claudine/lib/src/composition/lifecycle.rs:69`, `claudine/cli/src/commands/wrap/composition/mod.rs:1751`

The spec defines `next.suggest` and `next.push` handoff behavior for `compose`, `inline-compose`, `sequence`, `shell`, and direct `prompt` nodes. The implementation ends a successful non-harness composition by emitting `Success` and returning the outcome; there is no `next` model, mutual-exclusion validation, handoff target parser, or executor.

**Verification level:** no tests cover `next.push`, `next.suggest`, or handoff-node validation. `next.push` can be covered at Level 1/CLI integration; interactive `suggest` needs at least PTY-style Level 1 and possibly Level 2 if terminal rendering is part of the UX.

### High: Action error propagation and `no_error` are not implemented

**Location:** `claudine/lib/src/composition/lifecycle.rs:288`, `claudine/lib/src/composition/lifecycle.rs:763`

The spec defines event-specific propagation rules for side-effect, expression-function, and shell action failures, plus a universal `no_error: true` escape hatch. The current lifecycle emitter intentionally logs or drops notification side-effect errors and never returns an event outcome. Since stack actions do not exist, setup-phase failures cannot transition to `failure`, terminal-phase action failures cannot be contained by rule, and `no_error` cannot be honored.

**Verification level:** no tests cover action failure semantics. These should be Level 1 unit tests around a pure stack executor with fake actions, plus CLI integration around shell action failures.

## Verification Matrix

| Requirement | Strongest observed verification | Status |
|---|---:|---|
| Legacy `start` / `success` / `blocked` / `failure` notification parsing and guard transitions | Level 1: `cargo test -p claudine composition::lifecycle --color=never` passed 43 tests | Implemented for legacy subset |
| `initialize` event timing and flow control | None | Gap |
| `stack` parsing, `when`, and action short-circuiting | None | Gap |
| Lifecycle actions: `Stop`, `Skip`, `Error`, `Proxy`, `Retry`, `Resume`, `Requeue` | None | Gap |
| Communication aliases `speak` and `stdout` | None | Gap |
| Ordered top-level property execution | Level 1 fixed-order tests for current behavior only | Gap |
| Shell action preflight whitelist | None for lifecycle stack shell actions | Gap |
| Loop lifecycle concerns once per iteration | None; current parser rejects those keys | Gap |
| `next.suggest` / `next.push` handoff | None | Gap |
| Event-specific action error propagation and universal `no_error` | None | Gap |

## Production Readiness

Not ready for production.

The current code provides a tested legacy notification subset, but it does not implement the feature described by the lifecycle formalization spec. The main production blocker is not test depth; it is missing functionality. Once the stack/event model exists, the review bar should require Level 1 coverage for parser/executor flow, CLI integration for user-visible outcomes, and Level 2 coverage for any requirements that assert real terminal rendering of lifecycle output.
