# Nextest Deep Dive: Rust’s Next-Generation Test Runner (Definitive Reference)

## Table of Contents

1. [What nextest is (and what it isn’t)](#what-nextest-is-and-what-it-isnt)
1. [Why nextest exists: the core problems with `cargo test`](#why-nextest-exists-the-core-problems-with-cargo-test)
1. [Key concepts and architecture](#key-concepts-and-architecture)
   1. [Process-per-test execution model](#process-per-test-execution-model)
   1. [Cross-binary scheduling and parallelism](#cross-binary-scheduling-and-parallelism)
   1. [Filtering via the filterset DSL](#filtering-via-the-filterset-dsl)
   1. [Profiles, overrides, and configuration layering](#profiles-overrides-and-configuration-layering)
   1. [Retries, flaky tests, and backoff strategies](#retries-flaky-tests-and-backoff-strategies)
   1. [Timeouts, slow tests, leaky tests, and fail-fast semantics](#timeouts-slow-tests-leaky-tests-and-fail-fast-semantics)
   1. [CI features: JUnit, partitioning (sharding), archiving](#ci-features-junit-partitioning-sharding-archiving)
1. [Installation and toolchain requirements](#installation-and-toolchain-requirements)
1. [Daily usage: CLI workflows](#daily-usage-cli-workflows)
   1. [Run tests](#run-tests)
   1. [List tests](#list-tests)
   1. [Filtering examples (from simple to advanced)](#filtering-examples-from-simple-to-advanced)
   1. [Controlling concurrency (threads)](#controlling-concurrency-threads)
1. [Configuration deep dive (`.config/nextest.toml`)](#configuration-deep-dive-confignextesttoml)
   1. [Configuration file discovery and precedence](#configuration-file-discovery-and-precedence)
   1. [Store directory](#store-directory)
   1. [Profiles and inheritance](#profiles-and-inheritance)
   1. [Overrides (per package / per test subset)](#overrides-per-package--per-test-subset)
   1. [Retries configuration patterns](#retries-configuration-patterns)
   1. [Timeouts and termination](#timeouts-and-termination)
   1. [JUnit output configuration](#junit-output-configuration)
1. [Advanced CI patterns](#advanced-ci-patterns)
   1. [Sharding with `--partition`](#sharding-with---partition)
   1. [Build once, run many: `archive` + `run --archive-file`](#build-once-run-many-archive--run---archive-file)
1. [Programmatic usage: `nextest-metadata`](#programmatic-usage-nextest-metadata)
1. [Ecosystem and integrations](#ecosystem-and-integrations)
   1. [cargo-llvm-cov](#cargo-llvm-cov)
   1. [cargo-mutants](#cargo-mutants)
   1. [insta / cargo-insta](#insta--cargo-insta)
   1. [nextest-rs crates](#nextest-rs-crates)
1. [Gotchas, limitations, and edge cases](#gotchas-limitations-and-edge-cases)
1. [When to use nextest (and when not to)](#when-to-use-nextest-and-when-not-to)
1. [Alternatives and adjacent tools](#alternatives-and-adjacent-tools)
1. [Versioning notes and changelog caveats](#versioning-notes-and-changelog-caveats)
1. [Licensing](#licensing)

---

## What nextest is (and what it isn’t)

**Nextest** is a **next-generation test runner for Rust**, most commonly used through the CLI tool **`cargo-nextest`**. It is designed for **infrastructure-grade reliability** and **CI-first ergonomics**, and is widely deployed from open-source projects to very large organizations.

A crucial framing:

* nextest is a **test runner**: it orchestrates how tests are discovered, scheduled, isolated, retried, timed out, reported, and partitioned.
* nextest is *not* a test framework: you generally keep writing normal Rust tests (`#[test]`, integration tests, etc.) and nextest runs them more effectively than `cargo test` in many scenarios.

---

## Why nextest exists: the core problems with `cargo test`

Rust’s built-in `cargo test` is excellent for small projects and quick local iterations, but it has structural constraints:

1. **Isolation limits (shared process per test binary)**  
   Within a given test binary, `cargo test` runs tests in a thread pool. Tests share process-level global state (environment variables, current directory changes, global singletons), and a crash can affect other tests in the same process.

1. **Parallelism bottleneck across binaries**  
   `cargo test` can parallelize within a test binary, but tends to run **different test binaries** (e.g., multiple `tests/*.rs`) more serially than you’d like. In workspaces and integration-heavy repos, this leaves CPU idle while one slow binary blocks progress.

1. **CI ergonomics**  
   Clean output, JUnit XML, sharding, retries for flaky tests, and reusable build artifacts are “table stakes” in many CI systems. nextest is designed to provide these as first-class primitives.

---

## Key concepts and architecture

Nextest is an ecosystem of crates and tools. The headline tool is:

* **`cargo-nextest`**: the CLI you run (`cargo nextest run`, `list`, `archive`, etc.)

Under the hood, nextest functionality is split into libraries:

* **`nextest-runner`**: core scheduling/execution engine
* **`nextest-metadata`**: machine-readable test lists and results; used for programmatic access
* **`nextest-filtering`**: the filterset DSL parser/evaluator
* Related ecosystem crates include **`quick-junit`** (fast JUnit XML generator) and **`guppy`** (Cargo graph/workspace metadata)

### Process-per-test execution model

The defining design choice: **each test runs in its own process**.

Implications:

* Stronger isolation: one test’s environment variable mutation is contained.
* Cleaner failure handling: crashes and panics are localized.
* Better scheduling: tests become independent “units” that can be distributed across available CPU cores.

This is especially valuable for:

* integration tests that touch filesystem/network
* tests that mutate env vars (`std::env::set_var`)
* tests that can crash due to unsafe code or FFI

Example showing why isolation matters:

````rust
#[test]
fn test_env_isolation() {
    // Safe with nextest: runs in its own process
    std::env::set_var("DATABASE_URL", "postgres://localhost:5432/test_db");
    assert_eq!(
        std::env::var("DATABASE_URL").unwrap(),
        "postgres://localhost:5432/test_db"
    );
}
````

### Cross-binary scheduling and parallelism

nextest schedules tests across the whole workspace (across binaries), often producing large speedups—commonly cited as **up to ~3× faster** than `cargo test` for big suites—because it keeps CPUs busy even if one binary contains slow tests.

### Filtering via the filterset DSL

nextest’s filterset DSL enables **precise selection** of tests by combining:

* test name matches (including regex forms)
* package selection
* test kind (lib/bin/example/etc.)
* platform constraints
* boolean operators (`and`, `or`, `not`)

This becomes a key productivity feature for large monorepos/workspaces.

### Profiles, overrides, and configuration layering

nextest is configuration-driven:

* you define **profiles** (`default`, `ci`, and custom ones)
* profiles can have **overrides** that apply to subsets of tests (e.g., a package, or regex-matching tests)
* configuration is loaded from multiple sources with an order of precedence (see [Configuration discovery and precedence](#configuration-file-discovery-and-precedence))

### Retries, flaky tests, and backoff strategies

Retries are a first-class feature—particularly for CI stability. You can:

* set a retry count
* choose backoff strategy (fixed/exponential)
* add delay, max-delay, jitter

This supports managing “known flaky” tests while still surfacing the underlying issues.

### Timeouts, slow tests, leaky tests, and fail-fast semantics

Nextest can:

* mark/handle **slow tests** (and optionally terminate them)
* apply **global timeouts**
* detect **leaky tests** (tests that spawn child processes and don’t clean them up)
* implement **fail-fast**, including modes that terminate tests immediately rather than waiting for stragglers

### CI features: JUnit, partitioning (sharding), archiving

For CI-scale usage, nextest supports:

* **JUnit XML output**
* **test partitioning/sharding** (`--partition`) across multiple CI workers
* **archiving** compiled test artifacts for reuse (`archive` + `run --archive-file`)

---

## Installation and toolchain requirements

### Recommended: pre-built binaries

````bash
# Linux
curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin

# macOS
curl -LsSf https://get.nexte.st/latest/mac | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin
````

### Via cargo install

````bash
# --locked is required
cargo install --locked cargo-nextest
````

### From source

````bash
cargo install --locked --path <nextest-source-path>
````

### MSRV (important nuance)

* **Building nextest**: requires **Rust 1.89+**
* **Running tests with nextest**: works with Rust **1.41+** (tested on recent versions)

This distinction matters if your project MSRV is low: you may still *run* tests with nextest, but contributors/CI machines must have a new enough Rust to *install/build* it from source.

---

## Daily usage: CLI workflows

### Run tests

````bash
# Run all tests in the workspace
cargo nextest run

# Run tests for a specific package
cargo nextest run --package my-crate

# Select a profile (e.g., CI behavior)
cargo nextest run --profile ci
````

### List tests

Useful for debugging filter expressions or CI selection:

````bash
cargo nextest list
````

### Filtering examples (from simple to advanced)

````bash
# All tests in a package
cargo nextest run 'package(my-crate)'

# Tests whose names match a regex
cargo nextest run 'test(/integration.*/)'

# Exclude slow tests by naming convention
cargo nextest run 'not test(/^slow::/)'

# Complex selection: integration package, exclude flaky namespace
cargo nextest run 'package(integration) and not test(/^flaky::/)'

# Platform-specific selection
cargo nextest run 'package(my-crate) and platform(x86_64-unknown-linux-gnu)'
````

### Controlling concurrency (threads)

````bash
# Set explicit thread count
cargo nextest run --test-threads 4

# Relative to CPU count: “2 fewer than num-cpus”
cargo nextest run -2
````

Note the semantic gotcha: negative shorthand (`-2`) is different from `--test-threads` and can confuse users—see [Thread count interpretation](#gotcha-thread-count-interpretation).

---

## Configuration deep dive (`.config/nextest.toml`)

### Configuration file discovery and precedence

nextest loads configuration from (highest priority first):

1. Repository config: **`.config/nextest.toml`**
1. Tool-specific configs passed via `--tool-config-file`
1. Embedded default configuration

This layering is powerful but can be confusing if you expect “one source of truth.” For infrastructure-grade reproducibility, prefer explicitly setting important values in your repo config.

### Store directory

A typical configuration sets a store directory under `target/nextest`:

````toml
[store]
dir = "target/nextest"
````

### Profiles and inheritance

Profiles let you tune behavior for local dev vs CI. A baseline:

````toml
[profile.default]
default-filter = "all()"
test-threads = "num-cpus"
retries = 0
slow-timeout = "60s"
````

**Gotcha:** custom profiles don’t implicitly inherit from `default`. You must declare inheritance:

````toml
[profile.custom]
inherits = "default"
retries = 3
````

### Overrides (per package / per test subset)

Overrides apply special rules to a subset of tests:

````toml
[profile.default]

[[profile.default.overrides]]
filter = 'package(my-core)'
retries = 2
slow-timeout = "120s"

[[profile.default.overrides]]
filter = 'package(my-db)'
slow-timeout = { period = "300s", terminate-after = 1 }
threads-required = "num-cpus"  # effectively serialize vs other tests (resource isolation)
````

### Retries configuration patterns

Simple retry count:

````toml
[profile.default]
retries = 3
````

Fixed backoff:

````toml
[profile.default]
retries = { backoff = "fixed", count = 3, delay = "1s" }
````

Exponential backoff with jitter:

````toml
[profile.default]
retries = { backoff = "exponential", count = 10, delay = "1s", max-delay = "10s", jitter = true }
````

### Timeouts and termination

Example slow-test policy:

````toml
[profile.default]
slow-timeout = { period = "60s", on-timeout = "fail" }
````

CI-style global timeout and slow-test override:

````toml
[profile.ci]
global-timeout = "2m"

[[profile.ci.overrides]]
filter = 'test(/^slow::/)'
slow-timeout = { period = "300s", terminate-after = 2 }
````

Leak detection:

````toml
[profile.default]
leak-timeout = "200ms"
# or
leak-timeout = { period = "500ms", result = "fail" }
````

Fail-fast termination mode (to avoid “straggler” tests running to completion):

````toml
[profile.default]
fail-fast = { max-fail = 1, terminate = "immediate" }
````

### JUnit output configuration

Example:

````toml
[profile.default.junit]
path = "target/nextest/junit.xml"
report-name = "nextest-run"
store-success-output = false
store-failure-output = true
````

**Gotcha:** the JUnit `path` can be **relative to the store directory**, not necessarily the repo root (see [JUnit path semantics](#gotcha-junit-output-path)).

---

## Advanced CI patterns

### Sharding with `--partition`

Run a subset (“shard”) of tests per worker:

````bash
# Shard 1 of 4
cargo nextest run --partition count:4,hash:1

# Shard 2 of 4
cargo nextest run --partition count:4,hash:2
````

This is commonly combined with CI matrix jobs.

### Build once, run many: `archive` + `run --archive-file`

Archive compiled test artifacts:

````bash
cargo nextest archive --archive-file test-archive.tar.zst
````

Reuse in later steps (or different machines, depending on environment constraints):

````bash
cargo nextest run --archive-file test-archive.tar.zst
````

This is especially useful when build and test are split across pipeline stages.

---

## Programmatic usage: `nextest-metadata`

For tooling, dashboards, custom CI logic, or meta-test infrastructure, `nextest-metadata` provides a Rust API.

Minimal listing example:

````rust
use nextest_metadata::ListCommand;

fn main() {
    let command = ListCommand::new();
    let test_list = command.exec().unwrap();
    println!("{:?}", test_list);
}
````

Iterating binaries and tests:

````rust
use nextest_metadata::ListCommand;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = ListCommand::new();
    let test_list = command.exec()?;

    for binary in test_list.test_bins() {
        println!("Binary: {}", binary.name());
        for test in binary.tests() {
            println!("  Test: {}", test.name());
        }
    }

    Ok(())
}
````

---

## Ecosystem and integrations

### cargo-llvm-cov

Modern LLVM source-based coverage with built-in nextest support:

````bash
cargo install cargo-nextest cargo-llvm-cov

# Collect coverage while executing through nextest
cargo llvm-cov nextest --html
# Report: target/llvm-cov/html/index.html
````

Why it matters: nextest’s process-per-test model changes execution shape; `cargo-llvm-cov` handles this integration cleanly.

### cargo-mutants

Mutation testing runs your suite many times; nextest can drastically reduce total time:

````bash
cargo mutants --test-tool nextest --jobs 8
````

### insta / cargo-insta

Snapshot testing benefits from nextest’s isolation and output capture. Typical usage:

````rust
#[test]
fn test_snapshot() {
    let value = vec![1, 2, 3];
    insta::assert_debug_snapshot!(value);
}
````

Run with nextest:

````bash
cargo nextest run
INSTA_UPDATE=all cargo nextest run
````

### nextest-rs crates

Key components in the org/ecosystem:

* `nextest-runner`: execution engine
* `nextest-metadata`: structured outputs and lists
* `nextest-filtering`: filterset DSL implementation
* `quick-junit`: high-performance JUnit writer
* `guppy`: Cargo graph/workspace analysis used by nextest

---

## Gotchas, limitations, and edge cases

### Gotcha: Doctests are not supported

This is the most cited limitation. Workaround:

````bash
# Run non-doctests with nextest
cargo nextest run

# Run doctests separately
cargo test --doc
````

### Gotcha: `--locked` is required for installation/build

````bash
cargo install --locked cargo-nextest
````

### Gotcha: Configuration precedence can surprise you

If a tool-specific config is applied (or defaults differ from what you expect), behavior may diverge. Know the precedence order and keep critical settings in `.config/nextest.toml`.

### Gotcha: Thread count interpretation

* `cargo nextest run -2` means “two fewer than CPU count”
* `--test-threads 4` means exactly 4 (or `num-cpus`)

Don’t assume the negative shorthand is just another spelling of `--test-threads`.

### Gotcha: Fail-fast may still allow straggling tests

Without termination configuration, nextest may allow already-running tests to finish, which can feel slower than `cargo test` in some suites. Use:

````toml
fail-fast = { max-fail = 1, terminate = "immediate" }
````

### Gotcha: Platform filtering host vs target

Cross-compilation scenarios can be tricky. Example override:

````toml
[[profile.default.overrides]]
platform = { host = "cfg(unix)", target = "aarch64-apple-darwin" }
````

Be explicit whether the filter should apply to **host** or **target**.

### Gotcha: Leaky tests (child processes not cleaned up)

Symptoms: hangs or “leak” detection failures. Configure `leak-timeout` and fix tests to terminate children properly.

### Gotcha: Profiles don’t inherit unless you say so

````toml
[profile.custom]
inherits = "default"
````

### Gotcha: JUnit output path may be relative to the store

If you set:

````toml
[profile.default.junit]
path = "junit.xml"
````

It may resolve under the store (e.g., `target/nextest/junit.xml`). Use an absolute path if needed:

````toml
[profile.default.junit]
path = "/absolute/path/to/junit.xml"
````

### Gotcha: Building vs running MSRV differ

Even if your project supports older Rust, *installing/building nextest* may require modern Rust (1.89+).

---

## When to use nextest (and when not to)

### Strong fits

* **Large test suites** (hundreds/thousands of tests)
* **CI/CD pipelines** needing JUnit, sharding, retries, clear output
* **Flaky test management** (retries with backoff)
* **Integration-heavy projects** with widely varying runtimes
* **Large workspaces/monorepos** where cross-binary scheduling matters
* Teams that need **reproducible, infrastructure-grade** test runs

### When it may not be worth it (or needs a hybrid)

* Heavy **doctest** reliance (you’ll need `cargo test --doc`)
* Very small projects where process-per-test overhead outweighs wins
* Environments where **resources are extremely constrained**
* Toolchains that can’t accommodate the **build MSRV** for nextest
* Situations requiring custom harness behaviors that aren’t compatible

---

## Alternatives and adjacent tools

Because nextest is a runner (not a framework), alternatives tend to be:

* the built-in runner (`cargo test`) or
* specialized wrappers/infrastructure tools

### `cargo test` (built-in)

Pros: zero install, supports doctests, simplest workflow  
Cons: cross-binary parallelism bottleneck, shared-process issues, output/CI ergonomics

### `libtest-mimic`

Best for custom/dynamic test generation while staying compatible with the libtest ecosystem—often still runnable under nextest.

### `cargo-insta` / `insta`

Snapshot testing workflow tooling; can integrate with nextest as the execution engine.

### `cargo-llvm-cov` / Tarpaulin

Coverage tools that wrap test execution. `cargo-llvm-cov` is commonly paired with nextest.

---

## Versioning notes and changelog caveats

The available research explicitly notes a **knowledge gap**: a verified, per-release changelog for the **`nextest` library crate** (as published on crates.io) wasn’t available in the source material, and version timelines are therefore **approximate**.

What is reliable from the research:

* nextest has matured through a long **0.x** era with significant feature growth.
* users often track **`cargo-nextest`** versions and behavior more than the underlying `nextest` crate.
* treat minor updates in 0.x as potentially breaking for library APIs (SemVer convention for 0.x).

For authoritative version-by-version changes, consult:

* the `cargo-nextest` release notes on the upstream GitHub repository
* crates.io pages for `cargo-nextest`, `nextest-runner`, `nextest-metadata`, etc.

---

## Licensing

* nextest is **dual-licensed** under:
  * **Apache License 2.0**
  * **MIT License**
* Documentation (nextest website) is licensed under **CC BY 4.0**
* The project derives from `diem-devtools` with upstream code under the same permissive licenses.

---

## Practical starter pack (copy/paste)

A solid baseline `.config/nextest.toml` with local + CI behavior:

````toml
nextest-version = "0.9.50"

[store]
dir = "target/nextest"

[profile.default]
default-filter = "all()"
test-threads = "num-cpus"
retries = 0
slow-timeout = { period = "60s", on-timeout = "fail" }
status-level = "pass"
failure-output = "immediate"
fail-fast = true

[profile.default.junit]
path = "junit.xml"
report-name = "nextest-run"
store-success-output = false
store-failure-output = true

[profile.ci]
inherits = "default"
retries = 3
fail-fast = false
status-level = "all"
failure-output = "immediate-final"
global-timeout = "2m"

[[profile.ci.overrides]]
filter = 'package(integration) and test(/^flaky::/)'
retries = 5

[[profile.ci.overrides]]
filter = 'test(/^slow::/)'
slow-timeout = { period = "300s", terminate-after = 2 }
````

Typical CI invocation:

````bash
cargo nextest run --profile ci
cargo test --doc   # if you rely on doctests
````

---

If you want, I can add a “recommended CI templates” section (GitHub Actions / GitLab CI) showing partitioned jobs + JUnit upload + archive reuse, using only the features described in the research.