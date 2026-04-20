# Leveraging OpenCode Logs Implementation Plan

This plan is based on:

- [`spec.md`](./spec.md)
- [`tech-design.md`](./tech-design.md)
- [`research.md`](./research.md)
- current code in `claudine/lib/src/stream/*`
- current wrapper integration in `claudine/cli/src/commands/wrap/{mod,composition,exec,profile}.rs`

It is intentionally implementation-first and scoped to the current tree, not the idealized design alone.

## Goal

Make OpenCode structured non-interactive runs consume `stderr` log lines in real time so Claudine can:

1. stop hanging on provider usage-cap / rate-limit failures
2. surface malformed skill / command / agent loads as warnings
3. preserve non-classified stderr diagnostics for operators
4. enrich the final `StreamExecutionSummary`, badges, and JSONL summary with stderr-derived diagnostics

## Validated Baseline

The current codebase already gives us the right seam for this work:

- `claudine/cli/src/commands/wrap/exec.rs::run_child_stream_semantic(...)` owns the stdout parser thread, stderr reader thread, child wait, and final `StreamExecutionSummary`.
- structured wrapper call sites exist in exactly two places:
    - `claudine/cli/src/commands/wrap/mod.rs`
    - `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/profile.rs` currently injects only `--format json` for OpenCode structured runs.
- `claudine/lib/src/stream/summary.rs` already has `stderr_text` and `rate_limit`, but no stderr diagnostics model and no absolute reset timestamp.
- `claudine/lib/src/stream/badges.rs` already derives end-of-run badges, so the lowest-risk path is to recompute badges after stderr merge instead of redesigning renderer behavior.
- `claudine/lib/src/stream/semantic.rs` already has usable variants: `Info`, `Warning`, and `Error { terminal, kind, extra }`. No new semantic event variant is required.

## High-Confidence Corrections From The Sample Logs

The sample stderr artifacts in this feature folder change the implementation details in a few important ways:

1. malformed asset lines use `err=... failed to load command`, not only `error=...`
2. uncaught failures can arrive as:
   - structured `ERROR ... service=default name=TypeError ... stack=... fatal`
   - raw ANSI `Error:` blocks on following lines
3. the plan should reuse:
   - `example-of-usage-limit.txt`
   - `errors.txt`
   as the seed material for test fixtures instead of inventing synthetic shapes first

Because of that, the parser/classifier work must explicitly cover both `error=` and `err=`-style tails and must test multi-line stderr sequences, not just isolated single records.

## Scope Decisions

- OpenCode only in this cycle.
- Structured non-interactive runs only.
- Inject `--print-logs --log-level ERROR` only in the OpenCode structured wrapper path.
- Reuse existing `SemanticEvent` variants and `SemanticErrorKind`.
- Keep raw passthrough for unclassified stderr lines.
- Recompute `summary.badges` after stderr enrichment in the wrapper layer, even if provider parsers already populated badges earlier.

## Phase Index

| Phase | Outcome | Depends on |
|---|---|---|
| 0 | Fixture set and parser contract locked | none |
| 1 | Pure OpenCode stderr parser + classifier | 0 |
| 2 | Summary model and badge/reporting support | 1 |
| 3 | Shared sink wrappers and stderr bridge | 1, 2 |
| 4 | Wrapper wiring, stderr suppression, summary merge | 3 |
| 5 | Early termination for pre-stream rate limits | 4 |
| 6 | Focused verification, fixtures, metadata cleanup | 1-5 |

## Phase 0: Lock Inputs And Test Fixtures

Files:

- create `claudine/lib/tests/fixtures/logs/`
- copy/trim from `claudine/features/2026-04-16-leveraging-logs/example-of-usage-limit.txt`
- copy/trim from `claudine/features/2026-04-16-leveraging-logs/errors.txt`

Tasks:

- [ ] Create parser fixtures from the feature artifacts instead of freehand samples.
- [ ] Add at least these fixture files:
    - `opencode-rate-limit.txt`
    - `opencode-malformed-assets.txt`
    - `opencode-uncaught-error.txt`
    - `opencode-mixed.txt`
- [ ] Keep the feature-folder source artifacts untouched; tests should depend on fixture copies under `claudine/lib/tests/fixtures/logs/`.
- [ ] Record the exact supported stderr shapes in comments or test names so later parser changes do not regress the `err=` nuance.

Exit criteria:

- parser fixtures represent the real stderr shapes already captured in the feature folder
- every later phase can build against deterministic local fixtures

## Phase 1: Add `stream::logs::opencode`

Files:

- create `claudine/lib/src/stream/logs/mod.rs`
- create `claudine/lib/src/stream/logs/opencode.rs`
- edit `claudine/lib/src/stream/mod.rs`

Tasks:

- [ ] Add the new `logs` module to the stream tree.
- [ ] Implement the parser types:
    - `OpenCodeLogRecord`
    - `ParsedOpenCodeStderrLine`
    - `LogLevel`
    - `AssetType`
    - `LogClassification`
- [ ] Parse the header with the regex from the design, but keep the body walker procedural and resilient.
- [ ] Parse timestamps to `DateTime<Utc>` rather than storing `NaiveDateTime`; that avoids converting later during summary merge and badge formatting.
- [ ] Support open-ended tags via `BTreeMap<String, String>`.
- [ ] Preserve `raw` on every structured record.

Parser-specific requirements:

- [ ] Treat `error=` as terminal-to-EOL.
- [ ] Handle `err=` lines from malformed asset logs without losing the trailing `failed to load ...` message.
- [ ] Preserve raw unmatched lines as `ParsedOpenCodeStderrLine::RawText`.
- [ ] Extract a typed `reset_at` when present in provider payloads or textual tails.

Classifier requirements:

- [ ] `RateLimit` from structured retry / 429 / code `1308` evidence.
- [ ] `MalformedAsset` from `failed to load skill|command|agent|config`.
- [ ] `ApiFailure` for non-rate-limit `AI_APICallError`.
- [ ] `AuthFailure` for auth-style failures.
- [ ] `UncaughtError` for raw `Error:` / ANSI fatal text and structured fatal TypeError-style records when appropriate.
- [ ] `Unclassified` for everything else.

Unit tests:

- [ ] header accept / reject
- [ ] JSON tag extraction
- [ ] `error=` terminal extraction
- [ ] malformed asset parsing with `err=... failed to load command`
- [ ] rate-limit reset-time extraction
- [ ] uncaught raw ANSI block fallback
- [ ] unknown tag tolerance

## Phase 2: Extend Summary, Badges, And Reporting

Files:

- edit `claudine/lib/src/stream/summary.rs`
- edit `claudine/lib/src/stream/badges.rs`
- edit `claudine/lib/src/stream/reporting.rs`

Tasks:

- [ ] Extend `RateLimitInfo` with `reset_at: Option<DateTime<Utc>>`.
- [ ] Add `StderrDiagnostics` to `summary.rs`.
- [ ] Add `stderr_diagnostics: Option<StderrDiagnostics>` to `StreamExecutionSummary`.
- [ ] Preserve serde compatibility with `#[serde(skip_serializing_if = "Option::is_none")]`.

Badge work:

- [ ] Add `BadgeCategory::Config`.
- [ ] Derive a config badge from malformed-asset counts.
- [ ] Derive a rate-limit badge from stderr diagnostics when `summary.error_kind` did not already yield a stronger badge.
- [ ] Include the formatted reset time in the rate-limit badge message when available.

Reporting work:

- [ ] Serialize `stderr_diagnostics` into `extra["provider_summary"]["stderr_diagnostics"]`.
- [ ] Keep `raw_summary`, `rate_limit`, and `context_usage` behavior unchanged.

Low-risk implementation choice:

- [ ] Do not remove badge derivation from provider parsers in this change.
- [ ] Instead, overwrite `summary.badges` after stderr merge in the wrapper layer so only the final summary path changes.

## Phase 3: Shared Sink Wrappers And `OpenCodeLogBridge`

Files:

- edit `claudine/lib/src/stream/semantic.rs`
- create bridge code in `claudine/lib/src/stream/logs/opencode.rs`

Tasks:

- [ ] Add `SharedSemanticSink<S>` backed by `Arc<Mutex<S>>`.
- [ ] Add `ObservedSemanticSink<S>` that flips `stdout_event_seen` before forwarding the first stdout semantic event.
- [ ] Keep both wrappers generic over `SemanticEventSink`.

Bridge responsibilities:

- [ ] ingest one stderr line
- [ ] parse and classify it
- [ ] update shared stderr summary state
- [ ] emit `SemanticEvent`s for consumed classifications
- [ ] return `Consumed` or `NotConsumed`

Event mapping:

- [ ] `RateLimit` after stdout activity -> `SemanticEvent::Warning`
- [ ] `RateLimit` before stdout activity -> `SemanticEvent::Error { terminal: true, kind: ApiRemote }` plus early-termination signal
- [ ] `MalformedAsset` -> `SemanticEvent::Warning`
- [ ] `AuthFailure` / `ApiFailure` -> `SemanticEvent::Error { terminal: true, kind: ApiRemote }`
- [ ] `UncaughtError` -> `SemanticEvent::Error { terminal: true, kind: Unknown }` unless classification can prove something stronger
- [ ] `Unclassified` -> no semantic event; let raw passthrough continue

Extra payload requirements:

- [ ] include `provider=opencode`
- [ ] include `source=stderr_log`
- [ ] include `classification`
- [ ] include `raw`
- [ ] attach `service`, `status_code`, `error_name`, `asset_type`, `path`, `reset_at` when known

## Phase 4: Wire The Bridge Into The Structured Wrapper Path

Files:

- edit `claudine/cli/src/commands/wrap/mod.rs`
- edit `claudine/cli/src/commands/wrap/composition.rs`
- edit `claudine/cli/src/commands/wrap/exec.rs`
- edit `claudine/cli/src/commands/wrap/profile.rs`

Tasks:

- [ ] Update `OpenCodeWrapper::apply_structured_stream(...)` to append:
    - `--format json`
    - `--print-logs`
    - `--log-level ERROR`
- [ ] Build `SharedSemanticSink` and `stdout_event_seen` in the two structured call sites.
- [ ] Pass `ObservedSemanticSink` into the stdout parser builder for OpenCode.
- [ ] Pass an optional OpenCode stderr bridge into `run_child_stream_semantic(...)`.

`run_child_stream_semantic(...)` changes:

- [ ] add an optional stderr-bridge/control parameter rather than branching by provider internally
- [ ] call the bridge from the stderr thread before deciding whether to passthrough the line
- [ ] suppress raw stderr echo only when the bridge returns `Consumed`
- [ ] keep existing noise-prefix filtering and stderr capture behavior

Summary merge rules:

- [ ] attach `stderr_text` from captured stderr for structured sessions
- [ ] attach `stderr_diagnostics` when at least one structured log line was parsed
- [ ] merge stdout/stderr `rate_limit` field-by-field
- [ ] recompute `summary.badges` after the merge
- [ ] leave existing parser-produced summary fields intact unless stderr explicitly enriches them

Important sequencing choice:

- [ ] do the merge in `run_child_stream_semantic(...)` after stdout parser join and stderr thread join
- [ ] do not let the bridge mutate `StreamExecutionSummary` directly

## Phase 5: Implement Early Termination For Pre-Stream Rate Limits

Files:

- edit `claudine/cli/src/commands/wrap/exec.rs`

Reason this is a separate phase:

The current non-timeout wait path uses `wait_with_signal_handling(...)`, which blocks on `child.wait()`. That is incompatible with stderr-triggered early termination.

Tasks:

- [ ] Add a small control channel from the stderr bridge back to the main wait loop.
- [ ] Introduce a polling wait loop used only when an early-termination receiver is present.
- [ ] Preserve the existing timeout-only helper path for non-bridge runs.
- [ ] Preserve current SIGINT forwarding semantics for structured OpenCode runs when switching to a polling loop.

Recommended implementation shape:

- [ ] use `child.try_wait()` in a loop
- [ ] check the early-termination receiver every 50-100ms
- [ ] respect explicit timeout if one was supplied
- [ ] on early rate-limit:
    - kill the process group
    - stop the heartbeat
    - synthesize non-zero exit state
    - finalize the summary with stderr diagnostics and merged badges

Synthetic summary requirements:

- [ ] `exit_code = 1`
- [ ] `is_error = true`
- [ ] `error_kind = "usage_limit_reached"`
- [ ] `error_message` rendered from the bridge classification
- [ ] `rate_limit.is_throttled = Some(true)`
- [ ] `rate_limit.reset_at` preserved when parsed

## Phase 6: Verification, Metadata, And Cleanup

Files:

- edit `claudine/lib/src/agents/opencode.rs`
- add tests in the touched modules

Tasks:

- [ ] Update `OpenCode` logging metadata so `debug_controls` advertises `--print-logs` and `--log-level ERROR`.
- [ ] Add unit tests for `summary.rs`, `badges.rs`, and `reporting.rs` serde/merge behavior.
- [ ] Add integration tests around the structured wrapper path using synthetic child stdout/stderr.

Required integration scenarios:

- [ ] stderr rate limit before any stdout semantic event -> synthetic failure, child terminated early
- [ ] stderr malformed asset during otherwise-successful stream -> warning event, success summary, config badge
- [ ] mixed structured stderr plus raw ANSI stack text -> parser stays resilient and only classified lines are suppressed
- [ ] final summary contains merged `stderr_text`, `stderr_diagnostics`, `rate_limit`, and recomputed badges

Suggested commands:

- `cargo test -p claudine stream::logs::opencode`
- `cargo test -p claudine summary`
- `cargo test -p claudine badges`
- `cargo test -p claudine reporting`
- `cargo test -p claudine-cli wrap`

## File Plan

New files:

- `claudine/lib/src/stream/logs/mod.rs`
- `claudine/lib/src/stream/logs/opencode.rs`
- `claudine/lib/tests/fixtures/logs/opencode-rate-limit.txt`
- `claudine/lib/tests/fixtures/logs/opencode-malformed-assets.txt`
- `claudine/lib/tests/fixtures/logs/opencode-uncaught-error.txt`
- `claudine/lib/tests/fixtures/logs/opencode-mixed.txt`

Modified files:

- `claudine/lib/src/stream/mod.rs`
- `claudine/lib/src/stream/semantic.rs`
- `claudine/lib/src/stream/summary.rs`
- `claudine/lib/src/stream/badges.rs`
- `claudine/lib/src/stream/reporting.rs`
- `claudine/cli/src/commands/wrap/profile.rs`
- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/lib/src/agents/opencode.rs`

## Risk Register

1. `err=` malformed-asset lines are not shaped like the spec's terminal `error=` lines.
Mitigation: fixture-first parser tests from `errors.txt`; do not assume a single terminal-key rule covers all cases.

2. Early termination can regress current SIGINT / timeout semantics.
Mitigation: isolate the polling wait loop to the OpenCode bridge path and preserve the existing helpers for all other runs.

3. Badge derivation currently happens inside provider parsers.
Mitigation: recompute badges after stderr merge instead of trying to centralize all providers in one refactor.

4. Double-rendering stderr is easy to introduce.
Mitigation: bridge returns explicit `Consumed` / `NotConsumed`; only consumed lines are suppressed from raw passthrough.

## Definition Of Done

- OpenCode structured runs inject `--print-logs --log-level ERROR`.
- stderr log lines are parsed and classified in real time.
- pre-stream rate-limit hangs terminate early with a populated summary.
- malformed skills / commands / agents surface as warnings and a config badge.
- unclassified stderr still reaches operators.
- `StreamExecutionSummary`, badges, and JSONL output all include stderr-derived diagnostics.
- focused unit and integration tests cover the real captured stderr shapes in this feature directory.
