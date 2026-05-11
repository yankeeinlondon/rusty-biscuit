Taking time to tests that need time is acceptable but most of the time when we have long running tests we're just not being smart about testing. One thing which we often overlook is the time it takes to "setup" and "tear down" a test's fixtures, etc.

In this feature we will create some testing infrastructure that we'll use with the `claudine` repo first but with intent to roll this out to all packages in this monorepo.

## Test Lifecycle with `rstest`

We will use [`rstest`](https://github.com/la10736/rstest) as the standard fixture framework across the monorepo. `rstest` provides `#[fixture]` for setup/teardown lifecycle via idiomatic Rust `Drop` implementations and parameterized injection into test functions.

This replaces the need for a custom `timed_test!` DSL. Tests look like standard Rust:

### Migration Policy

Existing tests should be left unchanged — there is no bulk migration. New tests and newly modified tests should use `#[rstest]`. Over time the codebase will naturally converge on the new pattern.

```rust
#[fixture]
fn large_document() -> Vec<u8> {
    load_large_fixture()
}

#[rstest]
fn parses_large_document(large_document: Vec<u8>) {
    let parsed = parse_document(&large_document);
    assert!(parsed.is_ok());
}
```

For teardown, fixtures implement `Drop` — the idiomatic Rust pattern:

```rust
#[fixture]
fn temp_project() -> TempProject {
    TempProject::create("my-test-project")
}

struct TempProject { dir: TempDir, /* ... */ }

impl TempProject {
    fn create(name: &str) -> Self {
        // setup logic
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        // teardown logic — runs automatically at end of test scope
    }
}
```

### Why `rstest` over a custom DSL

- Standard, well-documented API that any Rust contributor already knows
- No custom syntax to learn — tests remain recognizable Rust functions
- Implicit teardown via `Drop` avoids redundant calls like `drop(doc)` that RAII already handles
- Supports async fixtures, parameterized cases, and shared state out of the box
- Avoids the stable-Rust function-name acquisition problem entirely (no `timed_test_fn!` needed)

### Composing with `serial_test`

The codebase currently uses `serial_test` for tests that share global state (env vars, singletons). `rstest` replaces `#[test]` with `#[rstest]`, so `#[serial]` from `serial_test` can no longer be stacked directly. Instead, use `rstest`'s built-in test attribute support: apply `#[rstest]` as the outermost attribute and place `#[serial_test::serial]` inside it. Alternatively, `rstest_reuse` provides a `#[serial]` modifier that composes natively with `#[rstest]`. The implementation should standardize on one approach and document it.

When migrating a test that currently uses `serial_test`, the conversion looks like:

```rust
// Before:
#[test]
#[serial_test::serial]
fn my_test() { /* ... */ }

// After:
#[rstest]
#[serial_test::serial]
fn my_test() { /* ... */ }
```

### Dev-Dependency

All packages should add `rstest` as a dev-dependency:

```toml
[dev-dependencies]
rstest = "0.25"
```

## Within-Test Phase Timing with `tracing` Spans

While `rstest` manages lifecycle, we still want to measure setup vs body vs teardown time at a granular level. We provide a lightweight `trace_phase!` macro from `test-toolkit` that opens a `tracing` span around a code block:

```rust
use test_toolkit::trace_phase;

#[rstest]
fn parses_large_document(large_document: Vec<u8>) {
    // Fixture injection already covers setup timing via rstest lifecycle

    trace_phase!("body", {
        let parsed = parse_document(&large_document);
        assert!(parsed.is_ok());
    });
}
```

For fixtures that need explicit phase instrumentation:

```rust
#[fixture]
fn large_document() -> Vec<u8> {
    trace_phase!("setup", {
        load_large_fixture()
    })
}
```

The `trace_phase!` macro simply creates a `tracing::span!(Level::INFO, phase_name)` and enters it for the duration of the block. The macro evaluates to the block's return value, so it can wrap both statement blocks (like assertions) and expression blocks (like fixture setup that returns a value). This integrates with any `tracing` subscriber.

We will assume that ALL packages in this monorepo will be using the `tracing` crate so that we can leverage traces and metrics in this endeavor.

## Per-Test Timing with `nextest`

We will leverage `cargo nextest` for per-test wall-clock timing, slow-test detection, and CI-consumable output. The monorepo's `_test` recipe in `just/devops.just` already uses nextest when available — this becomes the primary timing surface.

### Nextest Configuration

Create a root-level `.config/nextest.toml` that establishes timing thresholds:

```toml
[profile.default]
# Mark tests slower than this as "slow" in output
slow-timeout = { period = "5s", terminate-after = 3 }

[profile.ci]
# Stricter thresholds for CI
slow-timeout = { period = "10s", terminate-after = 2 }
# JUnit XML for CI dashboards
junit = { path = "test-results.xml" }
```

### Timing Output

- **Default (`cargo nextest run`):** Nextest prints a per-test timing summary with slow-test highlighting. No env vars or custom logging needed.
- **CI (`cargo nextest run --profile ci`):** Generates JUnit XML that CI tools (GitHub Actions, etc.) can consume for test timing dashboards and flake detection.
- **Detailed breakdown (`-- --nocapture` with `RUST_LOG=test_toolkit=trace`):** The `trace_phase!` spans appear in output, showing setup/body/teardown breakdown within each test.

### Why nextest over custom logging

- Avoids reinventing per-test timing, parallel-safe file output, and slow-test detection
- The `TEST_TIMING_LOG` env var approach has edge cases with nextest's parallel runner (multiple tests contending on one file)
- Nextest's JUnit XML integrates with CI dashboards out of the box
- The monorepo already uses nextest in CI via `devops.just`

## Package/Directory Strategy

Since the `trace_phase!` macro and any shared test utilities are meant to be a shared resource for all packages in this monorepo, we define a `test-toolkit` package at `tools/test-toolkit`.

Other packages add it as a dev-dependency:

```toml
[dev-dependencies]
test-toolkit = { path = "../../tools/test-toolkit" }
```

The `test-toolkit` crate provides:

- `trace_phase!` macro
- `EnvGuard` — an RAII guard that sets an env var on creation and restores/removes it on `Drop`, replacing the hand-rolled `PlayaDryRunGuard` pattern already in use across `claudine` tests
- A `tracing` subscriber init helper for test binaries that want structured span output
- Common fixture helpers (temp dirs, mock server scaffolding) as the need arises

### Initial Scope

The initial implementation should include `trace_phase!` and `EnvGuard`. The `EnvGuard` is already being hand-rolled in existing tests (e.g., `PlayaDryRunGuard` in `canonical_dispatch.rs`), so extracting it gives `test-toolkit` immediate concrete value beyond a single macro. The tracing subscriber helper and additional fixture helpers can follow as patterns emerge.

## Drift Maintenance

Once implemented, the local agent skill at `.claude/skills/rust-testing` should be updated to reflect the testing approach defined in this spec (rstest fixtures, trace_phase!, EnvGuard, nextest config) so that the skill remains the authoritative reference for Rust testing patterns in this monorepo.

## Just Recipes

We already provide a `_test` **just** recipe in `just/devops.just`. The recipe already detects and prefers `nextest` when available — no wrapping logic is needed. The Rust solution is self-contained via `rstest` + `test-toolkit` + `nextest` config.

The only addition is ensuring `.config/nextest.toml` is present at the repo root with the timing thresholds above.
