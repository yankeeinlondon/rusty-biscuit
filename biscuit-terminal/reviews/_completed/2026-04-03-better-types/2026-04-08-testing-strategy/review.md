# Testing Strategy Review: biscuit-terminal

**Date:** 2026-04-08  
**Reviewer:** Code Review  
**Package Areas:** `biscuit-terminal/lib`, `biscuit-terminal/cli`

---

## Executive Summary

The biscuit-terminal package demonstrates solid testing foundations with 1146+ unit tests across 32 modules, comprehensive CLI integration tests using `assert_cmd`/`predicates`, and early adoption of property-based testing and PTY-based terminal testing. However, significant gaps exist in snapshot coverage, escape code rendering verification, and performance regression detection. This review recommends introducing criterion-based benchmarks for rendering hot paths and expanding snapshot coverage for terminal output.

**Note on TUI Testing:** The biscuit-terminal package is a terminal utility library and CLI, not a TUI application. It does not use ratatui or bubble tea. The `claudine config` command referenced in the request is `claudine mcp config`, a text-based interactive prompt using the `inquire` crate—not a graphical TUI. For TUI testing patterns in this monorepo, see `queue/cli/src/tui/` which uses `ratatui::backend::TestBackend`.

---

## 1. Testing Strategies Employed

### 1.1 Library Unit Tests

The library embeds `#[cfg(test)] mod tests` in 32 source modules, providing comprehensive coverage of internal components:

| Module Area | Test Count | Notable Patterns |
|------------|-----------|-----------------|
| `terminal.rs` | ~20 tests | Builder pattern, optimistic constructor |
| `discovery/detection.rs` | ~7 tests | Platform-specific assertions |
| `discovery/osc_queries.rs` | ~50 tests | RGB value operations, color queries |
| `discovery/fonts.rs` | ~20 tests | Font detection |
| `utils/layout.rs` | ~60 tests | Layout, margins, alignment, proptest |
| `utils/color.rs` | ~30 tests | Color conversion, luminance |
| `components/status.rs` | ~20 tests | Status bar rendering |
| `components/filesystem.rs` | ~40 tests | Tree building, filtering |
| `components/prose.rs` | ~50 tests | Token parsing, HTML-like tags |

**Strengths:**

- Tests use `Terminal::new_optimistic(width)` for deterministic rendering
- `Terminal::builder()` allows overriding specific fields without full detection
- Platform-specific assertions (`#[cfg(target_os = "macos")]`)
- Tests are co-located with implementation for good locality

**Weaknesses:**

- Many tests are "don't panic" tests that verify no crash but don't validate output correctness
- No systematic snapshot testing of rendered output
- OSC query functions tested minimally

### 1.2 Library Integration Tests

Located at `lib/tests/integration.rs` (477 lines):

```rust
// Key test categories:
- Terminal struct population and field consistency
- Detection function roundtrips (Terminal vs standalone functions)
- OSC queries (bg_color, text_color, cursor_color)
- Clipboard OSC52 sequence construction
- Mode 2027 support detection
- Eval functions (line_widths, has_escape_codes)
- Config paths for known terminals
```

**Notable:** The `test_repeated_terminal_creation` test intentionally limits iteration to 5 to avoid "OSC query flooding in TTY environments"—this demonstrates good awareness of test environment constraints.

### 1.3 Property-Based Testing

Located in `lib/src/utils/layout.rs` (lines 748-783):

```rust
proptest! {
    #[test]
    fn prop_layout_never_panics(
        text in ".*",
        terminal_width in 1..=500u32,
        margin_left in 0..=100u32,
        margin_right in 0..=100u32,
        indentation in 0..=100u32
    ) { ... }

    #[test]
    fn prop_available_width_is_bounded(...) { ... }
}
```

**Status:** Minimal—only 2 property tests exist. The proptest regressions file `proptest-regressions/utils/layout.txt` tracks 1 known failure case.

### 1.4 CLI Integration Tests

Located at `cli/tests/integration_test.rs` (1412 lines, 60+ tests):

**Test Categories:**

| Category | Count | Tools |
|---------|-------|-------|
| Metadata output (`--json`, `--help`, `--version`) | ~15 | `assert_cmd` + `predicates` |
| Shell completions (bash, zsh, fish, invalid) | 4 | `assert_cmd` |
| Content analysis | 3 | JSON parsing assertions |
| Chart commands (pie, bar, line, quadrant) | ~25 | JSON output validation |
| Diagram commands (flowchart, timeline, state, ERD) | ~15 | JSON + string assertions |
| PTY tests | 2 | `expectrl` |
| Snapshot tests | 2 | `insta` |
| NO_COLOR compliance | 1 | Output inspection |

**Example PTY test pattern:**

```rust
#[test]
#[serial_test::serial(visualization)]
fn test_graph_expression_meta_outputs_render_metadata_in_a_pty() {
    let mut cmd = Command::new(bin_path);
    cmd.env("CI", "1").env("NO_COLOR", "1").env("TERM_PROGRAM", "Ghostty");
    let mut p = Session::spawn(cmd).expect("Failed to spawn bt in PTY");
    p.expect("\"file_size_bytes\":").expect("...");
}
```

### 1.5 Testing Tools Summary

| Tool | Purpose | Usage |
|------|---------|-------|
| `assert_cmd` | CLI binary testing | `cargo_bin_cmd!`, `.assert().success()` |
| `predicates` | Assertion building | `predicate::str::contains()`, `.not()` |
| `expectrl` | PTY/TTY session testing | `Session::spawn()`, `.expect()` |
| `insta` | Snapshot testing | `insta::assert_snapshot!` |
| `proptest` | Property-based testing | `proptest!` macro |
| `serial_test` | Test serialization | `#[serial]` attribute |
| `tempfile` | Temporary fixtures | `TempDir::new()` |

---

## 2. Identified Gaps

### 2.1 Snapshot Coverage is Insufficient

**Current:** Only 2 snapshot tests exist:

- `test_prose_snapshot` — verifies `Hello [1mworld[0m!` output
- `test_columns_snapshot` — verifies two-column layout

**Problem:** The library produces styled/ansi output for many components:

- `prose` command with styling tokens
- `quote`, `list`, `padleft`, `padright` commands
- All chart and diagram commands
- Terminal metadata output

**Recommendation:** Add insta snapshots for:

1. All text-styling commands with various token combinations
2. Layout components with different width constraints
3. Chart renderers (already tested via JSON, but visual output untested)

### 2.2 Escape Code Rendering Not Verified

The `eval` module calculates visual widths and detects escape codes:

```rust
assert_eq!(line_widths("\x1b[31mred\x1b[0m"), vec![3]);
assert!(has_escape_codes("\x1b[31mred\x1b[0m"));
```

But these are unit-tested in isolation. The actual rendering pipeline—from styled input through layout to final escape-coded output—is not systematically verified.

**Recommendation:** Add integration tests that:

1. Render styled content through full pipeline
2. Verify escape codes are preserved correctly through word wrapping
3. Verify visual width calculations match rendered output

### 2.3 Discovery Functions Lack Mocking

Functions like `osc_queries::bg_color()` and `osc_queries::text_color()` query the terminal via OSC sequences. Tests only verify "doesn't panic"—they cannot verify correct behavior without mocking the terminal response.

**Current workaround:** Tests skip in non-TTY environments:

```rust
// test_osc_queries_dont_panic - but actually queries may return None
let _bg = bg_color(); // May be None in test env
```

**Recommendation:** Consider adding a mock terminal emulator interface or `FakeTerminal` test helper for deterministic OSC query testing.

### 2.4 Component Integration Testing Sparse

The 20+ renderable components have inline unit tests but lack systematic integration tests. For example:

- `components/two_column.rs` — only inline unit tests
- `components/list.rs` — only inline unit tests  
- `components/progress.rs` — only inline unit tests

**Recommendation:** Add integration tests in `lib/tests/` that exercise component compositions (e.g., prose inside a section inside a layout).

### 2.5 CLI Commands Module Not Independently Testable

The `cli/src/commands.rs` logic is called from `main.rs` but has no standalone tests. Testing requires building the full binary.

**Current pattern:**

```rust
// From integration_test.rs
cargo_bin_cmd!("bt")
    .arg("pie-chart")
    .arg("--json")
    .arg("Dogs: 386")
    .assert()
    .success();
```

**Recommendation:** Extract command logic into testable functions in the library, enabling unit tests without binary invocation.

---

## 3. Terminal Effectiveness Testing

### 3.1 What's Tested

| Capability | Test Coverage |
|------------|--------------|
| TTY detection | Via PTY tests (`expectrl`) |
| Color depth detection | Unit tests with `#[cfg]` platform checks |
| Image support detection | Only "don't panic" test |
| Underline support | Only "don't panic" test |
| OSC8 link support | Only "don't panic" test |
| Terminal dimensions | Tested in integration |
| Font detection | Basic field access test |
| Background color (OSC 110/111/10) | Query test, no verification |

### 3.2 What's Missing

**Critical missing tests:**

1. **Visual output correctness** — No verification that rendered output displays correctly in various terminals
2. **Kitty graphics protocol** — Image rendering uses `viuer` + `resvg` but only tested via JSON output
3. **Fallback behavior** — When terminal lacks capability X, verify graceful fallback
4. **Cross-terminal compatibility** — No tests for 13+ supported terminals (Kitty, WezTerm, iTerm2, Ghostty, etc.)

### 3.3 Limitations of Current Approach

Terminal testing inherently cannot fully simulate all terminal emulators. The current strategy of:

1. Testing with `Terminal::new_optimistic()` for deterministic rendering
2. Testing with PTY for TTY detection
3. Testing JSON output for machine-readable verification

...is a reasonable pragmatic approach. Full visual regression would require screenshots across 13+ terminals which is impractical for unit tests.

---

## 4. CLI Best Practices Compliance

### 4.1 What's Done Well

| Practice | Status |
|----------|--------|
| `--json` flag | ✅ Tested |
| `--help` flag | ✅ Tested |
| `--version` flag | ✅ Tested |
| Shell completions (bash/zsh/fish) | ✅ Tested |
| Invalid shell error | ✅ Tested |
| `NO_COLOR` compliance | ✅ Tested |
| Exit codes | ⚠️ Mostly implicit |
| `--quiet` / `--verbose` | ❌ Not tested |
| STDOUT vs STDERR separation | ❌ Not verified |
| Error output to STDERR | ❌ Not verified |

### 4.2 Missing CLI Tests

1. **Verbosity flags** — No tests for `-v`, `-vv`, `--quiet`, `--silent`
2. **Error exit codes** — No explicit assertions on exit code 1 vs 2
3. **Output stream verification** — No tests confirming errors go to STDERR
4. **Conflicting flags** — e.g., `--json --no-output` behavior
5. **Timeout/cancellation** — No tests for long-running operations

### 4.3 Recommendations

Add tests for:

```rust
#[test]
fn test_json_flag_outputs_json_to_stdout_only() {
    let output = cargo_bin_cmd!("bt")
        .arg("--json")
        .output()
        .expect("...");
    assert!(output.stderr.is_empty()); // No stderr in JSON mode
}

#[test]
fn test_error_goes_to_stderr() {
    let output = cargo_bin_cmd!("bt")
        .arg("pie-chart") // Missing required data
        .arg("--json")
        .output()
        .expect("...");
    assert!(!output.stderr.is_empty());
    assert_eq!(output.status.code(), Some(1));
}
```

---

## 5. Claudine Config TUI Assessment

**Clarification:** The biscuit-terminal package does not contain a TUI. The `claudine config` command (`claudine mcp config`) is a text-based interactive prompt using the `inquire` crate—it is not a graphical TUI using ratatui or bubble tea.

**For TUI testing patterns in this monorepo**, see `queue/cli/src/tui/` which demonstrates proper ratatui testing:

```rust
// From queue/cli/src/tui/render.rs
#[test]
fn render_produces_output_with_empty_task_list() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new();

    terminal.draw(|frame| render(&mut app, frame)).expect("...");
    
    let buffer = terminal.backend().buffer();
    // Verify content...
}
```

This pattern—using `ratatui::backend::TestBackend` for deterministic rendering tests—is the recommended approach for ratatui TUIs.

---

## 6. Performance Testing Recommendation

### 6.1 Should Performance Testing Be Added?

**Recommendation: YES**

**Rationale:**

1. The library performs complex text processing (word wrapping, escape code parsing, Unicode width calculation)
2. SVG rasterization via `resvg` is computationally expensive
3. Layout calculations are called for every rendered component
4. Terminal detection queries `/dev/tty` which can be slow
5. CI would catch regressions before they reach users

### 6.2 Recommended Tool: Criterion

**Criterion** (`cargo bench` ecosystem) is the gold standard for Rust benchmarking:

- Statistical analysis (detects variance)
- Memory profiling
- ASCII chart output
- Comparison between commits

**Alternative:** `divan` offers faster compile times but fewer features.

### 6.3 What to Benchmark

| Module | Function | Rationale |
|--------|----------|-----------|
| `utils/layout` | `Layout::apply_layout` | Hot path for all rendering |
| `utils/layout` | `Layout::available_width` | Called per-layout |
| `utils/block_constraint` | `wrap_lines` | Core word-wrapping |
| `utils/escape_codes` | `line_widths` | Escape code parsing |
| `discovery/detection` | `Terminal::new` | Expensive due to OSC queries |
| `discovery/osc_queries` | `bg_color` | TTY query latency |
| `components/mermaid` | Mermaid rendering | SVG processing |
| `components/terminal_image` | Image rendering | Resvg rasterization |

### 6.4 Level of Effort

**Low (1-2 days):**

1. Add `criterion = "1.x"` to `[dev-dependencies]`
2. Create `lib/benches/` with basic benchmark suite
3. Add `[[bench]]` to `Cargo.toml` for criterion
4. Run benchmarks locally to establish baselines

**Medium (1 week for full suite):**

1. Write 10-15 focused benchmarks covering hot paths
2. Integrate into CI with `cargo bench -- --noplot`
3. Add performance regression thresholds
4. Document expected performance characteristics

### 6.5 CI Integration

```yaml
# Example .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench -- --noplot --baseline master
  env:
    CARGO_BENCH_BASELINE: master
    
- name: Check for regression
  run: |
    cargo bench -- --noplot --save-baseline current
    cargo bench -- --noplot --compare-with master || exit 1
```

### 6.6 Recommendation

**Add criterion-based benchmarks for the top 5 hot paths:**

1. `Layout::apply_layout`
2. `line_widths`  
3. `wrap_lines`
4. `Terminal::new` (detection)
5. Mermaid rendering

This provides good coverage with modest effort and would catch significant regressions.

---

## 7. Summary of Recommendations

### High Priority

1. **Add performance benchmarks** — Add criterion benchmarks for hot paths; integrate into CI with regression detection.

2. **Expand snapshot coverage** — Add insta snapshots for styled text output from `prose`, `quote`, `list`, `padleft`, `padright`.

3. **Add verbosity flag tests** — Test `-v`, `-vv`, `--quiet`, `--silent` behavior and output stream separation.

4. **Verify STDERR for errors** — Add explicit assertions that errors go to STDERR with correct exit codes.

### Medium Priority

1. **Add escape code rendering integration tests** — Verify styled content through the full pipeline maintains correct escape codes.

2. **Add CLI command integration tests** — Test `commands.rs` logic independently or add more binary integration tests.

3. **Add mock terminal interface** — For deterministic OSC query testing without real terminal.

4. **Component composition tests** — Add integration tests in `lib/tests/` for component combinations.

### Lower Priority

1. **Expand property-based testing** — Add more `proptest` tests for layout, text processing, and escape code parsing.

2. **Cross-terminal compatibility tests** — Consider adding tests for fallback behavior when terminals lack capabilities.

---

## 8. Test Utilities Available

### Terminal Test Helpers in Library

```rust
// From terminal.rs - these are excellent utilities:
Terminal::new_optimistic(width)  // Fast, deterministic, full capabilities
Terminal::builder()              // Override specific fields
Terminal::new_tty()            // Sets is_tty=true without real detection
```

### Recommended Additional Helpers

```rust
// Mock terminal for OSC query testing
struct FakeTerminal {
    bg_color: Option<RgbValue>,
    text_color: Option<RgbValue>,
    // ...
}

impl osc_queries::OsciQuery for FakeTerminal {
    fn bg_color(&self) -> Option<RgbValue> { self.bg_color }
    // ...
}
```

---

## Appendix: Test Statistics

| Metric | Library | CLI |
|--------|---------|-----|
| Test files | 1 integration + 32 inline | 1 integration |
| Test functions | ~1146 unit + ~30 integration | ~60 integration |
| Snapshot tests | 0 | 2 |
| Property tests | 2 | 0 |
| PTY tests | 0 | 2 |
| Serial tests | 0 | 2 |

---

*Review generated: 2026-04-08*
