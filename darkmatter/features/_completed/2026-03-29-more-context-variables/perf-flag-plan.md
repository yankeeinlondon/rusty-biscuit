# Plan: `md compose --perf`

## Goal

Add a `--perf` flag to `md compose` that prints a structured performance report to `STDERR` after a successful compose run completes. The report should:

- never contaminate `STDOUT`
- work for markdown, HTML, JSON, and `--show` compose flows
- be rendered through `biscuit_terminal::components::block_quote::BlockQuote`
- use a yellow left block via `with_left_block_color(Color::BasicColor(BasicColor::Yellow))`

## Current State

The existing compose flow already has the right high-level insertion points:

- `darkmatter/cli/src/args.rs`
    - `Command::Compose` owns the compose-specific CLI flags
- `darkmatter/cli/src/commands.rs`
    - `run_subcommand()` threads compose flags into `run_compose(...)`
    - `run_compose(...)` already has a post-stdout `STDERR` diagnostics section for compose warnings and deferred validation issues
- `darkmatter/lib/src/markdown/compose/mod.rs`
    - the compose executor already has explicit stage boundaries:
        - effective state build
        - inline pre operations
        - transclusion phase
        - inline post operations
- `darkmatter/lib/src/markdown/compose/types.rs`
    - `ComposeReport` is the existing structure for compose diagnostics and counts
    - `ComposeReport::merge(...)` already aggregates child transclusion results
- `biscuit-terminal/lib/src/components/block_quote.rs`
    - `BlockQuote` supports `with_left_block_color(...)`
    - the correct yellow constructor is `Color::BasicColor(BasicColor::Yellow)`

## Design Decisions

### 1. Collect timings in the library, render them in the CLI

The timing data should be gathered inside the compose library, not reconstructed in the CLI. The CLI only sees one top-level `md.compose_with(options)` call, while the library knows about:

- stage ordering
- recursive child composes
- transclusion preparation vs resolution vs application
- child report merging

This keeps the performance model correct across transclusion graphs and makes the data reusable outside the CLI if needed later.

### 2. Keep the flag opt-in and make the instrumentation no-op by default

`--perf` should enable timing collection explicitly. When the flag is absent:

- no perf report should be rendered
- `ComposeReport` should carry no perf payload
- instrumentation paths should short-circuit quickly instead of always paying `Instant`/allocation overhead

### 3. Aggregate across the full compose graph

The most useful report is the total cost of the full compose operation, including recursively composed child markdown files. The plan should therefore aggregate timings through `ComposeReport::merge(...)` rather than emit one section per child file.

That keeps the output compact and answers the practical question: "where did the full compose run spend time?"

### 4. Separate command-envelope timings from compose-pipeline timings

The compose hot path currently includes meaningful work outside the library pipeline:

- input loading
- source-path resolution
- runtime context capture
- reference validation
- compose options construction

Those should appear in a separate `Command Setup` section in the final stderr report. The library-owned timings should appear under `Compose Pipeline`.

This makes the report useful for the real `md compose` command instead of only the inner library call.

## Proposed Data Model

### CLI-local timings

Add a small CLI-only struct in `darkmatter/cli/src/commands.rs` or a nearby helper module:

```rust
struct CliComposePerfReport {
    load_input: Duration,
    resolve_input: Duration,
    capture_context: Duration,
    validate_references: Duration,
    build_options: Duration,
    compose_pipeline: Duration,
    total: Duration,
}
```

This is only for the command envelope around `run_compose(...)`.

### Library timings

Extend `darkmatter/lib/src/markdown/compose/types.rs` with opt-in perf structs:

```rust
pub struct ComposePerfMetric {
    pub name: String,
    pub elapsed: Duration,
    pub calls: usize,
}

pub struct ComposePerfReport {
    pub total: Duration,
    pub metrics: Vec<ComposePerfMetric>,
}
```

Recommended metric set for the first version:

- `effective_state_build`
- `text_replacement`
- `page_blocks`
- `interpolation`
- `shell_expansion`
- `transclusion_parse`
- `transclusion_prepare`
- `transclusion_resolve`
- `transclusion_apply`
- `cleanup`
- `normalization`

Then add to `ComposeReport`:

```rust
pub perf: Option<ComposePerfReport>
```

`ComposeReport::merge(...)` should sum matching metric durations and call counts so recursive transclusions roll up naturally.

## Instrumentation Plan

### Step 1. Add the CLI flag

File: `darkmatter/cli/src/args.rs`

- Add `perf: bool` to `Command::Compose`
- Help text should make the stderr behavior explicit, for example:
    - `Emit a compose performance report to stderr after completion`

File: `darkmatter/cli/src/commands.rs`

- Thread the new `perf` argument through `run_subcommand(...)`
- Extend `run_compose(...)` to accept a `perf: bool` parameter

### Step 2. Add opt-in perf support to `ComposeOptions`

File: `darkmatter/lib/src/markdown/compose/types.rs`

- Add a boolean to `ComposeOptions`, for example `perf_enabled: bool`
- Add a builder like `with_perf(enabled: bool) -> Self`
- Default should remain `false`

This keeps the instrumentation contract explicit and avoids timing work when the flag is not requested.

### Step 3. Add perf report types and merging

File: `darkmatter/lib/src/markdown/compose/types.rs`

- Add `ComposePerfMetric`
- Add `ComposePerfReport`
- Add `perf: Option<ComposePerfReport>` to `ComposeReport`
- Update `ComposeReport::new()` / `Default`
- Update `ComposeReport::merge(...)` so:
    - `total` is summed or otherwise rolled into the parent consistently
    - matching stage metrics are merged by metric name
    - call counts are accumulated

Important detail:

- The merged metrics should be deterministic in output order
- Do not depend on hash-map iteration order for final rendering

### Step 4. Add a small internal timing helper

Suggested file: `darkmatter/lib/src/markdown/compose/perf.rs`

Add a no-op-friendly helper to keep `mod.rs` readable, for example:

- `PerfCollector`
- `PerfMetricKind` enum
- `measure(kind, || ...)`
- `record(kind, duration)`

This avoids scattering raw `Instant::now()` blocks throughout the compose executor.

### Step 5. Instrument the library compose pipeline

File: `darkmatter/lib/src/markdown/compose/mod.rs`

Measure the following boundaries:

1. `run_compose_pipeline_internal(...)`
   - total compose time
2. effective state construction
   - from `EffectiveStateBuilder::new()` through `.build()?`
3. inline pre operations
   - one timing per operation:
     - replacement
     - page blocks
     - interpolation
     - shell expansion
4. transclusion phase
   - parse directives/frontmatter/toc discovery
   - prepare transclusions
   - resolve prepared transclusions
   - apply replacements/sections back to the parent document
5. inline post operations
   - cleanup
   - normalization

Implementation notes:

- record metrics only when `options.perf_enabled` is true
- keep the per-operation timing inside the existing helper methods where possible
- child compose calls should produce child `ComposeReport` perf payloads that get merged into the parent

### Step 6. Capture CLI-envelope timings

File: `darkmatter/cli/src/commands.rs`

In `run_compose(...)`, time these top-level steps when `perf` is true:

1. `load_markdown(input)`
2. input path resolution (`resolve_file_path`)
3. `ComposeContext::capture()`
4. validation block (`md.validate_references(...)`)
5. compose options construction and mutation
6. `md.compose_with(options)`

Then compute a top-level command total.

Notes:

- Validation timing should be `0` or omitted for stdin-based compose where validation does not run
- The CLI timings should not be shoved into `ComposeReport`; keep them local to the command and combine them only when formatting the stderr output

## STDERR Rendering Plan

### Step 7. Add a dedicated formatting helper

File: `darkmatter/cli/src/commands.rs` or `darkmatter/cli/src/output.rs`

Add a helper such as:

```rust
fn format_compose_perf_report(
    cli_perf: &CliComposePerfReport,
    compose_perf: &ComposePerfReport,
) -> String
```

The helper should:

- build a compact multiline string with stable ordering
- use human-readable durations
    - milliseconds for sub-second spans
    - seconds with decimals for larger spans
- include both:
    - `Command Setup`
    - `Compose Pipeline`

Recommended output shape:

```text
compose performance

Command Setup
load input: 8ms
resolve input: 1ms
capture context: 1.72s
validate references: 812ms
build options: 2ms
compose pipeline: 54ms
total: 2.60s

Compose Pipeline
effective state build: 1ms
text replacement: 0ms (1 call)
page blocks: 0ms (1 call)
interpolation: 3ms (1 call)
shell expansion: 0ms
transclusion parse: 1ms
transclusion prepare: 4ms
transclusion resolve: 39ms
transclusion apply: 1ms
cleanup: 2ms
normalization: 5ms
total: 54ms
```

### Step 8. Wrap the output in a yellow `BlockQuote`

Use `biscuit-terminal` directly in the CLI:

```rust
use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::utils::color::{BasicColor, Color};
```

Render with:

```rust
let rendered = BlockQuote::from(body)
    .with_left_block_color(Color::BasicColor(BasicColor::Yellow))
    .render_optimistic(None);
```

Why `render_optimistic(None)`:

- it avoids needing a live `Terminal` instance just to format stderr text
- it produces deterministic output for tests
- the requirement is about the `BlockQuote` wrapper and yellow bar, not about terminal capability detection

### Step 9. Place the perf report at the end of the current stderr flow

File: `darkmatter/cli/src/commands.rs`

Current order is:

1. emit composed content to stdout or artifact output
2. emit compose warnings to stderr
3. emit deferred validation issues to stderr

Add perf output after both existing stderr diagnostics. That keeps current warnings/validation behavior intact and makes the perf block the final informational trailer.

Behavioral rule:

- only print the perf report on successful compose completion
- do not print it on hard validation failure (`exit(2)`) or compose errors

## Test Plan

### CLI parsing

File: `darkmatter/cli/src/args.rs` tests

- `md compose doc.md --perf` sets `Command::Compose { perf: true, ... }`
- absence of `--perf` leaves it `false`

### Compose report aggregation

File: `darkmatter/lib/src/markdown/compose/types.rs` tests

- merging two `ComposeReport`s combines perf durations by metric name
- call counts accumulate
- `perf: None` + `perf: Some(...)` behaves correctly

### Compose instrumentation

File: `darkmatter/lib/src/markdown/compose/mod.rs` tests

- when perf is disabled, `report.perf` is `None`
- when perf is enabled, `report.perf` is populated
- documents with recursive `::file` transclusions roll child timings into the parent report
- operations that do not run remain zero or absent in a consistent way

### CLI formatting

File: `darkmatter/cli/src/commands.rs` or output helper tests

- formatted report contains `Command Setup` and `Compose Pipeline`
- the rendered string includes the `BlockQuote` border
- the border contains the yellow ANSI prefix generated from `Color::BasicColor(BasicColor::Yellow)`
- the helper writes only to stderr in the success path

### End-to-end behavior

Prefer a CLI integration test covering:

- `md compose sample.md --perf`
- composed content still goes to stdout
- perf report goes to stderr
- stdout is unchanged relative to the same compose without `--perf`

## Documentation Updates

### CLI docs

File: `darkmatter/docs/cli/compose.md`

- add `--perf` to the options list
- document that the performance report is emitted to `stderr`
- mention that the report is printed after compose completes and after any existing warnings

### Top-level CLI index

File: `darkmatter/docs/cli/index.md`

- optionally add a short compose example using `--perf`

## File-Level Change Summary

Expected touch set:

- `darkmatter/cli/src/args.rs`
- `darkmatter/cli/src/commands.rs`
- `darkmatter/lib/src/markdown/compose/types.rs`
- `darkmatter/lib/src/markdown/compose/mod.rs`
- `darkmatter/lib/src/markdown/compose/perf.rs` (new, recommended)
- `darkmatter/docs/cli/compose.md`
- `darkmatter/docs/cli/index.md` (optional but recommended)

## Recommended Implementation Order

1. Add the CLI flag and thread it through `run_compose(...)`
2. Add `ComposeOptions::with_perf(...)`
3. Add `ComposePerfReport` / `ComposePerfMetric` and merge support
4. Instrument `run_compose_pipeline_internal(...)` and stage helpers
5. Capture CLI-envelope timings in `run_compose(...)`
6. Add the `BlockQuote` stderr formatter
7. Add tests
8. Update docs

## Risks / Things To Watch

- Do not let the perf report interfere with non-markdown outputs; it must stay on `stderr`
- Do not rely on `HashMap` order for rendered metrics
- Be careful not to double-count nested timings unintentionally when summing totals and children
- Keep the disabled path cheap; `--perf` is diagnostic tooling, not a permanent hot-path tax
- Preserve current warning/validation stderr behavior exactly and append perf output after it
