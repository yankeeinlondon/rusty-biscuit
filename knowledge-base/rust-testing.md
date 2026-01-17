---
name: rust-testing
description: Comprehensive guide to testing in Rust covering unit tests, integration tests, property-based testing, benchmarking, fuzzing, and CI integration
created: 2025-12-08
hash: 0f9e54ed12d86aad
tags:
  - rust
  - testing
  - cargo
  - nextest
  - proptest
  - criterion
  - mockall
---

# Testing in Rust

Rust has a **built-in testing framework** that provides comprehensive support for unit tests, integration tests, and documentation tests right out of the box. The language's design principles of reliability and correctness are reflected in its testing capabilities. No external dependencies are required for basic testing functionality, making it immediately accessible through Cargo.

This guide covers:

- **Test Types**: unit, integration, documentation, and property-based tests
- **Test Runners**: `cargo test` vs `cargo nextest`
- **Best Practices**: organization, naming, mocking, and TDD
- **Advanced Techniques**: property-based testing, benchmarking, fuzzing, coverage, snapshot testing
- **CI/CD Integration**: wiring tests into pipelines

## Table of Contents

- [Testing in Rust](#testing-in-rust)
  - [Table of Contents](#table-of-contents)
  - [The Rust Testing Ecosystem](#the-rust-testing-ecosystem)
    - [Ecosystem Overview](#ecosystem-overview)
  - [Types of Tests](#types-of-tests)
    - [Unit Tests](#unit-tests)
      - [Testing for Panics](#testing-for-panics)
      - [Returning Results from Tests](#returning-results-from-tests)
    - [Integration Tests](#integration-tests)
      - [Project Structure](#project-structure)
      - [Simple Integration Test](#simple-integration-test)
      - [Shared Test Utilities](#shared-test-utilities)
    - [Documentation Tests](#documentation-tests)
      - [Hiding Setup Code](#hiding-setup-code)
    - [Property-Based Tests](#property-based-tests)
  - [Test Runners](#test-runners)
    - [cargo test: The Default Runner](#cargo-test-the-default-runner)
    - [cargo nextest: Next-Generation Runner](#cargo-nextest-next-generation-runner)
      - [Comparison](#comparison)
  - [Best Practices](#best-practices)
    - [Test Organization](#test-organization)
      - [Recommended Project Layout](#recommended-project-layout)
    - [Naming Conventions](#naming-conventions)
    - [Test-Driven Development in Rust](#test-driven-development-in-rust)
    - [Mocking and Test Doubles](#mocking-and-test-doubles)
  - [Advanced Testing Techniques](#advanced-testing-techniques)
    - [Property-Based Testing with Proptest](#property-based-testing-with-proptest)
      - [Constrained Generators](#constrained-generators)
      - [Algebraic Properties](#algebraic-properties)
    - [Benchmarking with Criterion](#benchmarking-with-criterion)
      - [Benchmarking Tools Comparison](#benchmarking-tools-comparison)
    - [Fuzz Testing](#fuzz-testing)
    - [Snapshot Testing with Insta](#snapshot-testing-with-insta)
    - [Test Coverage](#test-coverage)
  - [CI/CD Integration](#cicd-integration)
    - [GitHub Actions Example](#github-actions-example)
    - [With Nextest](#with-nextest)
    - [Extended Pipeline](#extended-pipeline)
  - [Capstone Example](#capstone-example)
    - [Library Code (src/lib.rs)](#library-code-srclibrs)
    - [Integration Test (tests/calculator\_integration.rs)](#integration-test-testscalculator_integrationrs)
  - [Quick Reference](#quick-reference)
    - [Essential Commands](#essential-commands)
    - [Test Attributes](#test-attributes)
    - [Assertion Macros](#assertion-macros)
    - [Testing Crates](#testing-crates)
  - [Resources](#resources)
  - [Recommendations Summary](#recommendations-summary)

---

## The Rust Testing Ecosystem

Rust ships with a **built-in test harness**:

- `#[test]` attribute to mark tests
- Assertion macros: `assert!`, `assert_eq!`, `assert_ne!`
- `cargo test` to build and run test binaries
- `rustdoc` for documentation tests

### Ecosystem Overview

| Category | Tools | Purpose |
|:---------|:------|:--------|
| **Built-in** | `cargo test` | Unit, integration, and doc tests |
| **Test Runners** | `cargo-nextest` | Faster execution, better CI integration |
| **Property Testing** | `proptest`, `quickcheck` | Random input generation, shrinking |
| **Mocking** | `mockall`, `mockito` | Trait-based mocks, HTTP mocking |
| **Benchmarking** | `criterion`, `divan` | Statistical micro-benchmarks |
| **Fuzzing** | `cargo-fuzz` | libFuzzer-based vulnerability testing |
| **Snapshot Testing** | `insta` | Golden file comparisons |
| **Coverage** | `cargo-llvm-cov`, `grcov` | Test coverage reports |
| **Fixtures** | `rstest` | Parameterized tests, fixtures |
| **Assertions** | `pretty_assertions` | Colorful diff output |
| **Containers** | `testcontainers` | Docker-based integration tests |

---

## Types of Tests

### Unit Tests

Unit tests target **small, focused pieces of functionality** within a single module. They live in the same file as the code being tested, within a `#[cfg(test)]` module.

```rust
pub fn add_two(a: i32) -> i32 {
    internal_adder(a, 2)
}

fn internal_adder(left: i32, right: i32) -> i32 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_adder_adds_correctly() {
        let result = internal_adder(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn add_two_works() {
        let result = add_two(2);
        assert_eq!(result, 4);
    }
}
```

Key points:

- Tests can access **private functions** via `use super::*`
- The `#[cfg(test)]` attribute ensures test code is only compiled during testing
- Unit tests are fast and tightly coupled to implementation

#### Testing for Panics

```rust
#[test]
#[should_panic(expected = "Index out of bounds")]
fn test_panic_on_invalid_index() {
    let vec = vec![1, 2, 3];
    vec[99];
}
```

#### Returning Results from Tests

Tests can return `Result<(), E>` to use the `?` operator:

```rust
#[test]
fn test_with_result() -> Result<(), String> {
    let result = some_fallible_operation()?;
    assert_eq!(result, expected_value);
    Ok(())
}
```

---

### Integration Tests

Integration tests validate your **public API** from the outside, the same way a consumer would use your crate:

- They live in a top-level `tests/` directory (sibling to `src/`)
- Each `.rs` file is compiled as a **separate crate**
- They can only access public items

#### Project Structure

```
my_project/
├── src/
│   ├── lib.rs
│   └── main.rs
├── tests/
│   ├── common/
│   │   └── mod.rs      # Shared test utilities
│   ├── api_smoke.rs
│   └── integration_flows.rs
└── Cargo.toml
```

#### Simple Integration Test

```rust
// tests/integration_add.rs
use my_project::add_two;

#[test]
fn add_two_handles_basic_inputs() {
    assert_eq!(add_two(2), 4);
    assert_eq!(add_two(0), 2);
    assert_eq!(add_two(-2), 0);
}
```

#### Shared Test Utilities

To share setup code without Cargo treating it as a test file, place helpers in a subdirectory:

```rust
// tests/common/mod.rs
pub fn setup() {
    println!("Setting up integration test environment");
}

// tests/integration_with_setup.rs
mod common;
use my_project::add_two;

#[test]
fn add_two_with_setup() {
    common::setup();
    assert_eq!(add_two(10), 12);
}
```

---

### Documentation Tests

Documentation tests are executable code examples embedded in doc comments. They serve as both documentation and tests:

```rust
/// Adds two to a number.
///
/// # Examples
///
/// ```
/// use my_project::add_two;
///
/// let result = add_two(2);
/// assert_eq!(result, 4);
/// ```
pub fn add_two(a: i32) -> i32 {
    a + 2
}
```

#### Hiding Setup Code

Use lines starting with `#` to hide boilerplate from rendered docs while still running it in tests:

```rust
/// # Examples
///
/// ```
/// # use my_project::add_two;
/// # let mut x = 5;
/// x = add_two(x);
/// assert_eq!(x, 7);
/// ```
```

Users see clean examples; the compiler runs the full code path.

---

### Property-Based Tests

Property-based testing verifies that **properties hold for many randomly generated inputs** rather than specific examples. When a test fails, the framework automatically **shrinks** the input to a minimal counterexample.

Popular frameworks: **Proptest** and **QuickCheck**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn reversing_twice_returns_original(xs: Vec<i32>) {
        let rev_rev: Vec<i32> = xs.iter().cloned().rev().rev().collect();
        prop_assert_eq!(xs, rev_rev);
    }

    #[test]
    fn sort_preserves_length(mut xs: Vec<i32>) {
        let len_before = xs.len();
        xs.sort();
        prop_assert_eq!(xs.len(), len_before);
    }
}
```

When to use property-based tests:

- Algorithms with algebraic laws (commutativity, associativity, identity)
- Parsers and serializers (`decode(encode(x)) == x`)
- Data structure invariants (heap properties, tree balancing)

---

## Test Runners

### cargo test: The Default Runner

`cargo test` builds test binaries with the harness enabled and runs all discovered tests in parallel:

```bash
# Run all tests (unit, integration, doctests)
cargo test

# Run tests whose names contain "add_two"
cargo test add_two

# Show output for passing tests
cargo test -- --nocapture

# Run tests in a single thread
cargo test -- --test-threads=1

# Run only doc tests
cargo test --doc

# Run only integration tests
cargo test --test integration_add
```

Flags after `--` are passed to the test binary, not Cargo.

---

### cargo nextest: Next-Generation Runner

For larger projects, **nextest** provides significant improvements:

- **Faster execution**, especially on large suites
- Runs tests as **individual processes** for better isolation
- Per-test **timeouts and retries**
- **CI-friendly** features: JUnit reports, test partitioning, archiving

```bash
# Install nextest
cargo install cargo-nextest

# Run all tests
cargo nextest run

# List all tests without running them
cargo nextest list

# Use a specific profile (configured in .config/nextest.toml)
cargo nextest run --profile ci
```

#### Comparison

| Feature | cargo test | cargo nextest |
|:--------|:-----------|:--------------|
| Execution model | Single test binary | Per-test process |
| Speed on large suites | Good | Often significantly faster |
| Test selection | Name filters | Rich expression-based filters |
| Flakiness handling | Manual | Retries, per-test timeouts |
| CI integration | Basic | JUnit, partitioning, archiving |
| Configuration | Limited | Profiles + TOML config |

For small-medium projects, `cargo test` is sufficient. For large monorepos and CI-heavy workflows, nextest is worth serious consideration.

---

## Best Practices

### Test Organization

#### Recommended Project Layout

```
my_project/
├── src/
│   ├── lib.rs          # Library code + unit tests
│   └── foo.rs          # More modules with inline tests
├── tests/
│   ├── common/
│   │   └── mod.rs      # Shared integration helpers
│   ├── api_smoke.rs    # Public API checks
│   └── flows.rs        # End-to-end scenarios
├── benches/
│   └── benchmarks.rs   # Criterion benchmarks
└── Cargo.toml
```

Guidelines:

| Test Type | Location | Purpose |
|:----------|:---------|:--------|
| **Unit tests** | Same file, `#[cfg(test)]` module | Test internal logic, private functions |
| **Integration tests** | `tests/` directory | Test public API, real-world scenarios |
| **Doc tests** | Doc comments | Document and verify usage examples |
| **Benchmarks** | `benches/` directory | Measure performance |

---

### Naming Conventions

Use descriptive names that indicate what is being tested:

**Behavior-oriented:**

```rust
#[test]
fn add_two_returns_sum_plus_two() { /* ... */ }

#[test]
fn user_creation_fails_on_duplicate_email() { /* ... */ }
```

**"it ..." style:**

```rust
#[test]
fn it_adds_two_to_the_input() { /* ... */ }

#[test]
fn it_rejects_duplicate_emails() { /* ... */ }
```

**Group related tests into submodules:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod user_creation {
        use super::*;

        #[test]
        fn it_creates_user_with_valid_data() { /* ... */ }

        #[test]
        fn it_rejects_duplicate_email() { /* ... */ }
    }

    mod authentication {
        use super::*;

        #[test]
        fn it_authenticates_with_valid_credentials() { /* ... */ }
    }
}
```

---

### Test-Driven Development in Rust

Rust's tooling is friendly to TDD:

1. **Write a failing test** that defines the expected behavior
2. **Implement minimal code** to make the test pass
3. **Refactor** while keeping tests green
4. **Repeat** for additional functionality

```rust
// Step 1: Write failing test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_errors_on_empty_slice() {
        let result = mean(&[]);
        assert!(result.is_err());
    }
}

// Step 2: Implement minimal code
pub fn mean(xs: &[f64]) -> Result<f64, &'static str> {
    if xs.is_empty() {
        return Err("empty slice");
    }
    Ok(xs.iter().sum::<f64>() / xs.len() as f64)
}
```

---

### Mocking and Test Doubles

Rust encourages **trait-based design** which makes mocking straightforward:

1. Extract dependencies behind a trait
2. Implement the trait for production types
3. Use `mockall` to generate mocks in tests

```rust
use mockall::automock;

#[derive(Debug)]
pub struct FetchError;

#[automock]
pub trait Fetcher {
    fn fetch(&self, url: &str) -> Result<String, FetchError>;
}

pub fn process_data<F: Fetcher>(fetcher: &F) -> String {
    let data = fetcher.fetch("https://example.com/data").unwrap();
    format!("processed: {data}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::eq;

    #[test]
    fn process_data_formats_response() {
        let mut fetcher = MockFetcher::new();

        fetcher
            .expect_fetch()
            .with(eq("https://example.com/data"))
            .times(1)
            .returning(|_| Ok("test data".into()));

        let result = process_data(&fetcher);
        assert_eq!(result, "processed: test data");
    }
}
```

For HTTP clients specifically, **mockito** can stand up a fake HTTP server instead of mocking traits.

---

## Advanced Testing Techniques

### Property-Based Testing with Proptest

Beyond basic properties, proptest supports:

#### Constrained Generators

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn binary_search_finds_element_if_ok(
        mut data in proptest::collection::vec(0..1000_i32, 1..100),
        element in 0..1000_i32,
    ) {
        data.sort();
        if let Ok(idx) = data.binary_search(&element) {
            prop_assert_eq!(data[idx], element);
        }
    }
}
```

#### Algebraic Properties

```rust
proptest! {
    #[test]
    fn add_is_commutative(a: i32, b: i32) {
        let calc = Calculator::default();
        prop_assert_eq!(calc.add(a, b), calc.add(b, a));
    }

    #[test]
    fn add_has_zero_identity(a: i32) {
        let calc = Calculator::default();
        prop_assert_eq!(calc.add(a, 0), a);
        prop_assert_eq!(calc.add(0, a), a);
    }
}
```

---

### Benchmarking with Criterion

Rust's older `#[bench]` is nightly-only. **Criterion** is the stable, de-facto standard:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| {
        b.iter(|| fibonacci(black_box(20)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

Run with:

```bash
cargo bench
```

Notes:

- `black_box` prevents compiler optimizations from eliminating the call
- Criterion provides statistical analysis, outlier detection, and trend tracking

#### Benchmarking Tools Comparison

| Tool | Stability | Best For | Key Features |
|:-----|:----------|:---------|:-------------|
| **Built-in `#[bench]`** | Nightly only | Quick microbenchmarks | Part of standard library |
| **Criterion** | Stable | Statistical analysis | Trend detection, detailed reports |
| **Divan** | Stable | Comparative benchmarking | Cache-disciplined measurements |
| **Hyperfine** | External tool | Command-line programs | Cross-language, statistical rigor |

---

### Fuzz Testing

Fuzzing throws **random, often malformed inputs** at your code to find crashes and vulnerabilities:

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Initialize fuzz target
cargo fuzz init

# Run fuzzing
cargo fuzz run fuzz_target_1
```

Example fuzz target:

```rust
// fuzz_targets/json_parser.rs
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = my_json_parser(s);
    }
});
```

Fuzzing complements other tests:

- Unit tests encode known scenarios
- Property tests encode invariants
- Fuzzing tries to break your code with unexpected inputs

---

### Snapshot Testing with Insta

**Insta** captures complex output and compares it against stored "golden files":

```rust
use insta::assert_snapshot;

#[test]
fn test_complex_output() {
    let output = generate_complex_structure();
    assert_snapshot!(output);
}
```

Key features:

- Supports various formats (text, JSON, YAML)
- Review and approve changes with `cargo insta review`
- Great for testing HTML output, AST dumps, or complex data structures

---

### Test Coverage

Coverage tools measure how much code is exercised by tests:

```bash
# Using cargo-llvm-cov
cargo install cargo-llvm-cov
cargo llvm-cov test
cargo llvm-cov report --html
```

Important nuance: **High coverage does not equal correct code**, but low coverage signals untested areas.

---

## CI/CD Integration

Rust's test tooling fits cleanly into CI systems:

### GitHub Actions Example

```yaml
name: Rust CI

on:
  push:
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: cargo build --verbose

      - name: Run tests
        run: cargo test --verbose
```

### With Nextest

```yaml
- name: Install nextest
  run: cargo install cargo-nextest

- name: Run tests with nextest
  run: cargo nextest run --profile ci
```

### Extended Pipeline

```yaml
- name: Check formatting
  run: cargo fmt --check

- name: Run clippy
  run: cargo clippy -- -D warnings

- name: Run tests
  run: cargo test --verbose

- name: Generate coverage
  run: cargo llvm-cov --html
```

---

## Capstone Example

A small calculator demonstrating multiple test approaches:

### Library Code (src/lib.rs)

```rust
/// A simple calculator that performs basic arithmetic operations.
///
/// # Examples
///
/// ```
/// use my_crate::Calculator;
///
/// let calc = Calculator::new();
/// assert_eq!(calc.add(2, 3), 5);
/// ```
#[derive(Default)]
pub struct Calculator;

impl Calculator {
    /// Creates a new calculator instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds two numbers.
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn new_creates_calculator() {
        let calc = Calculator::new();
        let _ = calc;
    }

    #[test]
    fn add_handles_basic_cases() {
        let calc = Calculator::new();
        assert_eq!(calc.add(2, 3), 5);
        assert_eq!(calc.add(-1, 1), 0);
        assert_eq!(calc.add(0, 0), 0);
    }

    proptest! {
        #[test]
        fn add_is_commutative(a: i32, b: i32) {
            let calc = Calculator::new();
            prop_assert_eq!(calc.add(a, b), calc.add(b, a));
        }

        #[test]
        fn add_has_zero_identity(a: i32) {
            let calc = Calculator::new();
            prop_assert_eq!(calc.add(a, 0), a);
            prop_assert_eq!(calc.add(0, a), a);
        }
    }
}
```

### Integration Test (tests/calculator_integration.rs)

```rust
use my_crate::Calculator;

#[test]
fn calculator_smoke_test() {
    let calc = Calculator::new();
    assert_eq!(calc.add(40, 2), 42);
}
```

Run everything:

```bash
cargo test
# or with nextest:
cargo nextest run
```

---

## Quick Reference

### Essential Commands

| Command | Purpose |
|:--------|:--------|
| `cargo test` | Run all tests |
| `cargo test add_two` | Run tests matching pattern |
| `cargo test -- --nocapture` | Show stdout/stderr |
| `cargo test -- --test-threads=1` | Run tests sequentially |
| `cargo test --doc` | Run only doc tests |
| `cargo nextest run` | Run with nextest |
| `cargo bench` | Run benchmarks |
| `cargo fuzz run target` | Run fuzz tests |

### Test Attributes

| Attribute | Purpose |
|:----------|:--------|
| `#[test]` | Mark function as test |
| `#[cfg(test)]` | Compile only during testing |
| `#[should_panic]` | Test expected panic |
| `#[should_panic(expected = "msg")]` | Test panic with message |
| `#[ignore]` | Skip test by default |

### Assertion Macros

| Macro | Purpose |
|:------|:--------|
| `assert!(expr)` | Assert expression is true |
| `assert_eq!(a, b)` | Assert equality |
| `assert_ne!(a, b)` | Assert inequality |
| `debug_assert!()` | Debug-only assertion |
| `prop_assert!()` | Proptest assertion |
| `prop_assert_eq!()` | Proptest equality |

### Testing Crates

| Crate | Purpose |
|:------|:--------|
| `proptest` | Property-based testing |
| `mockall` | Trait-based mocking |
| `criterion` | Benchmarking |
| `insta` | Snapshot testing |
| `rstest` | Fixtures and parameterized tests |
| `pretty_assertions` | Colorful diff output |
| `testcontainers` | Docker-based integration tests |

---

## Resources

- [The Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Rust By Example - Testing](https://doc.rust-lang.org/rust-by-example/testing.html)
- [cargo-nextest Documentation](https://nexte.st/)
- [Proptest Book](https://proptest-rs.github.io/proptest/intro.html)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Mockall Documentation](https://docs.rs/mockall/latest/mockall/)
- [Insta Documentation](https://insta.rs/)
- [cargo-fuzz Documentation](https://rust-fuzz.github.io/book/)

---

## Recommendations Summary

1. **Start with the basics**: Use `cargo test` for unit and integration tests
2. **Document with doctests**: Put examples in `///` comments to keep docs accurate
3. **Organize tests cleanly**: Unit tests inline, integration tests in `tests/`
4. **Adopt property-based tests** for algorithms with invariants
5. **Use nextest for large projects**: Better performance and CI integration
6. **Mock via traits**: Keep dependencies behind traits for testability
7. **Benchmark critical paths**: Use Criterion to track performance over time
8. **Add fuzzing and coverage** for parser/protocol code and to find untested areas

With these pieces in place, Rust's testing story becomes one of the language's major strengths for building robust, maintainable systems.
