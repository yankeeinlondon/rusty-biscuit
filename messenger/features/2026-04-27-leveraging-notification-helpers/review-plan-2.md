---
phases: 4
start_phase: 4
created: 2026-05-06
source: review-2.md
related:
  - spec.md
  - tech-design.md
  - plan-2.md
packages:
  - messenger
  - messenger-cli
source_files_during_phase_4:
  - messenger/cli/src/lib.rs
  - messenger/cli/src/main.rs
  - messenger/cli/Cargo.toml
  - messenger/cli/tests/info_snapshot.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
---

# Review Plan 2: Desktop Helper Polish and Regression Coverage

## Objective

Implement every fix suggested in `review-2.md` for the desktop notification helper work. The review found no production blockers, so this plan is a focused hardening pass: align maintainer-facing docs with the shipped behavior, close timeout and registration test gaps, prove target-platform native fallback, make the helper suite friendlier to `cargo nextest`, and add snapshot coverage for `messenger info`.

The feature intent from the spec and technical design remains unchanged: desktop helpers are detected, reported, installable through the CLI, and used as the preferred delivery path before falling back to native desktop APIs.

## Phase 1 - Documentation Alignment

**Goal**: Remove the stale "opportunistic" mental model from backend rustdoc and make internal docs match `messenger/docs/user-guide.md`.

### Steps

1. Audit module-level docs in:
   - `messenger/lib/src/provider/desktop/linux.rs`
   - `messenger/lib/src/provider/desktop/macos.rs`
   - `messenger/lib/src/provider/desktop/windows.rs`
   - `messenger/lib/src/provider/desktop/helpers/mod.rs`
   - `messenger/lib/src/provider/desktop/backend.rs`
2. Replace language such as "opportunistic layer" and "best-effort enrichment" when it describes helper routing as a whole.
3. Use the shipped model consistently:
   - Helpers are the primary delivery path when installed and scored above zero.
   - Native APIs remain the fallback/floor.
   - Best-effort wording is still valid for specific unsupported fields that are intentionally dropped.
4. Keep public rustdoc concise and avoid changing public APIs.

### Test Coverage Expectations

No behavioral tests are required for this phase. Existing rustdoc must compile cleanly and lint should not reveal stale references.

### Verification

```sh
cargo check -p messenger --features desktop
cargo clippy -p messenger --features desktop --all-targets -- -D warnings
```

## Phase 2 - Helper Timeout and Registration Coverage

**Goal**: Prove timeout configuration and Windows AppID registration shell-outs through the same stub-binary integration path used by helper sends.

### Steps

1. Extend `messenger/lib/src/tests/desktop_helpers.rs` with explicit timeout tests for every helper:
   - `dunstify`: sleeps longer than the notice-only ceiling and falls through to `notify-send`.
   - `notify-send`: sleeps longer than its 5s ceiling and returns `HelperError::Timeout { timeout_ms: 5000 }`, or proves backend fallback if tested through `LinuxBackend`.
   - `snoretoast`: notice-only timeout is 5s; interactive requests remain unbounded where intended.
   - `burnttoast`: notice-only timeout is 10s; interactive requests remain unbounded where intended.
   - `terminal-notifier`: timeout is 5s.
   - `alerter`: notice-only timeout uses the 60s ceiling; interactive requests have no timeout.
2. Prefer helper-level timeout tests when a backend-level fallback would require sleeping through multiple helpers. Use backend-level tests only where the review specifically asks for fallback chain coverage.
3. Keep all env-var-driven tests under `serial_test::serial` and use the existing `EnvGuard` cleanup pattern.
4. Add a dedicated SnoreToast registration integration test:
   - Construct `SnoreToastHelper` without bypassing registration.
   - Point it at `stub_snoretoast`.
   - Assert the stub observes the `-install` argv contract and the helper marks the AppID as usable before send.
5. Add or preserve a BurntToast registration integration test:
   - Construct `BurntToastHelper` without `mark_app_id_registered()`.
   - Point PowerShell execution at `stub_burnttoast` or the existing PowerShell stub path.
   - Assert the generated script includes the `New-BTAppId` registration contract before send.
6. If any helper currently lacks a deterministic "sleep" env var in its stub, add one to the matching file under `messenger/lib/tests/bin/stub_*/main.rs`.

### Test Coverage Expectations

The desktop helper integration suite should cover per-helper timeout constants, timeout error mapping, fallback after timeout where the backend owns the fallback, and real registration shell-out contracts for both Windows helpers.

Expected additions:

- At least one timeout test for each of the six helpers.
- At least one registration shell-out test for `snoretoast -install`.
- At least one registration shell-out test for BurntToast `New-BTAppId`.

### Verification

```sh
cargo test -p messenger --features desktop --lib desktop_helpers
cargo test -p messenger --features desktop --lib provider::desktop::helpers::process
cargo test -p messenger --features desktop --lib
```

## Phase 3 - Native Fallback and Nextest Compatibility

**Goal**: Ensure fallback to native backends is proven on actual target runners, and reduce redundant stub builds under `cargo nextest`.

### Steps

1. Add Linux target-platform fallback coverage in `messenger/lib/src/provider/desktop/linux.rs`:
   - Gate with `#[cfg(target_os = "linux")]`.
   - Construct `LinuxBackend::with_helpers(..., vec![])` or a failing helper chain.
   - Send a simple notice-only request.
   - Assert successful receipts include `helper_used=native`.
   - If the CI runner lacks a notification D-Bus session, treat `Transport` errors that clearly indicate no notification service as an environment skip, not a product failure. Document the condition in the test.
2. Add Windows target-platform fallback coverage in `messenger/lib/src/provider/desktop/windows.rs`:
   - Gate with `#[cfg(target_os = "windows")]`.
   - Provide bootstrap/AppID state that allows WinRT native send to be attempted.
   - Construct a backend with no helpers or helpers scoring zero.
   - Assert a successful native receipt includes `helper_used=native`; if the runner cannot show toasts, assert the error occurs after bootstrap validation and documents the OS limitation.
3. Keep non-target tests that verify clear transport errors on macOS/Linux/Windows when native APIs are unavailable.
4. Optimize stub binary lookup in `messenger/lib/src/tests/desktop_helpers.rs`:
   - Cache each stub path with `OnceLock<PathBuf>` or a `OnceLock<HashMap<&'static str, PathBuf>>`.
   - Preserve the existing fallback that runs `cargo build --bin <stub> -p messenger --features desktop` when the binary is absent.
5. Update `.github/workflows/messenger-desktop-tests.yml` to pre-build helper stubs before tests:

```sh
cargo build -p messenger --features desktop --bins
```

6. If `cargo nextest` is available in CI or local developer workflows, add a documented optional verification command. Do not require nextest installation in the normal workflow unless the repository already does so elsewhere.

### Test Coverage Expectations

The suite should prove both fallback shapes:

- Cross-platform fake/stub fallback: helper fails or times out, next helper/native is selected.
- Target-platform native fallback: on Linux and Windows runners, native delivery is attempted and either succeeds with `helper_used=native` or fails with a clear environment-specific transport error after reaching the native path.

The helper stub suite should avoid repeated `cargo build` work inside a single test process.

### Verification

```sh
cargo build -p messenger --features desktop --bins
cargo test -p messenger --features desktop --lib provider::desktop::linux
cargo test -p messenger --features desktop --lib provider::desktop::windows
cargo test -p messenger --features desktop --lib desktop_helpers
```

Optional, when installed:

```sh
cargo nextest run -p messenger --features desktop
```

## Phase 4 - `messenger info` Snapshot Tests and Final Validation

**Goal**: Add regression tests for the CLI reporting surface and run the messenger package-area verification commands.

### Steps

1. Add CLI integration tests under `messenger/cli/tests/info_snapshot.rs`.
2. Use `insta` snapshots for:
   - `messenger info --plain`
   - `messenger info --json`
3. Make snapshots deterministic:
   - Use a temporary config directory/file.
   - Avoid depending on host-installed helper state where possible by injecting or mocking the detection record. If current CLI code cannot inject detection cleanly, first extract a small render function that accepts an `InfoRecord`, then snapshot that render output.
   - Redact absolute paths, OS versions, helper versions, and home directories with `insta` redactions.
4. Include at least one configured desktop route and one non-desktop route in the snapshot input so the route table and helper/election sections are both covered.
5. Keep `messenger install` out of the snapshot suite unless it can run fully dry-run with deterministic sniff output.
6. Update docs only if the snapshots expose a mismatch between documented and actual `messenger info` flags or output.

### Test Coverage Expectations

The CLI should have stable regression coverage for both human-readable and JSON `messenger info` output. The tests should not require installed helper CLIs, network access, or platform-specific notification services.

### Verification

```sh
cargo test -p messenger-cli info
cargo test -p messenger-cli
cargo test -p messenger --features desktop --lib
cargo clippy -p messenger --features desktop --all-targets -- -D warnings
cargo clippy -p messenger-cli --all-targets -- -D warnings
```

Package-area convenience commands:

```sh
just -f messenger/justfile test
just -f messenger/justfile lint
```

CI parity command set:

```sh
cargo check -p messenger --all-features
cargo build -p messenger --features desktop --bins
cargo test -p messenger --features desktop --lib
cargo test -p messenger-cli
```

## Implementation Notes

- Do not change public desktop provider APIs unless a test seam is impossible without excessive host coupling. Prefer extracting pure render/build functions over adding test-only public knobs.
- Keep env-var mutation serialized with `serial_test::serial`.
- Do not make tests depend on real notification helpers being installed.
- For target-platform native fallback tests, distinguish product failures from missing CI desktop services with explicit error assertions.
- Update `messenger/docs/user-guide.md`, `messenger/lib/README.md`, or `.claude/skills/messenger/SKILL.md` only if behavior or workflow guidance changes during implementation.

## Completion Criteria

1. Backend rustdoc consistently describes helpers as the primary helper-backed path and native APIs as fallback.
2. Every helper has deterministic timeout coverage.
3. SnoreToast and BurntToast registration shell-out contracts are covered by stub integration tests.
4. Linux and Windows native fallback paths are exercised on their target platforms or explicitly skip only unsupported CI desktop-service conditions.
5. Stub binary lookup does not repeatedly build the same binary within one test process, and CI pre-builds the stubs.
6. `messenger info --plain` and `messenger info --json` have deterministic snapshot coverage.
7. Messenger library and CLI tests plus clippy pass with the commands listed above.
