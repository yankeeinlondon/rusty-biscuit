# High-Confidence Plan To Address Claudine Testing Issues

**Date:** 2026-04-08
**Scope:** `claudine/lib` and `claudine/cli`
**Primary inputs:** `review.md`, verified code layout, `rust-testing` skill guidance

## Why this plan is high confidence

This plan is based on the review findings plus direct validation of the current code and test layout:

- `claudine/cli/tests/` currently has no shared `common/` module.
- `claudine/lib/src/harness/validate.rs` and `claudine/lib/src/composition/sequence.rs` contain the cited high-risk logic.
- `claudine/cli/src/output.rs` and `claudine/cli/src/main.rs` match the review’s rendering and routing concerns.
- `claudine/cli/src/commands/config_tui/` confirms the TUI is large, stateful, and lightly tested.
- `claudine/justfile` still uses `cargo test`, while the repo-level `.config/nextest.toml` is already configured for retries.

This plan also follows the `rust-testing` skill’s preferred patterns:

- Keep unit tests inline for private/internal logic.
- Move shared integration helpers into `tests/common/mod.rs`.
- Prefer `cargo nextest run` for routine verification.
- Use `insta` for complex output surfaces.
- Keep PTY coverage manual-only when determinism is not worth the cost.

## Success criteria

- The highest-risk untested paths in `harness`, `composition`, and CLI output/routing are covered by stable automated tests.
- CLI integration tests stop duplicating the same workspace/setup helpers.
- TUI coverage moves from ad hoc logic tests to at least one stable rendering and one event-path test for each high-risk area.
- Performance testing exists for the hottest synchronous/runtime-critical surfaces.
- The `rust-testing` skill is updated so future reviews/plans can rely on better guidance for this exact class of Rust CLI/TUI testing.

## Execution order

1. Stabilize test infrastructure and baseline commands.
2. Close the safety-critical library gaps first.
3. Cover CLI rendering, routing, color, and output-channel behavior.
4. Add TUI tests in a way that reduces future test friction instead of freezing current architecture in place.
5. Add targeted performance/property/snapshot expansion only after the high-risk correctness gaps are closed.

## Phase 0: Establish a stable testing baseline

**Goal:** make test work repeatable before adding coverage.

**Changes**

- Standardize on these verification commands for this workstream:
  - `cargo --version`
  - `rustc --version`
  - `cargo nextest run -p claudine`
  - `cargo nextest run -p claudine-cli`
- Keep `just test` working, but treat `cargo nextest run` as the primary local validation path for this initiative.
- Add a short testing note to the review follow-up or package docs clarifying:
  - normal verification uses `nextest`
  - PTY tests remain manual-only
  - snapshots require `cargo insta review` / `cargo insta accept`

**Why first**

- The `rust-testing` skill explicitly recommends verifying the active toolchain and preferring `nextest`.
- This reduces false signals while the suite is changing.

**Exit criteria**

- Everyone working this plan uses the same validation commands.
- Snapshot and PTY expectations are documented once instead of rediscovered per PR.

## Phase 1: Consolidate CLI integration-test infrastructure

**Goal:** eliminate duplicated helpers before expanding CLI coverage.

**Primary file targets**

- `claudine/cli/tests/common/mod.rs`
- `claudine/cli/tests/wrap_commands.rs`
- `claudine/cli/tests/sequence_cli.rs`
- `claudine/cli/tests/mcp_cli.rs`
- `claudine/cli/tests/skills_integration.rs`
- `claudine/cli/tests/handle_repo_config.rs`
- `claudine/cli/tests/protect_cli.rs`
- `claudine/cli/tests/pty_tests.rs`

**Changes**

- Create `claudine/cli/tests/common/mod.rs` and move shared helpers into it:
  - executable/file writers
  - JSON writers
  - ANSI stripping
  - temp workspace/repo builders
  - MCP catalog/default seeding
  - shared `TestWorkspace`-style setup
- Keep PTY-only helpers isolated behind small, clearly named functions instead of mixing them into general-purpose utilities.
- Convert each integration test file to `mod common;` and use the shared helpers.
- Normalize helper naming so new tests naturally reuse the same setup path.

**Why this is the first code change**

- It directly follows the `rust-testing` skill’s `tests/common/mod.rs` guidance.
- It lowers the cost of every later CLI test in this plan.

**Exit criteria**

- No major integration test file owns a private copy of `write_executable`, `write_json`, `strip_ansi`, or ad hoc workspace bootstrap logic.
- All existing CLI integration tests still pass under `cargo nextest run -p claudine-cli`.

## Phase 2: Close the safety-critical library gaps

**Goal:** cover the checks most likely to affect safety or real-world composition behavior.

**Primary file targets**

- `claudine/lib/src/harness/validate.rs`
- `claudine/lib/src/composition/sequence.rs`
- `claudine/lib/src/dispatch/mod.rs`

**Changes**

- In `harness/validate.rs`, add inline unit tests for:
  - `ValidationKind::ShellCommand`
  - `ValidationKind::NoDirtySourceCode`
  - failure messaging when shell commands fail
  - clean vs dirty git working tree behavior using temp repos
- In `composition/sequence.rs`, add inline unit tests for sequence reference resolution:
  - `@` magic lookup
  - `!` package-relative references
  - `vault:` references
  - `{{ENV}}` expansion paths
  - `~/...`, absolute, and plain relative paths
  - external file load error surfaces
- In `dispatch/mod.rs`, add direct tests for:
  - `finalize_response`
  - exit/blocking semantics
  - any small pure helpers that decide final status or event metadata

**Implementation notes**

- Keep these as inline unit tests in the source files rather than adding new integration tests; that matches the `rust-testing` skill and keeps access to private helpers.
- Use `tempfile` and `serial_test` when env/global state is involved.

**Exit criteria**

- The review’s P0 library gaps are all covered by stable tests.
- The new tests target behavior, not implementation trivia.

## Phase 3: Cover CLI rendering and command behavior

**Goal:** test the most visible CLI behavior without relying on fragile PTY automation.

**Primary file targets**

- `claudine/cli/src/output.rs`
- `claudine/cli/src/main.rs`
- `claudine/cli/tests/`

**Changes**

- Add output-focused tests for `output.rs`:
  - `log_wrapper_header`
  - `log_wrapper_env_details`
  - `log_dry_run`
  - `style_cli_switches`
- Prefer a small refactor if needed so rendering logic returns a string or renderable value that tests can assert on directly, while the logging wrapper remains thin.
- Use `insta` snapshots for styled output, with redaction for timestamps, temp paths, and IDs.
- Add integration tests for color/plain behavior:
  - `NO_COLOR=1`
  - `FORCE_COLOR=1`
  - `--plain`
- Add output-channel tests for representative non-wrapper commands:
  - structured/machine output goes to stdout
  - status/errors go to stderr
- Add command-routing smoke tests that exercise each `Commands` family with minimal valid inputs and assert handler-specific behavior or error messages.
- Add completion-output tests for `bash`, `zsh`, `fish`, and `elvish`.

**Recommended implementation shape**

- Add a dedicated integration test file such as `claudine/cli/tests/command_routing.rs`.
- Keep routing tests shallow: prove the command reached the intended handler, not the whole command’s full behavior.
- Reuse `tests/common/mod.rs` for HOME/config/bootstrap setup.

**Why this phase is before TUI**

- It closes the most user-visible blind spots with modest refactoring risk.
- It avoids deep architecture work before the suite has better coverage around the CLI shell.

**Exit criteria**

- Styled output functions have deterministic tests.
- Command routing regressions would fail CI quickly.
- Completion generation and color/plain behavior are covered.

## Phase 4: Build a testable TUI foundation, then add high-value coverage

**Goal:** improve TUI coverage without locking in the current highly coupled structure.

**Primary file targets**

- `claudine/cli/src/commands/config_tui/app.rs`
- `claudine/cli/src/commands/config_tui/reducers.rs`
- `claudine/cli/src/commands/config_tui/widgets/modal.rs`
- `claudine/cli/src/commands/config_tui/widgets/toggle.rs`
- `claudine/cli/src/commands/config_tui/tabs/services.rs`
- `claudine/cli/src/commands/config_tui/tabs/actions.rs`
- `claudine/cli/src/commands/config_tui/tabs/preferences.rs`
- `claudine/cli/src/commands/config_tui/tabs/tts.rs`

**Changes**

- First extract pure reducers/transition helpers for the most important tab logic:
  - services protect-rule staging
  - actions CRUD/navigation
  - preferences provider/agent selection
- Keep render functions close to current shape initially, but make them testable with stable app fixtures.
- Add `ratatui::backend::TestBackend` rendering tests for:
  - `widgets/toggle.rs`
  - `widgets/modal.rs`
  - `tabs/services.rs`
- Add event simulation tests for:
  - services tab protect-rule toggling and commit/cancel flow
  - actions tab modal navigation
  - app-level overview/detail and modal push/pop behavior
- Add narrow-width and normal-width render cases once the first TestBackend tests land.

**Recommended sequencing inside this phase**

1. Prove out the pattern on `toggle` and `modal`.
2. Add services-tab rendering and event tests.
3. Add app state-machine tests.
4. Expand to actions/preferences/tts after the pattern is stable.

**Why not start with a full TUI rewrite**

- The review is correct that the architecture is tightly coupled, but a full redesign is not required to get meaningful coverage.
- Small reducer extraction plus TestBackend coverage is the lowest-risk path to compounding value.

**Exit criteria**

- At least one rendering snapshot and one event-path test exist for the key TUI surfaces.
- The services tab no longer has zero test coverage.
- App-level mode/modal transitions are tested.

## Phase 5: Expand robustness and performance once correctness gaps are closed

**Goal:** use the already-declared tools (`proptest`, `insta`) and add performance visibility where it matters.

**Primary file targets**

- `claudine/cli/Cargo.toml`
- `claudine/lib/Cargo.toml`
- `claudine/lib/benches/` or `claudine/cli/benches/` as appropriate
- parser/formatting modules identified below

**Changes**

- Make an explicit decision on `proptest`:
  - either add initial property tests
  - or remove the dependency until real use lands
- Preferred first property-test targets:
  - stream parsing newline/termination helpers
  - output truncation boundaries
  - composition template interpolation edge cases
  - matcher/loader pattern handling where invariants are clear
- Expand snapshot coverage for stable structured output:
  - additional help output
  - hooks/provider/log/report surfaces
  - selected TUI buffers
- Add Criterion benchmarks for the highest-value paths:
  - `ProtectService` evaluation
  - stream parser throughput
  - dispatch/config-loading hot path

**CI recommendation**

- Do not gate normal PRs on benchmarks initially.
- Run benchmarks on a scheduled or dedicated job and review regressions out of band first.

**Exit criteria**

- `proptest` is either used intentionally or removed.
- Benchmark infrastructure exists for the three highest-impact surfaces.
- Snapshot usage covers the major stable text/JSON output surfaces rather than only help text.

## Recommended PR breakdown

**PR 1: Test infrastructure + safety-critical library coverage**

- Phase 1
- Phase 2

**PR 2: CLI rendering, color/plain, routing, and completions**

- Phase 3

**PR 3: TUI testability foundation + first rendering/event tests**

- Phase 4

**PR 4: Property tests, snapshot expansion, and benchmarks**

- Phase 5

This split keeps reviewable scope small and lets the highest-risk gaps land first.

## Verification checklist

Use these commands throughout implementation:

```bash
cargo --version
rustc --version
cargo nextest run -p claudine
cargo nextest run -p claudine-cli
cargo test -p claudine-cli --test pty_tests -- --ignored
```

For snapshot-heavy changes:

```bash
cargo insta pending-snapshots
cargo insta review
```

For benchmarks after Criterion is added:

```bash
cargo bench -p claudine
```

## Risks and controls

- **Risk:** output tests become brittle because they capture too much formatting detail.
  - **Control:** snapshot only stable surfaces and redact temp paths, timestamps, and IDs.
- **Risk:** TUI tests stall because state setup is too heavy.
  - **Control:** extract small reducer helpers first and build a compact app-fixture constructor.
- **Risk:** command-routing tests become integration duplicates of feature tests.
  - **Control:** keep them shallow and handler-oriented.
- **Risk:** performance work distracts from correctness gaps.
  - **Control:** keep benchmarks in the last phase and out of PR gating initially.

## Improve the `rust-testing` skill

The final step of this work should be updating the local `rust-testing` skill so it better supports Rust CLI/TUI repos like Claudine.

**Recommended additions**

- Add a section on large CLI integration suites that demonstrates `tests/common/mod.rs` extraction and helper-sharing patterns.
- Add Ratatui `TestBackend` guidance with examples for:
  - rendering widgets to a buffer
  - snapshotting buffers with `insta::assert_debug_snapshot!`
  - testing key-event reducers separately from rendering
- Add ANSI/color-output testing guidance covering:
  - `NO_COLOR`
  - `FORCE_COLOR`
  - `--plain`
  - stdout vs stderr assertions with `assert_cmd`
- Add PTY guidance showing when `expectrl` tests should remain `#[ignore]` manual smoke tests instead of blocking CI.
- Expand the snapshot-testing guidance with redaction patterns for temp dirs, UUIDs, timestamps, ANSI output, and HOME-dependent strings.
- Expand the `nextest` guidance with:
  - retry/override examples for flaky or PTY-related tests
  - a recommendation for package-scoped commands in monorepos
- Add a criterion section tailored to CLI/lib hot paths and non-gating benchmark adoption.

**Concrete deliverable**

- Update `.claude/skills/rust-testing/SKILL.md`.
- Add or expand supporting docs under `.claude/skills/rust-testing/` for TUI testing, CLI output testing, and snapshot redaction patterns.

That skill update should happen after at least PR 2, so the examples can reflect real patterns proven in the Claudine codebase instead of speculative advice.
