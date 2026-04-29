# Debug Review 2: Remaining Gaps and Next Steps

Follow-up to `debug-review.md`. This document covers items from the original review that are still outstanding, plus new gaps discovered in the same scope.

## What Was Done Since the Last Review

These items from the original review are now resolved:

- **`--debug <level>` global flag added** (`claudine/cli/src/args.rs:39-41`) with all five levels (trace, debug, info, warn, error).
- **Verbose/debug coupling severed**. `-v/--verbose` now only controls presentation detail. Tracing level is independent.
- **DEBUG env var removed**. No custom `DEBUG` override in `main.rs` anymore.
- **Dedicated tracing initialization** extracted into `claudine/cli/src/telemetry.rs` with `init_tracing()`, `build_env_filter()`, and `RelativePathEventFormat`.
- **Precedence contract implemented**: `RUST_LOG` > `--debug <level>` > default `warn`.
- **`FmtSpan::CLOSE` enabled** when debugging is active, giving automatic span timing.
- **Root CLI span** added (`telemetry::root_span()`) with command, subcommand, plain, cwd, repo_root, pid, and wrapper-specific fields (interactive, quiet, silent, repo_mode, mcp_enabled).
- **Wrapper session span** added (`claudine/cli/src/commands/wrap/mod.rs:915-926`) with binary_path, has_prompt, interactive_requested, yolo_requested, model_override, child_pid.
- **Dispatch event span** added (`claudine/lib/src/dispatch/mod.rs:228-237`) with provider, event, session_id, tool_name, can_block, repo_root.
- **Hook action spans** added in `dispatch/runner.rs` (6 `info_span!` sites at lines 90, 103, 116, 137, 250, 263).
- **Wrapper orchestration spans** added throughout wrap/mod.rs (14 span sites covering binary resolution, handler dispatch, attempt cycles, validation stages, launch, and post-processing).
- **Stream module partially instrumented** (`claudine/lib/src/stream/mod.rs` now has 5 `trace!` call sites at lines 102-165; `stream/logs/opencode.rs` has 4 tracing sites).
- **Naming cleanup** done: `verbose_requested` renamed to `detail_requested` in wrap/mod.rs.
- **Help text updated** (`claudine/cli/src/commands/help.rs:164-169`) shows separate `-v` and `--debug` entries.
- **README updated** (`claudine/cli/README.md:154`) documents the debug contract correctly.
- **Passthrough semantics preserved**: no `--debug` extraction from wrapped provider args.

## Still Outstanding

### 1. Protect evaluation has no spans

`claudine/lib/src/services/protect/` has zero tracing call sites. The dispatch layer opens a `dispatch_protect_pre` / `dispatch_protect_post` span boundary around the call, but the protect service itself is a black box.

The original review recommended exposing:

- `policy_mode`
- `posture`
- `finding_count`
- `outcome`
- `redaction_count`
- `provider`
- `event`

These fields should be emitted from inside `evaluate.rs`, not just from the dispatch wrapper. Without them, debugging why a protect decision was made requires reproducing the exact input and reading the source.

### 2. Harness module is nearly silent

`claudine/lib/src/harness/` has one `tracing::warn!` in `speech.rs:37` and nothing else. The original review called out these boundaries for instrumentation:

- plan parse
- pre-validation
- launch attempt
- timeout handling
- post-validation
- handler resolution
- retry or redirect decision

The wrapper layer now has attempt-cycle spans (`wrap/mod.rs:2736-2748`), but those wrap the harness from the outside. The harness internals — where retry/redirect decisions actually happen — remain unobservable.

### 3. Silent modules with zero tracing

These modules still have no tracing call sites at all:

- `claudine/lib/src/linking` — hook registration and linking logic
- `claudine/lib/src/permissions` — permission resolution
- `claudine/lib/src/system_prompt` — system prompt composition
- `claudine/cli/src/commands/compose` — composition pipeline
- `claudine/cli/src/commands/init` — initialization wizard

Any failure or unexpected behavior in these modules is invisible in traces.

### 4. Stream instrumentation is partial

The stream module has improved since the original review (`stream/mod.rs` now has 5 `trace!` sites, `stream/logs/opencode.rs` has 4), but instrumentation is not complete:

- No spans (only `trace!` macros, no `info_span!` or `debug_span!` boundaries).
- No logging when session ID or model becomes known from stream data.
- No logging of tool call increments beyond the dispatch layer.
- No logging when parser falls back, skips, or finishes in an error state.
- `stream/logs/codex.rs` has no tracing of its own (only parsing logic for Codex's tracing-format output).

The original review's recommendation to add per-parser event logging and structured error/fallback tracing is still partially valid.

### 5. No `#[instrument]` usage anywhere

The entire codebase uses manual `info_span!().entered()` / `.in_scope()` patterns rather than the `#[instrument]` attribute. This is not a correctness problem, but it means:

- Span fields must be manually specified at every call site.
- Adding or renaming span fields requires finding all manual construction sites.
- The span name and function name can drift apart.

This is lower priority than filling the silent-module gaps, but worth noting as a consistency improvement.

## New Items in Scope

### 6. Wrapper session span is missing some high-value fields

The wrapper session span (`wrap/mod.rs:915-926`) includes binary_path, has_prompt, interactive_requested, and model_override, but is missing:

- `provider` (available from the enclosing scope but not on the span)
- `session_id` (marked as `Empty` but never populated once known from stream)
- `structured_mode` (marked as `Empty` but unclear if it is ever set)
- `child_pid` (marked as `Empty` — should be recorded when the child is spawned)

Fields declared as `tracing::field::Empty` that are never recorded add noise to span definitions without diagnostic value. Either record them when the data becomes available or remove the declaration.

### 7. No dispatch span for non-canonical events

The dispatch layer opens a span only for the canonical event path (`dispatch_canonical_event`). If the adapter parse fails or the event is skipped before reaching canonical dispatch, there is no span and only a `debug!` line at best. Adding a lightweight span at the adapter parse boundary (which already exists at `dispatch/mod.rs:173`) with the failure reason would make it easier to see why an event was silently dropped.

### 8. Composition and sequence commands have no tracing

`compose` and `sequence` are multi-step orchestration commands (prompt generation, file writes, provider invocation) with zero tracing. These are the CLI-facing analogues of the dispatch pipeline and should have at least:

- a root span per composition/sequence run
- a span per step
- timing for prompt generation and provider invocation phases

### 9. Telemetry formatter does not include span names in output

The custom `RelativePathEventFormat` in `telemetry.rs` renders event messages, detail fields, and file locations, but does not render the span hierarchy (parent span names). When `FmtSpan::CLOSE` is active, the close events emit timing (`time.busy`, `time.idle`), but the human reading the output cannot tell which span closed without correlating timestamps. Consider including the span name in the formatted output, at least at `debug` level.

## Suggested Priority Order

If only a small amount of work is available:

1. **Instrument protect evaluation** — add spans with structured fields inside `services/protect/evaluate.rs`.
2. **Instrument harness internals** — add spans for retry/redirect decisions and validation outcomes.
3. **Fill the `field::Empty` promises** — populate `child_pid`, `session_id`, and `structured_mode` on the wrapper session span, or remove the declarations.
4. **Add tracing to composition/sequence commands** — these are complex orchestration paths with no visibility.
5. **Add spans to stream parsers** — promote `trace!` log lines to span boundaries with structured fields.
6. **Add tracing to linking, permissions, system_prompt** — fill the remaining silent modules.
7. **Consider `#[instrument]` migration** — lower priority consistency improvement.

## Summary

The original review had 9 recommendations. Substantial progress has been made on recommendations 1 (separate verbose/debug), 2 (dedicated tracing substrate), 3 (orchestration spans), and 8 (naming cleanup). Recommendations 4 (performance visibility), 5 (structured trace fields), 6 (stream instrumentation), 7 (passthrough semantics), and 9 (doc updates) are partially or fully addressed.

The biggest remaining gaps are the completely silent subsystems (protect, harness, linking, permissions, system_prompt, compose, init) and the unpopulated `field::Empty` promises on the wrapper session span.
