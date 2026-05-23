# Lifecycle Formalization for Claudine Prompts

## The Prompt Lifecycle

![lifecycle](./lifecycle.png)

A Claudine prompt document moves through a fixed set of lifecycle events. Today four of them exist as `LifecycleSignal` variants in `composition/lifecycle.rs` (`start`, `success`, `blocked`, `failure`), but they only support a handful of communication properties. This feature formalizes the full event set, introduces a unified per-event configuration model, and folds two pre-existing composition concerns (`loop`, `next`) into that model.

### Event Inventory

| Event | New? | Fires when |
|---|---|---|
| `initialize` | **new** | Prompt file has been identified, before any pre-flight checks have run |
| `start` | existing | Pre-flight checks have all passed; about to invoke the agent |
| `blocked` | existing | Pre-flight checks failed; agent will not be invoked |
| `success` | existing | Agentic loop completed without error |
| `failure` | existing | Agentic loop returned an error |
| `loop` | **repurposed** | An iteration of a looping prompt boundary (see [Loop](#loop)) |
| `next` | **new** | Composition completed successfully and a handoff is configured |

### Event Purpose

- **`initialize`** — quick communication, environment prep, or opt-out before the prompt commits to running. Cheap, runs even when pre-flight will fail.
- **`start`** — best moment to announce *this* prompt is running. Pre-flight has passed; the agent will be invoked next.
- **`blocked`** — communicate why the prompt was rejected; optionally proxy to a different prompt.
- **`success`** — communicate completion, capture metrics, fire webhooks, advance a sequence.
- **`failure`** — communicate failure, recover with `Handle`, optionally proxy.
- **`loop`** — per-iteration boundary inside a looping prompt; layers lifecycle-event behavior on top of the existing iteration controls.
- **`next`** — handoff to another document on successful completion.

## Configuration Model

Every lifecycle event is configured under a frontmatter key matching its name and shares the same configuration shape.

```yaml
start:
    # Legacy communication properties (backward compatible)
    say: "Starting build for {{ctx.repo}}"
    notify: "Build starting"
    effect: small-group-cheer

    # The stack: ordered list of conditional actions
    stack:
        - when: "env.AGENT == 'claude'"
          action: say
          message: "Using Claude provider"
        - "shell(git status --short)"
```

### Top-Level Communication Properties

Each event accepts the following shorthand properties at its top level. They are unconditional and execute **before** the stack, in the order written.

| Property | Purpose |
|---|---|
| `say`, `speak` | TTS via host's speech provider |
| `effect` | Sound effect from the embedded library |
| `message` | Send via configured messenger route (Slack, Discord, WhatsApp, Signal, Telegram) |
| `notify` | OS desktop notification (replaces the deprecated `desktop:` alias) |
| `stderr` | Plain string to STDERR |
| `stdout` | Plain string to STDOUT |

All values support [Darkmatter interpolation](@darkmatter/docs/topics/darkmatter-expressions.md).

### The Stack

`stack:` is an **ordered list, processed top to bottom.** Each element is a `LifecycleStackItem`:

```rust
pub struct LifecycleStackItem {
    pub when: Option<Expression>,  // omitted == always true
    pub action: ActionRef,
}
```

- `when:` is a [Darkmatter conditional expression](@darkmatter/docs/topics/darkmatter-expressions.md). When omitted, the item always executes.
- `action:` is exactly one action (see [Actions](#actions)). If you want multiple effects, write multiple stack items.

### Stack Processing

```text
For each stack item, top to bottom:
  evaluate `when` →
    false: skip (no-op)
    true:  execute action
      • communication, side effect, expression call, shell → continue to next item
      • lifecycle action (Stop/Skip/Error/Proxy/Handle) → stop processing this event

If the stack runs to completion without a lifecycle action matching, the event ends normally.
```

## Actions

An action is one of five categories. Most accept a **short form** (string) and a **long form** (mapping with `action:` key plus parameters).

### Lifecycle Actions

These actions change what happens next. The first one whose `when:` matches **terminates stack processing** for the current event.

| Action | Where valid | Effect |
|---|---|---|
| `Stop` | every event | End this event's stack cleanly. Composition continues. |
| `Skip` | `initialize` only | Mark the prompt as skipped without running it. Pre-flight does not run. Emits an `INFO` log line indicating the skip. In a sequence, advances to the next step. |
| `Error` | every event | Mark this event as failed with a reason. At `start`/`initialize` this prevents agent invocation; at `success` it converts the outcome to failure. |
| `Proxy` | `initialize`, `blocked`, `failure` | Hand off execution to another prompt document. (Re-entry semantics: see [Open Design Gaps](#open-design-gaps)). |
| `Handle` | `blocked`, `failure` | Recover via a sub-action (`retry`, `resume`, `proxy`, `requeue`). (Sub-action shape: see [Open Design Gaps](#open-design-gaps)). |

Short forms: `"stop"`, `"skip"`, `"proxy(@prompts/foo.md)"`, `"error(\"reason\")"`.

### Communication Actions

The same channels as the top-level properties, but each is a discrete stack item (so it can be guarded by `when:`).

| Action | Short form | Long form parameter |
|---|---|---|
| `say` / `speak` | `say("hi")` | `message:` |
| `effect` | `effect(applause)` | `sound:` |
| `message` | `message("done")` | `message:` (optionally `route:` to pin to a specific channel) |
| `notify` | `notify("done")` | `message:` |
| `stderr` | `stderr("warn")` | `message:` |
| `stdout` | `stdout("ok")` | `message:` |

```yaml
success:
    stack:
        - when: "env.AGENT == 'claude'"
          action: say
          message: "Claude finished"
        - action: effect
          sound: applause
```

### Side-Effect Actions

Mutating operations (file creation, frontmatter updates, HTTP POSTs, etc.) are provided by the Darkmatter side-effects system introduced in [`more-context-variables`](@darkmatter/features/_unscheduled/more-context-variables/spec.md). The lifecycle stack invokes them by name:

```yaml
start:
    stack:
        - "ensure_file(@prompts/state.md)"
        - action: set_frontmatter
          file: "@spec.md"
          prop: "status"
          value: "in-progress"
```

This spec deliberately does **not** define the side-effect catalog. See the Darkmatter spec for the authoritative list and call shapes. **Dependency:** the Darkmatter side-effects work must ship before this lifecycle feature ships.

### Expression-Function Actions

Read-only [Darkmatter expression functions](@darkmatter/docs/topics/darkmatter-expressions.md) may also be invoked as stack actions, primarily for logging their result or feeding it into a subsequent `set_frontmatter` step. They are mostly useful inside `when:` clauses; standalone use is supported but rare.

### Shell Actions

Bespoke shell commands invoked from the stack.

```yaml
- "shell(git status --short)"
- action: shell
  command: "git push origin HEAD"
  on_error: "Push failed; check network"
  no_error: false
```

| Long-form parameter | Purpose |
|---|---|
| `command` | The shell command string (required) |
| `on_error` | Message to emit when the command exits non-zero |
| `no_error` | Boolean; when `true`, suppress error exit codes without altering STDOUT/STDERR |

**Pre-flight requirement:** every shell command in every reachable stack must pass Claudine's command whitelist during pre-flight, exactly like body-level shell expansions today.

## Per-Event Details

### `initialize`

Runs as soon as the prompt file is identified. Pre-flight checks have **not** run. Use for:

- Cheap upfront communication ("starting pre-flight…")
- Opting out via `Skip` (e.g., feature-flag a prompt off)
- Proxying to a different prompt via `Proxy` (e.g., route based on env)
- Preparing the environment via side effects so pre-flight checks will succeed

### `start`

Pre-flight has passed; the agent is about to be invoked. Last chance to communicate before non-determinism takes over.

### `blocked`

A pre-flight check failed. The agent will not be invoked. Common patterns: communicate the failure, `Proxy` to a fallback prompt, or `Handle` (e.g., re-trigger pre-flight after fixing the offending condition).

### `success`

Agentic loop completed cleanly. Common patterns: announce outcome, commit a side effect (e.g., `append_jsonl` to a log), or chain via `next`.

### `failure`

Agentic loop errored. Common patterns: announce failure, `Handle` with retry/resume/proxy/requeue, or simply communicate and exit.

### `loop`

The `loop` event is **both** an iteration-control configuration **and** a lifecycle event. The two concern groups share one frontmatter block.

#### Iteration Controls (already implemented)

These keys govern *how* the prompt iterates:

| Key | Type | Purpose | Default |
|---|---|---|---|
| `while` | expression | Continue while truthy. Mutually exclusive with `until`. | — |
| `until` | expression | Continue until truthy. Mutually exclusive with `while`. | — |
| `action` / `actions` | string or list | Per-iteration frontmatter mutations | — |
| `max` | positive integer | Hard iteration cap | `100` |
| `fail_fast` | boolean | Abort the loop on first iteration error | `true` |
| `on_rate_limit` | enum: `pause` \| `abort` \| `continue` | Behavior on provider rate-limit | `pause` |

Per-iteration `action` operations (existing DSL):

- `increment(prop)`, `decrement(prop)` — numeric mutation
- `set(prop, value)` — assign
- `append(prop, value)`, `prepend(prop, value)` — array mutation
- `merge(prop, value)` — shallow object merge

Both DSL string form (`"increment(phase)"`) and structured form (`{op: "increment", prop: "phase"}`) are accepted.

Ambient variables exposed inside each iteration:

- `_loop_count` (1-based iteration number)
- `_loop_is_first`, `_loop_is_last` (booleans; `_loop_is_last` is speculative)
- `_loop_last_output`, `_loop_last_exit_code` (from previous iteration)

#### Lifecycle Concerns

In addition to iteration controls, the `loop` block accepts the standard lifecycle-event properties (`say`, `notify`, `effect`, `message`, `stderr`, `stdout`, `stack`). **When these fire within the loop cycle is a design gap (D2 below).**

```yaml
phase: 1
total_phases: 6
loop:
    until: "phase > total_phases"
    action: "increment(phase)"
    max: 10
    fail_fast: true
    on_rate_limit: pause

    # Lifecycle concerns:
    say: "Loop iteration {{_loop_count}}"
    stack:
        - when: "_loop_is_first"
          action: notify
          message: "Loop started"
```

### `next`

Opt-in: present means a handoff is configured. Mutually exclusive `suggest` vs `push` keys determine interactivity.

```yaml
# Interactive: prompt the user
next:
    suggest:
        compose: "the-next-thing.md"

# Non-interactive: run immediately
next:
    push:
        compose: "the-next-thing.md"
```

The handoff target supports the same node kinds as execution groups: `compose`, `inline-compose`, `sequence`, `shell`, `prompt`. (Exact allowed kinds: see [Open Design Gaps](#open-design-gaps).)

Standard lifecycle-event properties (`say`, `notify`, `effect`, `message`, `stack`) are also accepted on the `next` event.

## Backward Compatibility

Existing prompts using only top-level communication properties continue to work unchanged:

```yaml
start:
    say: "hi"
success:
    effect: applause
failure:
    message: "Something broke"
```

Adding `stack:` is purely additive. The top-level properties fire first, then the stack is processed.

## Open Design Gaps

These are tracked separately and will be resolved in the design-gap pass following this spec:

1. **D1.** What concretely differentiates `initialize` from `start` beyond timing? What preconditions can fail `initialize`?
2. **D2.** How do lifecycle concerns and iteration controls interleave in `loop`? Does `loop.stack` fire per-iteration, or only on loop entry/exit?
3. **D3.** Full shape of the `Handle` action — what does each sub-action (`retry`, `resume`, `proxy`, `requeve`) look like?
4. **D4.** Error propagation when a side-effect or shell action fails inside a stack — does it transition to the `failure` event, or merely stop the stack with an error?
5. **D5.** Exact allowed node kinds and shape for `next.suggest` and `next.push`.
6. **D6.** `Proxy` re-entry semantics — does the proxied document re-enter at `initialize` or `start`?

## Dependencies

- **Darkmatter side-effects spec** ([`more-context-variables`](@darkmatter/features/_unscheduled/more-context-variables/spec.md)) — must be finalized and implemented before this feature lands. Side effects are the substrate for non-lifecycle, non-shell, non-communication actions.
- **Darkmatter expression engine** ([`expressions`](@darkmatter/docs/topics/darkmatter-expressions.md)) — already implemented; used for `when:` conditions and interpolation in messages.
