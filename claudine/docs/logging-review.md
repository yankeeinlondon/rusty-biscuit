# Claudine Logging Review

## Scope

This review covers Claudine's logging and log-reporting implementation against:

- `claudine/docs/functional-overview.md`
- `claudine/docs/log-reporting.md`

I reviewed the library reporting stack, the CLI reporting command, log ingestion, aggregation, derived metrics, and the available tests. I also ran `just test` inside `claudine/`.

## Executive Summary

The high-level architecture is mostly pointed in the right direction:

- The `claudine logs` reporting path is library-first. Schema management, ingestion, typed queries, and metric computation live in `claudine/lib/src/reporting/`, while `claudine/cli/src/commands/logs.rs` is mostly argument parsing, sync orchestration, and rendering.
- The JSONL log action itself is library-owned, which is also correct.

The main problems are not the overall layering. They are in the correctness and fit of the aggregation layer:

1. `Report` actions can write to `stdout` before Claudine emits a blocking provider response, which risks corrupting hook protocols.
2. Reporting only indexes the default `~/.claudine/logs` tree, so custom file targets and server-only logging can make `claudine logs` partial or blind.
3. Several derived metrics are computed from truncated tool lists, so they change when `--top` changes.
4. Error recovery is materially under-counted because it is keyed too coarsely and counts unique recovered tool/session pairs instead of recovered error events.
5. Incremental sync can silently miss rewritten files, and a single malformed line aborts ingestion for the entire file.
6. Session and repo aggregation semantics are weaker than the design intends.
7. The implementation only covers part of the reporting surface described in the design docs.
8. Test coverage for reporting behavior is too thin relative to the amount of query and ingestion logic now present.

## Findings And Recommendations

### P1: `Report` actions currently share `stdout` with blocking hook responses

`execute_actions()` always executes `HookAction::Report` inline (`claudine/lib/src/dispatch/runner.rs:23-40`), and `execute_report()` prints directly to `stdout` via `println!` (`claudine/lib/src/dispatch/runner.rs:359-369`). For blockable events, dispatch then returns the provider response, and `claudine handle` prints that provider payload to the same `stdout` stream afterward (`claudine/lib/src/dispatch/mod.rs:140-181`, `claudine/cli/src/commands/handle.rs:31-52`).

That is fine for non-blocking observation events, but it is unsafe for blocking hooks because one transport is now carrying both:

- human-facing report text
- machine-facing provider response payloads

Recommendation:

- Do not allow `Report` to emit on blocking events, or route it to `stderr` instead.
- If `Report` needs to stay available everywhere, make dispatch return side-channel report messages and let the CLI decide where they can be safely written.

### P1: `claudine logs` is not guaranteed to see all canonical log data

The reporting design treats JSONL under `~/.claudine/logs/` as the canonical source for `metrics.db`, and `ReportingStore::open_default()` always indexes that tree (`claudine/lib/src/reporting/paths.rs:29-36`, `claudine/lib/src/reporting/mod.rs:29-33`). But runtime config allows:

- per-action file targets
- a global `default_log_target`
- server-only log targets

See `LogTarget` and `GlobalSettings.default_log_target` (`claudine/lib/src/actions/hook_action.rs`, `claudine/lib/src/events/config.rs:40-47`) plus log-target resolution in the runner (`claudine/lib/src/dispatch/runner.rs:36-39`, `claudine/lib/src/dispatch/runner.rs:297-356`).

Recommendation:

- Decide whether the default log tree is mandatory or merely a convenience.
- If mandatory, validate config so every reporting-enabled setup still writes the canonical JSONL stream under `~/.claudine/logs/`.
- If optional, the reporting store needs explicit source registration so `claudine logs` can discover custom file targets as part of normal operation.

### P1: Derived metrics currently depend on presentation limits

`daily_summary`, `sessions`, and `tools` all compute metrics from the same `tools` slice they also use for display (`claudine/lib/src/reporting/queries.rs:94-123`, `claudine/lib/src/reporting/queries.rs:136-149`, `claudine/lib/src/reporting/queries.rs:162-177`). That slice is truncated by `load_tool_stats(..., top_n)` before `summarize_metrics()` sees it, and `summarize_metrics()` derives research/action/delegation ratios directly from the provided tool rows (`claudine/lib/src/reporting/metrics.rs:35-78`).

That means:

- `DailySummary.metrics.research_vs_action_ratio` is based on only the top 5 tools.
- `SessionsReport.metrics.research_vs_action_ratio` is based on only the top 25 tools.
- `ToolsReport.metrics.*` can change just by changing `--top`.

Recommendation:

- Split "full aggregation input" from "display-limited ranked list".
- Compute derived metrics from an unbounded aggregate query.
- Keep `top_tools` as a separate, presentation-oriented slice.

### P1: Error recovery rate is not counting recovered errors correctly

The recovery calculation uses `(session_key, tool_name)` as its identity and stores recoveries in a `HashSet`, so repeated recoveries for the same tool in the same session collapse to one recovered unit (`claudine/lib/src/reporting/metrics.rs:81-107`).

This under-counts recovery in exactly the kind of sessions the report is meant to analyze:

- repeated retries of the same tool
- concurrent calls to the same tool
- providers that expose a better correlation key such as `tool_use_id`

Recommendation:

- Track recovery at the individual tool-attempt level, not per `(session, tool_name)`.
- Promote a stable correlation key into the reporting schema when available, ideally `tool_use_id`.
- Fall back to `(session_key, tool_name, ordinal)` only when the provider gives no better identifier.

### P1: Incremental sync can miss rewritten files, and malformed lines are too destructive

The rebuild decision only looks at file shrinkage or a hash of the first 4096 bytes (`claudine/lib/src/reporting/ingest.rs:142-163`, `claudine/lib/src/reporting/ingest.rs:325-329`). `modified_at` is recorded but not used. A same-sized rewrite beyond the first 4 KB can therefore be missed entirely.

Separately, any parse error aborts `sync_file()` and rolls back the whole transaction for that file (`claudine/lib/src/reporting/ingest.rs:238-255`, `claudine/lib/src/reporting/ingest.rs:300-306`). One bad line prevents all later good lines from being ingested.

Recommendation:

- Use a stronger rebuild signal: `modified_at` plus full-file fingerprint, or at least a stronger sampled fingerprint.
- Treat malformed lines as per-line failures and continue ingesting the rest of the file.
- Make `parse_failures` count actual bad records, not just failed files.

### P1: Session and repo aggregation are not yet fit for purpose

`session_count` is currently `COUNT(DISTINCT session_key)` over all events in range (`claudine/lib/src/reporting/queries.rs:416-427`), not a count of `session_start` events. That diverges from the design doc's session metric definition and is also sensitive to conservative anonymous fallbacks.

Session summaries are reconstructed from raw events using `MAX(...)` for repo, branch, model, permission mode, and package fields (`claudine/lib/src/reporting/queries.rs:514-540`). For sessions that cross branches, packages, or repos, the label becomes "lexicographically largest value seen", not a meaningful first/last/primary value.

Repo aggregation is keyed only by `repo_name` (`claudine/lib/src/reporting/queries.rs:281-323`), and total repo count is also based only on `repo_name` (`claudine/lib/src/reporting/queries.rs:426-427`). That will conflate `org-a/foo` and `org-b/foo`.

Recommendation:

- Count sessions explicitly from `session_start`, while separately tracking "active sessions seen in range" if that is also useful.
- Persist session identity quality so anonymous fallbacks can be excluded or clearly labeled in summaries.
- Key repos by `repo_org/repo_name` or, better, by repo root.
- Replace `MAX(...)` session labeling with explicit semantics: first seen, last seen, or most frequent.
- Either start reading from the `sessions` table or remove it until it becomes the real read model. Right now it adds write complexity but is not used by query paths (`claudine/lib/src/reporting/schema.rs:64-78`, `claudine/lib/src/reporting/queries.rs:506-540`).

### P2: The shipped reporting surface is only a subset of the intended design

The design doc describes `claudine logs trends --days N` (`claudine/docs/log-reporting.md:274-284`), but the CLI exposes no `--days` flag at all (`claudine/cli/src/commands/logs.rs:22-60`). The implementation supports `week`, `month`, or explicit `--from/--to`, which is workable, but it is not what the design currently promises.

More broadly, the reporting types only expose a narrow subset of the documented metrics (`claudine/lib/src/reporting/types.rs:122-143`). Missing or only partially implemented items include:

- daily/session duration in the summary output
- file read/edit drilldowns
- bash command summaries
- write volume
- repo switch frequency
- subagent duration
- busiest hours / day-of-week patterns
- tool diversity index
- skill invocation analysis
- sparkline-style trend rendering from the examples

Recommendation:

- Decide whether the doc is aspirational or normative.
- If normative, implement the missing library query types first, then expose them through the CLI.
- If aspirational, explicitly mark the unfinished items as future work and trim the current README/docs to the shipped surface.

### P2: Some reporting output is structurally inconsistent

`DerivedMetrics` includes `delegation_ratio` (`claudine/lib/src/reporting/types.rs:94-100`), but the CLI never renders it (`claudine/cli/src/commands/logs.rs:648-668`). That is a straightforward completeness gap.

The `Report` hook action's JSON mode also emits a different JSON shape than the canonical log schema: it rewrites `event` into PascalCase and strips nulls (`claudine/lib/src/dispatch/runner.rs:372-440`, `claudine/lib/src/dispatch/runner.rs:467-491`). That may be fine for terminal-friendly output, but it is a poor machine-oriented reporting contract because it diverges from the JSONL event stream.

The same issue shows up in the reporting query layer: `queries::errors()` truncates `error` and derives a shortened `context` before the CLI ever sees the data (`claudine/lib/src/reporting/queries.rs:182-237`), and `ErrorRecord` has no raw payload fields (`claudine/lib/src/reporting/types.rs:180-188`). That means `claudine logs --json` cannot return a fuller machine-oriented error record even though the docs position the CLI as the formatting layer.

Recommendation:

- Render `delegation_ratio` anywhere the other derived metrics are rendered.
- For `ReportFormat::Json`, either emit canonical `EventMeta` field conventions or clearly document that it is a terminal-facing convenience format rather than a stable machine schema.
- Keep truncation and terminal summarization in the CLI. The library should return typed raw/structured fields and let each renderer decide how much to abbreviate.

### P3: The CLI/library reporting boundary is mostly correct, with one caveat

For the `claudine logs` feature, the boundary is good. Business logic is in the library, and the CLI is mostly orchestration plus rendering (`claudine/lib/src/reporting/mod.rs`, `claudine/cli/src/commands/logs.rs:87-165`).

The one caveat is `HookAction::Report`: formatting and stdout emission happen directly inside the library runner via `println!` (`claudine/lib/src/dispatch/runner.rs:359-369`). I would not treat this as a major problem today because hook execution belongs in the library, but it does mean presentation is not completely abstracted from business execution.

Recommendation:

- Keep the `logs` architecture as-is.
- If stricter separation matters later, inject an output sink/writer into dispatch rather than printing directly from the library.

## Test Coverage Review

Current coverage is not proportionate to the amount of reporting logic now in place.

What exists:

- one store-level filter test (`claudine/lib/src/reporting/mod.rs:129-210`)
- one schema migration test (`claudine/lib/src/reporting/schema.rs:219-280`)
- three metric unit tests (`claudine/lib/src/reporting/metrics.rs:118-160`)
- one log-file write test for the log action (`claudine/lib/src/dispatch/runner.rs:673-698`)
- two CLI integration tests around `handle` writing logs and preserving package context (`claudine/cli/tests/handle_repo_config.rs:51-154`)

What is missing:

- query correctness tests for `daily_summary`, `sessions`, `tools`, `errors`, `repos`, and `trends`
- sync tests for file truncation, same-size rewrite, malformed lines, duplicate reruns, and mixed dated/non-dated files
- metrics tests covering repeated retries, concurrent same-tool calls, and anonymous session fallbacks
- CLI tests for `claudine logs` subcommands, both terminal and `--json`
- tests that assert the CLI-rendered metrics stay stable when `--top` changes
- tests for `LogTarget::Server`
- tests for `ReportFormat::{Text,Compact,Json}` beyond the current JSON shape smoke test

Recommendation:

- Add fixture-driven end-to-end reporting tests in the library first.
- Add a focused CLI integration suite for `claudine logs ... --json` so rendering bugs do not hide query bugs.
- Add regression tests for the specific aggregation flaws called out above before changing the implementation.

## Verification Notes

I ran `just test` inside `claudine/`.

- `claudine` library tests passed.
- Most `claudine-cli` tests passed.
- Two unrelated PTY tests failed: `pty_non_interactive_detection` and `pty_wrapper_summary_shows_badges`.

That failure does not appear to be caused by the logging/reporting code, but it does mean the package-level test run is not currently fully green.
