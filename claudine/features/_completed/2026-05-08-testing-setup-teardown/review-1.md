---
agent: gemini
model: ""
ready: false
---

# Feature Review: Testing Setup & Teardown (2026-05-08)

I have reviewed the implementation of the `testing-setup-teardown` feature. While the core infrastructure (`test-toolkit` library, `rstest` integration, and `nextest` configuration) has been established, there are significant functional gaps and missed opportunities for adoption that prevent this feature from being considered production-ready.

## Findings

### High Severity

#### Gap: Missing Tracing Subscriber Helper in `test-toolkit`
The specification explicitly states that `test-toolkit` should provide "a `tracing` subscriber init helper for test binaries that want structured span output." This helper is missing from `tools/test-toolkit/src/lib.rs`. Without it, the `trace_phase!` macro produces no output in standard test runs, failing the requirement that "spans appear in output, showing setup/body/teardown breakdown."

#### Gap: `claudine-cli` Dev-Dependencies Omitted
The spec mandates that "All packages should add `rstest` as a dev-dependency" and "Other packages add it [`test-toolkit`] as a dev-dependency." `claudine/cli/Cargo.toml` was not updated. This prevents the CLI package from adopting the new testing standards, even though it contains numerous tests that currently use local, redundant `EnvGuard` implementations.

#### Incomplete Migration: Insufficient Pattern Demonstration
While the "no bulk migration" policy is respected, migrating only a single test in `claudine/lib` and zero tests in `claudine/cli` is insufficient to establish the pattern across a high-impact package area. At least one migration in the CLI package (e.g., in `composition/mod.rs` or `timeouts.rs`) should have been performed to validate the `test-toolkit` integration in a PTY/CLI context.

### Medium Severity

#### Verification Level Mismatch: Nextest Configuration
The requirement "Nextest prints a per-test timing summary with slow-test highlighting" is only verified by the presence of the `.config/nextest.toml` file. There is no Level 1 test that executes `cargo nextest` against a dummy slow test to verify that the configuration is actually being picked up and that the output correctly highlights the breach.

#### Ergonomics: Unsafe EnvGuard Constructors
The `EnvGuard::set` and `EnvGuard::remove` constructors are `unsafe`. While this aligns with Rust 2024's treatment of the process environment, it places a high burden on test authors. A safe wrapper that asserts/verifies the presence of `#[serial_test::serial]` or uses a global lock could significantly improve ergonomics.

### Low Severity

#### Trace Level Sensitivity
`trace_phase!` defaults to `Level::INFO`. Users must be aware that if their subscriber (like the one in `claudine-cli/telemetry.rs`) defaults to `Level::WARN`, these spans will be silent unless `RUST_LOG` is explicitly configured. This is mentioned in the spec but could be better handled by the missing subscriber helper.

## Test Rigor Matrix

| Requirement | Strongest Test | Level | Status |
|-------------|----------------|-------|--------|
| `trace_phase!` macro functionality | Unit tests in `test-toolkit` | 1 | **Ready** |
| `EnvGuard` RAII restoration | Unit tests in `test-toolkit` | 1 | **Ready** |
| `rstest` + `EnvGuard` integration | `canonical_dispatch.rs` (lib) | 1 | **Ready** |
| Nextest slow-test highlighting | (None) | - | **Gap** |
| Trace span visibility in output | (None) | - | **Gap** (Missing subscriber helper) |

## Recommendations

1. **Implement the Tracing Subscriber Helper:** Add a standard `init_test_tracing()` function to `test-toolkit` that configures an INFO-level subscriber suitable for `nextest` output.
2. **Update `claudine-cli` Manifest:** Add `rstest` and `test-toolkit` to `claudine/cli/Cargo.toml`.
3. **Migrate a CLI Test:** Replace one local `EnvGuard` in `claudine/cli` (e.g., in `timeouts.rs`) with the `test-toolkit` version to verify cross-package compatibility.
4. **Add Nextest Verification:** Create a Level 1 integration test (perhaps in `tools/test-toolkit/tests`) that runs `cargo nextest` against a controlled slow test to verify the `nextest.toml` configuration.
