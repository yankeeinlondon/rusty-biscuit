# Darkmatter Tracing & Debugging Review

Date: 2026-04-03

## Executive Summary

The darkmatter package has tracing infrastructure in place (CLI subscriber, `tracing` dependency
in both lib and cli) but **severely underutilizes it**. The library has ~90 source files and only
a handful contain any tracing calls. The most complex subsystems -- compose pipeline orchestration,
shell expansion executor, transclusion resolver, interpolation evaluator -- have **zero tracing
instrumentation**. Meanwhile, test files contain `eprintln!` debugging artifacts that should have
been `tracing::debug!` calls.

More fundamentally, the current design **conflates two distinct output channels**: the `--verbose`
flag directly drives `tracing_subscriber` filter levels, mixing raw developer-oriented trace
output with user-facing CLI output. These must be separated.

## Core Design Principle: Verbose vs. Debug

The CLI has two distinct audiences for additional output, and they require different treatment:

### `--verbose` / `-v` -- User-Facing Enrichment

The verbose flag controls **crafted, styled messages** presented through the CLI's own rendering
pipeline (Prose, Status, styled terminal output). Verbose output should be indistinguishable in
quality from the CLI's normal output -- it's just *more* of it.

Examples of verbose output:
- A summary line after `md rm` showing which properties were removed
- Extra detail in `md delta` showing per-section change counts
- A progress indicator during long compose operations
- Compose report statistics after pipeline completion

Verbose output uses `biscuit-terminal` components and respects the CLI's visual style. It is
**never** raw tracing subscriber output.

### `--debug <level>` + `RUST_LOG` -- Developer-Facing Diagnostics

Debug/trace output is **raw, structured diagnostic logging** meant for developers debugging
the library or CLI internals. It uses `tracing_subscriber`'s compact formatter, prints to
stderr, and looks like standard Rust log output:

```
2026-04-03T10:15:32.123Z DEBUG darkmatter::compose: running operation operation=Interpolation phase=InlinePre
2026-04-03T10:15:32.125Z TRACE darkmatter::compose::interpolation: evaluating expr="{{ ctx.today }}"
```

This output is **useful but visually crude** compared to the rest of the CLI. It should be
triggered by:

1. **`--debug <level>`** -- A dedicated CLI flag (e.g., `--debug 1` for INFO, `--debug 2` for
   DEBUG, `--debug 3` for TRACE)
2. **`RUST_LOG` environment variable** -- The idiomatic Rust mechanism
   (e.g., `RUST_LOG=darkmatter=debug md compose doc.md`)

### Future: Styled Debug Output Under `--verbose`

In the future, tracing output *could* be surfaced under `-v` flags, but **only** through a
custom `tracing_subscriber::Layer` that renders events using the CLI's own styled components
(Prose/Status) rather than the default compact formatter. Until that styled layer exists,
`--verbose` and `--debug` must remain separate channels.

## Current State

### What's Working

| Area | Status | Details |
|------|--------|---------|
| CLI subscriber setup | Needs rework | `init_tracing()` in `cli/src/main.rs:21-54` conflates `-v` with tracing |
| Verbose flag | Partially used | `-v` count via `clap::ArgAction::Count`, but only checked as boolean |
| `tracing` dependency | Declared | Both `lib/Cargo.toml` and `cli/Cargo.toml` |
| `tracing-test` dev dep | Declared | `lib/Cargo.toml` (unused in any test) |
| Error types | Excellent | Comprehensive `thiserror` enums across all modules |
| CLI error display | Good | `color_eyre` with deduplicated cause chains in `main.rs:63-95` |

### Tracing Coverage by Module

Files with **some** tracing (sparse but present):
- `lib/src/markdown/output/terminal.rs` -- Best instrumented: `#[instrument]` on `render_table_row` and `resolve_image`, `debug!`/`warn!` for image handling, terminal width
- `lib/src/markdown/highlighting/themes.rs` -- `info!` for theme detection results (5 calls)
- `lib/src/markdown/inline_html.rs` -- `trace!` for HTML extraction fallbacks (8 calls)
- `lib/src/markdown/reference/html.rs` -- `trace!` for skipped HTML extraction (1 call)
- `lib/src/markdown/compose/cache/runtime.rs` -- `debug!`/`warn!` for cache operations (11 calls)
- `lib/src/markdown/compose/shell_expansion/store.rs` -- `warn!` for malformed whitelist lines (1 call)
- `lib/src/mermaid/mod.rs` -- `#[instrument]` + `trace!` on render (2 calls)
- `lib/src/mermaid/render_terminal.rs` -- `#[instrument]` on render function (1 call)
- `cli/src/output.rs` -- `warn!` for invalid TERMINAL_IMAGES env var (1 call)

Files with **zero** tracing (significant gaps):
- `lib/src/markdown/compose/mod.rs` -- **Pipeline orchestrator** (500+ lines, 3 phases, 10+ operations)
- `lib/src/markdown/compose/shell_expansion/executor.rs` -- **Shell command execution** (225 lines)
- `lib/src/markdown/compose/shell_expansion/policy.rs` -- **Security policy checks**
- `lib/src/markdown/compose/transclusion/` -- **All transclusion files** (resolver, parser, engine, wrappers)
- `lib/src/markdown/compose/interpolation/` -- **All interpolation files** (evaluator, parser, lexer)
- `lib/src/markdown/compose/page_blocks/` -- **Page block engine**
- `lib/src/markdown/compose/toc_linking/` -- **TOC linking pipeline**
- `lib/src/markdown/compose/replacement.rs` -- **Text replacement**
- `lib/src/markdown/compose/conditions.rs` -- **Condition evaluation**
- `lib/src/markdown/reference/validate.rs` -- **Reference validation**
- `lib/src/markdown/reference/local.rs` -- **Local reference resolution**
- `lib/src/markdown/cleanup.rs` -- **Markdown normalization**
- `lib/src/markdown/output/html.rs` -- **HTML rendering**
- `lib/src/diff/` -- **All diff modules**
- `cli/src/commands.rs` -- **All CLI command handlers** (800+ lines)

### Anti-patterns Found

#### 1. `init_tracing()` conflates `--verbose` with raw tracing output

The current `init_tracing()` in `cli/src/main.rs:21-54` maps the `-v` count directly to
`tracing_subscriber` filter levels:

```rust
// Current (problematic):
fn init_tracing(verbose: u8) {
    if verbose == 0 { return; }
    let base_filter = match verbose {
        1 => "info,md=info,darkmatter=info",
        2 => "info,md=debug,darkmatter=debug",
        _ => "debug,md=trace,darkmatter=trace",
    };
    // ... initializes tracing_subscriber with compact formatter
}
```

This means `md -v compose doc.md` would dump raw tracing lines to stderr alongside the CLI's
own styled output. The verbose flag should **never** drive the tracing subscriber. Instead:

- `--verbose` controls **styled CLI messages** (rendered via Prose/Status)
- `--debug <level>` and/or `RUST_LOG` control **raw tracing output**

#### 2. `eprintln!` in test code instead of `tracing::debug!`

Several test functions use `eprintln!` for debugging output that always prints, even in
passing tests:

```
lib/src/markdown/inline/mod.rs:494       eprintln!("Events:");
lib/src/markdown/output/html.rs:687      eprintln!("Code block HTML: {}", html);
lib/src/markdown/output/terminal.rs:4970 eprintln!("Table at width {}:\n{}", narrow_width, plain);
lib/src/markdown/output/terminal.rs:6450 eprintln!("Raw output bytes:");
lib/src/markdown/output/terminal.rs:6502 eprintln!("Plain output:\n{:?}", plain);
lib/src/markdown/output/terminal.rs:6704 eprintln!("Raw output (escapes visible):");
lib/src/markdown/output/terminal.rs:6788 eprintln!("Width {}: Found blank line...", width, i + 1);
```

These should either be removed (if no longer needed) or converted to `tracing::debug!` so they
only appear when `RUST_LOG` is set during test runs.

#### 3. Verbose flag underutilized for styled output

In `cli/src/commands.rs`, the verbose flag is only checked as `cli.verbose > 0` in three
places (`run_rm` line 751, `print_delta` line 154, `run_clean` line 90). The multi-level
counting (`-v`/`-vv`) is collected but never meaningfully tiered -- everything is a boolean
check. There's an opportunity to define richer verbose tiers for styled CLI output.

#### 4. `tracing-test` declared but unused

`lib/Cargo.toml` declares `tracing-test = "0.2"` as a dev dependency. No test file uses
`tracing_test::traced_test` or any other facility from this crate.

## Recommendations

### Priority 1: Separate `--verbose` from `--debug` in the CLI

This is the foundational change. Everything else builds on having the right plumbing.

**Step 1a: Add `--debug` flag to `cli/src/args.rs`:**

```rust
/// Enable developer debug logging (1=INFO, 2=DEBUG, 3=TRACE, 4=TRACE+locations).
/// Alternatively, set RUST_LOG environment variable.
#[arg(
    long = "debug",
    value_name = "LEVEL",
    global = true,
    hide = true,  // developer-facing, not in --help by default
)]
pub debug_level: Option<u8>,
```

Using `hide = true` keeps `--help` clean for end users. Developers who need it know to look
for it (or use `RUST_LOG`). It can be revealed with `md --help-all` or documented in a
`DEBUGGING.md`.

**Step 1b: Rewrite `init_tracing()` in `cli/src/main.rs`:**

```rust
/// Initialize tracing subscriber for developer debug output.
///
/// Triggered by `--debug <level>` or `RUST_LOG` env var. The `--verbose` flag
/// is NOT involved here -- it controls styled user-facing output only.
///
/// Debug levels:
/// - 1: INFO (phase transitions, high-level operations)
/// - 2: DEBUG (decisions, resolved values, cache hits/misses)
/// - 3: TRACE (per-item details, raw values)
/// - 4+: TRACE with file/line source locations
fn init_tracing(debug_level: Option<u8>) {
    // RUST_LOG takes precedence if set
    let level = match std::env::var("RUST_LOG") {
        Ok(_) => {
            // RUST_LOG is set -- use it directly, ignore --debug flag
            None
        }
        Err(_) => debug_level,
    };

    let filter = match level {
        Some(1) => "warn,md=info,darkmatter=info",
        Some(2) => "warn,md=debug,darkmatter=debug",
        Some(n) if n >= 3 => "info,md=trace,darkmatter=trace",
        None if std::env::var("RUST_LOG").is_ok() => {
            // Let EnvFilter parse RUST_LOG
            &std::env::var("RUST_LOG").unwrap()
        }
        _ => return, // No debug output requested
    };

    let env_filter = EnvFilter::try_new(filter)
        .unwrap_or_else(|_| EnvFilter::new("warn"));
    let show_locations = debug_level.unwrap_or(0) >= 4;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(show_locations)
                .with_line_number(show_locations)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}
```

**Step 1c: Update `run()` call site:**

```rust
fn run() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    init_tracing(cli.debug_level);  // <-- debug, not verbose
    // ...
}
```

**Step 1d: Keep `--verbose` for styled output only.**

The existing `cli.verbose > 0` checks in `commands.rs` (lines 90, 137, 154, 751) are correct
in spirit -- they gate user-facing styled output. These should remain and be expanded with
richer tiered behavior over time. For example:

| Level | Meaning | Example Output |
|-------|---------|----------------|
| 0 (default) | Essential output only | Result content, errors |
| 1 (`-v`) | Progress + summaries | "Composed 3 transclusions, 12 interpolations" |
| 2 (`-vv`) | Detailed breakdown | Per-operation stats, resolved paths, cache status |

All verbose output should be rendered through `biscuit-terminal` components (Prose, Status)
to match the CLI's visual style.

### Priority 2: Instrument the Compose Pipeline

The compose pipeline is the most complex subsystem and the hardest to debug when something goes
wrong. Add `#[instrument]` and tracing events to make the pipeline observable via `--debug` /
`RUST_LOG`.

**`lib/src/markdown/compose/mod.rs`** -- Pipeline orchestrator:

```rust
use tracing::{debug, info, instrument, trace, warn};

#[instrument(skip_all, fields(source = ?options.source, operations = ?options.enabled_operations()))]
pub fn compose_with(&self, options: ComposeOptions) -> MarkdownResult<(Markdown, ComposeReport)> {
    // ...
}

// Inside run_compose_pipeline_internal, at each phase:
info!(operation = %operation, phase = %operation.phase(), "compose: running operation");

// After each operation completes:
debug!(operation = %operation, "compose: operation complete");
```

**`lib/src/markdown/compose/shell_expansion/executor.rs`** -- Shell execution:

```rust
#[instrument(skip_all, fields(
    command = %directive.raw_command,
    executable = %directive.executable,
    line = directive.line,
    working_dir = %working_dir.display()
))]
pub fn execute_command(...) -> Result<String, ShellExpansionError> {
    debug!(args = ?directive.args, "executing shell command");
    // ...on success:
    debug!(exit_code = 0, output_len = output.len(), "shell command succeeded");
    // ...on timeout:
    warn!(timeout = ?timeout, elapsed = ?start.elapsed(), "shell command timed out");
}
```

**`lib/src/markdown/compose/transclusion/resolver.rs`** -- File resolution:

```rust
#[instrument(skip_all, fields(target = %target, depth = depth))]
pub fn resolve_transclusion(...) {
    debug!("resolving transclusion target");
    // ...on cache hit:
    trace!("transclusion cache hit");
    // ...on file load:
    debug!(path = %resolved_path.display(), "loaded transclusion source");
}
```

### Priority 3: Instrument Reference Validation

**`lib/src/markdown/reference/validate.rs`**:

```rust
#[instrument(skip_all, fields(file = ?self.source_path(), ref_count))]
pub fn validate_references(...) {
    debug!(options = ?options, "starting reference validation");
    // After collecting references:
    tracing::Span::current().record("ref_count", refs.len());
    // Per-reference:
    trace!(kind = ?ref_.kind, target = %ref_.target, "validating reference");
    // On issue found:
    debug!(kind = ?issue.kind, severity = ?issue.severity, target = %issue.target, "validation issue");
}
```

### Priority 4: Instrument Interpolation

**`lib/src/markdown/compose/interpolation/evaluator.rs`**:

```rust
// On each expression evaluation:
trace!(expr = %raw_expr, "evaluating interpolation expression");
// On resolution:
trace!(expr = %raw_expr, result = %result, "interpolation resolved");
// On unresolved variable:
debug!(variable = %name, "unresolved interpolation variable");
```

**`lib/src/markdown/compose/interpolation/lexer.rs`**:

```rust
// After finding all expressions:
debug!(count = locations.len(), "found interpolation expressions");
```

### Priority 5: Add Tracing to CLI Command Handlers

These go through `--debug` / `RUST_LOG`, NOT through `--verbose`:

**`cli/src/commands.rs`**:

```rust
use tracing::{debug, info, instrument};

#[instrument(skip_all, fields(command = "compose", input = ?input))]
pub fn run_compose(...) -> Result<()> {
    info!("starting compose pipeline");
    // ...after compose completes:
    info!(
        warnings = report.warnings.len(),
        replacements = report.text_replacements_applied,
        transclusions = report.transclusions_resolved,
        "compose pipeline complete"
    );
}

#[instrument(skip_all, fields(command = "render", input = ?input, output = ?output))]
pub fn run_render(...) -> Result<()> {
    debug!("rendering document");
}

#[instrument(skip_all, fields(command = "validate", input = ?input))]
pub fn run_validate(target: ValidateTarget) -> Result<()> {
    info!("starting reference validation");
}
```

### Priority 6: Enrich `--verbose` Styled Output

Separately from tracing, expand the verbose flag's styled output in `cli/src/commands.rs`.
This output uses `biscuit-terminal` Prose/Status components, not `tracing` macros.

**Example for `run_compose`:**

```rust
if cli.verbose > 0 {
    use biscuit_terminal::prelude::{Status, StatusState};
    let status = Status::from_prose(format!(
        "Composed <b>{}</b> transclusions, <b>{}</b> interpolations, <b>{}</b> replacements",
        report.transclusions_resolved,
        report.interpolations_applied,
        report.text_replacements_applied,
    )).state(StatusState::Success);
    eprintln!("{}", status.render(&Terminal::default()));
}

if cli.verbose > 1 {
    // -vv: per-operation breakdown
    if let Some(perf) = &report.perf {
        for metric in perf.metrics() {
            let status = Status::from_prose(format!(
                "<dim>{:20}</dim> {:>8.2}ms",
                metric.kind, metric.duration.as_secs_f64() * 1000.0
            ));
            eprintln!("{}", status.render(&Terminal::default()));
        }
    }
}
```

### Priority 7: Clean Up Test `eprintln!` Calls

Convert debugging `eprintln!` in tests to either:

1. **Remove** if they were one-off debugging artifacts
2. **Convert to `tracing::debug!`** and use `#[tracing_test::traced_test]` if the output is
   genuinely useful for future debugging. This keeps test output clean by default but visible
   with `RUST_LOG=debug cargo test`.

### Priority 8: Leverage `tracing-test` in Tests

Since `tracing-test` is already declared, use it for tests that exercise tracing-instrumented
code:

```rust
#[tracing_test::traced_test]
#[test]
fn test_compose_with_shell_expansion() {
    // ... test code ...
    // Assert on captured logs:
    assert!(logs_contain("executing shell command"));
}
```

This enables testing that tracing output is correct and that expected events are emitted.

## Implementation Order

| Phase | Scope | Estimated Files |
|-------|-------|-----------------|
| 1 | Separate `--verbose` from `--debug`, rewrite `init_tracing()` | 3 files (args, main, commands) |
| 2 | Instrument compose pipeline + shell executor | 3 files |
| 3 | Instrument transclusion resolver + interpolation evaluator | 4 files |
| 4 | Instrument reference validation + CLI command handlers | 3 files |
| 5 | Enrich `--verbose` styled output in CLI commands | 2-3 files |
| 6 | Clean up test `eprintln!` + add `tracing_test` usage | 6-8 files |
| 7 | Remaining modules (cleanup, diff, page blocks, toc linking) | 5-6 files |

## Guiding Principles

1. **Verbose is for users, debug is for developers** -- `--verbose` produces styled, crafted
   messages via `biscuit-terminal`. `--debug` and `RUST_LOG` produce raw tracing output via
   `tracing_subscriber`. These are separate channels with separate audiences.

2. **Libraries emit, apps configure** -- All `tracing` calls go in the library; the CLI
   configures the subscriber. This is already the architecture; it just needs population.

3. **Use structured fields** -- Prefer `debug!(path = %p.display(), size = bytes)` over
   `debug!("loaded {} ({} bytes)", p.display(), bytes)`. Structured fields work with JSON
   log formatters and OpenTelemetry exporters.

4. **Level discipline** (for `tracing` events, NOT for `--verbose`):
   - `error!` -- Never in library code (use `Result` propagation instead)
   - `warn!` -- Recoverable issues that affect behavior (fallbacks, degraded paths)
   - `info!` -- Phase transitions, high-level operation starts/completions
   - `debug!` -- Decisions, resolved values, cache hits/misses
   - `trace!` -- Per-item iteration, raw values, detailed internals

5. **`#[instrument]` guidelines**:
   - Always `skip_all` unless arguments are small and cheap to `Debug`
   - Use `fields(key = %value)` to record what matters
   - Don't instrument hot inner loops (e.g., per-character parsing)

6. **`error!` in CLI only** -- The CLI's error handler in `main.rs` is the right place for
   error display. Library code should propagate errors, not log them.

7. **RUST_LOG is always available** -- Even without `--debug`, developers can always set
   `RUST_LOG=darkmatter=trace` to get full diagnostics. The `--debug` flag is just a
   convenience shorthand.
