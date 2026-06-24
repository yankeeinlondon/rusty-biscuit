# Composition Lifecycle Notifications

Claudine compositions support **lifecycle notifications** declared in Markdown frontmatter. These notifications emit side effects—TTS speech, sound effects, terminal messages, and outbound messaging—at key moments during a composition run.

## Lifecycle Properties

Seven frontmatter properties control lifecycle behavior. Each accepts an object with notification fields and an optional ordered `stack:` of actions:

| Property | Emitted when |
|----------|-------------|
| `initialize` | Prompt file has been identified and frontmatter has parsed, before schema validation and shell pre-flight |
| `start` | Pre-flight checks have passed; immediately before provider invocation |
| `success` | The provider session completed without error |
| `blocked` | Composition exited before the provider child was spawned (e.g., pre-flight denial, schema validation failure) |
| `failure` | The provider session exited with an error |
| `finalize` | Once per iteration, immediately after `success`/`blocked`/`failure` |
| `loop` | Post-`finalize` gate that evaluates the loop's `while`/`until` condition and can run additional lifecycle concerns |

Legacy prompts that only configure `start`, `success`, `blocked`, and `failure` continue to work unchanged.

## Notification Fields

Each lifecycle property is an object containing any of these fields:

| Field | Type | Description |
|-------|------|-------------|
| `say` | string | Text to speak via TTS. Mutually exclusive with `say_first`. |
| `say_first` | string | Text to speak via TTS, but **before** sound effects. Mutually exclusive with `say`. |
| `effect` | string | Sound effect to play (kebab-case name; see [Sound Effects](#sound-effects)). |
| `message` | string | Message dispatched via the configured messaging route (Discord, Slack, Signal, WhatsApp, or webhooks). |
| `notify` | string | Local desktop notification title. Zero-config; does not require a messaging route. |
| `stdout` | string | Plain prose line written to **stdout**. **No status glyph.** Note: stdout is otherwise reserved for pipeable command output, so this interleaves with the composed/provider output on the same stream — use deliberately. |
| `stderr` | string | Plain prose line written to stderr. **No status glyph** — inline styling and links are honored, but no status decoration. |
| `info` | string | Status line rendered with an info style. |
| `warn` | string | Status line rendered with a warning style. |
| `success` | string | Status line rendered with a success style. |
| `stack` | list | Ordered list of conditional actions (see [Stacks](#stacks)). |

Most lifecycle output is written to stderr, messaging routes, or desktop notifications. The `stdout` channel is the lone exception: it writes to stdout, which is otherwise reserved for pipeable command output, so reach for it only when you specifically want lifecycle text on stdout.

### Audio Ordering

When both speech and an effect are configured, the order depends on which speech field is used:

- `say` + `effect` → **effect first, then speech**
- `say_first` + `effect` → **speech first, then effect**

This lets you choose whether a sound effect acts as an introduction (`say_first`) or a conclusion (`say`) to the spoken message.

## Stacks

A `stack` is an ordered list of conditional actions executed after the top-level notification fields. Each stack item can include:

- `when` — an optional Darkmatter condition expression. Omitted means the item always runs.
- `action` — a single action or a list of actions.
- `no_error` — when `true`, an errored action is logged but does not stop the stack or change the composition outcome.

```yaml
success:
  stack:
    - when: "env.SEND_MESSAGE == 'true'"
      action: message("Build passed on main")
    - action: [say("Done"), effect("confirmation")]
```

Actions run in order. The first lifecycle control action (`skip`, `stop`, `error`, etc.) terminates the stack for that event.

### Action Forms

Actions can be written in short form or long form.

**Short form:** `verb(args)`

```yaml
success:
  stack:
    - action: say('All done')
    - action: effect('confirmation')
    - action: shell('git tag release-{{version}}')
```

Arguments are Darkmatter expressions. Multi-word strings must be quoted:

```yaml
success:
  stack:
    - action: say('hello world')  # ok
    # - action: say(hello world)  # ERROR: unquoted multi-word literal
```

**Long form:** an object with an `action` verb key plus named parameters.

```yaml
success:
  stack:
    - action: shell
      command: "git push origin HEAD"
      on_error: "push failed"
      no_error: true
    - action: message
      message: "Deployed {{version}}"
      route: "deployments"
```

When `action` is a scalar string, sibling keys are the action's parameters. When `action` is an array, each element is self-contained and sibling parameters are not allowed.

### Flow Control Actions

Lifecycle flow control actions terminate the current event's stack and influence runtime flow:

| Action | Valid in | Effect |
|--------|----------|--------|
| `stop` | every event | End this event's stack cleanly; composition continues with the current outcome |
| `skip` | `initialize` only | Whole-document opt-out: no provider invocation, no `finalize`, no `loop` |
| `error("reason")` | every event | Mark this event as failed; at `success`/`finalize` it converts success to failure |
| `proxy("@other.md")` | every event | Hand off to another prompt document, entering the target at its own `initialize` |
| `retry(N)` | every event | Retry the current prompt N additional times (re-runs pre-flight pre-launch, re-invokes the agent post-launch) |
| `resume("message")` | every event | Resume the agent session with a follow-up message. Needs a live session — pre-launch it surfaces a `ResumeWithoutSession` error |
| `defer("5m")` | every event | Defer this prompt to **run again later** — a fresh scheduled run after the delay (not an in-place pause), via the rendezvous deferred-execution scheduler. **Not implemented yet:** `defer` parses and dispatches but currently surfaces a typed `LifecycleDeferNotImplemented` error until the rendezvous backend is ready. |

At most one flow-control action may appear in a stack item, and it must be the last action.

**Flow control is universal.** Flow control reacts to **state** — an error, a missing file, an `env` value, frontmatter — and an error is just one kind of state. So `error`/`stop`/`retry`/`resume`/`defer`/`proxy` are valid in **every** event. The headline example: a `success` stack can `resume("you finished but never wrote abc.md — create it as instructed")` when the agent completed cleanly but an expected artifact is missing. The only placement rule is `skip` (`initialize`-only). Apparent event-specific behavior is **runtime capability**, not placement: `resume` needs a live session (pre-launch → `ResumeWithoutSession`) and `retry`'s re-entry point is derived from whether the provider had launched. This is enforced once, at parse time; at runtime every event's stack dispatches its control through the same event-agnostic path. The iteration `loop:` (while/until) is a separate mechanism and is never coupled to handler dispatch.

The provider run-loop events — `start`, `success`, `failure`, `finalize` — dispatch `retry`/`resume`/`proxy` fully (this is where `success` + `resume` lives). The events that sit *outside* that loop — `initialize`, a compose pre-flight `blocked`, and the `loop` gate — handle `error`/`stop` (and `proxy`/`skip` at `initialize`) directly, but `retry`/`proxy` from those events have no re-entry loop to act on yet, so they surface a clear typed error (`LifecycleSetupPhaseRecoveryUnsupported`) rather than a silent no-op. Put recovery on a post-launch event, or use `initialize` `proxy` for pre-launch routing. `defer` (deferred re-execution) is **not implemented in any event yet** — it always surfaces `LifecycleDeferNotImplemented` until its rendezvous backend lands.

### Shell Actions

The `shell` action runs an approved shell command. Commands are collected during pre-flight shell approval alongside `::shell` directives and `$(...)` frontmatter expressions.

```yaml
start:
  stack:
    - action: shell
      command: "npm run typecheck"
      on_error: "typecheck failed"
```

A non-zero exit code is an action error unless `no_error: true` is set.

### Side-Effect Actions

Any Darkmatter side-effect verb can be invoked by name:

```yaml
start:
  stack:
    - action: set_frontmatter('state.md', 'status', 'in-progress')
success:
  stack:
    - action: set_frontmatter('state.md', 'status', 'done')
```

Long-form side-effect actions accept named parameters that are reordered into the verb's positional signature:

```yaml
success:
  stack:
    - action: http_post
      url: "https://example.com/hook"
      body: "{{payload}}"
```

### Expression-Function Actions

Any Darkmatter read-only expression function can be invoked for its result. The result is logged in the lifecycle/status style.

```yaml
start:
  stack:
    - action: file_exists('@docs/plan.md')
```

### `no_error`

The `no_error` flag can be set on any action category. When `true`, an unintentional action error is logged but does not stop the stack or change the composition outcome.

```yaml
start:
  stack:
    - action: shell
      command: "git status --short"
      no_error: true
    - action: info('continuing')
```

## Lifecycle Context

Stack expressions have access to three lifecycle-only globals in addition to frontmatter, `ctx.*`, `env.*`, and `doc.*`:

| Global | Available in | Fields |
|--------|--------------|--------|
| `err` | `blocked`, `failure`, `finalize` | `kind`, `variant`, `msg` |
| `timing` | every event | `document_ms`, `total_ms`, `step_ms` (all optional) |
| `current` | every event | `current.ctx.*`, `current.env.*` (lazy snapshots at event time) |

`err` is only meaningful in events that can carry an error. Using bare `err` (or `err.*`) in `initialize`, `start`, `success`, or `loop` is rejected at parse time.

```yaml
failure:
  stack:
    - when: "err.variant == 'ShellCommandDenied'"
      action: notify("Shell command was denied")
```

### `doc.err` Escape Hatch

A frontmatter property literally named `err` can still be reached through the `doc` namespace. This is the only way to reference an `err` value in no-error events.

```yaml
err: "user-configured reason"
start:
  stack:
    - action: stderr('{{doc.err}}')
```

## Loop Gate Concerns

The `loop` property carries both iteration controls (`while`/`until`, `action`/`actions`, `max`, `fail_fast`) and lifecycle concerns (`say`, `stack`, etc.). Lifecycle concerns inside `loop` fire on every gate pass, including the terminal pass that exits the loop.

```yaml
loop:
  while: "iteration < max_iterations"
  actions:
    - increment(iteration)
  stderr: "checking loop condition"
  stack:
    - action: info('loop gate reached')
```

Loop execution runs `initialize` once at the start, then re-enters each iteration at `start` without re-running `initialize`, schema validation, or shell pre-flight. `success`, `failure`, and `finalize` fire once per iteration. The loop condition is evaluated **after** lifecycle concerns and **before** per-iteration mutations are applied.

## Examples

### Minimal: terminal status only

```yaml
---
start:
  stderr: "Starting code review..."
success:
  stderr: "Code review complete"
---
```

### TTS with sound effect

```yaml
---
start:
  say: "Starting the deployment pipeline"
  effect: "confirmation"
success:
  say: "Deployment finished successfully"
  effect: "crowd-applause"
failure:
  say: "Deployment failed"
  effect: "sad-trombone"
---
```

### Initialize and finalize

```yaml
---
initialize:
  stderr: "Setting up workspace"
start:
  stderr: "Running agent"
success:
  stderr: "Agent finished"
finalize:
  stderr: "Cleaning up"
---
```

### Conditional stack with `err`

```yaml
---
failure:
  stack:
    - when: "err.kind == 'CompositionError'"
      action: notify("Composition failed")
    - action: say('Something went wrong')
---
```

### Short-form expression arguments

```yaml
---
start:
  stack:
    - action: info('running {{agent}}')
    - action: shell('git fetch origin {{branch}}')
---
```

### `no_error` shell action

```yaml
---
start:
  stack:
    - action: shell
      command: "which optional-tool"
      no_error: true
    - action: info('continuing')
---
```

### Loop lifecycle concerns

```yaml
---
iteration: 1
max_iterations: 3
loop:
  while: "iteration <= max_iterations"
  actions:
    - increment(iteration)
  stderr: "loop gate"
  stack:
    - action: info('iteration {{_loop_count}}')
---
```

### Recover from a usage cap by switching providers

When a provider hits a usage cap or rate limit, the failure surfaces in the `failure` event with that classification in `err.variant` — the provider's raw `error_kind` (e.g. `usage_limit_reached`, `quota_exceeded`, `rate_limit`). There is no in-place "switch provider" action; instead `proxy` hands the same task off to a sibling prompt that pins a different agent.

```yaml
---
agent: claude
prompt: "Implement the feature described in @spec.md"
failure:
  stack:
    - when: "err.variant == 'usage_limit_reached' || err.variant == 'quota_exceeded' || err.variant == 'rate_limit'"
      action:
        - warn('Claude usage cap reached — handing off to Codex')
        - proxy('@prompts/feature-codex.md')
---
```

`@prompts/feature-codex.md` is the same task with `agent: codex` in its frontmatter; `proxy` starts it fresh at its own `initialize`. Because `err.variant` carries the provider's raw `error_kind`, widen the guard to match the kinds your provider emits — run the prompt once to observe the value, or read it off the session badge.

### Verify an artifact on success, then retry once from `finalize`

`success` can double-check that the agent actually produced what it claimed. Raising `error` there converts the outcome to failure and routes through the `failure` event **before** `finalize`. Because `finalize` is the optional-error terminal event, it carries that `err` and can recover from it — so the verification lives in `success` and the single retry lives in `finalize`.

```yaml
---
prompt: "Generate the release notes and write them to @output/RELEASE.md"
success:
  stack:
    # The agentic loop returned cleanly — confirm the file really exists.
    - when: "!file_exists('@output/RELEASE.md')"
      action: error('agent reported success but @output/RELEASE.md was never written')
finalize:
  stack:
    # `finalize` carries `err` after the success-side downgrade. Retry the
    # whole run exactly once; the retried attempt re-enters at `start`.
    - when: "err"
      action: retry(1)
---
```

On the retried attempt the agent runs again and `success` re-verifies the file. With `retry(1)` the budget allows exactly one extra attempt; if the file is still missing after it, `finalize` carries `err` once more, the retry budget is spent, and the run ends in failure. To announce that terminal case, add a guarded `warn` ahead of the `retry` item (the first matching control action ends the stack, so order the `warn` before the `retry`).

### Resume after a timeout

Both the wall-clock `timeout` and the step-silence `step_timeout` surface in the `failure` event with `err.variant` of `timeout` or `step_timeout`. `resume` continues the **same** agent session — context intact — with a follow-up message, which is usually better than `retry` for a timeout (retry re-runs the invocation from scratch).

```yaml
---
prompt: "Refactor @src/engine.rs and make the test suite pass"
timeout: 20m
step_timeout: 5m
failure:
  stack:
    - when: "err.variant == 'timeout' || err.variant == 'step_timeout'"
      action: resume('You were stopped by a timeout. Continue from where you left off and finish the task.')
---
```

`resume` is valid only in `failure` and defaults to a single attempt (`max_attempts: 1`). Its string argument binds to the required `message:` parameter.

## Sound Effects

The `effect` field accepts a kebab-case name from the built-in catalog. Names are matched after stripping hyphens and lowercasing.

- see [sound effects](./sound-effects.md) for an enumeration of sound effects

## State Machine

Claudine uses a `LifecycleRunGuard` to enforce correct state transitions and guarantee terminal signals are emitted.

### Signal States

| Signal | Terminal Status | stderr Style |
|--------|----------------|--------------|
| `Initialize` | Non-terminal | Info |
| `Start` | Non-terminal | Info |
| `Success` | Terminal | Success |
| `Blocked` | Terminal | Error |
| `Failure` | Terminal | Error |
| `Finalize` | Non-terminal | Info |
| `Loop` | Non-terminal | Info |

### Drop Safety-Net

If `start` is emitted but no terminal signal (`success`, `blocked`, `failure`) is emitted before the guard drops, a terminal signal is automatically fired:

| `start` emitted? | Provider launched? | Drop emits |
|------------------|-------------------|------------|
| No | — | Nothing |
| Yes | No | `Blocked` |
| Yes | Yes | `Failure` |

This ensures compositions that panic or exit unexpectedly still report a terminal lifecycle signal.

### Explicit Control

Code using the guard can:

- Call `emit_initialize_once()` to emit `initialize` (idempotent; fires exactly once across a loop run)
- Call `emit_start_once()` to emit `start` (idempotent)
- Call `mark_provider_launched()` after the provider child spawns
- Call `emit_terminal(signal)` to emit a terminal signal and suppress the drop safety-net
- Call `emit_blocked_or_failure()` to emit `Blocked` (pre-launch) or `Failure` (post-launch)
- Call `emit_finalize_once()` to emit `finalize` once after a terminal signal
- Call `reset_for_next_iteration()` to reset per-iteration state for the next loop iteration
- Call `defuse()` to suppress the drop safety-net without emitting anything

## Validation

Claudine validates lifecycle frontmatter at parse time. Errors prevent composition from starting.

### `LifecycleSayConflict`

Both `say` and `say_first` are present in the same notification. Only one speech field is allowed.

```yaml
start:
  say: "Starting"
  say_first: "Also starting"  # ERROR: conflict
```

### `LifecycleUnknownEffect`

The `effect` field references a sound effect name not in the compiled catalog.

```yaml
start:
  effect: "unknown-sound"  # ERROR: unknown effect
```

### `LifecycleInvalid`

The notification object contains unknown fields or a type mismatch (e.g., `say: 123`).

```yaml
start:
  say: "Valid"
  unknown_field: "value"  # ERROR: unknown field
```

### `LifecycleInterpolationLeak`

A rendered lifecycle string still contains a `{{ … }}` span after composition. This guards against unresolved template syntax reaching user-visible side effects.

```yaml
success:
  stderr: "Done: {{missing_var}}"  # ERROR: interpolation leaked
```

### `LifecycleUndefinedVariable`

A lifecycle string references a bare variable that is undefined after composition. Darkmatter resolves unknown bare variables to an empty string silently, so this guard inspects raw lifecycle strings before composition.

```yaml
success:
  stderr: "Done: {{undefined_key}}"  # ERROR: undefined variable
```

### `LifecycleErrNotAvailable`

A bare `err` reference appears in an event that never carries an error (`initialize`, `start`, `success`, `loop`).

```yaml
start:
  stack:
    - action: stderr('{{err.msg}}')  # ERROR: err not available in start
```

### Empty String Normalization

Empty strings and whitespace-only strings are normalized to `null`, so these are equivalent:

```yaml
start:
  say: ""
  message: "   "
```

Both `say` and `message` are treated as absent.

## Non-Object Frontmatter

If the frontmatter is not an object (e.g., a bare string or list), lifecycle parsing returns a default empty configuration with no notifications.

## Integration Notes

- **TTS**: Uses the global TTS configuration from `claudine.toml` (voice, rate, provider). If no TTS settings are configured, uses system defaults.
- **Messaging**: Requires a configured messaging route. See [Configuring Actions](configuring-actions.md).
- **Desktop notifications**: Zero-config. Emitted via `notify` independently of messaging routes. Failures are non-fatal.
- **stderr/info/warn**: Rendered as styled status badges using the terminal's capability detection (circular theme with color-coded state).
- **Audio playback**: Blocking. Sound effects and TTS play sequentially, not in parallel, to avoid overlapping audio.
- **stdout**: The lone lifecycle channel that writes to stdout (all others target stderr, messaging, or desktop notifications). Because stdout is otherwise reserved for pipeable command output, lifecycle `stdout` text interleaves with the composed/provider output on that stream — opt in deliberately when a pipeline (`claudine compose <file> | other-tool`) should see the text.

## Related Topics

- [Composition](composition.md) — the composition pipeline and loop behavior
- [Configuring Actions](configuring-actions.md) — messaging routes and action configuration
- [Non-Interactive Sessions](non-interactive-sessions.md) — stderr rendering and terminal output
