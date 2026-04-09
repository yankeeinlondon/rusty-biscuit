# Claudine Testing Strategy Review

**Date:** 2026-04-08
**Scope:** `claudine/lib` and `claudine/cli`
**Reviewer:** Automated analysis with CLI/TUI/Rust-testing skill guidance

---

## Executive Summary

The Claudine package area has a substantial test suite: 126+ inline `mod tests` blocks across the library, 20+ in the CLI, and 7 dedicated integration test files totaling ~5,973 lines. The library's core modules -- dispatch, protect, composition, harness validation -- have meaningful unit test coverage with good edge-case exploration. The CLI integration tests are thorough for the wrapper, MCP, skills, and sequence surfaces.

However, several systemic gaps exist: **no shared test utilities** (massive duplication), **the TUI (`claudine config`) has minimal test coverage**, **terminal rendering functions are largely untested**, **command routing has no dedicated tests**, and **there is zero performance testing infrastructure**. The two PTY tests are permanently `#[ignore]` and serve only as manual smoke tests.

---

## 1. Current Testing Inventory

### 1.1 Test Infrastructure

| Dimension | Status |
|-----------|--------|
| Test runner | `cargo test` / `cargo nextest` (monorepo-wide `.config/nextest.toml` with 3 retries) |
| Justfile recipe | `just test` in `claudine/justfile` runs `cargo test -p claudine` then `cargo test -p claudine-cli` |
| Shared test utilities | **None** -- every integration test file is self-contained with private helpers |
| CI-specific config | Nextest retries for flaky PTY/timing tests |

### 1.2 Library Tests (`claudine/lib`)

| Category | Count |
|----------|-------|
| Inline `#[cfg(test)]` modules | 126 source files |
| Dedicated integration test files | 1 (`tests/canonical_dispatch.rs`, 6 tests) |
| Dev-dependencies | `tokio` (rt-multi-thread), `tempfile`, `serial_test`, `serde_json` |

### 1.3 CLI Tests (`claudine/cli`)

| File | Lines | Tests | Focus |
|------|-------|-------|-------|
| `wrap_commands.rs` | 3,274 | ~70+ | Wrapper execution, env, streaming, composition, MCP |
| `sequence_cli.rs` | 773 | 11 | Sequence fail-fast, external refs, state interpolation |
| `mcp_cli.rs` | 750 | 15 | MCP show/export/sync/alias/wrapper dry-run |
| `skills_integration.rs` | 591 | 19 | Skills listing, filtering, fix, detail view |
| `handle_repo_config.rs` | 176 | 2 | Repo-scoped config, package context |
| `protect_cli.rs` | 84 | 1 | Protect decision JSON output |
| `pty_tests.rs` | 58 | 2 (ignored) | PTY-based badge and non-interactive detection |
| Inline `mod tests` | -- | 20 files | TUI reducers, init wizard, wrap helpers, output, telemetry |

| Dev-dependencies | `assert_cmd`, `predicates`, `insta` (JSON), `expectrl`, `proptest` |
| Snapshot files | 3 (help output, sensitive env, wrapper flags) |

### 1.4 Test Tool Summary

| Tool | Used For | Where |
|------|----------|-------|
| `assert_cmd` | CLI integration tests | All `cli/tests/` files |
| `predicates` | String/content assertions | CLI integration tests |
| `insta` | Snapshot testing (3 snapshots) | `wrap_commands.rs` |
| `expectrl` | PTY-based tests (2, ignored) | `pty_tests.rs` |
| `proptest` | Property-based testing | Listed as dev-dep, **no tests found** |
| `tempfile` | Temporary directories | Library tests, CLI tests |
| `serial_test` | Global state isolation | Library tests |
| `tokio::test` | Async test runtime | Library dispatch tests |

---

## 2. Strategies and Utilities Evaluation

### 2.1 Strengths

1. **Good use of the lib/cli split.** The library has no I/O assumptions and is tested independently. The CLI tests exercise the binary end-to-end with `assert_cmd`.

2. **Thorough integration coverage for hot paths.** `wrap_commands.rs` (3,274 lines, ~70 tests) covers wrapper execution, env injection, streaming, composition, MCP, and exit code propagation comprehensively.

3. **Async tests where appropriate.** The dispatch module uses `#[tokio::test]` for async functions; sync modules use `#[test]`.

4. **Edge-case exploration.** Protect service tests cover symlinks, relative path traversal, cross-field false positives, and allow_paths interactions. Preflight tests exercise the shared approval cache thoroughly.

5. **Serial test isolation.** `serial_test` is available and used for tests sharing environment variables.

6. **Snapshot testing started.** Three `insta` snapshots for help output and sensitive env reporting.

### 2.2 Gaps and Weaknesses

#### G1: No Shared Test Utilities (High Priority)

Seven integration test files independently define the same helpers:

| Duplicated Helper | Files |
|---|---|
| `write_executable()` | `wrap_commands.rs`, `pty_tests.rs`, `sequence_cli.rs` |
| `write_file()` / `write()` | `wrap_commands.rs`, `protect_cli.rs`, `handle_repo_config.rs` |
| `strip_ansi()` | `wrap_commands.rs`, `sequence_cli.rs` (identical impls) |
| `TestWorkspace` struct | `mcp_cli.rs`, `skills_integration.rs`, `handle_repo_config.rs` |
| `write_json()` | `wrap_commands.rs`, `mcp_cli.rs` |
| `seed_catalog/seed_defaults` | `wrap_commands.rs`, `mcp_cli.rs` |

**Recommendation:** Create `claudine/cli/tests/common/mod.rs` with shared helpers. Rust integration tests can use this via `mod common;` in each test file. Estimated effort: 2-3 hours to extract, consolidate, and update imports.

#### G2: `proptest` Declared but Unused (Medium Priority)

`proptest` is listed as a dev-dependency in `claudine/cli/Cargo.toml` but no property-based tests exist anywhere in the codebase.

**Recommendation:** Either add property tests for parser-heavy modules (stream parsers, template engine, config parsing) or remove the dependency to reduce compile times. If adding tests, good candidates include:
- `dispatch::loader` regex matcher patterns
- `composition::sequence` template rendering
- `output::truncate_args` boundary behavior
- `stream::ensure_message_newline` edge cases

Estimated effort: 4-6 hours for initial property tests across 3-4 modules.

#### G3: Snapshot Testing Underutilized (Medium Priority)

Only 3 `insta` snapshots exist despite `insta` being available. Several areas would benefit from snapshot testing:
- Wrapper header rendering (badge composition, env details)
- Help output for all subcommands (not just the top-level)
- JSON output for `hooks --describe`, `hooks --variables`, `providers`, `logs`
- Protect rule report formatting
- TUI tab rendering (via `TestBackend`)

**Recommendation:** Expand snapshot testing to cover all structured output surfaces. Estimated effort: 4-6 hours.

#### G4: Inline Library Test Quality Varies (Medium Priority)

Detailed analysis of six key library modules:

| Module | Tests | Coverage Assessment |
|--------|-------|-------------------|
| `dispatch/mod.rs` | 20 | Good for core paths; `finalize_response`, `dispatch_event_meta`, and canonical full-pipeline untested |
| `harness/validate.rs` | 24 | Good breadth; `FrontmatterPropChanged/Unchanged`, `NoDirtySourceCode`, `ShellCommand` completely untested |
| `stream/mod.rs` | 4 | Minimal; `ensure_message_newline` untested, fallback paths untested |
| `services/protect/service.rs` | 18 | Strong -- best coverage of the set |
| `composition/sequence.rs` | 22 | Good for inline/external; FileReference resolution paths (`@`, `!`, `vault:`, `{{ENV}}`) untested |
| `composition/preflight.rs` | 14 | Good shared-cache testing; "no handler" error path and post-check sources untested |

**Critical untested paths:**
- `harness/validate.rs`: `ShellCommand` and `NoDirtySourceCode` are safety-critical checks with zero tests
- `composition/sequence.rs`: All FileReference magic paths (`@`, `!`, `vault:`) are the real-world resolution paths but have no test coverage
- `dispatch/mod.rs`: `finalize_response` determines blocking behavior and exit codes but has no direct test

---

## 3. Terminal Testing Evaluation

### 3.1 Terminal Rendering Functions (CLI)

The CLI's `output.rs` has 11 inline tests for pure-logic functions (`truncate_args`, `shell_escape`, `try_format_api_error`). However, the following rendering functions that produce styled terminal output have **zero unit tests**:

| Function | Purpose | Risk |
|----------|---------|------|
| `log_wrapper_header()` | Primary user-facing header with badges, profile, compose display | High -- most visible output |
| `log_compose_prompt()` | Markdown rendering via Darkmatter in blockquote | Medium |
| `log_wrapper_env_details()` | Env variable styling (red strikethrough, orange, green) | Medium |
| `log_dry_run()` | Entire dry-run display | Medium |
| `style_cli_switches()` | Switch-highlighting parser | Low |

The barrier appears to be perceived difficulty of asserting on styled output, but `Terminal::new_optimistic(80)` is already used in test helpers and provides a non-TTY test terminal. These functions can be tested.

**Recommendation:** Add unit tests for `log_wrapper_header` at minimum, using `Terminal::new_optimistic()` and snapshot testing for the styled output. Use `insta` with `assert_snapshot!` to capture the full rendered string. Estimated effort: 3-4 hours.

### 3.2 PTY Testing

Two PTY tests exist in `pty_tests.rs`, both `#[ignore]` with the note "timing-sensitive." The rationale is sound -- PTY-based `expect` patterns are inherently racy. However:

- The non-PTY integration tests cover the same logical paths (badge injection, env setup, passthrough args) via `assert_cmd`, so correctness is verified.
- The PTY tests serve as manual smoke tests for TTY-layer output integrity.
- `expectrl` is a dev-dependency specifically for these tests.

**Recommendation:** Keep the current approach. The `#[ignore]` PTY tests are correctly positioned as manual-only smoke tests. The investment to make them deterministic would be high and the return low. Consider adding a `just test-pty` recipe that runs `cargo test -- --ignored` explicitly.

### 3.3 NO_COLOR / FORCE_COLOR Testing

No tests verify that `NO_COLOR` suppresses colors or that `FORCE_COLOR=1` enables them in non-TTY contexts. This is a CLI best practice requirement.

**Recommendation:** Add 2-3 integration tests:
- `claudine hooks --json` with `NO_COLOR=1` produces no escape codes on stdout
- `claudine providers` with `FORCE_COLOR=1` produces colored output even when piped
- Verify `--plain` flag strips escape codes

Estimated effort: 2 hours.

---

## 4. CLI Best Practices Compliance

### 4.1 Exit Code Testing

Exit codes are well-tested in the wrapper integration tests:
- Success (exit 0) propagation from child
- Error (exit 1) propagation from child
- Usage errors (exit 2) for invalid arguments

**Gap:** No tests verify exit codes for non-wrapper commands (`hooks`, `providers`, `mcp`, `logs`, `skills`, etc.).

### 4.2 STDOUT vs STDERR Separation

The wrapper tests verify that `--json` produces valid JSON on stdout. However:
- No tests verify that status/progress messages go to stderr for non-wrapper commands
- No tests verify that errors always go to stderr, even in `--json` mode (for non-wrapper commands)

**Recommendation:** Add integration tests for stderr-only output for `claudine hooks`, `claudine providers`, and `claudine mcp list`. Estimated effort: 2 hours.

### 4.3 Command Routing

There is **no dedicated command routing test**. The `--help` snapshot test implicitly verifies that all subcommands are registered with clap, but no test exercises the `match cli.command` dispatch table in `main.rs` for every variant.

**Recommendation:** Add a test that verifies each `Commands` variant dispatches to the correct handler. This can be done by testing that `claudine <subcommand>` with minimal valid args reaches the handler (even if it errors due to missing files/env). Estimated effort: 3-4 hours.

### 4.4 Shell Completions

Shell completions are generated via `claudine completions <shell>`. This subcommand exists but has no integration test verifying it produces valid output for each shell.

**Recommendation:** Add tests for bash, zsh, fish, and elvish completion output. Estimated effort: 1-2 hours.

---

## 5. TUI (`claudine config`) Testing Evaluation

### 5.1 Architecture Overview

The TUI is built with Ratatui using an immediate-mode pattern:
- **State:** Single mutable `App` struct (app.rs)
- **Rendering:** Direct read from `&App` in tab-specific render functions
- **Events:** Key dispatch chain from `App::handle_key` to tab-specific handlers

The architecture does **not** follow the recommended Model-View-Action separation. Render functions are tightly coupled to `&App`, and there is no stateless view function that can accept arbitrary state. This makes testing significantly harder.

### 5.2 Current Test Coverage

| Module | Lines | Tests | Coverage |
|--------|-------|-------|----------|
| `reducers.rs` | 269 | 13 | Good -- pure functions for voice and messenger config |
| `tabs/actions.rs` | 1,461 | 2 | Minimal -- only effective-view merge and scope switching |
| `tabs/messenger.rs` | 712 | 2 | Minimal -- sorted names and 'S' key modal |
| `tabs/preferences.rs` | 502 | 2 | Minimal -- stale agent filtering |
| `tabs/services.rs` | 430 | 0 | **None** |
| `tabs/tts.rs` | 500 | 0 | **None** |
| `widgets/modal.rs` | 158 | 0 | **None** |
| `widgets/toggle.rs` | 56 | 0 | **None** |
| `app.rs` | 415 | 0 | **None** |
| `mod.rs` (entry) | 489 | 0 | **None** |

**Total: 19 tests across ~4,999 lines of TUI code.**

### 5.3 Critical Gaps

1. **No TestBackend tests.** The TUI has never been rendered to a `TestBackend` buffer. No screenshot/snapshot of any tab exists. This means layout regressions are invisible.

2. **No event simulation tests.** Key handlers for all tabs except a single test in messenger.rs are untested. The full modal stack navigation (up to 5 levels deep in Actions) has no test coverage.

3. **No resize testing.** The TUI has no minimum-size guard tests. At narrow widths, text may truncate or overflow without detection.

4. **Services and TTS tabs have zero tests.** These tabs handle protect rule toggling (security-sensitive), TTS provider selection, and voice configuration. The protect rule modal staging/commit logic is particularly important to test.

5. **App state machine untested.** The Overview/Detail mode transition, modal push/pop, and dirty flag tracking have no tests.

### 5.4 Recommendations

#### R1: Extract Pure Functions (High Priority, 8-12 hours)

The current architecture tightly couples key handling to `&mut App`. To make the TUI testable:

1. Extract pure reducer functions from each tab's `handle_key` -- return `(App, Option<Event>)` or a command type instead of mutating `App` directly. This follows the Elm/Redux pattern.
2. The `reducers.rs` module already does this for voice selection and messenger config creation. Extend this pattern to all tabs.
3. Priority targets: `services.rs` (protect rule staging), `actions.rs` (event/action CRUD), `preferences.rs` (agent/provider selection).

#### R2: Add TestBackend Rendering Tests (Medium Priority, 6-8 hours)

For each tab, create at least one test that:
1. Constructs an `App` with known state
2. Renders the tab to a `TestBackend` buffer
3. Asserts the buffer with `insta::assert_debug_snapshot!`

This requires refactoring render functions to accept a reference to a data struct instead of `&App`, or constructing a full `App` in test context. The latter is simpler but requires mocking `cached_agents` and other runtime data.

Start with:
- `widgets/toggle.rs` -- simplest widget, good proof of concept
- `widgets/modal.rs` -- centering logic, layout correctness
- `tabs/services.rs` -- protect rule grid rendering

#### R3: Add Event Simulation Tests (Medium Priority, 6-8 hours)

For each tab, test the key handler by:
1. Constructing an `App` with known state
2. Calling the tab's `handle_key(app, key)` function with simulated `KeyEvent` instances
3. Asserting on the resulting `App` state

Priority: services tab (protect rules), actions tab (CRUD operations), preferences tab (agent selection).

#### R4: Test the App State Machine (Low Priority, 2-3 hours)

Add tests for:
- Overview -> Detail transition (Enter key)
- Detail -> Overview transition (Esc key)
- Modal push/pop cycle
- Dirty flag setting and config persistence

---

## 6. Performance Testing Evaluation

### 6.1 Current State

**Zero performance testing infrastructure exists** in the claudine package area. No benchmark files, no `criterion` dependency, no `#[bench]` functions, no performance regression tests.

### 6.2 Candidate Surfaces for Benchmarking

| Surface | Rationale | Difficulty |
|---------|-----------|------------|
| `dispatch` pipeline (config load + matcher eval + action execution) | Hot path called on every hook event | Medium -- requires tempdir setup |
| `protect::ProtectService` evaluation | Called per bash command, write path, and MCP response; regex-heavy | Low -- already well-structured, synchronous |
| `stream` parsers (6 providers) | Called per line of streaming output; must be sub-millisecond | Low -- in-memory string parsing |
| `composition::sequence` plan resolution | File I/O + YAML parsing + template rendering | Medium -- requires fixture files |
| `config` loading and merging | Called on every dispatch; JSON parsing + repo detection | Medium -- requires filesystem setup |
| `reporting::ingest` (JSONL to SQLite) | Bulk data operation | Medium -- requires SQLite setup |
| `linking::symlink` operations | Filesystem-bound, called during sync | Low -- pure I/O benchmarks |

### 6.3 Recommended Tools

| Tool | Purpose | Effort to Integrate |
|------|---------|-------------------|
| `criterion` (0.5) | Statistical benchmarking with regression detection | Low -- add to dev-deps, create `[[bench]]` sections |
| `cargo bench` | Built-in runner for criterion | Zero -- already available |
| `insta` snapshot benchmarks | Capture timing baselines alongside snapshots | None -- already available |
| `divan` (0.1) | Simpler alternative to criterion for parametric benchmarks | Low -- alternative to criterion |

### 6.4 Recommendation

**Recommended addition: targeted criterion benchmarks for the 3 highest-impact surfaces.**

**Why:** Claudine sits between the user and the wrapped CLI. Any latency added by the dispatch pipeline, protect evaluation, or stream parsing directly impacts perceived responsiveness of every agentic CLI session. Establishing performance baselines now prevents regressions as the codebase grows.

**Recommended scope (phase 1):**

1. **`ProtectService` evaluation** (bash command, write path, MCP response)
   - Why: Called on every tool call and MCP response in protect-enabled sessions
   - Difficulty: Low -- synchronous, well-tested, isolated service
   - Effort: 2-3 hours

2. **Stream parser throughput** (all 6 provider parsers)
   - Why: Must parse streaming output in real-time; any delay causes visible lag
   - Difficulty: Low -- pure string parsing, no I/O
   - Effort: 2-3 hours

3. **Dispatch pipeline** (config load + matcher evaluation)
   - Why: Hot path on every hook event
   - Difficulty: Medium -- needs tempdir and config fixtures
   - Effort: 4-5 hours

**Total estimated effort for phase 1:** 8-11 hours
**Ongoing maintenance:** ~1 hour per month to review criterion reports and update baselines

**Phase 2 (optional, lower priority):**
- Config loading/merging benchmarks
- JSONL ingestion benchmarks
- Sequence plan resolution benchmarks

**Is this recommended?** Yes, with the following caveats:
- Start with phase 1 only -- the protect and stream surfaces are the most impactful
- Run benchmarks in CI with `cargo bench` on a dedicated job (not per-PR) to avoid slowing the build
- Use criterion's HTML reports for manual review and regression detection
- Do not gate PRs on benchmark results initially -- use them as informational signals

---

## 7. Prioritized Action Items

### P0: Critical (Do First)

| # | Item | Effort | Impact |
|---|------|--------|--------|
| 1 | Create shared test utilities (`tests/common/mod.rs`) | 2-3h | Eliminates ~400 lines of duplication, reduces maintenance burden |
| 2 | Add tests for `harness/validate.rs` safety-critical paths (`ShellCommand`, `NoDirtySourceCode`) | 3-4h | These are security gates with zero test coverage |
| 3 | Add tests for `composition/sequence.rs` FileReference resolution paths | 2-3h | These are the real-world paths users will actually use |

### P1: High (Do Soon)

| # | Item | Effort | Impact |
|---|------|--------|--------|
| 4 | Add `TestBackend` + `insta` snapshot tests for TUI tabs (start with widgets, then services) | 6-8h | First-ever rendering tests; catches layout regressions |
| 5 | Add unit tests for `output.rs` rendering functions (header, env details, dry-run) | 3-4h | Most visible user output is untested |
| 6 | Add `NO_COLOR` / `FORCE_COLOR` / `--plain` integration tests | 2h | CLI best practice compliance |
| 7 | Add command routing tests for all `Commands` variants | 3-4h | Prevents routing regressions when adding new subcommands |
| 8 | Add TUI event simulation tests for services tab (protect rules) | 3-4h | Security-sensitive toggle/staging logic untested |

### P2: Medium (Plan for Next Cycle)

| # | Item | Effort | Impact |
|---|------|--------|--------|
| 9 | Extract pure reducers from TUI tab handlers | 8-12h | Enables comprehensive TUI testing; improves architecture |
| 10 | Add criterion benchmarks for protect, stream parsers, dispatch | 8-11h | Performance regression detection |
| 11 | Expand snapshot testing for structured JSON output surfaces | 4-6h | Catches output format regressions |
| 12 | Add property tests with `proptest` for parsers and templates | 4-6h | Edge-case discovery for complex parsing logic |
| 13 | Add shell completion output tests | 1-2h | Prevents completion breakage |
| 14 | Add stdout/stderr separation tests for non-wrapper commands | 2h | CLI best practice compliance |

### P3: Low (Nice to Have)

| # | Item | Effort | Impact |
|---|------|--------|--------|
| 15 | Add TUI resize tests (narrow/wide/edge dimensions) | 2-3h | Catches layout overflow bugs |
| 16 | Add TUI app state machine tests (mode transitions, dirty flags) | 2-3h | Catches state machine regressions |
| 17 | Add PTY test documentation and `just test-pty` recipe | 1h | Makes manual PTY testing discoverable |
| 18 | Phase 2 benchmarks (config loading, JSONL ingestion, sequence resolution) | 6-8h | Broader performance visibility |

---

## 8. Summary

The Claudine package area has built a solid testing foundation for its core library dispatch pipeline and CLI wrapper execution. The 70+ wrapper integration tests are a genuine strength. However, the testing strategy has three systemic weaknesses:

1. **Infrastructure debt:** No shared test utilities creates maintenance burden and inconsistency. The `proptest` dependency is unused.

2. **TUI testing is critically underdeveloped.** The `claudine config` TUI has 19 tests for ~5,000 lines of code. Two tabs and all widgets have zero tests. The architecture does not support easy testing because rendering and state mutation are tightly coupled.

3. **Terminal output rendering is a blind spot.** The most user-visible output functions have no unit tests despite having a non-TTY test terminal available.

The recommended path forward is: consolidate test utilities (P0-1), add safety-critical test coverage (P0-2, P0-3), then systematically add TestBackend rendering tests and output formatting tests (P1) before tackling the larger TUI architecture refactoring (P2-9) and performance benchmarking (P2-10).
