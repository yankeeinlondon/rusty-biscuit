---
name: rust-testing
description: Expert guidance for testing Rust code including unit tests, integration tests, property-based testing with proptest, mocking with mockall, benchmarking with criterion, and test runners like cargo-nextest
hash: a7d02c40efcd27f4
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
```

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
| rstest | Fixtures and parameterized tests | `rstest = "0.18"` |
| pretty_assertions | Better diff output | `pretty_assertions = "1"` |
| insta | Snapshot testing | `insta = "1"` |
| testcontainers | Docker-based integration tests | `testcontainers = "0.15"` |

## Resources

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-nextest](https://nexte.st/)
- [Proptest Book](https://proptest-rs.github.io/proptest/proptest/index.html)
- [Criterion User Guide](https://bheisler.github.io/criterion.rs/book/)
