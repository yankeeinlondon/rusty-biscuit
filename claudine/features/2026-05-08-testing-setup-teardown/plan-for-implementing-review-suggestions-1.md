---
created: "2026-05-09T19:05:48"
review: "claudine/features/2026-05-08-testing-setup-teardown/review-1.md"
spec: "claudine/features/2026-05-08-testing-setup-teardown/spec.md"
phases: 4
current_phase: 2
source_files_during_phase_1:
  - tools/test-toolkit/Cargo.toml
  - tools/test-toolkit/src/lib.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tools/test-toolkit/tests/nextest_config_verification.rs
  - tools/test-toolkit/justfile
docs_updated_during_phase_2:
  - .config/nextest.toml
docs_created_during_phase_2:
  - tools/test-toolkit/README.md
skills_files_updated_during_phase_2: []
packages:
  - test-toolkit
---

# Plan: Implement Review Suggestions for Testing Setup & Teardown

This plan addresses all gaps identified in the review of the `testing-setup-teardown` feature.

## Phase 1: Core Infrastructure — `test-toolkit` Subscriber Helper

**Goal:** Close the high-severity gap where `test-toolkit` lacks the tracing subscriber helper required for `trace_phase!` spans to produce visible output.

- [x] Add `tracing-subscriber` as a dependency in `tools/test-toolkit/Cargo.toml`
  - Use `tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }`
- [x] Implement `pub fn init_test_tracing()` in `tools/test-toolkit/src/lib.rs`
  - Configure a subscriber at `Level::INFO` (matching `trace_phase!`'s default span level)
  - Use `EnvFilter::from_default_env()` so `RUST_LOG` overrides still work
  - Guard with `Once` or `std::sync::Once` so multiple calls in the same test binary are idempotent
  - Return a handle that can be dropped to restore the prior subscriber (or return `()` if no restoration is needed)
- [x] Add unit tests for `init_test_tracing()` in `tools/test-toolkit/src/lib.rs` `#[cfg(test)]` block
  - Test that calling `init_test_tracing()` twice does not panic
  - Test that a `trace_phase!("verify", { 1 + 1 })` span is emitted and captured by a mock subscriber after initialization
- [x] Update rustdoc for `trace_phase!` to reference `init_test_tracing()` and explain the level default

**Validation checkpoint:** `cargo test -p test-toolkit` passes and new subscriber tests appear in output.

---

## Phase 2: Nextest Configuration & Verification

**Goal:** Close the medium-severity gap where nextest configuration exists only as a file on disk with no proof it is being honored.

- [x] Create `.config/nextest.toml` at the repository root with the content specified in the spec:
  ```toml
  [profile.default]
  slow-timeout = { period = "5s", terminate-after = 3 }

  [profile.ci]
  slow-timeout = { period = "10s", terminate-after = 2 }
  junit = { path = "test-results.xml" }
  ```
- [x] Add a new integration test file: `tools/test-toolkit/tests/nextest_config_verification.rs`
  - Define a single test that sleeps for `6s` (exceeds the `5s` default threshold)
  - Annotate the test with `#[test]` (not rstest) so it runs as a plain unit test
  - The test body should simply `std::thread::sleep(Duration::from_secs(6))` and then `assert!(true)`
- [x] Add a shell-level verification script or justfile recipe (or a `#[test]` that shells out) that runs:
  ```bash
  cargo nextest run --profile default -p test-toolkit --test nextest_config_verification 2>&1 | grep -i slow
  ```
  - This proves the `slow-timeout` configuration is picked up and slow tests are flagged
- [x] Document in `tools/test-toolkit/README.md` (or create one) how to run nextest with the workspace config

**Parallelizable with Phase 1:** Yes — the nextest config file and the verification test do not depend on the subscriber helper.

**Validation checkpoint:** Running `cargo nextest run --test nextest_config_verification` flags the 6-second test as slow in stderr.

---

## Phase 3: CLI Package Integration

**Goal:** Close the high-severity gap where `claudine-cli` cannot adopt the new testing standards because its `Cargo.toml` lacks the required dev-dependencies.

- [ ] Add `rstest = "0.25"` to `[dev-dependencies]` in `claudine/cli/Cargo.toml`
- [ ] Add `test-toolkit = { path = "../../tools/test-toolkit" }` to `[dev-dependencies]` in `claudine/cli/Cargo.toml`
- [ ] Identify one CLI test that modifies or reads process environment variables as the migration candidate
  - Search for `std::env::set_var`, `std::env::remove_var`, or tests that are sensitive to env state
  - Good candidates: tests in `claudine/cli/tests/wrap_commands.rs` that set timeout-related env vars, or tests that manipulate `PLAYA_DRY_RUN`
- [ ] Migrate the chosen test to use `test_toolkit::EnvGuard` and `#[rstest]`
  - Replace any hand-rolled env set/restore logic with `EnvGuard::set` / `EnvGuard::remove`
  - Add `#[serial_test::serial]` if the test touches global env state
  - Wrap fixture setup in `trace_phase!("setup", { ... })` if a fixture is extracted
  - Ensure the test compiles and passes
- [ ] Add a brief comment in the migrated test explaining why `test-toolkit` is used (pattern documentation for future migrations)

**Validation checkpoint:** `cargo test -p claudine-cli --test <migrated_test_name>` passes.

---

## Phase 4: Ergonomics, Documentation & Final Validation

**Goal:** Address the medium-severity ergonomics concern and the low-severity trace-level sensitivity, then validate the entire feature end-to-end.

- [ ] Evaluate adding a safe convenience wrapper around `EnvGuard`
  - Option A: `EnvGuard::set_serial` that internally asserts `serial_test::serial` is active via a compile-time or runtime check
  - Option B: A global `Mutex` inside `test-toolkit` that `EnvGuard` acquires on construction, making the constructors safe without requiring `#[serial]`
  - Option C: Document the `unsafe` rationale more prominently and provide a cookbook example showing the `#[rstest] + #[serial_test::serial]` pattern
  - **Decision criteria:** If Option B can be implemented without measurably slowing test suites, prefer it; otherwise prefer Option C with a clear code example in rustdoc
- [ ] Update `trace_phase!` rustdoc to warn about `Level::INFO` default and recommend `init_test_tracing()` or `RUST_LOG=test_toolkit=trace`
- [ ] Update `.claude/skills/rust-testing/SKILL.md` to document:
  - `rstest` fixture pattern with `Drop`-based teardown
  - `trace_phase!` usage with `init_test_tracing()`
  - `EnvGuard` with `#[serial_test::serial]`
  - Nextest configuration and how to verify it
- [ ] Run the full `claudine` package-area test suite (`just test` or `cargo nextest run -p claudine -p claudine-cli -p test-toolkit`)
- [ ] Run `cargo clippy -p test-toolkit -p claudine-cli` to ensure no new warnings

**Validation checkpoint:**
- All tests in `test-toolkit`, `claudine`, and `claudine-cli` pass under nextest
- The slow-test verification test is correctly flagged as slow
- The migrated CLI test passes and uses `rstest` + `test-toolkit`
- `.claude/skills/rust-testing/SKILL.md` reflects the new helpers

---

## Dependency Graph

```
Phase 1 (Subscriber Helper)
    │
    ├─► Phase 2 (Nextest Config)        [parallelizable]
    │
    └─► Phase 3 (CLI Integration)
            │
            └─► Phase 4 (Ergonomics & Docs)
```

## Risk Flags

- **EnvGuard safety model:** Changing the `unsafe` constructors to safe ones (Option B) is a behavioral change. If a global lock is introduced, it must be documented that `EnvGuard` is now self-serializing and `#[serial]` is still recommended for correctness but not strictly required for memory safety.
- **nextest.toml path:** The file must live at the repo root `.config/nextest.toml`. Verify that `cargo nextest` picks it up without `NEXTEST_CONFIG_FILE` being set.
- **Tracing subscriber interference:** `init_test_tracing()` must not panic if a subscriber is already set by the test binary or by another crate. Use `tracing::dispatcher::set_global_default` only if no global default exists, or prefer `tracing_subscriber::fmt::init()` wrapped in `Once`.
