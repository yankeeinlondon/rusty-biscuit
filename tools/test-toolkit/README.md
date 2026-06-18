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
