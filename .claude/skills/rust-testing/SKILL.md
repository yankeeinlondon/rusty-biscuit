---
name: rust-testing
description: |
  Monorepo testing guide: L1/L2/L3 taxonomy, canonical just recipes,
  `require_level!` gating, nextest filtersets, and fuzzing. Load this
  before writing or reviewing tests in the rusty-biscuit workspace.
hash: 8eaceadb34324bef-3ffcf7556a491d3d
last_updated: 2026-06-01
---
# Rust Testing — Rusty Biscuit Monorepo

## Decision Tree: "What tier should my test live in?"

Start at the **requirement**, not the code:

```text
Does the test need a real terminal, browser, or device to verify behaviour?
├── NO  → Is it slow (>5 s) or does it hammer an external API?
│   ├── NO  → L1 (default). Name it normally.
│   └── YES → L1 with `slow_` prefix so sanity skips it.
├── YES → Is it a headless browser test?
│   ├── YES → Browser tier. Name it `browser_*`.
│   └── NO  → Does it need OS keyboard/mouse injection?
│       ├── YES → L3. Name it `level3_*`. Requires RUN_LEVEL3=1.
│       └── NO  → L2. Name it `level2_*`. Requires a harness (tmux/WezTerm/Chrome).
```

If the only meaningful coverage of a public API requires a real resource,
document the exception in `docs/testing-strategy.md`; do not force it into
`sanity`.

## Test Levels

| Level   | Prefix     | Resource            | Skip when absent     | Hard-fail env                   |
|---------|------------|---------------------|----------------------|---------------------------------|
| L1      | (none)     | In-process only     | Never                | —                               |
| L2      | `level2_`  | Real terminal / PTY | Harness missing      | `BISCUIT_TEST_LEVEL_REQUIRED=2` |
| L3      | `level3_`  | OS keyboard/mouse   | `RUN_LEVEL3` unset   | `BISCUIT_TEST_LEVEL_REQUIRED=3` |
| Browser | `browser_` | Chrome/Chromium     | Browser missing      | `BISCUIT_BROWSER_REQUIRED=1`    |
| Real    | `real_`    | External device/API | Resource missing     | Per-package env vars            |
| Slow    | `slow_`    | None (slow L1)      | Excluded from sanity | —                               |

## Gating Tests

Use `test_toolkit::require_level!` at the top of a test body:

```rust
use test_toolkit::{require_level, Level};

#[test]
#[serial_test::serial]
fn level2_renders_in_real_terminal() {
    require_level!(Level::L2, WezTermHarness::available(), "WezTerm");
    // ... test body
}
```

For browser tests:

```rust
#[tokio::test]
#[serial_test::serial(browser)]
async fn browser_computed_style_matches() {
    if !biscuit_browser_harness::require_browser() { return; }
    // ... test body
}
```

## Canonical Just Recipes

Every curated package area defines these 12 recipes:

| Recipe         | Meaning                                                                                                                                                                                                                                                                                                                                      |
|----------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `sanity`       | Fast confidence (≤15 s). `cargo nextest run --lib --bins -E '!set:slow'`.                                                                                                                                                                                                                                                                    |
| `test`         | Full L1 suite.                                                                                                                                                                                                                                                                                                                               |
| `test-l2`      | Real-terminal tests. Pre-spawns one shared pane per backend via `biscuit-harness-broker`, exports `BISCUIT_SHARED_*_ID` env vars, runs nextest with `-j 1`, tears panes down in a trap. Tests use `<Backend>Harness::shared_or_spawn()` to attach to the pre-spawned pane and fall back to per-process spawning when the env var is missing. |
| `test-l3`      | OS keyboard/mouse tests.                                                                                                                                                                                                                                                                                                                     |
| `test-browser` | Headless browser tests.                                                                                                                                                                                                                                                                                                                      |
| `test-real`    | External resource tests.                                                                                                                                                                                                                                                                                                                     |
| `lint`         | Clippy + fmt check.                                                                                                                                                                                                                                                                                                                          |
| `bench`        | Criterion benchmarks (no-op if opted out).                                                                                                                                                                                                                                                                                                   |
| `coverage`     | Per-package LCOV.                                                                                                                                                                                                                                                                                                                            |
| `doctest`      | `cargo test --doc`.                                                                                                                                                                                                                                                                                                                          |
| `fuzz`         | `cargo +nightly fuzz run` (no-op if no targets).                                                                                                                                                                                                                                                                                             |
| `all`          | `sanity → lint → doctest → test → test-l2 → test-browser`.                                                                                                                                                                                                                                                                                   |

Delegate to shared recipes in `just/devops.just` (e.g. `@just _test my-crate`).

## Nextest Filtersets

The `.config/nextest.toml` does not yet define named filterset aliases (nextest
feature limitation). The shared `_sanity`, `_test_l2`, etc. recipes pass the
filter expression directly:

- `sanity`: `-E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/))'`
- `test-l2`: `-E 'test(/level2_/)'`
- `test-l3`: `-E 'test(/level3_/)'`
- `test-browser`: `-E 'test(/browser_/)'`
- `test-real`: `-E 'test(/real_/)'`

## Leaked Process Detection

Two complementary layers catch tests that spawn child processes and fail to
reap them:

1. **nextest `LEAK` (per test, all platforms).** `.config/nextest.toml` sets
   `leak-timeout = { period = "100ms", result = "fail" }` on both profiles, so a
   test that exits while a child still holds its stdout/stderr **fails the run**.
   Clean tests are not slowed — only a leak waits the window out. Drop
   `result = "fail"` to downgrade leaks to a non-fatal warning.
2. **`just test-leaks` (post-run sweep, all platforms).** Wraps `just test` in
   `leak-sweep` (`tools/test-toolkit`, `--features leak-sweep`). It diffs the
   process list before/after the whole run and reports survivors whose
   executable or command line is under the repo (exit code `99`). Catches
   detached orphans that closed the test's pipes — which `LEAK` cannot see.
   Attribution is by workspace path, not parent PID (orphan reparenting is
   OS-specific).

## Environment Contract

| Variable                          | Purpose                                      |
|-----------------------------------|----------------------------------------------|
| `BISCUIT_TEST_LEVEL=1\|2\|3`        | Max level to run; higher tiers skip cleanly. |
| `BISCUIT_TEST_LEVEL_REQUIRED=2\|3` | Missing harness panics instead of skipping.  |
| `BISCUIT_BROWSER_REQUIRED=1`      | Missing Chrome panics instead of skipping.   |
| `RUN_LEVEL3=1`                    | Opt-in for OS-keyboard-injection tests.      |

## Fixtures and Env Guards

Use `test_toolkit::EnvGuard` for process-env setup/teardown and
`#[serial_test::serial]` when mutating global state:

```rust
use test_toolkit::{trace_phase, EnvGuard};
use rstest::{fixture, rstest};

#[fixture]
fn dry_run() -> EnvGuard {
    EnvGuard::set_safe("PLAYA_DRY_RUN", "1")
}

#[rstest]
#[tokio::test]
#[serial_test::serial]
async fn dispatch_with_dry_run(#[from(dry_run)] _g: EnvGuard) {
    // ...
}
```

## Browser Tests

Assert on **computed styles**, not source substrings or screenshots:

```rust
let mut h = ChromeHarness::new();
h.spawn().await?;
h.render_html(&wrap_fragment("<div class='x'>hi</div>", "#fff")).await?;
let bg = h.computed_style(".x", "background-color").await?;
assert_eq!(bg, "rgb(17, 27, 39)");
```

## Fuzzing

Fuzz targets live in `<crate>/fuzz/` and require nightly Rust. Run locally:

```bash
cd biscuit-file/lib/fuzz
cargo +nightly fuzz run pdf_extract -- -runs=1000
```

Fuzz is **not** part of `sanity`, `test`, or PR gates. It runs nightly in CI.

## Key Crates

| Crate                     | Purpose                                                                                                                                                         |
|---------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `test_toolkit`            | `require_level!`, `EnvGuard`, `trace_phase!`                                                                                                                    |
| `biscuit_test_harness`    | Terminal harnesses (WezTerm, Kitty, tmux, Apple Terminal); `SharedHarness` + per-backend `shared_or_spawn()`; `biscuit-harness-broker` binary used by `test-l2`. For backend selection and API, load the `biscuit-test-harness` skill via the Skill tool. |
| `biscuit_browser_harness` | Headless Chrome harness (`ChromeHarness`, `require_browser`)                                                                                                    |
| `criterion`               | Benchmarking                                                                                                                                                    |
| `rstest`                  | Fixtures and parameterization                                                                                                                                   |
| `serial_test`             | Serialize env/stateful tests                                                                                                                                    |
| `pretty_assertions`       | Better diffs                                                                                                                                                    |
| `insta`                   | Snapshot testing                                                                                                                                                |

## Topic Pages

Open the topic file when the task matches:

| Topic                                                                | File                                    |
|----------------------------------------------------------------------|-----------------------------------------|
| L2 WezTerm capture gotchas (SGR collapsing, semicolon vs colon form). For backend selection / harness API, load the `biscuit-test-harness` skill via the Skill tool. | `wezterm-harness-pitfalls.md`           |
| CLI output (channels, color modes, completions, snapshots)           | `cli-output-testing.md`                 |
| TUI rendering and event/reducer tests                                | `tui-testing.md`                        |
| Browser tests (computed-style assertions)                            | `browser-testing.md`                    |
| Integration tests                                                    | `integration-tests.md`                  |
| Unit tests                                                           | `unit-tests.md`                         |
| Snapshots and redaction                                              | `snapshots.md`, `snapshot-redaction.md` |
| Doc tests                                                            | `doc-tests.md`                          |
| Mocking                                                              | `mocking.md`                            |
| Property testing                                                     | `property-testing.md`                   |
| Fuzzing                                                              | `fuzzing.md`                            |
| Performance testing tool choice (Criterion vs Divan)                 | `performance-testing.md`                |
| Criterion benchmarking (getting started → deep dive → Bencher)       | `criterion.md`                          |
| Nextest details                                                      | `nextest.md`                            |

## Resources

- `docs/testing-strategy.md` — human-facing deep dive
- `just/devops.just` — shared `_*` lifecycle recipes
- `.config/nextest.toml` — slow-timeout and retry config