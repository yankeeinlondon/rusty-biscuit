# Composition Lifecycle Notifications

Claudine compositions support **lifecycle notifications** declared in Markdown frontmatter. These notifications emit side effects—TTS speech, sound effects, terminal messages, and outbound messaging—at four key moments during a composition run.

## Lifecycle Properties

Four frontmatter properties control lifecycle behavior. Each accepts an object with notification fields:

| Property | Emitted when |
|----------|-------------|
| `start` | Composition begins |
| `success` | Composition completes successfully |
| `blocked` | Composition exits before the provider child is spawned (e.g., pre-flight denial, user cancellation) |
| `failure` | Composition exits after the provider child is spawned (e.g., provider error, crash) |

## Notification Fields

Each lifecycle property is an object containing any of these fields:

| Field | Type | Description |
|-------|------|-------------|
| `say` | string | Text to speak via TTS. Mutually exclusive with `say_first`. |
| `say_first` | string | Text to speak via TTS, but **before** sound effects. Mutually exclusive with `say`. |
| `effect` | string | Sound effect to play (kebab-case name; see [Sound Effects](#sound-effects)). |
| `message` | string | Message dispatched via the configured messaging route (Discord, Slack, Signal, WhatsApp). |
| `stderr` | string | Styled status line written to stderr. |

### Audio Ordering

When both speech and an effect are configured, the order depends on which speech field is used:

- `say` + `effect` → **effect first, then speech**
- `say_first` + `effect` → **speech first, then effect**

This lets you choose whether a sound effect acts as an introduction (`say_first`) or a conclusion (`say`) to the spoken message.

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

### Speech before effect

```yaml
---
success:
  say_first: "All done"
  effect: "confirmation"
---
```

This speaks "All done" and then plays the confirmation sound.

### Messaging integration

```yaml
---
success:
  message: "Build passed on main"
failure:
  message: "Build failed on main"
  say: "The build has failed"
---
```

The `message` field dispatches through the messaging system configured in `claudine.toml` (see [Configuring Actions](configuring-actions.md)).

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
| `Start` | Non-terminal | Info |
| `Success` | Terminal | Success |
| `Blocked` | Terminal | Error |
| `Failure` | Terminal | Error |

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

- Call `emit_start_once()` to emit `start` (idempotent)
- Call `mark_provider_launched()` after the provider child spawns
- Call `emit_terminal(signal)` to emit a terminal signal and suppress the drop safety-net
- Call `emit_blocked_or_failure()` to emit `Blocked` (pre-launch) or `Failure` (post-launch)
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
- **stderr**: Rendered as a styled status badge using the terminal's capability detection (circular theme with color-coded state).
- **Audio playback**: Blocking. Sound effects and TTS play sequentially, not in parallel, to avoid overlapping audio.

## Related Topics

- [Composition](composition.md) — the five-stage composition pipeline
- [Configuring Actions](configuring-actions.md) — messaging routes and action configuration
- [Non-Interactive Sessions](non-interactive-sessions.md) — stderr rendering and terminal output
