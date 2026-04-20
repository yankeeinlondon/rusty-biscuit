# Debug Review: Claudine Library and CLI

This review focuses on making Claudine materially easier to debug in production and during local development, without conflating human-facing verbosity with diagnostic logging.

## Summary

The current direction is good but incomplete:

- The CLI already initializes `tracing` and supports `RUST_LOG`.
- The wrapper already emits useful human-facing stderr summaries.
- The dispatch pipeline has a few good `debug!` and `info!` checkpoints.

The main problems are:

- Debugging is currently coupled to `-v/--verbose` in both behavior and help text.
- Tracing coverage is shallow and uneven across the codebase.
- There are no spans or timing boundaries, so flow and performance are hard to reconstruct.
- The most important modules for debugging provider execution, stream parsing, harness retries, and policy decisions are either lightly traced or completely silent.

## Current State

### 1. Debugging is coupled to verbosity today

These places currently mix presentation verbosity with diagnostic intent:

- `claudine/cli/src/args.rs:14-16`
- `claudine/cli/src/main.rs:28-29`
- `claudine/cli/src/main.rs:105-109`
- `claudine/cli/src/commands/help.rs:164-165`
- `claudine/cli/src/commands/wrap/mod.rs:751`
- `claudine/cli/src/commands/wrap/mod.rs:3230-3232`
- `claudine/cli/README.md:135-136`
- `claudine/lib/src/agents/codex.rs:147`

The most direct issue is that `-vv` currently means "debug" while `-v` is also used for richer human output. That creates an ambiguous contract:

- Sometimes `verbose` means "show more UI detail".
- Sometimes it means "raise tracing level".
- In wrapper mode it can also be extracted from passthrough args instead of being forwarded.

That is exactly the coupling we should remove.

### 2. Human-facing summaries are better than the tracing layer

The wrapper summaries are already useful for users:

- `claudine/cli/README.md:134-148`
- `claudine/cli/src/output.rs:26-116`
- `claudine/cli/src/output.rs:119-186`
- `claudine/cli/src/output.rs:189-236`

But these are not a substitute for debugging traces. They tell the operator what happened at a coarse level, not why Claudine took a branch, how long a stage took, or where a failure was introduced.

### 3. Tracing exists, but coverage is sparse and mostly log-line based

A quick scan shows roughly 65 tracing macro call sites across `claudine/lib/src` and `claudine/cli/src`, but zero `#[instrument]` usages and effectively no span-based structure.

Coverage is concentrated in a few places:

- `dispatch`: 29 call sites
- `messaging`: 11 call sites
- `wrap`: 7 call sites
- `services/protect`: 6 call sites
- `composition`: 6 call sites

Several important areas are silent or nearly silent:

- `claudine/lib/src/stream`: 0 call sites
- `claudine/lib/src/harness`: 1 call site
- `claudine/lib/src/linking`: 0 call sites
- `claudine/lib/src/permissions`: 0 call sites
- `claudine/lib/src/system_prompt`: 0 call sites
- `claudine/cli/src/commands/compose`: 0 call sites
- `claudine/cli/src/commands/init`: 0 call sites

For a tool whose main job is orchestrating provider execution and event handling, this is not enough.

### 4. The existing tracing is useful, but not structured enough

Examples of good early signals:

- Dispatch start and skip reasons in `claudine/lib/src/dispatch/mod.rs:109-143`
- Per-action warnings in `claudine/lib/src/dispatch/runner.rs:93-120`
- Policy snapshot resolution in `claudine/lib/src/services/protect/evaluate.rs:37-89`
- Wrapper stream dispatch and malformed-line suppression in `claudine/cli/src/commands/wrap/mod.rs:352-359`, `claudine/cli/src/commands/wrap/mod.rs:532-537`, and `claudine/cli/src/commands/wrap/exec.rs:726-730`

What is missing is hierarchy:

- No root span for a CLI invocation
- No session span for a wrapped provider run
- No event span for a dispatched hook
- No action span for each hook action
- No attempt span for harness retries and validations
- No structured latency measurements

That makes correlation and performance analysis much harder than it needs to be.

## Recommendations

### 1. Separate presentation verbosity from diagnostics completely

`-v/--verbose` should remain a human-output switch only.

It should control things like:

- showing more detailed wrapper summaries
- showing full composed prompts
- expanding agent/skill/command listings
- richer stderr presentation in interactive use

It should not affect:

- tracing level
- subscriber configuration
- whether debug logs are emitted

### Recommended CLI contract

Add a new global option:

```text
--debug <level>
```

Supported levels should be explicit:

- `trace`
- `debug`
- `info`
- `warn`
- `error`

I would not add a short `-d` alias at the top level right now. Wrapped providers often already use `-d` or `--debug`, and Claudine should not make that ambiguity worse.

### Recommended precedence

Use standard `RUST_LOG` conventions as the primary escape hatch:

1. `RUST_LOG`
2. `--debug <level>`
3. default filter

That keeps the standard Rust behavior for advanced users while making debugging discoverable for everyone else.

### Recommended defaults

- Default level with no debug flag: `warn`
- `--debug debug`: equivalent to `RUST_LOG=claudine=debug`
- `--debug trace`: equivalent to `RUST_LOG=claudine=trace`

If you want per-module granularity, use `RUST_LOG` directly:

```bash
RUST_LOG=claudine::dispatch=trace,claudine::stream=debug claudine codex "..."
```

### Strong recommendation: de-emphasize or remove the custom `DEBUG` env override

`claudine/cli/src/main.rs:89-98` currently treats `DEBUG` as a logging override. That is nonstandard and unnecessary once `--debug <level>` exists.

I would either:

- remove it, or
- keep it temporarily as deprecated compatibility behavior

But the public contract should be `RUST_LOG` plus `--debug <level>`, not `DEBUG`.

### 2. Introduce a real tracing substrate, not just more log lines

The current `tracing_subscriber::fmt().init()` setup in `claudine/cli/src/main.rs:31-34` is enough to print logs, but not enough to provide strong debugging support.

### Recommended subscriber behavior

Create a dedicated tracing initialization path with:

- an `EnvFilter`
- a human-readable stderr layer
- optional span lifecycle events
- consistent target/module display
- optional future JSON/file sink support

Important feature: enable span close events when debugging is on.

Specifically, configure `FmtSpan::CLOSE` for debug sessions so long-running spans emit timing on completion. That gives immediate flow and performance value without adding manual timers everywhere.

### Recommended direction

Introduce something like:

- `DebugConfig`
- `init_tracing(config: &DebugConfig)`
- `build_env_filter(rust_log: Option<&str>, debug_level: Option<LevelFilter>)`

Keep this logic out of `main.rs` so it is easy to test and reason about.

### 3. Add spans at the actual orchestration boundaries

This is the biggest missing piece.

### Root CLI span

Every invocation should start with a root span containing:

- `command`
- `subcommand`
- `plain`
- `cwd`
- `repo_root` when known
- `pid`

For wrapped runs, add:

- `provider`
- `interactive`
- `quiet`
- `silent`
- `repo_mode`
- `mcp_enabled`

### Wrapper session span

`claudine/cli/src/commands/wrap/mod.rs:728-820` is the beginning of the highest-value execution flow. It should open a session span before the heavy lifting begins.

That span should eventually include:

- `provider`
- `binary_path`
- `structured_mode`
- `has_prompt`
- `interactive_requested`
- `yolo_requested`
- `model_override`
- `session_id` once known
- `child_pid` once spawned

### Dispatch event span

`claudine/lib/src/dispatch/mod.rs:102-223` should run inside a span with fields like:

- `provider`
- `event`
- `session_id`
- `tool_name`
- `can_block`
- `repo_root`

This is the natural unit for understanding why a hook did or did not run.

### Hook action spans

`claudine/lib/src/dispatch/runner.rs:23-134` should emit a child span per action with:

- `action_index`
- `action_kind`
- `blocking`
- `timeout_ms` for call actions
- `target_kind` for log actions
- `command` for shell actions

This is where timing matters most.

### Protect evaluation span

`claudine/lib/src/services/protect/evaluate.rs` already logs some useful decisions, but it should also expose:

- `policy_mode`
- `posture`
- `finding_count`
- `outcome`
- `redaction_count`
- `provider`
- `event`

This will make debugging safety and permission behavior much easier.

### Harness attempt span

The harness layer is nearly silent today, but it is exactly the kind of subsystem that needs traces. Add spans around:

- plan parse
- pre-validation
- launch attempt
- timeout handling
- post-validation
- handler resolution
- retry or redirect decision

This is where "why did Claudine retry, resume, or redirect?" should become obvious.

### 4. Make performance visible at the same boundaries

The user asked for traces that help with both flow and performance. The answer is not separate performance logging. The answer is timed spans on orchestration units.

### Measure these first

- wrapper preflight duration
- binary resolution duration
- MCP session composition duration
- config load and compile duration
- dispatch duration per event
- protect evaluation duration
- hook action duration
- child process total runtime
- stream parse duration
- summary write duration
- harness validation duration

### High-value latency questions the traces should answer

- Why was a wrapper launch slow before the child even started?
- Did stream parsing degrade because the provider emitted malformed lines?
- Was delay inside provider runtime, hook dispatch, policy evaluation, or post-processing?
- Which hook action dominated total event time?
- How much time was spent in retries versus useful work?

### 5. Add structured trace fields, not just string messages

If the goal is debuggability, fields matter more than prose.

### Good stable fields

- `provider`
- `event`
- `session_id`
- `tool_name`
- `tool_call_id` where available
- `model`
- `action_index`
- `action_kind`
- `policy_mode`
- `protect_outcome`
- `repo_root`
- `cwd`
- `child_pid`
- `exit_code`
- `attempt`

### Avoid by default

- full prompt text
- full tool input payloads
- raw env var values
- message bodies
- MCP secrets

### Better alternatives

- lengths
- hashes
- counts
- selected keys
- redacted previews

The tracing system should be safe by default. Deep payload logging can be allowed at `trace` level, but only after redaction rules are explicit and tested.

### 6. Instrument the stream layer directly

The `stream` directory currently has zero tracing call sites, which is a major observability gap.

That matters because stream parsing is one of the hardest parts of this system to debug:

- provider output formats drift
- malformed lines appear in stdout
- session IDs and models may arrive late
- tool lifecycle events need correlation
- summaries are synthesized after the fact

### Recommended stream instrumentation

For each parser:

- log when a recognizable event type is parsed
- log when session ID or model becomes known
- log tool call increments
- log when summary fields are updated
- log when parser falls back, skips, or finishes in an error state

Key files:

- `claudine/lib/src/stream/*.rs`
- `claudine/cli/src/commands/wrap/exec.rs:706-739`
- `claudine/cli/src/commands/wrap/mod.rs:339-539`

This should be mostly `trace!` and span fields, not noisy user-facing stderr.

### 7. Stop stealing debug semantics from wrapped providers

This matters once Claudine gets its own `--debug <level>`.

Today wrapper passthrough extraction already interprets `--verbose` inside wrapped args:

- `claudine/cli/src/commands/wrap/mod.rs:3230-3232`

Do not repeat that pattern for `--debug`.

### Recommended rule

- `claudine --debug debug codex ...` means "debug Claudine"
- `claudine codex -- --debug ...` means "debug the child provider"

That keeps the contract clean and avoids hard-to-explain flag theft.

### 8. Clean up naming inside the code

A lot of internal variables currently use `verbose` when they really mean "show more human detail".

Examples:

- `verbose_requested`
- `cli.verbose`
- help text that describes `-vv` as debug

I recommend renaming internal presentation toggles toward:

- `detail_requested`
- `show_extended_summary`
- `ui_verbose`

This is not cosmetic. It will reduce future regressions where someone accidentally ties tracing behavior back to the human-output flag.

### 9. Update the public docs and capability metadata

Once the debug model is fixed, the docs should reflect the separation consistently.

Update at least:

- `claudine/cli/src/args.rs`
- `claudine/cli/src/commands/help.rs`
- `claudine/cli/README.md`
- `claudine/lib/src/agents/codex.rs`
- any wrapper help snapshots that still say `-vv` means debug

I would also update the Claudine capability metadata to list debug controls as:

- `RUST_LOG`
- `--debug <level>`

and not `--verbose`.

## Suggested Implementation Plan

### Phase 1: Fix semantics and entrypoint wiring

- Add global `--debug <level>`
- Stop deriving tracing level from `-v/--verbose`
- Prefer `RUST_LOG`, then `--debug`, then default `warn`
- Remove or deprecate `DEBUG`
- Update help text, README, and snapshots

### Phase 2: Add root spans and timing

- Add root CLI span
- Add wrapper session span
- Add dispatch event span
- Add hook action spans
- Enable span close timing in debug sessions

### Phase 3: Fill the biggest observability gaps

- Instrument `stream`
- Instrument `harness`
- Expand protect tracing from branch logs to structured spans
- Add safe payload summaries and redaction-aware trace fields

### Phase 4: Optional richer debug sinks

- optional `--debug-file <path>`
- optional JSON trace output
- optional OTEL bridge

This aligns with the existing research direction in `claudine/docs/research/agent-observability/integration-strategies.md`, but Phase 1 through Phase 3 should happen before external observability backends.

## Concrete Examples

### Good user-facing behavior

```bash
claudine --debug debug codex "summarize this repository"
```

Expected result:

- normal Claudine UX on stdout/stderr
- tracing emitted to stderr
- no dependency on `-v`

```bash
claudine -v codex "summarize this repository"
```

Expected result:

- richer human-facing summary
- no debug logs unless `RUST_LOG` or `--debug` is set

```bash
RUST_LOG=claudine::dispatch=trace,claudine::stream=debug claudine codex "summarize this repository"
```

Expected result:

- highly targeted diagnostics for flow and parser issues
- no need to over-log unrelated modules

## Highest-Priority Suggestions

If only a small amount of work happens now, do these first:

1. Remove the `-vv == debug` behavior.
2. Add a global `--debug <level>` flag.
3. Move tracing setup into a dedicated initialization function.
4. Add root spans for wrapper sessions and dispatch events.
5. Add action timing in `dispatch::runner`.
6. Instrument the `stream` and `harness` modules, which are currently the biggest blind spots.

That would produce a much better debugging environment without changing Claudine's user-facing output model or overcomplicating the CLI.
