# test-toolkit

Shared test lifecycle helpers for the Rusty Biscuit workspace.

## What it provides

This crate solves three common pain points when writing tests in a Rust workspace:

1. **Tracing output in tests** — `trace_phase!` macro and `init_test_tracing()` to emit structured spans around setup/body/teardown phases.
2. **Safe environment variable mutation** — `EnvGuard` RAII guard that restores env vars after test completion, even when tests panic.
3. **Nextest integration** — Works out of the box with the workspace `.config/nextest.toml` for slow-test detection and JUnit reporting.

## Usage

Add `test-toolkit` as a dev-dependency:

```toml
[dev-dependencies]
test-toolkit = { path = "../../tools/test-toolkit" }
```

## API

### `trace_phase!` macro

Wrap a code block in a named tracing span so you can see setup vs body vs teardown timing in test output.

```rust
use test_toolkit::{init_test_tracing, trace_phase};

#[test]
fn my_test() {
    init_test_tracing();

    let db = trace_phase!("setup", {
        Database::connect()
    });

    trace_phase!("body", {
        db.query("SELECT 1").unwrap();
    });

    trace_phase!("teardown", {
        db.close();
    });
}
```

Spans are created at `INFO` level. If you do not see output, call `init_test_tracing()` at the start of your test binary or set `RUST_LOG=info`.

### `EnvGuard` — safe environment variable mutation

Temporarily set or remove an environment variable. The original value is restored when the guard is dropped, even if the test panics.

**Safe constructors** (preferred for most tests):

```rust
use test_toolkit::EnvGuard;

#[test]
fn env_test() {
    let _guard = EnvGuard::set_safe("MY_API_KEY", "test-key");

    // variable is "test-key" here
    assert_eq!(std::env::var("MY_API_KEY").unwrap(), "test-key");

    // automatically restored to previous value (or removed if unset) at end of scope
}
```

**Unsafe constructors** (use with `#[serial_test::serial]`):

```rust
use test_toolkit::EnvGuard;

#[test]
#[serial_test::serial]
fn env_test() {
    let _guard = unsafe { EnvGuard::set("MY_API_KEY", "test-key") };
    // ...
}
```

The safe variants acquire an internal mutex, so they can be used without `#[serial_test::serial]` in most cases. For heavy concurrent test suites, `#[serial_test::serial]` is still recommended.

### `init_test_tracing()`

Configure a tracing subscriber at `INFO` level so `trace_phase!` spans are visible. Multiple calls in the same test binary are idempotent.

```rust
use test_toolkit::init_test_tracing;

#[test]
fn my_test() {
    init_test_tracing();
    // trace_phase! spans will now appear in output
}
```

### `require_level!` and `Backend` — per-backend L2 enforcement

`require_level!` takes the level, an availability probe, and a description of
what the test needs. Pass a `Backend` rather than a bare string wherever the
requirement is one of the four real-terminal harnesses:

```rust
use test_toolkit::{Backend, Level, require_level};

#[test]
fn level2_renders_in_real_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
}
```

A `&str` still works and is the right choice for requirements with no backend
identity (`"PTY (/dev/ptmx)"`, `"WezTerm + cliclick"`), but such a test can
never be demanded by CI and contributes no execution evidence.

`Backend::as_str()` returns the stable identifier — `tmux`, `wezterm`, `kitty`,
`apple-terminal` — shared verbatim with `scripts/ci/affected_scope.py`'s
`KNOWN_L2_BACKENDS` and the `backends` arrays in `.github/ci/areas.json`.
`Backend::label()` is the separate human-readable name used in diagnostics, so
rewording a skip message cannot break a CI policy file.

#### `BISCUIT_TEST_REQUIRED_BACKENDS`

`BISCUIT_TEST_LEVEL_REQUIRED=2` turns every L2 skip into a panic, which is
unusable on a headless runner that can host tmux but not a GUI emulator.
`BISCUIT_TEST_REQUIRED_BACKENDS` scopes the same force to named backends:

```bash
BISCUIT_TEST_REQUIRED_BACKENDS=tmux            # one
BISCUIT_TEST_REQUIRED_BACKENDS=tmux,wezterm    # several
```

- a listed backend that is unavailable → panic
- an unlisted backend that is unavailable → clean skip, as before
- entries are trimmed and lowercased, and matched **exactly**: `tmux2` and
  `wez` are errors, not near-misses
- an unknown or empty entry panics even when the harness is present, because a
  silently dropped requirement can never fail again
- unset or blank means no requirement; it composes with, and does not replace,
  `BISCUIT_TEST_LEVEL_REQUIRED`

#### `backend-proof` binary — execution evidence

An installed `tmux` plus zero tmux tests is not evidence. While
`BISCUIT_TEST_REQUIRED_BACKENDS` is set, every gate decision is appended to
`backend-executions.jsonl` in the staging directory (`$BISCUIT_JUNIT_STAGE_DIR`,
else `target/nextest/ci-reports` under the workspace root), one JSON object per
line:

```json
{"backend":"tmux","test":"level2_dirty_tree::level2_dirty_tree_renders_in_tmux","decision":"run"}
```

`decision` is `run`, `skip`, or `panic`. Each line is a single `O_APPEND` write,
so the concurrent test processes nextest spawns cannot interleave. Recording is
a no-op when no backend is required, so local development pays nothing.

`backend-proof` then asserts that each required backend produced at least one
`run`:

```bash
cargo run -p test-toolkit --features backend-proof --bin backend-proof -- reset
just test-l2 <area>
cargo run -p test-toolkit --features backend-proof --bin backend-proof -- verify
```

`reset` must precede the run, or stale evidence satisfies the check. Exit codes:
`0` proved (or nothing required), `1` a required backend executed no test, `2`
bad configuration or unreadable evidence. `--stage-dir` and `--required`
override the environment.

### `leak-sweep` binary — post-run orphan detector

A cross-platform (macOS / Windows / Linux) helper that runs a command, then
reports any **child process that outlived it**. It snapshots the live process
set before and after the run and flags new survivors whose executable or command
line points inside the workspace root.

This complements nextest's per-test `LEAK` status: `LEAK` only catches children
still holding a test's stdout/stderr pipes, whereas this sweep also catches
detached orphans that closed or redirected those handles. Attribution is by
workspace path (not parent PID), because orphan reparenting is OS-specific.

It is feature-gated so library consumers do not inherit `clap`/`sysinfo`:

```bash
# wrap any command
cargo run -p test-toolkit --features leak-sweep --bin leak-sweep -- cargo nextest run

# or, from the repo root, wrap the whole test run:
just test-leaks            # all areas
just test-leaks claudine   # specific areas
```

Exit codes: the wrapped command's status, or `99` when the command succeeded but
leaked processes were found. Pass `--warn-only` to report without failing,
`--root <path>` to override the attribution root, and `--settle-ms <n>` to tune
the grace period before the final snapshot.

## Running tests

Use `cargo test` for standard test execution, or `cargo nextest run` for parallel, profile-aware test execution:

```bash
# Standard test run
cargo test -p test-toolkit

# Nextest run (picks up .config/nextest.toml)
cargo nextest run -p test-toolkit
```

## Nextest configuration

The workspace `.config/nextest.toml` defines slow-test thresholds:

- **default profile**: Tests slower than `5s` are flagged as slow after 3 periods.
- **ci profile**: Tests slower than `10s` are flagged as slow after 2 periods, and JUnit XML is written to `test-results.xml`.

## Verification

To verify that nextest picks up the slow-timeout configuration, run the `nextest_config_verification` integration test and check that the 6-second test is flagged as slow:

```bash
cargo nextest run --profile default -p test-toolkit --test nextest_config_verification 2>&1 | grep -i slow
```

Or use the justfile recipe:

```bash
just verify-nextest-config
```
