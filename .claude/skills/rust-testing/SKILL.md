---
name: rust-testing
description: |-
  Monorepo testing guide: L1/L2/L3 taxonomy, canonical just recipes,
  `require_level!` gating, nextest filtersets, and fuzzing. Load this
  before writing or reviewing tests in the rusty-biscuit workspace.
hash: 1acc7c1c76b11142-e4cbc766e6e491e6
last_updated: 2026-07-13
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
| `test-l2`      | Real-terminal tests. **Default (serial):** pre-spawns one shared pane per backend via `biscuit-harness-broker`, exports `BISCUIT_SHARED_*_ID`, runs nextest `-j 1`, tears panes down in a trap; tests `<Backend>Harness::shared_or_spawn()` attach to that pane. **Parallel self-spawn mode (`BISCUIT_L2_THREADS=N`):** skips the broker, exports no `BISCUIT_SHARED_*`, runs `-j N`; every `shared_or_spawn()` then takes its owned-pane fallback, so there is no shared resource. See "Running L2 Tests" below. |
| `test-l3`      | OS keyboard/mouse tests.                                                                                                                                                                                                                                                                                                                     |
| `test-browser` | Headless browser tests. Runs `-j 1` (one Chrome at a time); the tier gets a 5s `leak-timeout` override for Chrome teardown.                                                                                                                                                                                                                                                                                                                      |
| `test-real`    | External resource tests.                                                                                                                                                                                                                                                                                                                     |
| `lint`         | Clippy + fmt check.                                                                                                                                                                                                                                                                                                                          |
| `bench`        | Criterion benchmarks (no-op if opted out).                                                                                                                                                                                                                                                                                                   |
| `coverage`     | Per-package LCOV.                                                                                                                                                                                                                                                                                                                            |
| `doctest`      | `cargo test --doc`.                                                                                                                                                                                                                                                                                                                          |
| `fuzz`         | `cargo +nightly fuzz run` (no-op if no targets).                                                                                                                                                                                                                                                                                             |
| `all`          | `sanity → lint → doctest → test → test-l2 → test-browser`.                                                                                                                                                                                                                                                                                   |

Delegate to shared recipes in `just/devops.just` (e.g. `@just _test my-crate`).

At the repository root, `just test` delegates to `_test_workspace`. It uses
Cargo metadata as the package source of truth, runs every workspace package,
continues after ordinary failures, and reports failed packages at the end.
Ctrl+C aborts the remaining packages and preserves exit code `130`. Optional
selectors may be exact package names or package-area paths.

Run `just check-test-interrupts` to verify that every package-area `test`
recipe also preserves Ctrl+C as exit `130`.

## Running L2 Tests (read before you run)

`level2_*` tests spawn **real terminal windows / panes**. Run them **only** via
`just test-l2`, never `cargo test` / `cargo nextest run -E 'test(/level2_/)'`
directly: the recipe owns pane spawning/teardown and the serial-vs-parallel mode
choice. Bypassing it leaks windows on timeout/panic and produces ambiguous
`osascript`/PTY failures that look like — but are not — code regressions.

- A wall of single-backend failures (e.g. every `*_in_wezterm`) usually means
  that emulator is **absent/unscriptable here**, not that the renderer broke —
  confirm the same test on an available backend (`_in_kitty`, `_apple_terminal`).
- The Apple Terminal backend is GUI-automated and especially fragile (focus,
  `do script` window reuse, orphan leaks). Before touching it or debugging an
  `level2_apple_terminal_*` failure, read **`apple-terminal-harness-pitfalls.md`**.
- Spawning must **never steal foreground focus** and must **never close a window
  it did not create** — these are hard harness invariants.

### Serialization is per-*resource*, not per-*tier*

The default `-j 1` is **conservative**, not fundamental. It protects two specific
hazards, not the tier as a whole:

1. **The single shared broker pane** — every `shared_or_spawn()` test attaches to
   *one* pane per backend, so two at once would clobber it.
2. **GUI backends with global OS state** — WezTerm/Kitty window lists and focus,
   and especially **Apple Terminal's single global AppleScript state**
   (`AppleTerminalHarness` *must* stay serial).

A test that (a) spawns its **own** uniquely-named session/PTY (e.g.
`TmuxHarness::new() + spawn_shell()`, or its own `tmux new-session -s
…_{pid}_{seq}` it kills at the end) and (b) targets a **headless** backend has
**no shared resource** and is parallel-safe. Most L2 suites are dominated by such
tests and pay the `-j 1` tax purely as collateral. This is what
`BISCUIT_L2_THREADS=N` exploits: with no `BISCUIT_SHARED_*` exported,
`shared_or_spawn()` itself falls back to an **owned, `Drop`-cleaned** pane, so the
whole tier self-isolates and runs at `-j N`. claudine wires this in its
`test-l2` at `min(cores, 8)` (~6× faster); other areas keep the serial default.

**Backend parallel-safety:** **tmux** = headless, immune to the host gotcha,
cleanup reaps only dead-pid sessions → fully parallel-safe (validated).
**WezTerm** = background panes (`biscuit-bg` workspace, off-screen) coexist and
sidestep the focus/window-list race → parallel-safe (validated), but spawn cost
is high. **Kitty** = same off-screen background model (likely parallel-safe, not
yet validated). **Apple Terminal** = **serial-only** (single global AppleScript
state + focus snapshot/restore). Prefer tmux for any L2 test you want to fan out.

Note the **`available()` bar is runtime reachability, not "installed"**: WezTerm
needs `WEZTERM_UNIX_SOCKET`, Kitty needs `KITTY_LISTEN_ON` — each exported *only*
to processes that terminal launches. So GUI-backend tests run only when the suite
is launched from inside that terminal (or it is cold-started with remote control;
see the `biscuit-test-harness` skill). A clean SKIP there is the host gotcha, not
a missing app.

**Parallel-safety prerequisites for a self-isolating L2 test:** unique temp dir
per test (key on `{pid}-{nanos}-{atomic}`, never a fixed path); cleanup that
reaps only **dead-pid** resources, never live ones; and no dependence on shared
pane geometry/state.

**Flakiness under parallel load** concentrates in timing-sensitive tests (signal
delivery, interactive choosers). `retries = 3` is the sanctioned backstop. Avoid
the **two-phase capture race**: polling for an *intermediate* marker (a chooser
hint) and then taking a *separate* `capture()` for the *final* content can grab a
half-painted frame under load — poll for the content you will assert on, in the
same loop, before capturing. Areas running parallel L2 also want a 1 s
`leak-timeout` override (see Leaked Process Detection) for concurrent child
teardown.

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
   - **Browser-tier override.** `test(/browser_/)` raises `leak-timeout` to
     `5s`. Headless Chrome's helper/crashpad processes inherit the test's
     stdout and need longer than 100ms to reap; without the grace they trip
     spurious `LEAK-FAIL`s even though the test exits cleanly. `result = "fail"`
     is kept so a genuinely runaway browser still fails. The tier also runs
     `-j 1` (see below) so only one Chrome tears down at a time —
     `#[serial(browser)]` cannot serialize them under nextest's
     process-per-test model.
   - **Parallel-L2 grace (generalizable).** Any tier run at `-j N` whose tests
     spawn short-lived children (tmux/PTY/git) wants the same treatment: their
     concurrent teardown lags the 100ms pipe-drain check and trips spurious
     `LEAK-FAIL`s on whichever test loses the race. The fix is a scoped 1s grace,
     e.g. `filter = 'package(claudine-cli) & test(/level2_/)'` with
     `leak-timeout = { period = "1s", result = "fail" }` (same shape as the
     `worktree` and `tts-audio` groups). Keep `result = "fail"` — the grace only
     widens the wait window, it does not stop catching a genuine leak.
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

### Headless and focus-isolation invariant

The Browser tier is always headless. A `browser_*` test must not create or
activate a visible window, request foreground focus, move the host pointer, or
inject host OS input. Do not use headed-browser flags, application activation,
`osascript`, `cliclick`, `xdotool`, `SendInput`, or equivalent desktop
automation in this tier. Browser tests must be safe to run while someone is
using the same workstation.

Drive keyboard, pointer, media-query, and viewport behavior through the
browser's automation protocol (for example CDP key/mouse dispatch and media
emulation), then assert on the live DOM, computed styles, accessibility state,
or used geometry. Browser-dispatched input is the correct evidence when the
requirement concerns browser behavior; duplicating it with Level-3 OS input
usually adds focus races and platform dependence without testing more product
logic.

Use Level 3 only when the requirement explicitly concerns the OS-to-application
input path itself. Such a test must live outside Browser-tier binaries, remain
opt-in through `RUN_LEVEL3=1`, and must not be added merely to strengthen an
HTML/CSS/DOM interaction test that headless browser automation already covers.

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
| L2 Apple Terminal pitfalls (`do script` reuse, focus-steal, **resolved:** orphan leaks, plain-text capture) | `apple-terminal-harness-pitfalls.md`    |
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
