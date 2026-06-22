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
| `stderr` | string | Styled status line written to stderr. |
| `info` | string | Status line rendered with an info style. |
| `warn` | string | Status line rendered with a warning style. |
| `stack` | list | Ordered list of conditional actions (see [Stacks](#stacks)). |

Lifecycle output is intentionally written to stderr, messaging routes, or desktop notifications only. There is **no `stdout` lifecycle channel** because stdout is reserved for pipeable command output.

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

### Control Actions

Lifecycle control actions terminate the current event's stack and influence runtime flow:

| Action | Valid in | Effect |
|--------|----------|--------|
| `stop` | every event | End this event's stack cleanly; composition continues with the current outcome |
| `skip` | `initialize` only | Whole-document opt-out: no provider invocation, no `finalize`, no `loop` |
| `error("reason")` | every event | Mark this event as failed; at `success`/`finalize` it converts success to failure |
| `proxy("@other.md")` | `initialize`, `blocked`, `failure` | Hand off to another prompt document (currently unsupported; raises a typed error) |
| `retry(N)` | `blocked`, `failure` | Retry the current prompt N additional times |
| `resume("message")` | `failure` only | Resume the agent session with a follow-up message |
| `requeue("5m")` | `blocked`, `failure` | Push this prompt onto the deferred-execution queue (unsupported without a queue integration) |

At most one control action may appear in a stack item, and it must be the last action.

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

## Sound Effects

The `effect` field accepts a kebab-case name from the built-in catalog. Names are matched after stripping hyphens and lowercasing.

### UI Sounds

`doorbell`, `doorbell-2`, `space-alarm`, `dit-hit-1`, `dit-hit-2`, `electronic-hit-fx1`–`fx6`, `bong`, `click`, `confirmation`, `drop-1`–`drop-4`, `error-1`–`error-2`, `glass-1`–`glass-4`, `maximize-1`–`maximize-5`, `minimize-1`–`minimize-3`, `mouseclick`, `pluck-1`–`pluck-2`, `question-1`–`question-2`, `select-1`–`select-4`, `switch-1`–`switch-2`

### Cartoon Sounds

`cartoon-accent1`–`accent12`, `cartoon-cry`

### Reaction Sounds

`crowd-applause`, `crowd-applause-recital`, `crowd-applause-stadium`, `crowd-laugh`, `crowd-laugh-applause`, `sad-trombone`, `small-group-cheer`, `female-astonished-gasp`, `sneeze`

### Sci-Fi Sounds

`high-down`, `high-up`, `two-tone`, `phase-jump-1`–`jump-5`, `phaser-down-1`–`down-3`

### Atmosphere Sounds

`creepy-dark-logo`, `elemental-magic-spell-impact`, `epic-orchestra-transition`, `mysterious-bass`, `retro-game`

### Motion Sounds

`air-reverse-burst`, `air-woosh`, `air-zoom-vacuum`, `arrow-whoosh`, `bicycle-horn`, `bottle-cork`, `bullet`

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

### `LifecycleStdoutRejected`

A `stdout` field or `stdout(...)` action was authored. stdout is reserved for pipeable command output.

```yaml
start:
  stdout: "hello"  # ERROR: stdout rejected
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
- **No stdout channel**: Lifecycle chatter never writes to stdout so `claudine compose <file> | other-tool` remains unambiguous.

## Related Topics

- [Composition](composition.md) — the composition pipeline and loop behavior
- [Configuring Actions](configuring-actions.md) — messaging routes and action configuration
- [Non-Interactive Sessions](non-interactive-sessions.md) — stderr rendering and terminal output
