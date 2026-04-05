# Tracing & Debugging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all recommendations from `docs/debug-review.md` — separate `--verbose` from `--debug`, instrument the library with structured tracing, enrich verbose CLI output, and clean up test `eprintln!` calls.

**Architecture:** The CLI gets a new `--debug <level>` flag (hidden) that drives `tracing_subscriber` for developer diagnostics. The existing `--verbose` flag remains exclusively for styled user-facing output via `biscuit-terminal`. Library code emits `tracing` events at appropriate levels; the CLI configures the subscriber.

**Tech Stack:** `tracing` (events + `#[instrument]`), `tracing-subscriber` (env-filter, compact fmt), `tracing-test` (dev), `biscuit-terminal` (Prose/Status for verbose output)

**Spec:** `darkmatter/docs/debug-review.md`

---

## File Map

### CLI changes (Phase 1)
- **Modify:** `cli/src/args.rs` — add `debug_level: Option<u8>` field
- **Modify:** `cli/src/main.rs` — rewrite `init_tracing()` to use `debug_level`
- **Modify:** `cli/src/commands.rs` — add `#[instrument]` to command handlers, enrich verbose output
- **Modify:** `cli/src/lib.rs` — update doc comments referencing `-v` for tracing

### Library instrumentation (Phases 2-4)
- **Modify:** `lib/src/markdown/compose/mod.rs` — instrument pipeline orchestrator
- **Modify:** `lib/src/markdown/compose/shell_expansion/executor.rs` — instrument shell execution
- **Modify:** `lib/src/markdown/compose/transclusion/resolver.rs` — instrument resolution
- **Modify:** `lib/src/markdown/compose/interpolation/evaluator.rs` — instrument evaluation
- **Modify:** `lib/src/markdown/compose/interpolation/lexer.rs` — instrument expression finding
- **Modify:** `lib/src/markdown/reference/validate.rs` — instrument validation
- **Modify:** `lib/src/markdown/compose/conditions.rs` — instrument condition evaluation
- **Modify:** `lib/src/markdown/compose/replacement.rs` — instrument text replacement
- **Modify:** `lib/src/markdown/compose/page_blocks/engine.rs` — instrument page blocks
- **Modify:** `lib/src/markdown/compose/toc_linking/mod.rs` — instrument TOC linking

### Test cleanup (Phase 5)
- **Modify:** `lib/src/markdown/output/terminal.rs` — remove/convert test `eprintln!`
- **Modify:** `lib/src/markdown/output/html.rs` — remove/convert test `eprintln!`
- **Modify:** `lib/src/markdown/inline/mod.rs` — remove/convert test `eprintln!`

---

## Task 1: Add `--debug` flag to CLI args

**Files:**
- Modify: `cli/src/args.rs:354-414` (Cli struct)

- [ ] **Step 1: Add `debug_level` field to `Cli` struct**

In `cli/src/args.rs`, add the `debug_level` field after the `verbose` field (after line 405):

```rust
    /// Enable developer debug logging (1=INFO, 2=DEBUG, 3=TRACE, 4=TRACE+locations).
    /// Alternatively, set RUST_LOG environment variable.
    #[arg(
        long = "debug",
        value_name = "LEVEL",
        global = true,
        hide = true,
    )]
    pub debug_level: Option<u8>,
```

- [ ] **Step 2: Update verbose doc comment**

Change the verbose field's doc comment (line 398) from:
```rust
    /// Increase verbosity (-v INFO, -vv DEBUG, -vvv TRACE, -vvvv TRACE with file/line)
```
to:
```rust
    /// Increase verbosity for styled user-facing output (-v summary, -vv detailed)
```

- [ ] **Step 3: Add test for `--debug` flag parsing**

Add to the `#[cfg(test)] mod tests` block at the bottom of `args.rs`:

```rust
    #[test]
    fn debug_flag_parses_level() {
        let cli = Cli::try_parse_from(["md", "--debug", "2", "doc.md"]).unwrap();
        assert_eq!(cli.debug_level, Some(2));
    }

    #[test]
    fn debug_flag_absent_is_none() {
        let cli = Cli::try_parse_from(["md", "doc.md"]).unwrap();
        assert_eq!(cli.debug_level, None);
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p darkmatter-cli -- args::tests`
Expected: All pass including new debug flag tests

- [ ] **Step 5: Commit**

```bash
git add darkmatter/cli/src/args.rs
git commit -m "feat(darkmatter-cli): add hidden --debug flag for developer tracing"
```

---

## Task 2: Rewrite `init_tracing()` and update call site

**Files:**
- Modify: `cli/src/main.rs:13-54` (init_tracing function)
- Modify: `cli/src/main.rs:98-102` (run function)

- [ ] **Step 1: Rewrite `init_tracing()` to accept `debug_level`**

Replace the entire `init_tracing` function (lines 13-54) in `cli/src/main.rs` with:

```rust
/// Initialize tracing subscriber for developer debug output.
///
/// Triggered by `--debug <level>` or `RUST_LOG` env var. The `--verbose` flag
/// is NOT involved here — it controls styled user-facing output only.
///
/// ## Debug levels
///
/// - 1: INFO (phase transitions, high-level operations)
/// - 2: DEBUG (decisions, resolved values, cache hits/misses)
/// - 3: TRACE (per-item details, raw values)
/// - 4+: TRACE with file/line source locations
fn init_tracing(debug_level: Option<u8>) {
    // RUST_LOG takes precedence if set
    let env_log = std::env::var("RUST_LOG").ok();

    let filter_str = match (&env_log, debug_level) {
        // RUST_LOG is set — use it directly, ignore --debug flag
        (Some(rust_log), _) => rust_log.clone(),
        // --debug flag provided
        (None, Some(1)) => "warn,md=info,darkmatter=info".to_string(),
        (None, Some(2)) => "warn,md=debug,darkmatter=debug".to_string(),
        (None, Some(n)) if n >= 3 => "info,md=trace,darkmatter=trace".to_string(),
        // No debug output requested
        _ => return,
    };

    let filter =
        EnvFilter::try_new(&filter_str).unwrap_or_else(|_| EnvFilter::new("warn"));
    let show_locations = debug_level.unwrap_or(0) >= 4;

    tracing_subscriber::registry()
        .with(filter)
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

- [ ] **Step 2: Update call site in `run()`**

Change line 102 in `cli/src/main.rs` from:
```rust
    init_tracing(cli.verbose);
```
to:
```rust
    init_tracing(cli.debug_level);
```

- [ ] **Step 3: Update lib.rs doc comment**

In `cli/src/lib.rs`, change lines 123-127 from:
```rust
//! # Verbose output for debugging
//! md README.md -v      # INFO level
//! md README.md -vv     # DEBUG level
//! md README.md -vvv    # TRACE level
```
to:
```rust
//! # Verbose output (styled summaries)
//! md README.md -v      # Summary after operations
//! md README.md -vv     # Detailed per-operation breakdown
//!
//! # Developer debug output (raw tracing)
//! md README.md --debug 1   # INFO level
//! md README.md --debug 2   # DEBUG level
//! RUST_LOG=darkmatter=trace md README.md   # Full trace
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p darkmatter-cli`
Expected: Compiles without warnings

- [ ] **Step 5: Verify `--debug` works end-to-end**

Run: `cargo run -p darkmatter-cli -- --debug 1 --help`
Expected: Help output appears, no tracing output (help exits before pipeline runs)

- [ ] **Step 6: Commit**

```bash
git add darkmatter/cli/src/main.rs darkmatter/cli/src/lib.rs
git commit -m "refactor(darkmatter-cli): separate --verbose from --debug in init_tracing"
```

---

## Task 3: Instrument compose pipeline orchestrator

**Files:**
- Modify: `lib/src/markdown/compose/mod.rs`

- [ ] **Step 1: Add tracing import**

At the top of `lib/src/markdown/compose/mod.rs`, after the existing `use` statements (after line 79), add:

```rust
use tracing::{debug, info, instrument, trace, warn};
```

- [ ] **Step 2: Instrument `compose_with`**

Add `#[instrument]` to the `compose_with` method. Find `pub fn compose_with` (around line 225) and add above it:

```rust
    #[instrument(skip_all, fields(source = ?options.source))]
```

- [ ] **Step 3: Instrument `run_compose_pipeline_internal`**

Add `#[instrument]` to the method. Find `pub(crate) fn run_compose_pipeline_internal` (around line 275) and add above it:

```rust
    #[instrument(skip_all, fields(source = ?options.source))]
```

- [ ] **Step 4: Add tracing to the operation loop**

Inside `run_compose_pipeline_internal`, find the operation loop (around line 403). Add info/debug events at key points.

After `for operation in ComposeOperation::default_order() {` and before the `if !options.is_enabled(*operation)` check, add:
```rust
                trace!(operation = %operation, enabled = options.is_enabled(*operation), "compose: checking operation");
```

After the `if !options.is_enabled(*operation) { continue; }` block, and before the `match operation.phase()` block, add:
```rust
                info!(operation = %operation, phase = %operation.phase(), "compose: running operation");
```

- [ ] **Step 5: Add tracing to stage methods**

In `run_replacement_stage` (around line 815), after the count is computed (before `count`), add:
```rust
        debug!(count, "compose: text replacements applied");
```

In `run_interpolation_stage` (around line 842), after `result.replacements` is computed (before `Ok`), add:
```rust
        debug!(count = result.replacements, "compose: interpolations applied");
```

In `run_shell_expansion_stage` (around line 883), at the start of the function after `let directives = ...`, add:
```rust
        debug!(directive_count = directives.len(), "compose: shell expansion directives found");
```

In `run_page_blocks_stage`, at the start add:
```rust
        debug!("compose: running page blocks");
```

In `run_normalization_stage`, at the start add:
```rust
        debug!("compose: running normalization");
```

- [ ] **Step 6: Add tracing to transclusion phase**

In `run_transclusion_phase` (around line 568), after the empty check:
```rust
        info!(operations = ?operations, "compose: starting transclusion phase");
```

After the rayon parallel resolution completes (around line 700), add:
```rust
        debug!(resolved = results.len(), "compose: transclusion resolution complete");
```

- [ ] **Step 7: Add warning for unknown options**

Find the loop that warns about unknown options (around line 924). If it uses `report.add_warning`, add a tracing warn before it:
```rust
                warn!(key = %key, "compose: unknown frontmatter option");
```

- [ ] **Step 8: Build and verify**

Run: `cargo build -p darkmatter`
Expected: Compiles without warnings

- [ ] **Step 9: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/mod.rs
git commit -m "feat(darkmatter): instrument compose pipeline with tracing"
```

---

## Task 4: Instrument shell executor

**Files:**
- Modify: `lib/src/markdown/compose/shell_expansion/executor.rs`

- [ ] **Step 1: Add tracing import**

After the existing `use` statements (around line 13), add:

```rust
use tracing::{debug, instrument, warn};
```

- [ ] **Step 2: Instrument `execute_command`**

Add above the `pub fn execute_command` function (line 87):

```rust
    #[instrument(skip_all, fields(
        command = %directive.raw_command,
        executable = %directive.executable,
        line = directive.line,
    ))]
```

- [ ] **Step 3: Add tracing events inside `execute_command`**

After the working directory is resolved (after `let working_dir = resolve_working_directory(...)`, around line 100), add:
```rust
    debug!(working_dir = %working_dir.display(), "shell: executing command");
```

After the command succeeds (where output is returned, in the success branch), add:
```rust
    debug!(exit_code = 0, output_len = output.len(), "shell: command succeeded");
```

In the timeout branch (where `ShellExpansionError::Timeout` is constructed), add before the error return:
```rust
    warn!(elapsed = ?start.elapsed(), "shell: command timed out");
```

- [ ] **Step 4: Instrument `resolve_working_directory`**

Add above `pub fn resolve_working_directory` (line 36):
```rust
    #[instrument(skip_all)]
```

- [ ] **Step 5: Build and verify**

Run: `cargo build -p darkmatter`
Expected: Compiles without warnings

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs
git commit -m "feat(darkmatter): instrument shell executor with tracing"
```

---

## Task 5: Instrument transclusion resolver

**Files:**
- Modify: `lib/src/markdown/compose/transclusion/resolver.rs`

- [ ] **Step 1: Add tracing import**

After existing `use` statements, add:

```rust
use tracing::{debug, instrument, trace};
```

- [ ] **Step 2: Instrument `resolve_target`**

Add above `pub(crate) fn resolve_target` (around line 9):

```rust
#[instrument(skip_all, fields(target = %raw_target, kind = ?kind))]
```

At the start of the function, add:
```rust
    debug!("transclusion: resolving target");
```

- [ ] **Step 3: Add tracing to `resolve_path`**

In the private `resolve_path` function, add at the start:
```rust
    trace!(raw_target = %raw_target, "transclusion: resolving path");
```

After successful path resolution (where `Ok(path)` would be returned), add:
```rust
    debug!(resolved = %path.display(), "transclusion: path resolved");
```

- [ ] **Step 4: Add tracing to URL resolution**

In `resolve_url_target`, add at the start:
```rust
    trace!(url = %raw_target, "transclusion: resolving URL target");
```

- [ ] **Step 5: Build and verify**

Run: `cargo build -p darkmatter`
Expected: Compiles without warnings

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
git commit -m "feat(darkmatter): instrument transclusion resolver with tracing"
```

---

## Task 6: Instrument interpolation evaluator and lexer

**Files:**
- Modify: `lib/src/markdown/compose/interpolation/evaluator.rs`
- Modify: `lib/src/markdown/compose/interpolation/lexer.rs`

- [ ] **Step 1: Add tracing to evaluator**

In `evaluator.rs`, after existing `use` statements, add:

```rust
use tracing::{debug, trace};
```

- [ ] **Step 2: Add tracing events in `Evaluator::eval`**

In the `eval` method (around line 242), at the start:
```rust
        trace!(expr = ?expr, "interpolation: evaluating expression");
```

After the result is computed (before returning), add for the `Value` case:
```rust
            trace!(result = %value, "interpolation: resolved");
```

For unresolved variables (in the variable lookup that returns `EvalResult::Error`), add:
```rust
            debug!(variable = %name, "interpolation: unresolved variable");
```

- [ ] **Step 3: Add tracing to lexer**

In `lexer.rs`, after existing `use` statements, add:

```rust
use tracing::debug;
```

- [ ] **Step 4: Add tracing to `find_all`**

In `ExpressionFinder::find_all` (around line 90), after the loop completes and before returning, add:
```rust
        debug!(count = locations.len(), "interpolation: found expressions");
```

- [ ] **Step 5: Build and verify**

Run: `cargo build -p darkmatter`
Expected: Compiles without warnings

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs darkmatter/lib/src/markdown/compose/interpolation/lexer.rs
git commit -m "feat(darkmatter): instrument interpolation evaluator and lexer with tracing"
```

---

## Task 7: Instrument reference validation

**Files:**
- Modify: `lib/src/markdown/reference/validate.rs`

- [ ] **Step 1: Add tracing import**

After existing `use` statements, add:

```rust
use tracing::{debug, info, instrument, trace};
```

- [ ] **Step 2: Instrument the main `validate` function**

Add above `pub(crate) fn validate` (around line 155):

```rust
#[instrument(skip_all)]
```

At the start of the function:
```rust
    info!("validate: starting reference validation");
```

After collecting references (where `references_scanned` is set):
```rust
    debug!(ref_count = report.references_scanned, "validate: references collected");
```

- [ ] **Step 3: Add per-reference tracing**

In `validate_local_path`, at the start:
```rust
    trace!(raw = %raw, "validate: checking local path");
```

In `validate_remote_urls_async`, at the start:
```rust
    debug!(url_count = records.len(), "validate: starting remote URL checks");
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p darkmatter`
Expected: Compiles without warnings

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/reference/validate.rs
git commit -m "feat(darkmatter): instrument reference validation with tracing"
```

---

## Task 8: Instrument remaining compose modules

**Files:**
- Modify: `lib/src/markdown/compose/conditions.rs`
- Modify: `lib/src/markdown/compose/replacement.rs`
- Modify: `lib/src/markdown/compose/page_blocks/engine.rs`
- Modify: `lib/src/markdown/compose/toc_linking/mod.rs`

- [ ] **Step 1: Instrument conditions.rs**

Add tracing import:
```rust
use tracing::{debug, trace};
```

In `evaluate_condition` (line 31), at the start:
```rust
    trace!(expr = %expr, line, "conditions: evaluating");
```

After result is computed:
```rust
    debug!(expr = %expr, result, "conditions: evaluated");
```

- [ ] **Step 2: Instrument replacement.rs**

Add tracing import:
```rust
use tracing::debug;
```

In `apply_replacements` (line 87), after the result is computed (before returning):
```rust
    debug!(count, "replacement: applied text replacements");
```

- [ ] **Step 3: Instrument page_blocks/engine.rs**

Add tracing import:
```rust
use tracing::debug;
```

In `render_page_blocks` (line 15), at the start:
```rust
    debug!(region_count = regions.len(), "page_blocks: rendering");
```

- [ ] **Step 4: Instrument toc_linking/mod.rs**

Add tracing import:
```rust
use tracing::{debug, trace};
```

In `process_toc_linking` (line 90), at the start:
```rust
    debug!("toc_linking: processing directives");
```

In `resolve_target_chain` (line 46), at the start:
```rust
    trace!(target = %directive.targets.join(" > "), "toc_linking: resolving target chain");
```

- [ ] **Step 5: Build and verify**

Run: `cargo build -p darkmatter`
Expected: Compiles without warnings

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/conditions.rs \
       darkmatter/lib/src/markdown/compose/replacement.rs \
       darkmatter/lib/src/markdown/compose/page_blocks/engine.rs \
       darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
git commit -m "feat(darkmatter): instrument conditions, replacement, page blocks, and TOC linking"
```

---

## Task 9: Add tracing to CLI command handlers

**Files:**
- Modify: `cli/src/commands.rs`

- [ ] **Step 1: Add tracing import**

At the top of `cli/src/commands.rs`, after the existing `use` statements (around line 19), add:

```rust
use tracing::{debug, info, instrument};
```

- [ ] **Step 2: Instrument `run_compose`**

Add above `pub fn run_compose` (line 314):

```rust
#[instrument(skip_all, fields(command = "compose"))]
```

At the start of the function:
```rust
    info!("starting compose pipeline");
```

- [ ] **Step 3: Instrument `run_render`**

Add above `pub fn run_render` (line 265):

```rust
#[instrument(skip_all, fields(command = "render"))]
```

At the start:
```rust
    debug!("rendering document");
```

- [ ] **Step 4: Instrument `run_validate`**

Add above `fn run_validate` (line 1355):

```rust
#[instrument(skip_all, fields(command = "validate"))]
```

At the start:
```rust
    info!("starting reference validation");
```

- [ ] **Step 5: Instrument other command handlers**

Add `#[instrument(skip_all)]` to each of these:
- `run_clean` (line 216)
- `run_get` (line 627)
- `run_set` (line 670)
- `run_rm` (line 703)
- `run_hash` (line 1062)
- `run_graph` (line 1859)

- [ ] **Step 6: Build and verify**

Run: `cargo build -p darkmatter-cli`
Expected: Compiles without warnings

- [ ] **Step 7: Commit**

```bash
git add darkmatter/cli/src/commands.rs
git commit -m "feat(darkmatter-cli): instrument CLI command handlers with tracing"
```

---

## Task 10: Enrich `--verbose` styled output

**Files:**
- Modify: `cli/src/commands.rs`

- [ ] **Step 1: Add verbose compose summary**

In `run_compose`, find the location after the compose pipeline completes and the report is available (after `let (result, report) = md.compose_with(options)?;` or similar). Add verbose summary output:

```rust
    if cli.verbose > 0 {
        use biscuit_terminal::components::status::{Status, StatusState};
        let status = Status::from_prose(format!(
            "Composed <b>{}</b> transclusions, <b>{}</b> interpolations, <b>{}</b> replacements",
            report.transclusions_resolved,
            report.interpolations_applied,
            report.replacements_applied,
        ))
        .state(StatusState::Success);
        let terminal = Terminal::default();
        eprintln!("{}", status.render(&terminal));
    }
```

Note: Check how the report is actually structured in the existing code. The field names above match `ComposeReport` fields — verify against the actual struct. Adjust field names if needed.

- [ ] **Step 2: Add verbose compose perf breakdown**

After the `-v` check above, add `-vv` output:

```rust
    if cli.verbose > 1 {
        if let Some(perf_report) = &report.perf {
            use biscuit_terminal::components::status::Status;
            let terminal = Terminal::default();
            for metric in &perf_report.metrics {
                let status = Status::from_prose(format!(
                    "<dim>{:20}</dim> {:>8.2}ms",
                    format!("{}", metric.kind),
                    metric.duration.as_secs_f64() * 1000.0
                ));
                eprintln!("{}", status.render(&terminal));
            }
        }
    }
```

Note: Verify the exact field names of `ComposePerfReport` and `ComposePerfMetric` against the types in `lib/src/markdown/compose/perf.rs`. The iteration pattern may use `.metrics()` method or `.metrics` field.

- [ ] **Step 3: Improve verbose rm output**

In `run_rm` (around line 750-768), replace the raw `eprintln!` with escaped ANSI codes with styled Prose output:

```rust
    } else if cli.verbose > 0 {
        use biscuit_terminal::components::prose::Prose;
        let props_label = if removed.len() == 1 {
            format!("<b>{}</b> property", removed[0])
        } else {
            format!(
                "<b>{}</b> properties",
                removed
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let remaining_label = remaining.join(", ");
        let terminal = Terminal::default();
        eprintln!(
            "{}",
            Prose::new(format!(
                "- removed the {} from frontmatter (<dim>remaining: <i>{}</i></dim>)",
                props_label, remaining_label
            ))
            .render(&terminal)
        );
    }
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p darkmatter-cli`
Expected: Compiles without warnings

- [ ] **Step 5: Commit**

```bash
git add darkmatter/cli/src/commands.rs
git commit -m "feat(darkmatter-cli): enrich --verbose styled output for compose and rm"
```

---

## Task 11: Clean up test `eprintln!` calls

**Files:**
- Modify: `lib/src/markdown/output/terminal.rs`
- Modify: `lib/src/markdown/output/html.rs`
- Modify: `lib/src/markdown/inline/mod.rs`

The following test `eprintln!` calls should be **removed** (they are debugging artifacts that always print, even in passing tests). These are in test functions and provide no value to automated test runs.

- [ ] **Step 1: Clean up terminal.rs test eprintln calls**

Remove `eprintln!` lines from these test functions in `lib/src/markdown/output/terminal.rs`:

- Line 4970: `eprintln!("Table at width {}:\n{}", narrow_width, plain);` in `test_table_very_narrow_width` — **remove**
- Lines 5113, 5125: In `test_table_width_visual` (already `#[ignore]`) — **leave** (it's an `#[ignore]` debug visualization test)
- Line 6450: `eprintln!("Raw output bytes:");` in `test_debug_highlight_codes` — **remove**
- Line 6464-6465: `eprintln!("\n---");` and `eprintln!("Plain output:\n{}", ...)` — **remove**
- Line 6470: `eprintln!("Yellow background code count: {}", bg_count);` — **remove**
- Line 6502-6504: In `test_strikethrough_section_no_blank_lines` — **remove** all three `eprintln!`
- Lines 6704, 6718, 6721-6722, 6724, 6726: In `test_debug_list_item_with_highlight` — **remove** all
- Lines 6788-6789, 6794: In `test_wrap_at_various_widths` — **remove** all

For each removal, delete the entire `eprintln!(...)` statement and any associated loop that existed only to print debug info (e.g., `for (i, line) in lines.iter().enumerate()` loops that only contain an `eprintln!`).

- [ ] **Step 2: Clean up html.rs test eprintln calls**

In `lib/src/markdown/output/html.rs`:

- Line 687: `eprintln!("Code block HTML: {}", html);` in `test_as_html_code_block` — **remove**
- Line 849: `eprintln!("HTML output: {}", html);` in `test_as_html_xss_prevention` — **remove**

- [ ] **Step 3: Clean up inline/mod.rs test eprintln calls**

In `lib/src/markdown/inline/mod.rs`:

- Lines 494, 496: The `eprintln!("Events:");` and `eprintln!("[{}] {:?}", i, e);` in `test_line77_inline_code_with_highlight` — **remove** the entire debug loop (the `eprintln!("Events:");` and the `for (i, e) in events.iter().enumerate() { eprintln!(...) }` block)

- [ ] **Step 4: Run tests to verify nothing broke**

Run: `cargo test -p darkmatter -- output::terminal::tests output::html::tests inline::tests`
Expected: All existing tests still pass

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/output/terminal.rs \
       darkmatter/lib/src/markdown/output/html.rs \
       darkmatter/lib/src/markdown/inline/mod.rs
git commit -m "chore(darkmatter): remove debugging eprintln! from test code"
```

---

## Task 12: Full build and integration test

**Files:** None (verification only)

- [ ] **Step 1: Run full library tests**

Run: `cargo test -p darkmatter`
Expected: All tests pass

- [ ] **Step 2: Run full CLI tests**

Run: `cargo test -p darkmatter-cli`
Expected: All tests pass

- [ ] **Step 3: Run lint**

Run: `cargo clippy -p darkmatter -p darkmatter-cli -- -D warnings`
Expected: No warnings

- [ ] **Step 4: Manual smoke test with `--debug`**

Create a test file and run compose with debug output:
```bash
echo '---\nname: test\n---\n# Hello {{name}}\nWorld' > /tmp/test-debug.md
cargo run -p darkmatter-cli -- --debug 2 compose /tmp/test-debug.md
```
Expected: Debug tracing output appears on stderr, composed content on stdout

- [ ] **Step 5: Manual smoke test with RUST_LOG**

```bash
RUST_LOG=darkmatter=trace cargo run -p darkmatter-cli -- compose /tmp/test-debug.md
```
Expected: Trace-level output on stderr

- [ ] **Step 6: Verify `--verbose` still works for styled output**

```bash
cargo run -p darkmatter-cli -- -v compose /tmp/test-debug.md
```
Expected: Styled summary on stderr (if verbose output was added in Task 10), no raw tracing lines

- [ ] **Step 7: Clean up temp file**

```bash
rm /tmp/test-debug.md
```
