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

For PTY or timing-sensitive smoke tests, prefer `#[ignore]` plus a manual command such as `cargo nextest run --test l1 pty_tests:: --run-ignored only` instead of hiding instability behind aggressive retries in the default profile.

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

## Reusing CI Validation

In this monorepo, PR CI always validates the affected packages. A subsequent
push to `main` may reuse that successful run only when its recorded Git tree
and integration base match exactly. `scripts/ci/reuse_validation.py` verifies
the PR association, latest run result, and unexpired receipt before skipping
the grid; missing evidence falls back to normal CI. The required `ci-gate`
check folds the skipped jobs and passes, the advisory summary links the
original validation, and the run's conclusion still gates release automation.
Use `workflow_dispatch` to force a fresh full-grid run. See
`.github/ci/README.md` for the receipt contract and local verification commands.

## Test Archives

Create portable test archives for remote execution:

```bash
# Create archive
cargo nextest archive --archive-file tests.tar.zst

# Run from archive (on different machine)
cargo nextest run --archive-file tests.tar.zst
```

**Every** hosted test cell runs this way, not just WSL2. One native owner per
planned build key produces the archive; L1, L2, browser, and the `wsl2-ubuntu`
guest verify and execute it with no Cargo, rustc, or linker in reach
(`fixes/2026-09-12-single-os-compile`, `.github/ci/README.md`). Linux owns both
its own cells and the guest's; macOS and native Windows own theirs.

Two consequences for a test author:

- **A test that reaches a compiler fails in CI, on every OS.** It is no longer
  only the WSL2 guest that lacks one.
- **Extraction is not free and is measured.** `cargo nextest run
  --archive-file` extracts inside the run, so CI reports the extraction window
  from `ci-build verify`'s own `--extract-to` instead, and a cell's download,
  verification, extraction, and execution appear as four separate numbers. A
  suite that slows down because its archive grew shows up in `extract_ms`, not
  in test time.

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

### A fixture path has the same trap, one level up

`env!("CARGO_MANIFEST_DIR")` is also the build host's checkout. `--workspace-remap`
**does** help here — it rewrites the *run-time* `CARGO_MANIFEST_DIR` to the
consumer's checkout — but only for code that reads the variable at run time.
`biscuit_test_harness::manifest_dir!()` is the counterpart of `bin_exe!`:

```rust
let fixture = biscuit_test_harness::manifest_dir!().join("tests/fixtures/sample.md");
```

### Examples, dynamic libraries, and other unarchived outputs

An archive carries test binaries, non-test **bin** targets, build-script output
directories, and linked paths. It does **not** carry an example (biscuit-terminal's
`discovery_probe`) or a workspace `dylib`; a test that needs one finds nothing in
the guest.

Declare them in the owning package's `[package.metadata.ci.tests]
archive-includes`, relative to the profile output directory, with `{DLL_PREFIX}`,
`{DLL_SUFFIX}`, and `{EXE_SUFFIX}` covering the three producers' spellings.
`ci-build produce` supplies the `<triple>/<profile>` prefix and generates the
nextest config. Writing `archive.include` into `.config/nextest.toml` by hand
still works but is per-path, has no glob support, and needs one entry per
target-dir layout (`debug/…` locally, `<triple>/debug/…` when CI builds with
`--target`) — which is exactly how `discovery_probe` was silently missing from
the macOS and Windows archives until it became a declared include.

An include only **copies** what the build produced, and `cargo nextest archive`
never builds an example. `ci-build produce` therefore builds every declared
`examples/<name>` entry before archiving; a hand-written `archive.include` for
an example has no such step behind it, and `on-missing = "ignore"` turns the
absence into a test that panics on a missing file instead of a build that
failed.

A binary a test *spawns* but does not link — another package's compile-time tool
— is a **build sidecar** instead: name it in `sidecars`, from the closed
vocabulary in `.github/ci/sidecars.json`.

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
