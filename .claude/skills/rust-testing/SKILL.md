---
name: rust-testing
description: Expert guidance for testing Rust — unit and integration tests, property-based testing with proptest, mocking with mockall, benchmarking with criterion, and runners like cargo-nextest. Use when writing or structuring Rust tests, adding property/mock/benchmark coverage, or choosing a test runner.
hash: 4eb80211fedd5ae8-6699cbd30e03eef2
---

# Rust Testing

Comprehensive testing patterns for Rust using the built-in framework, cargo-nextest, proptest, mockall, criterion, and related tools.

## Core Principles

- Place unit tests in `#[cfg(test)] mod tests` within the same file as the code
- Place integration tests in `tests/` directory at project root (each file is a separate crate)
- Use `use super::*;` to access private functions in unit tests
- Prefer trait-based design for mockability
- Use descriptive test names: `fn it_returns_error_for_invalid_input()`
- Structure tests with AAA pattern: Arrange, Act, Assert
- Run `cargo nextest run` instead of `cargo test` for better performance and output
- Verify the active Rust toolchain before trusting test results in multi-toolchain environments
- In this workspace, new and modified tests should prefer `#[rstest]` for fixtures and parameterization; do not bulk-migrate unrelated tests just for style consistency
- Use `test_toolkit::EnvGuard` for process environment setup/teardown and serialize those tests with `#[serial_test::serial]`
- Use `test_toolkit::trace_phase!` around meaningful setup/body/teardown boundaries when tracing would help diagnose fixture or integration-test hangs
- For HTML/CSS render output, drive a real headless browser with `chromiumoxide` and assert on **computed styles** (`getComputedStyle`), not pixel screenshots — computed-style assertions are deterministic and cross-platform stable; reserve screenshots for opt-in visual baselines. Skip cleanly when no Chrome/Chromium is found (see [Browser Render Testing](./browser-testing.md))

## Quick Reference

### Project Structure

```
my_project/
├── src/
│   └── lib.rs          # Unit tests with #[cfg(test)]
├── tests/
│   ├── common/
│   │   └── mod.rs      # Shared test utilities
│   └── integration.rs  # Integration tests (public API only)
├── benches/
│   └── bench.rs        # Criterion benchmarks
└── Cargo.toml
```

### Large CLI/TUI Layout

```
my_cli/
├── src/
│   ├── main.rs
│   ├── output.rs
│   └── tui/
│       ├── app.rs
│       ├── reducers.rs
│       └── widgets/
├── tests/
│   ├── common/
│   │   └── mod.rs      # Shared tempdir / file / git / ANSI helpers
│   ├── command_routing.rs
│   └── workflow_tests.rs
└── benches/
    └── hot_paths.rs
```

### Essential Commands

```bash
cargo test                      # Run all tests
cargo test test_name            # Filter by name
cargo test -- --nocapture       # Show println! output
cargo nextest run               # Faster test runner
cargo nextest run -E 'test(auth)'  # Filter with expressions
cargo bench                     # Run criterion benchmarks
cargo +nightly test             # Pin a newer toolchain when default cargo is too old
cargo +stable nextest run       # Pin stable explicitly when shell cargo is inconsistent
cargo nextest run -p my_crate   # Package-scoped monorepo verification
just test                       # Package-area verification when an area justfile exists
just lint                       # Package-area lint verification when an area justfile exists
```

In this workspace, the root `.config/nextest.toml` keeps package-scoped nextest as the preferred runner. The default profile treats tests as slow after 5 seconds and terminates after 3 slow periods; the CI profile treats tests as slow after 10 seconds, terminates after 2 slow periods, and writes JUnit output to `test-results.xml`.

## Toolchain Troubleshooting

In multi-toolchain environments, the unqualified `cargo` on `PATH` may not be the toolchain you think it is. Before reporting test results or acting on build failures, verify the active toolchain:

```bash
cargo --version
rustc --version
rustup toolchain list
which cargo
```

If the workspace uses Rust 2024 edition or dependencies with a newer MSRV than the default toolchain, pin the command explicitly:

```bash
cargo +stable test
cargo +1.86.0 test
cargo +nightly test
cargo +nightly nextest run
```

Use this when you see failures like:

- Cargo cannot parse `edition = "2024"`
- dependencies require a newer `rustc`
- repeated runs resolve to different Cargo/Rust versions

When you need to pin a toolchain to get reliable results, include the exact command and toolchain version in your report.

## Topics

### Test Types

- [Unit Tests](./unit-tests.md) - Testing isolated functions and private code
- [Integration Tests](./integration-tests.md) - Testing public API as external consumer
- [Documentation Tests](./doc-tests.md) - Executable examples in doc comments

### Advanced Testing

- [Property-Based Testing](./property-testing.md) - Proptest for invariant verification
- [Mocking](./mocking.md) - Mockall for isolating dependencies
- [Benchmarking](./benchmarking.md) - Criterion for performance measurement
- [CLI Output Testing](./cli-output-testing.md) - stdout/stderr, ANSI, and shell completion checks
- [TUI Testing](./tui-testing.md) - Ratatui `TestBackend` rendering and event-path tests
- [Browser Render Testing](./browser-testing.md) - Headless Chrome (chromiumoxide) computed-style assertions and screenshot inspection for HTML/CSS output

### Tools

- [cargo-nextest](./nextest.md) - Enhanced test runner
- [Fuzz Testing](./fuzzing.md) - cargo-fuzz for security testing
- [Snapshot Testing](./snapshots.md) - Insta for complex output verification
- [Snapshot Redaction](./snapshot-redaction.md) - Stable snapshots for temp paths, IDs, ANSI, and timestamps

## Common Patterns

### Basic Unit Test

```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_returns_sum() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn add_handles_negative() {
        assert_eq!(add(-1, 1), 0);
    }
}
```

### Test with Result Return

```rust
#[test]
fn parse_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_str("key=value")?;
    assert_eq!(config.get("key"), Some("value"));
    Ok(())
}
```

### Workspace Fixtures and Env Guards

For Rusty Biscuit workspace tests, prefer `rstest` for new or touched tests that need fixtures or case parameterization. Keep `#[rstest]` visually first, then stack the async/test runtime attribute when needed, and add `#[serial_test::serial]` directly on the same test when it touches process-global state such as environment variables.

Use fully qualified `#[serial_test::serial]` in migrated tests so the synchronization requirement is obvious at the call site. Do not use `rstest_reuse` for local migrations unless there is a real repeated-case matrix to share.

```rust
use rstest::{fixture, rstest};
use test_toolkit::{trace_phase, EnvGuard};

#[fixture]
fn playa_dry_run() -> EnvGuard {
    trace_phase!("setup_playa_dry_run", {
        // Safe constructor: acquires an internal mutex during creation/drop.
        EnvGuard::set_safe("PLAYA_DRY_RUN", "1")
    })
}

#[rstest]
#[tokio::test]
#[serial_test::serial]
async fn dispatch_sound_effect_action(#[from(playa_dry_run)] _dry_run: EnvGuard) {
    // arrange, act, assert
}
```

`EnvGuard` restores the previous value on drop, including restoring nested guards in stack order. It provides two API styles:

- **Safe constructors** (`set_safe`, `remove_safe`) acquire an internal mutex during creation and drop. They can be used without `#[serial_test::serial]` in test suites that do not otherwise touch the process environment. Heavy concurrent test suites should still use `#[serial_test::serial]` to avoid lock contention.
- **Unsafe constructors** (`set`, `remove`) require the caller to ensure serialization. Use these when the test is already annotated with `#[serial_test::serial]` and you want to avoid the internal lock overhead.

`trace_phase!` creates an `INFO` tracing span and returns the wrapped block result. It is intended for observable fixture or integration-test boundaries, not as decoration around every assertion.

#### `trace_phase!` and `init_test_tracing()`

`trace_phase!` emits spans at `INFO` level. The default tracing subscriber is typically `ERROR` level, so spans are invisible unless you initialize a subscriber or raise the env filter.

```rust
use test_toolkit::{init_test_tracing, trace_phase};

#[test]
fn example_with_tracing() {
    init_test_tracing(); // one-time, idempotent
    trace_phase!("setup", {
        // fixture setup here
    });
    trace_phase!("body", {
        // test body here
    });
}
```

Alternatively, run tests with `RUST_LOG=info` or `RUST_LOG=test_toolkit=info` instead of calling `init_test_tracing()`.

#### Nextest Configuration and Verification

The workspace `.config/nextest.toml` defines slow-test thresholds:

- **default profile**: Tests slower than `5s` are flagged as slow after 3 periods.
- **ci profile**: Tests slower than `10s` are flagged as slow after 2 periods, and JUnit XML is written to `test-results.xml`.

Verify the configuration is honored:

```bash
# Run the verification test
cargo nextest run --profile default -p test-toolkit --test nextest_config_verification

# Or use the justfile recipe
just verify-nextest-config
```

### Shared CLI Integration Helpers

```rust
// tests/common/mod.rs
pub struct TestWorkspace {
    root: std::path::PathBuf,
}

pub fn write(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

pub fn strip_ansi(input: &str) -> String {
    biscuit_terminal::prelude::strip_escape_codes(input)
}
```

```rust
// tests/command_routing.rs
mod common;

use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn command_writes_machine_output_to_stdout() {
    let output = cargo_bin_cmd!("my-cli")
        .args(["completions", "bash"])
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(!output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
```

### Ratatui Render Test

```rust
use insta::assert_debug_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn widget_render_matches_snapshot() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| render(frame, frame.area(), &app)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    assert_debug_snapshot!(buffer);
}
```

### ANSI / Plain Output Assertions

```rust
let output = cargo_bin_cmd!("my-cli")
    .env("NO_COLOR", "1")
    .args(["providers"])
    .assert()
    .success()
    .get_output()
    .clone();

let stdout = String::from_utf8(output.stdout).unwrap();
assert_eq!(stdout, strip_ansi(&stdout));
```

Use `FORCE_COLOR=1` when you need styled output in non-TTY integration tests, and `--plain` when the CLI exposes an explicit no-ANSI mode that should override env-based color forcing.

### Headless Browser Render Test (chromiumoxide)

Assert on what a real browser *computes* from your HTML/CSS, not on source
substrings. Wrap a render fragment into a standalone document, load it over a
`file://` URL, and read `getComputedStyle`. See [Browser Render Testing](./browser-testing.md)
for the `find_chrome` locator, the screenshot path, and skip-clean details.

```rust
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::StreamExt;

#[tokio::test]
#[serial_test::serial(browser)]
async fn code_block_background_computes() {
    let Some(chrome) = find_chrome() else { return; }; // skip if no browser

    // Wrap the render fragment in a full document with a page background.
    let doc = format!(
        "<!doctype html><html><body style=\"background:#202020\">{}</body></html>",
        render_html_fragment(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("page.html");
    std::fs::write(&path, doc).unwrap();

    let config = BrowserConfig::builder()
        .chrome_executable(chrome)
        .arg("--no-sandbox")
        .build()
        .unwrap();
    let (browser, mut handler) = Browser::launch(config).await.unwrap();
    // The handler MUST be polled or no CDP traffic flows.
    let pump = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let page = browser.new_page(format!("file://{}", path.display())).await.unwrap();
    page.wait_for_navigation().await.unwrap();
    let bg: String = page
        .evaluate(
            "getComputedStyle(document.querySelector('.code-block')).backgroundColor",
        )
        .await
        .unwrap()
        .into_value()
        .unwrap();

    let mut browser = browser;
    browser.close().await.ok();
    pump.abort();

    assert_eq!(bg, "rgb(17, 27, 39)"); // browser-computed, not source-matched
}
```

### Expected Panic

```rust
#[test]
#[should_panic(expected = "index out of bounds")]
fn panics_on_invalid_index() {
    let v = vec![1, 2, 3];
    let _ = v[10];
}
```

## Key Crates

| Crate | Purpose | Cargo.toml |
|-------|---------|------------|
| proptest | Property-based testing | `proptest = "1"` |
| mockall | Mock generation | `mockall = "0.13"` |
| criterion | Benchmarking | `criterion = "0.5"` |
| rstest | Fixtures and parameterized tests | `rstest = "0.25"` |
| pretty_assertions | Better diff output | `pretty_assertions = "1"` |
| insta | Snapshot testing | `insta = "1"` |
| testcontainers | Docker-based integration tests | `testcontainers = "0.15"` |
| chromiumoxide | Headless-browser (CDP) HTML/CSS render tests | `chromiumoxide = { version = "0.7", default-features = false, features = ["tokio-runtime"] }` |

## Resources

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-nextest](https://nexte.st/)
- [Proptest Book](https://proptest-rs.github.io/proptest/proptest/index.html)
- [Criterion User Guide](https://bheisler.github.io/criterion.rs/book/)
