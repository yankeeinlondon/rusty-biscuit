# Start, Success, Blocked, and Failure Tech Design

This document turns the `start` / `success` / `blocked` / `failure` spec into an implementation-ready design for Claudine's existing composition, wrapper, messaging, and harness pipeline.

Primary inputs:

- `claudine/features/2026-04-02-start-success-error/spec.md`
- `claudine/features/2026-03-27-compose-refactor/tech-design.md`
- `claudine/features/2026-03-30-validation-reporting/tech-design.md`
- current composition executor in `claudine/cli/src/commands/wrap/composition.rs`
- current harness loop in `claudine/cli/src/commands/wrap/mod.rs`
- current harness reporting/audit model in `claudine/lib/src/harness/`
- current outbound notification primitives in `claudine/lib/src/messaging/send.rs`, `claudine/lib/src/dispatch/runner.rs`, and `claudine/lib/src/harness/speech.rs`

The core design decision is to treat these properties as composition lifecycle notifications layered on top of the existing wrapper and harness flow, not as a second hook system and not as a separate validation engine.

## Summary

This feature adds four optional frontmatter properties:

1. `start`
2. `success`
3. `blocked`
4. `failure`

Each property describes a lifecycle notification bundle that can target:

1. TTS
2. a sound effect
3. configured messaging routes
4. stderr via `Status`

Implementation has four major parts:

1. parse typed lifecycle notification configs from effective frontmatter
2. build a reusable lifecycle emitter that handles audio ordering plus best-effort non-audio fan-out
3. integrate deterministic trigger points into both harness and non-harness composition execution
4. classify terminal outcomes as either `blocked` or `failure` based on whether the provider launch ever began

The result is:

- one schema for all four lifecycle states
- one execution contract for audio ordering
- one state classification model across compose and inline-compose
- no duplication between harness and direct composition paths

## Goals

1. Support `start`, `success`, `blocked`, and `failure` in both `claudine compose` and `claudine inline-compose`.
2. Read lifecycle config from effective frontmatter, not raw source frontmatter.
3. Preserve the spec's audio-order semantics for `speak` vs `speak_first`.
4. Reuse Claudine's existing TTS, messaging, sound-effect, and `Status` infrastructure where possible.
5. Keep all lifecycle notifications best-effort so notification failures do not change the main composition result.
6. Keep `blocked` and `failure` semantically distinct.

## Non-Goals

1. Turning lifecycle notifications into general hook `HookAction`s.
2. Adding image attachments or provider-specific rich media to lifecycle messaging in v1.
3. Adding Handlebars interpolation for lifecycle strings in v1.
4. Emitting lifecycle notifications for general wrapper commands outside `compose` and `inline-compose`.
5. Redesigning validation handlers, retry semantics, or stream summaries.

## Current Baseline

Today Claudine already has most of the primitives this feature needs:

1. `claudine/cli/src/commands/wrap/composition.rs` owns wrapper-grade composition execution and already distinguishes harness vs non-harness paths.
2. `claudine/cli/src/commands/wrap/mod.rs::run_harness_loop(...)` already has explicit boundaries for:
   - source-file reporting
   - shell audit
   - pre-checks
   - provider execution
   - inline closure
   - post-checks
   - terminal failure banners
3. `claudine/lib/src/harness/report.rs` already renders lifecycle-like stderr statuses through `Status::from_prose(...)`.
4. `claudine/lib/src/messaging/send.rs` already knows how to route Markdown text through Claudine messaging settings.
5. `claudine/lib/src/dispatch/runner.rs` already executes `Speak`, `SoundEffect`, and `Message` as best-effort side effects.

The gap is orchestration:

1. frontmatter has no typed model for lifecycle notifications
2. composition execution has no notion of lifecycle-state emission
3. audio ordering currently exists only implicitly in the spec
4. messaging/TTS helpers are not exposed in a way that composition lifecycle code can reuse cleanly

## Spec Clarifications

The spec is intentionally incomplete. This design resolves the missing behavior explicitly.

### 1. Lifecycle config comes from effective frontmatter

Lifecycle properties are read from the same effective frontmatter already used for:

1. harness detection
2. provider hint selection
3. inline closure planning

That means Darkmatter composition and `--set` overrides can change lifecycle notification behavior.

### 2. `start` fires once

`start` is emitted exactly once per top-level composition execution, immediately before the first provider launch.

It does not re-fire for:

1. retry handlers
2. resume handlers
3. redirect handlers
4. inline closure retries

Those are recovery steps inside the same already-started run.

### 3. `blocked` means the provider never launched

`blocked` is reserved for terminal outcomes that occur before the first provider process starts.

Examples:

1. harness shell audit fails and no handler recovers
2. pre-checks fail and no handler recovers
3. non-harness inline writability check fails
4. redirect/retry recovery is exhausted before the first launch

Non-examples:

1. agent exits non-zero
2. timeout after launch
3. inline closure rewrite fails after a successful agent run
4. post-checks fail after the agent has already run

Those are `failure`.

### 4. Early errors before effective frontmatter exists do not emit lifecycle notifications

Some failures happen before Claudine can reliably obtain lifecycle config:

1. file reference resolution failures
2. Darkmatter composition failures during initial preparation
3. malformed source that prevents effective frontmatter creation

For those cases, Claudine keeps the current error behavior and emits no lifecycle notification because the configuration that would define that notification is not yet trustworthy.

### 5. `blocked` and `failure` are terminal and mutually exclusive

At most one of these terminal states fires:

1. `blocked` if launch never began
2. `failure` if launch began but the overall run still ended unsuccessfully

If the run succeeds, only `success` fires.

### 6. `stderr` uses fixed state mapping

`stderr` text is rendered through `Status::from_prose(...)` using these fixed states:

1. `start` -> `Info`
2. `success` -> `Success`
3. `blocked` -> `Failure`
4. `failure` -> `Failure`

`blocked` intentionally maps to `Failure`, not `Warning`, because it is terminal and user action is required to proceed.

### 7. `speak` and `speak_first` are mutually exclusive

If both are set on the same lifecycle property, parsing fails with a frontmatter validation error before execution begins.

### 8. String treatment is fixed in v1

To keep the feature bounded:

1. `stderr` is Prose markup and is rendered as such
2. `message` is sent as already-resolved Markdown text
3. `speak` / `speak_first` are spoken literally
4. no Handlebars interpolation is applied in v1

If templating is desired later, it should be added deliberately with a composition-specific context model rather than silently reusing hook-event interpolation.

### 9. Missing messaging config is a no-op

If a lifecycle property defines `message` but Claudine has no active messaging route configured, the messaging send is skipped. This should be a debug/warn-only condition, not a user-facing error.

## Frontmatter Schema

Each lifecycle property uses the same object shape:

```yaml
start:
  speak: "Starting now"
  effect: crowd-applause
  message: "Starting now"
  stderr: "Starting"
```

or:

```yaml
success:
  speak_first: "The work completed successfully"
  effect: power-up
  message: "Completed successfully"
  stderr: "<b>Success</b>"
```

Recommended typed model:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleNotification {
    #[serde(default)]
    pub speak: Option<String>,
    #[serde(default)]
    pub speak_first: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionLifecycleConfig {
    #[serde(default)]
    pub start: Option<LifecycleNotification>,
    #[serde(default)]
    pub success: Option<LifecycleNotification>,
    #[serde(default)]
    pub blocked: Option<LifecycleNotification>,
    #[serde(default)]
    pub failure: Option<LifecycleNotification>,
}
```

Validation rules:

1. `speak` and `speak_first` cannot both be present
2. empty strings are treated as absent after trim
3. unknown keys are rejected
4. unknown sound-effect names are rejected at parse/validation time

## Execution Model

### Audio ordering

Each lifecycle notification produces up to two audio phases and one non-audio fan-out:

1. non-audio fan-out:
   - `stderr`
   - `message`
2. audio phase 1:
   - `effect`, or `speak_first`, or `speak`
3. audio phase 2:
   - the remaining audio item when both speech and effect are configured

Ordering rules:

1. `speak` + `effect`
   - phase 1 = `effect`
   - phase 2 = `speak`
2. `speak_first` + `effect`
   - phase 1 = `speak_first`
   - phase 2 = `effect`
3. only one audio output
   - phase 1 = that output
   - no phase 2

### Awaiting behavior

Lifecycle emission is synchronous with respect to the composition pipeline:

1. `stderr` and `message` are dispatched immediately
2. audio phases are awaited in order
3. the pipeline advances only after the lifecycle emission completes

This is important for `start` because we do not want startup speech/effects overlapping with the agent's live output.

### Failure isolation

Notification failures are never fatal to the composition run.

Examples:

1. TTS provider missing
2. sound effect playback failure
3. messaging route misconfiguration
4. stderr render failure

These should log warnings and continue.

## State Classification

The execution pipeline needs one explicit state tracker:

```rust
pub struct LifecycleRuntimeState {
    pub start_emitted: bool,
    pub provider_launch_started: bool,
}
```

Classification rule:

1. before first provider launch:
   - terminal error => `blocked`
2. after first provider launch:
   - terminal error => `failure`
3. terminal success:
   - `success`

`start_emitted` and `provider_launch_started` are closely related, but keeping both makes the guard conditions explicit and simplifies tests.

## Recommended Module Layout

### `claudine/lib/src/composition/lifecycle.rs` (new)

Own typed parsing and execution planning for lifecycle notifications.

Recommended responsibilities:

1. parse `CompositionLifecycleConfig` from effective frontmatter
2. validate mutual exclusivity and effect names
3. normalize empty strings away
4. build an execution plan with fixed audio ordering
5. expose a single best-effort emission API

Recommended public API shape:

```rust
pub enum LifecycleSignalKind {
    Start,
    Success,
    Blocked,
    Failure,
}

pub fn parse_lifecycle_config(frontmatter: &serde_json::Value)
    -> Result<CompositionLifecycleConfig, CompositionError>;

pub fn emit_lifecycle_signal(
    config: &CompositionLifecycleConfig,
    kind: LifecycleSignalKind,
    runtime: &LifecycleRuntimeContext<'_>,
);
```

### `claudine/lib/src/composition/types.rs`

Extend `PreparedComposition` with parsed lifecycle config so downstream code does not repeatedly deserialize the same frontmatter:

```rust
pub struct PreparedComposition {
    ...
    pub lifecycle: CompositionLifecycleConfig,
}
```

### `claudine/lib/src/composition/prepare.rs`

After effective frontmatter is produced, parse lifecycle config once and store it on `PreparedComposition`.

This ensures:

1. `compose` and `inline-compose` behave consistently
2. `--set` overrides can influence lifecycle behavior
3. later execution code only reads typed data

### `claudine/lib/src/messaging/send.rs`

Extract a lower-level "already rendered" send entrypoint so composition lifecycle code can reuse messaging without manufacturing fake `EventMeta`.

Recommended shape:

```rust
pub fn execute_message(
    message_template: &str,
    image_template: Option<&str>,
    meta: &EventMeta,
    messaging: &RuntimeMessagingSettings,
)

pub fn execute_resolved_message(
    text: &str,
    image: Option<&str>,
    cwd: Option<&Path>,
    repo_root: Option<&Path>,
    messaging: &RuntimeMessagingSettings,
)
```

`execute_message(...)` remains the interpolation wrapper. Lifecycle notifications call `execute_resolved_message(...)`.

### `claudine/lib/src/dispatch/runner.rs` or a new shared notification helper

Extract reusable helpers for:

1. speaking resolved text with `GlobalSettings::tts`
2. playing a named sound effect

The current hook-runner helpers are too event/template-specific to be called cleanly from composition lifecycle code.

### `claudine/cli/src/commands/wrap/composition.rs`

Own lifecycle integration for:

1. parsing lifecycle config from `PreparedComposition`
2. emitting `start` in non-harness and harness entrypoints
3. classifying pre-launch terminal failures as `blocked`
4. passing lifecycle runtime context into the harness loop

### `claudine/cli/src/commands/wrap/mod.rs`

Integrate lifecycle transitions into `run_harness_loop(...)` for:

1. `start` after shell audit and pre-check success, before the first provider attempt
2. `blocked` for terminal pre-launch failures
3. `failure` for terminal post-launch failures
4. `success` after inline closure and post-check success

## Runtime Context

The emitter needs enough context to reuse existing settings and terminal rendering:

```rust
pub struct LifecycleRuntimeContext<'a> {
    pub settings: &'a GlobalSettings,
    pub messaging: &'a RuntimeMessagingSettings,
    pub term: &'a Terminal,
    pub source_path: &'a Path,
    pub repo_root: Option<&'a Path>,
}
```

Recommended loading strategy in composition execution:

1. call `claudine::dispatch::loader::load_runtime_config(None, effective_repo_root)` once
2. use it for:
   - favorite-provider lookup if desired later
   - `GlobalSettings::tts`
   - `RuntimeMessagingSettings`
3. if config loading fails because no config exists, fall back to:
   - default `GlobalSettings`
   - default `RuntimeMessagingSettings`

Lifecycle notifications must not require a Claudine config file to exist.

## Integration Points

### Compose executor (`execute_composition_request`)

This function should:

1. load runtime settings/context once
2. parse lifecycle config from `request.prepared.lifecycle`
3. create `LifecycleRuntimeState`
4. emit `blocked` for terminal failures that occur after `PreparedComposition` exists but before provider launch

Concrete pre-launch `blocked` cases here:

1. harness parse failure after effective frontmatter is available
2. harness preflight shell approval failure with no recovery path
3. non-harness inline writability failure

Concrete non-cases here:

1. provider selection failure before prepared composition is available
2. initial source resolution failure

### Harness loop (`run_harness_loop`)

Add lifecycle params:

```rust
pub(crate) fn run_harness_loop(
    ...
    lifecycle: &CompositionLifecycleConfig,
    lifecycle_state: &mut LifecycleRuntimeState,
    lifecycle_ctx: &LifecycleRuntimeContext<'_>,
    ...
)
```

Trigger points:

1. after shell audit passes and pre-checks pass, but before `build_harness_launch(...)`
   - emit `start` if not already emitted
   - set `provider_launch_started = true`
2. terminal shell-audit failure before launch
   - emit `blocked`
3. terminal pre-check failure before launch
   - emit `blocked`
4. terminal agent failure after launch
   - emit `failure`
5. terminal inline-closure failure after launch
   - emit `failure`
6. terminal post-check failure after launch
   - emit `failure`
7. final successful completion
   - emit `success`

### Non-harness direct path (`execute_direct_without_harness`)

Trigger points:

1. immediately before child execution
   - emit `start`
   - mark launch started
2. exit code `0`
   - emit `success`
3. non-zero exit / interruption treated as terminal unsuccessful run
   - emit `failure`

### Non-harness inline path (`execute_inline_without_harness`)

Trigger points:

1. immediately before child execution
   - emit `start`
   - mark launch started
2. child success + valid replacement body + successful closure
   - emit `success`
3. child success but closure extraction/rewrite fails
   - emit `failure`
4. child exits non-zero or is interrupted
   - emit `failure`

## Error Handling Policy

### Parsing errors

Invalid lifecycle config is a frontmatter error and aborts execution before launch.

Examples:

1. `speak` and `speak_first` both set
2. `start: "string"` instead of object
3. unknown keys
4. invalid sound effect name

### Runtime notification errors

These are warnings only.

Examples:

1. TTS unavailable
2. messaging route missing secrets
3. audio player unavailable

### Handler interaction

Handlers remain the recovery mechanism for harness failures. Lifecycle notifications do not alter handler resolution.

Important rule:

1. if a handler recovers a pre-launch failure and Claudine eventually reaches the first provider launch, no `blocked` notification is emitted
2. if recovery is exhausted before first launch, emit `blocked`
3. if recovery happens after launch and is later exhausted, emit `failure`

## Documentation Impact

Same-change documentation updates should include:

1. `claudine/lib/README.md`
   - mention lifecycle notifications in composition/harness sections
2. composition docs
   - document the new frontmatter schema and timing semantics
3. any guardrails/help docs for inline-compose examples

## Test Plan

### Library tests

Add focused unit tests for `composition/lifecycle.rs`:

1. parses valid lifecycle object
2. rejects both `speak` and `speak_first`
3. trims empty strings to absent
4. rejects unknown keys
5. rejects invalid sound effect names
6. computes correct audio order for:
   - `speak` + `effect`
   - `speak_first` + `effect`
   - speech only
   - effect only

### Messaging reuse tests

Add tests covering resolved-message execution:

1. lifecycle emitter can send rendered message text without `EventMeta`
2. missing route is a no-op

### Wrapper integration tests

Add CLI tests for composition flows:

1. `start.stderr` emits before provider launch
2. `success.stderr` emits after successful direct compose
3. `success.stderr` emits after successful inline closure
4. pre-check failure before launch emits `blocked.stderr`
5. shell-audit denial before launch emits `blocked.stderr`
6. agent non-zero exit after launch emits `failure.stderr`
7. inline closure failure after launch emits `failure.stderr`
8. post-check failure after launch emits `failure.stderr`
9. retry before first launch that eventually succeeds emits `start` and `success`, but not `blocked`
10. retry after launch that eventually fails emits `failure`, not `blocked`
11. `start` is emitted once even across redirect/retry/resume loops

### Audio-order tests

Add unit/integration coverage around scheduling rather than real audio playback:

1. `speak` + `effect` orders effect before speech
2. `speak_first` + `effect` orders speech before effect
3. non-audio targets are dispatched before waiting on phase-two audio

## Recommended Implementation Order

1. add typed lifecycle config parsing to `claudine/lib/src/composition/`
2. extract reusable resolved-message and resolved-speech helpers
3. implement lifecycle emitter with deterministic audio ordering
4. wire lifecycle context into `execute_composition_request`
5. wire lifecycle transitions into `run_harness_loop`
6. wire non-harness direct and inline paths
7. add CLI and unit coverage
8. update docs

## Open Questions Resolved By This Design

1. Is this a harness-only feature?
   - No. It applies to all compose and inline-compose runs.
2. Does `blocked` mean "any failure"?
   - No. It is only for pre-launch terminal stops.
3. Should retries re-emit `start`?
   - No.
4. Are lifecycle messages templated?
   - No in v1.
5. Does missing messaging config fail the run?
   - No.
