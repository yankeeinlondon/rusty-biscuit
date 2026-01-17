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

### Essential Commands

```bash
cargo test                      # Run all tests
cargo test test_name            # Filter by name
cargo test -- --nocapture       # Show println! output
cargo nextest run               # Faster test runner
cargo nextest run -E 'test(auth)'  # Filter with expressions
cargo bench                     # Run criterion benchmarks
```

## Topics

### Test Types

- [Unit Tests](./unit-tests.md) - Testing isolated functions and private code
- [Integration Tests](./integration-tests.md) - Testing public API as external consumer
- [Documentation Tests](./doc-tests.md) - Executable examples in doc comments

### Advanced Testing

- [Property-Based Testing](./property-testing.md) - Proptest for invariant verification
- [Mocking](./mocking.md) - Mockall for isolating dependencies
- [Benchmarking](./benchmarking.md) - Criterion for performance measurement

### Tools

- [cargo-nextest](./nextest.md) - Enhanced test runner
- [Fuzz Testing](./fuzzing.md) - cargo-fuzz for security testing
- [Snapshot Testing](./snapshots.md) - Insta for complex output verification

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
