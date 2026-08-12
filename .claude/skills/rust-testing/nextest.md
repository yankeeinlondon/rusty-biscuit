# cargo-nextest

Nextest is a faster, more powerful test runner for Rust projects.

## Installation

```bash
cargo install cargo-nextest
```

## Basic Usage

```bash
# Run all tests
cargo nextest run

# Run with more parallelism
cargo nextest run -j 8

# List tests without running
cargo nextest list

# Run specific test
cargo nextest run test_name

# Run tests in specific package
cargo nextest run -p my_crate
```

In monorepos, prefer package-scoped verification while iterating on one area:

```bash
cargo nextest run -p my_lib
cargo nextest run -p my_cli
```

## Advantages Over cargo test

| Feature | cargo test | cargo nextest |
|---------|------------|---------------|
| Execution model | Single process per test binary | Separate process per test |
| Speed on multi-core | Good | Up to 3x faster |
| Flaky test handling | Manual | Built-in retries |
| Output | Mixed stdout | Clean, structured |
| Test isolation | Shared process | Full isolation |
| CI features | Basic | JUnit, partitioning |

## Filtering Tests

### By Name

```bash
# Tests containing "auth"
cargo nextest run auth

# Tests starting with "test_user"
cargo nextest run 'test_user*'
```

### With Expressions

```bash
# Tests in a specific package
cargo nextest run -E 'package(my_crate)'

# Tests matching a pattern
cargo nextest run -E 'test(/auth/)'

# Binary tests only (no doc tests)
cargo nextest run -E 'kind(test)'

# Combine expressions
cargo nextest run -E 'package(core) & test(/validation/)'
```

## Configuration

Create `.config/nextest.toml` in your project:

```toml
[profile.default]
retries = 0
test-threads = "num-cpus"
fail-fast = true
slow-timeout = { period = "60s", terminate-after = 2 }

[profile.ci]
retries = 2
test-threads = 4
fail-fast = false

[profile.ci.junit]
path = "target/nextest/ci/junit.xml"
```

Use profiles:

```bash
cargo nextest run --profile ci
```

## Handling Flaky Tests

```toml
# .config/nextest.toml
[profile.default]
retries = 2  # Retry failed tests up to 2 times

# Mark specific tests as flaky
[[profile.default.overrides]]
filter = "test(/flaky/)"
retries = 3
```

For PTY or timing-sensitive smoke tests, prefer `#[ignore]` plus a manual command such as `cargo test --test pty_tests -- --ignored` instead of hiding instability behind aggressive retries in the default profile.

## Slow Test Detection

```toml
[profile.default]
slow-timeout = { period = "30s" }  # Warn after 30s

# Terminate very slow tests
slow-timeout = { period = "60s", terminate-after = 2 }
```

## CI Integration

### GitHub Actions

```yaml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install nextest
        uses: taiki-e/install-action@nextest

      - name: Run tests
        run: cargo nextest run --profile ci

      - name: Upload test results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: test-results
          path: target/nextest/ci/junit.xml
```

### Test Partitioning

Split tests across CI jobs:

```yaml
jobs:
  test:
    strategy:
      matrix:
        partition: [1, 2, 3, 4]
    steps:
      - name: Run tests (partition ${{ matrix.partition }}/4)
        run: cargo nextest run --partition count:${{ matrix.partition }}/4
```

## Test Archives

Create portable test archives for remote execution:

```bash
# Create archive
cargo nextest archive --archive-file tests.tar.zst

# Run from archive (on different machine)
cargo nextest run --archive-file tests.tar.zst
```

The `wsl2-ubuntu` leg of every area's CI runs this way (`.github/workflows/_wsl-ci.yml`):
the archive is built on `ubuntu-latest` and only executed inside the guest.

### Never bake a binary path with `env!`

A test that spawns a sibling binary must read its path from the environment at
**run time**. `env!("CARGO_BIN_EXE_<name>")` resolves at compile time to a path
inside the *build* host's target directory; nextest extracts an archive into a
temp directory, so that path does not exist and every such test dies with
`Os { code: 2, kind: NotFound }` — green locally, red on `wsl2-ubuntu` only.

Use `biscuit_test_harness::bin_exe!`, which prefers nextest's run-time
republication of the path and falls back to the compile-time value:

```rust
use biscuit_test_harness::bin_exe;

let output = Command::new(bin_exe!("so-you-say")).arg("--help").output()?;
```

It returns a `PathBuf`; suites that interpolate the path into a shell command
line wrap it once in a `OnceLock<String>` helper (see
`claudine/cli/tests/common/mod.rs`'s `claudine_bin`).

`--workspace-remap` does not help here: it relocates *source* paths, not the
build's target directory.

### Examples are built but not archived

An archive carries test binaries and non-test **bin** targets. A test that
spawns an *example* (biscuit-terminal's `discovery_probe`) finds nothing in the
guest unless the example is named in `archive.include` in `.config/nextest.toml`
— which is per-path, has no glob support, and needs one entry per target-dir
layout (`debug/…` locally, `<triple>/debug/…` when CI builds with `--target`).

## Output Formats

```bash
# Default (human-readable)
cargo nextest run

# JSON for tooling
cargo nextest run --message-format json

# JUnit XML for CI
cargo nextest run --profile ci  # If junit configured in profile
```

## Heavy Tests

Mark resource-intensive tests:

```toml
# .config/nextest.toml
[[profile.default.overrides]]
filter = "test(/integration/)"
threads-required = 2  # Reserve 2 slots for this test
```

## Serial Tests

Force sequential execution:

```toml
[[profile.default.overrides]]
filter = "test(/database/)"
test-threads = 1
```

## Common Commands

```bash
# Show what would run
cargo nextest list

# Run and show all output
cargo nextest run --no-capture

# Run only failed tests from last run
cargo nextest run --run-ignored

# Generate machine-readable output
cargo nextest run --message-format json > results.json

# Monorepo-focused commands
cargo nextest run -p my_crate --fail-fast
cargo nextest run -p my_cli -E 'test(/routing|completions/)'
```

## Limitations

- Doc tests run via `cargo test --doc` (nextest focuses on binary tests)
- Some `cargo test` flags not supported

```bash
# Full test suite including doc tests
cargo nextest run && cargo test --doc
```

## Related

- [Unit Tests](./unit-tests.md) - Writing tests
- [Integration Tests](./integration-tests.md) - Testing public API
- [Benchmarking](./criterion.md) - Performance testing
