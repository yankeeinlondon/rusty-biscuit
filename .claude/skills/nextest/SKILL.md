---
name: nextest
description: Use when the user is running or optimizing Rust tests (especially CI/workspaces) and wants faster, more reliable execution than `cargo test`, including filtering, retries for flaky tests, timeouts, partitioning/sharding, JUnit reports, and archiving.
---

# Nextest (cargo-nextest) Skill

Use this skill to design, configure, and run Rust test suites with **nextest**: faster parallel execution, process-per-test isolation, CI-ready reporting, retries for flaky tests, and advanced filtering.

## Quick decision: should we use nextest?

* Use nextest when: large workspaces, many integration tests, CI sharding, flaky/slow tests, need JUnit, need isolation from global state.
* Avoid/augment nextest when: you rely heavily on **doctests** (run `cargo test --doc` separately), very small suites where process overhead dominates.

Details: [When to use vs not](when-to-use.md)

## Core commands (most common)

* Run all tests: `cargo nextest run`
* List tests: `cargo nextest list`
* Run with a profile: `cargo nextest run --profile ci`
* Use filters (filterset DSL): `cargo nextest run 'package(my-crate) and not test(/^slow::/)'`
* Shard in CI: `cargo nextest run --partition count:4,hash:1`
* Archive & reuse build artifacts:
  * `cargo nextest archive --archive-file test-archive.tar.zst`
  * `cargo nextest run --archive-file test-archive.tar.zst`

Practical cookbook: [Commands & recipes](commands-and-recipes.md)

## Configuration entrypoint

Create `.config/nextest.toml` for reproducible behavior across dev/CI. Nextest loads config (highest priority first):

1. repository `.config/nextest.toml`
1. tool config via `--tool-config-file`
1. embedded defaults

Config templates and profiles: [Config & profiles](config-and-profiles.md)

## Flaky/slow/hanging tests

Use retries (optionally with backoff), slow timeouts, leak detection, and fail-fast termination.

Guidance + examples: [Reliability features](reliability-features.md)

## Filtering (the DSL)

Use `package()`, `test()`, `kind()`, `platform()`, and boolean ops `and/or/not`. Prefer filters for targeted runs and per-test overrides.

Reference + examples: [Filtering DSL](filtering-dsl.md)

## CI integrations

* JUnit output for CI test reporting
* Coverage via `cargo-llvm-cov nextest`
* Mutation testing via `cargo-mutants --test-tool=nextest`
* Snapshot testing with `insta` (works well with nextest’s output isolation)

Patterns: [CI & tooling integrations](ci-integrations.md)

## Common gotchas (checklist)

* Doctests aren’t supported → run `cargo test --doc` separately.
* Installing/building nextest requires `--locked`.
* New profiles don’t inherit unless `inherits = "default"`.
* JUnit path is relative to the **store dir** (often `target/nextest`), not the repo root.
* Fail-fast may still allow stragglers unless terminate mode is configured.
* Thread count semantics: `-2` means “2 fewer than CPU count”; `--test-threads` expects explicit number or `num-cpus`.
* Cross-compilation platform filters: understand **host vs target**.
* Leaky tests: enable `leak-timeout` and fix child-process cleanup.

Full details: [Gotchas & fixes](gotchas.md)

---
## How Claude should operate with this skill

When invoked:

1. Ask what environment the user cares about (local dev vs CI), workspace size, and whether doctests are required.
1. Propose a minimal setup: installation + a starter `.config/nextest.toml` with `default` and `ci` profiles.
1. Provide the exact command(s) to run now, then optional improvements (filters, overrides, sharding, archive).
1. If user reports flakes/timeouts/hangs: recommend retries/backoff, slow-timeout, leak-timeout, and fail-fast termination settings.

---
--- FILE: when-to-use.md ---
# When to use nextest (and when not to)

## Use nextest when

1. **Large test suites / workspaces**
   
   * Many crates and multiple integration test binaries.
   * Nextest runs tests across binaries concurrently (often significantly faster than `cargo test`).
1. **CI/CD pipelines**
   
   * Need **JUnit XML** output.
   * Need test **sharding/partitioning** across multiple CI workers.
   * Want consistent, profile-based configuration.
1. **Flaky tests**
   
   * Built-in retries with backoff strategies and better flake visibility.
1. **Isolation matters**
   
   * Tests touch env vars, filesystem, global state, ports, or crash occasionally.
   * Process-per-test reduces cross-test interference.
1. **Slow/hanging tests**
   
   * Identify slow tests, set per-test/per-package timeouts, terminate hangs.

## Don’t use (or supplement) nextest when

1. **You depend on doctests**
   
   * Nextest does not support Rust doctests (stable limitation).
   * Use a two-step approach in CI:
     * `cargo nextest run`
     * `cargo test --doc`
1. **Very small projects / micro test suites**
   
   * Process-per-test overhead may not pay off.
1. **Extremely old Rust toolchains**
   
   * Running tests with nextest works on old Rust (project MSRV can be low),
   * but **building/installing nextest** requires modern Rust (per docs, 1.89+ to build).
1. **Tools that require `cargo test` semantics/output**
   
   * Some pipelines are hardwired to `cargo test`; validate tool compatibility.

---
--- FILE: commands-and-recipes.md ---
# Commands & recipes

## Install

Recommended: prebuilt binaries (fastest), otherwise cargo install.

* Prebuilt (Linux/macOS) per nextest docs:
  
  * Linux: `curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin`
  * macOS: `curl -LsSf https://get.nexte.st/latest/mac | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin`
* Cargo install (must use `--locked`):
  
  * `cargo install --locked cargo-nextest`

## Day-to-day

* Run all tests:
  * `cargo nextest run`
* List tests (debug filters, verify discovery):
  * `cargo nextest list`
* Use a config profile:
  * `cargo nextest run --profile ci`
* Reduce concurrency (resource-constrained / DB-bound tests):
  * `cargo nextest run --test-threads 4`
  * `cargo nextest run -2` (2 fewer than CPU count)

## Targeted runs (filtersets)

* Single package:
  * `cargo nextest run 'package(my-crate)'`
* Regex-ish name selection:
  * `cargo nextest run 'test(/^auth::/)'`
* Exclude slow tests:
  * `cargo nextest run 'not test(/^slow::/)'`
* Integration but not flaky:
  * `cargo nextest run 'package(integration) and not test(/^flaky::/)'`

## CI sharding / partitioning

Run on N workers with hash partitioning:

* Worker 1 of 4:
  * `cargo nextest run --partition count:4,hash:1`
* Worker 2 of 4:
  * `cargo nextest run --partition count:4,hash:2`

## Archive and reuse (build once, run many)

* Create archive after building tests:
  * `cargo nextest archive --archive-file test-archive.tar.zst`
* Run in CI from archive (saves compile time in split pipelines):
  * `cargo nextest run --archive-file test-archive.tar.zst`

---
--- FILE: config-and-profiles.md ---
# Configuration & profiles (`.config/nextest.toml`)

## Minimal starter config (local + CI)

Create `.config/nextest.toml`:

````toml
nextest-version = "0.9.50"

[store]
dir = "target/nextest"

[profile.default]
default-filter = "all()"
test-threads = "num-cpus"
retries = 0
status-level = "pass"
failure-output = "immediate"
fail-fast = true
slow-timeout = { period = "60s", on-timeout = "fail" }

[profile.ci]
inherits = "default"
retries = 3
fail-fast = false
status-level = "all"
failure-output = "immediate-final"
global-timeout = "2m"

[profile.ci.junit]
path = "junit.xml"
````

Notes:

* `inherits = "default"` is important; profiles do **not** implicitly inherit.
* JUnit `path` is commonly **relative to the store directory** (see gotchas).

## Per-package overrides

Use overrides to tailor retries/timeouts/threads for specific packages or patterns:

````toml
[profile.default]

[[profile.default.overrides]]
filter = 'package(my-core)'
retries = 2
slow-timeout = "120s"

[[profile.default.overrides]]
filter = 'package(my-db)'
slow-timeout = { period = "300s", terminate-after = 1 }
threads-required = "num-cpus" # effectively serializes on a full-machine resource budget
````

## Tool-config-file vs repo config

If a user wants to avoid committing config, they can supply:

* `cargo nextest run --tool-config-file path/to/nextest.toml`

But remember config precedence can confuse outcomes—prefer a single committed `.config/nextest.toml` for important settings.

---
--- FILE: reliability-features.md ---
# Reliability features: flakes, timeouts, leaks, fail-fast

## Retries for flaky tests

Simple retry count:

````toml
[profile.ci]
retries = 3
````

With backoff:

````toml
[profile.ci]
retries = { backoff = "exponential", count = 5, delay = "1s", max-delay = "10s", jitter = true }
````

Good practice:

* Keep retries mostly in **CI profile**, not default local runs.
* Use overrides for known flaky groups rather than blanket high retries.

Example: flaky integration subset:

````toml
[[profile.ci.overrides]]
filter = 'package(integration) and test(/^flaky::/)'
retries = 5
````

## Slow tests and termination

Mark/act on slow tests:

````toml
[profile.ci]
slow-timeout = { period = "300s", terminate-after = 2 }
````

Per-pattern slow allowance:

````toml
[[profile.ci.overrides]]
filter = 'test(/^slow::/)'
slow-timeout = { period = "300s", terminate-after = 2 }
````

## Global timeout

Hard cap for the entire run (useful in CI to prevent infinite hangs):

````toml
[profile.ci]
global-timeout = "2m"
````

## Leaky test detection (child processes not cleaned up)

If tests spawn children and don’t reap them, enable leak detection:

````toml
[profile.ci]
leak-timeout = "200ms"
# or stricter:
# leak-timeout = { period = "500ms", result = "fail" }
````

## Fail-fast that actually stops work

Default fail-fast may allow “straggler” tests to continue. Prefer terminate mode:

````toml
[profile.ci]
fail-fast = { max-fail = 1, terminate = "immediate" }
````

Use this when:

* slow integration tests make “fail-fast” otherwise slower than expected.

---
--- FILE: filtering-dsl.md ---
# Filtering DSL (filtersets)

Nextest supports a filter expression language to precisely select tests.

## Core predicates

* `all()` — everything
* `test(<pattern>)` — match test names (often regex-like, e.g. `/^foo::/`)
* `package(<name>)` — tests from a Cargo package
* `kind(<type>)` — lib/bin/example/etc. (depending on suite)
* `platform(<spec>)` — target platform filtering

## Boolean logic

* `and`, `or`, `not`

## Examples

* All tests in one package:
  * `cargo nextest run 'package(my-crate)'`
* Select by name:
  * `cargo nextest run 'test(my_test_name)'`
* Regex-y selection:
  * `cargo nextest run 'test(/integration.*/)'`
* Exclude slow module:
  * `cargo nextest run 'not test(/^slow::/)'`
* Combine:
  * `cargo nextest run 'package(integration) and not test(/^flaky::/)'`

## Platform filtering

Common case:

* `cargo nextest run 'package(my-crate) and platform(x86_64-unknown-linux-gnu)'`

Cross compilation nuance: understand host vs target. In config overrides, you may need:

````toml
[[profile.default.overrides]]
platform = { host = "cfg(unix)", target = "aarch64-apple-darwin" }
````

If filters don’t behave as expected, confirm which platform (host/target) you meant.

---
--- FILE: ci-integrations.md ---
# CI & tooling integrations

## JUnit XML reporting

In config:

````toml
[profile.ci.junit]
path = "junit.xml"
report-name = "nextest-run"
store-success-output = false
store-failure-output = true
````

Run:

* `cargo nextest run --profile ci`

Gotcha: JUnit `path` is often relative to the store dir (see gotchas).

## Coverage: cargo-llvm-cov + nextest

Install:

* `cargo install cargo-nextest cargo-llvm-cov`

Run coverage with nextest as backend:

* `cargo llvm-cov nextest --html`

This handles nextest’s process-per-test model and aggregates coverage correctly.

## Mutation testing: cargo-mutants

Run with nextest:

* `cargo mutants --test-tool nextest --jobs 8`

Use when you need repeated full-suite runs; nextest speed helps a lot.

## Snapshot testing: insta

Nextest works well with insta output isolation:

* `cargo nextest run`
* Update snapshots:
  * `INSTA_UPDATE=all cargo nextest run`

## Archiving for CI pipelines

When builds and test execution happen in different jobs:

* Build+archive job:
  * `cargo nextest archive --archive-file test-archive.tar.zst`
* Test job:
  * `cargo nextest run --archive-file test-archive.tar.zst`

---
--- FILE: gotchas.md ---
# Gotchas & fixes

## 1) Doctests are not supported

Symptom: doctests don’t run under nextest.
Fix:

* `cargo nextest run` (unit/integration)
* `cargo test --doc` (doctests)

## 2) Installing/building requires `--locked`

Fix:

* `cargo install --locked cargo-nextest`

## 3) Config precedence surprises

Config sources (highest priority first):

1. `.config/nextest.toml`
1. `--tool-config-file ...`
1. embedded defaults

Fix: keep critical settings in one place; when debugging, specify `--tool-config-file` explicitly.

## 4) Thread count semantics

* `cargo nextest run -2` means “CPU count minus 2”.
* `--test-threads` expects a number or `num-cpus`.

Fix: pick one style and standardize in docs/CI scripts.

## 5) Fail-fast still allows stragglers

Fix: configure termination mode:

````toml
[profile.ci]
fail-fast = { max-fail = 1, terminate = "immediate" }
````

## 6) Platform filtering: host vs target confusion

Fix: in overrides, be explicit:

````toml
platform = { host = "cfg(unix)", target = "aarch64-apple-darwin" }
````

## 7) Leaky tests (child processes not cleaned up)

Symptom: tests “finish” but runner hangs or warns about leaks.
Fix:

* enable leak timeout:
  * `leak-timeout = "200ms"`
* fix test cleanup (reap/kill child processes, drop handles).

## 8) Profiles don’t inherit unless specified

Fix:

````toml
[profile.ci]
inherits = "default"
retries = 3
````

## 9) JUnit path is relative to store dir

Symptom: JUnit file not where you expected.
Fix:

* Use absolute path, or remember it lands under store (often `target/nextest/...`).

## 10) MSRV mismatch: building vs running

* Building/installing nextest requires modern Rust (per docs, 1.89+).
* Running your project’s tests can still target a lower MSRV.

Fix: in CI, install nextest using a modern toolchain even if your project MSRV is lower.
