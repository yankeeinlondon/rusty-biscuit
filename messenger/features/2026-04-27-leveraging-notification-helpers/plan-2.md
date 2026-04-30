---
phases: 5
start_phase: 2
created: 2026-04-29
source: review-1.md
related:
  - spec.md
  - tech-design.md
  - plan.md
source_files_during_phase_1:
  - messenger/lib/src/provider/desktop/helpers/burnttoast.rs
  - messenger/lib/src/provider/desktop/helpers/mod.rs
  - messenger/lib/src/provider/desktop/helpers/notify_send.rs
  - messenger/lib/src/provider/desktop/helpers/snoretoast.rs
  - messenger/lib/src/provider/desktop/linux.rs
  - messenger/lib/src/provider/desktop/macos.rs
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/lib/src/provider/desktop/windows.rs
  - messenger/lib/src/tests/validation.rs
docs_updated_during_phase_1:
  - messenger/features/2026-04-27-leveraging-notification-helpers/plan-2.md
docs_created_during_phase_1: []
skills_files_updated_during_phase: []
source_files_during_phase_2:
  - messenger/lib/Cargo.toml
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/lib/src/provider/desktop/linux.rs
  - messenger/lib/src/provider/desktop/windows.rs
  - messenger/lib/src/tests/mod.rs
  - messenger/lib/src/tests/validation.rs
  - messenger/lib/src/tests/desktop_helpers.rs
  - messenger/lib/tests/bin/stub_dunstify/main.rs
  - messenger/lib/tests/bin/stub_notify_send/main.rs
  - messenger/lib/tests/bin/stub_snoretoast/main.rs
  - messenger/lib/tests/bin/stub_burnttoast/main.rs
  - messenger/lib/tests/bin/stub_terminal_notifier/main.rs
  - messenger/lib/tests/bin/stub_alerter/main.rs
  - messenger/justfile
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - messenger/lib/src/provider/desktop/helpers/dunstify.rs
  - messenger/lib/src/provider/desktop/helpers/notify_send.rs
  - messenger/lib/src/provider/desktop/linux.rs
  - messenger/cli/src/info.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - messenger/lib/src/provider/desktop/windows.rs
  - messenger/lib/src/tests/desktop_helpers.rs
docs_updated_during_phase_4:
  - messenger/features/2026-04-27-leveraging-notification-helpers/plan-2.md
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
packages:
  - messenger
---

# Plan 2: Linux & Windows Functional Completeness for Desktop Notification Helpers

## Executive Summary

The original implementation of the desktop notification helpers feature (plan.md, phases 1–6) delivered working helper adapters for all three OSes but left Linux and Windows in a "best effort" posture: backend modules and helper units are gated by `#[cfg(target_os = …)]`, the cross-platform integration test suite (`desktop_helpers.rs` + stub binaries) was never built, and a metadata naming bug causes test failures on macOS. This plan fixes those gaps and elevates Linux and Windows to **functionally complete** status by:

1. Making all backends and helpers compile on every platform (so tests run everywhere).
2. Fixing the `helper_used` metadata naming bug.
3. Building the missing stub-binary integration tests for Linux and Windows helpers.
4. Expanding unit-test coverage for edge cases that matter on those platforms.
5. Wiring the tests into CI so they run on native runners.

---

## Problem Statement

### 1. Platform-gated compilation hides Linux/Windows tests on macOS

`messenger/lib/src/provider/desktop/mod.rs` gates the three backend modules with `#[cfg(target_os = …)]`. On a macOS host (our primary dev environment and the only CI runner we currently exercise), `linux.rs` and `windows.rs` are not compiled at all. The unit tests inside them—especially the `FakeHelper` election/fallback/replace tests—are invisible.

`helpers/mod.rs` also gates the six helper sub-modules by OS. The extensive `build_args()` and `parse_output()` unit tests inside `dunstify.rs`, `notify_send.rs`, `snoretoast.rs`, and `burnttoast.rs` do not run on macOS.

### 2. `linux.rs` cannot compile on macOS even if the cfg is removed

Unlike `macos.rs` and `windows.rs`—which keep their platform-native imports inside `#[cfg(target_os = …)]` functions—`linux.rs` imports `notify_rust::{Hint, Notification, Timeout, Urgency}` unconditionally and calls methods (`.hint()`, `.urgency()`, `.timeout()`, `.action()`) that do not exist in the macOS build of `notify-rust`.

### 3. The `helper_used` metadata uses snake_case, but tests expect PascalCase

`HelperName` is `sniff::programs::NotificationHelper`, which derives `strum::Display` with `serialize_all = "snake_case"`. `to_string()` therefore emits `terminal_notifier`, `snore_toast`, `burnt_toast`, `notify_send`. The backend tests in `macos.rs` assert `Some("TerminalNotifier")`, causing **3 failures on macOS**. The same mismatch exists in `linux.rs` and `windows.rs` tests but is hidden because those files do not compile on macOS.

### 4. Integration tests with stub binaries were never implemented

The tech design (§14.3) and review-1.md both call out the missing `messenger/lib/tests/desktop_helpers.rs` and `tests/bin/stub_*` binaries. Without these, we have no automated verification that the real `tokio::process::Command` invocation, stdout parsing, timeout handling, and fallback logic work end-to-end.

### 5. "Best effort" assumption in the original plan

Plan.md phase 2.6 says: "Validation: Integration test with stub dunstify/notify-send binaries on PATH"—but that test was never written. The original plan treats native APIs as the "universal floor" and helpers as opportunistic. For Linux and Windows to be **functionally complete**, the helpers must be proven as the primary delivery path, not a nice-to-have enhancement.

---

## Goals

1. **Cross-platform compilation**: Every backend file and every helper module compiles on every OS. Unit tests for all six helpers run on macOS, Linux, and Windows hosts.
2. **Naming consistency**: `helper_used` metadata matches what the backend tests expect, or the tests are updated to match the canonical naming. Either way, there are **zero failing tests** on every platform.
3. **Stub-binary integration tests**: `messenger/lib/tests/desktop_helpers.rs` exercises real `Command` invocations against stub binaries for all six helpers, with emphasis on Linux (`dunstify`, `notify-send`) and Windows (`snoretoast`, `burnttoast`) paths.
4. **Edge-case coverage**: Tests prove that platform-specific edge cases (Linux daemon mismatch, Windows PNG oversize, Windows AppID registration failure, old libnotify) degrade gracefully and do not crash.
5. **CI parity**: Linux and Windows test jobs run the new integration tests on their respective runners.

---

## Phase 1 — Cross-Platform Compilation & Naming Fix

**Goal**: Remove the `#[cfg(target_os = …)]` barriers so every file compiles on every host, and fix the metadata naming bug.

### Step 1.1 — Gate `notify-rust` usage in `linux.rs`

Restructure `linux.rs` to follow the same pattern as `windows.rs` and `macos.rs`:

- Move `use notify_rust::…` inside `#[cfg(target_os = "linux")]`.
- Gate `build_native_notification`, `native_send`, `native_replace`, and `map_urgency` with `#[cfg(target_os = "linux")]`.
- Add `#[cfg(not(target_os = "linux"))]` stubs for `native_send` and `native_replace` that return a transport error (`"native Linux notifications only available on Linux"`).
- `build_native_notification` does not need a non-Linux stub; it is only called from `native_send`/`native_replace`.

**Validation**: `cargo check -p messenger-lib --features desktop` passes on macOS.

### Step 1.2 — Remove `#[cfg(target_os = …)]` from `desktop/mod.rs` backend imports

In `messenger/lib/src/provider/desktop/mod.rs`, change:

```rust
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
```

to unconditional imports. Each backend already contains non-target stubs for its native functions, so this is safe after Step 1.1.

**Validation**: `cargo check -p messenger-lib --features desktop` passes on macOS, Linux, and Windows.

### Step 1.3 — Remove `#[cfg(target_os = …)]` from helper module declarations

In `helpers/mod.rs`, make all six helper modules unconditional:

```rust
pub(crate) mod alerter;
pub(crate) mod burnttoast;
pub(crate) mod dunstify;
pub(crate) mod election;
pub(crate) mod notify_send;
pub(crate) mod process;
pub(crate) mod snoretoast;
pub(crate) mod terminal_notifier;
```

The helper modules are pure CLI adapters (argv builders and stdout parsers). They use only `std`/`tokio` process APIs and compile everywhere.

**Validation**: `cargo check -p messenger-lib --features desktop` passes on all platforms.

### Step 1.4 — Fix the `helper_used` metadata naming bug

**Option A (recommended)**: Update the three backend test suites (`linux.rs`, `macos.rs`, `windows.rs`) to expect the canonical `snake_case` names that `strum::Display` produces:
- `TerminalNotifier` → `terminal_notifier`
- `Alerter` → `alerter`
- `SnoreToast` → `snore_toast`
- `BurntToast` → `burnt_toast`
- `Dunstify` → `dunstify`
- `NotifySend` → `notify_send`

This keeps the public metadata format stable and aligned with sniff's serialization.

**Option B**: Add a `HelperName::display_name()` method that returns PascalCase, and update the backends to use it. This is more work and creates two naming conventions.

We choose **Option A**.

**Validation**: `cargo test -p messenger-lib --features desktop --lib provider::desktop` passes on macOS with zero failures.

### Checkpoint 1

- `cargo test -p messenger-lib --features desktop --lib` passes on macOS.
- `cargo check -p messenger-lib --features desktop` passes on all three OSes.
- All six helper unit-test suites (argv, score, parse) are now compiled and run on macOS.

---

## Phase 2 — Stub Binary Infrastructure

**Goal**: Build the missing integration-test harness described in the tech design §14.3.

### Step 2.1 — Create stub helper binaries under `messenger/lib/tests/bin/`

Each stub is a tiny Rust program that reads behavior flags from env vars and prints the exact stdout / exits with the exact status code the real helper would produce.

Directory layout:

```
messenger/lib/tests/bin/
  stub_dunstify/main.rs
  stub_notify_send/main.rs
  stub_snoretoast/main.rs
  stub_burnttoast/main.rs
  stub_terminal_notifier/main.rs
  stub_alerter/main.rs
```

Cargo discovers binaries under `tests/bin/*/main.rs` automatically when running integration tests. We add a `build.rs` or use `CARGO_TARGET_DIR` to ensure the stubs are built and their paths known at test time.

**Stub contract per helper**:

| Helper | Env var control | stdout | exit codes |
|---|---|---|---|
| `stub_dunstify` | `STUB_DUNSTIFY_ID=42`, `STUB_DUNSTIFY_ACTION=ok` | `42` then `ok` on `--wait` | `0` |
| `stub_notify_send` | `STUB_NOTIFY_SEND_ID=99` | `99` | `0` |
| `stub_snoretoast` | `STUB_SNORETOAST_EXIT=4`, `STUB_SNORETOAST_STDOUT=Yes` | `Yes` | `0`–`5`, `-1` |
| `stub_burnttoast` | `STUB_BURNTTOAST_JSON={...}` | `__MESSENGER_ACTIVATION__\t{...}` | `0` |
| `stub_terminal_notifier` | n/a | empty | `0` |
| `stub_alerter` | `STUB_ALERTER_TYPE=actionClicked`, `STUB_ALERTER_VALUE=ok` | `{"activationType":"...", "activationValue":"..."}` | `0` |

**Validation**: `cargo test -p messenger-lib --test desktop_helpers -- --list` discovers tests.

### Step 2.2 — Write `messenger/lib/tests/desktop_helpers.rs`

This integration-test file exercises the **real** `tokio::process::Command` paths through each helper. It does **not** use `FakeHelper`; it constructs actual `DunstifyHelper`, `SnoreToastHelper`, etc. instances pointing at the stub binaries.

Test structure per helper:

1. **Success path**: Build helper with stub on PATH → send a notice-only request → assert `helper_used`, `notification_id`.
2. **Interactive path**: Send with actions / reply → assert correct activation metadata.
3. **Replace path**: Call `replace()` → assert same id returned.
4. **Timeout path**: Set stub env var to sleep longer than helper timeout → assert fallback to next helper.
5. **Non-zero exit path**: Set stub env var for error exit → assert fallback.
6. **Parse error path**: Set stub to print garbage → assert `ProviderError` (no fallback).

Platform gating:
- The file itself compiles on all platforms because helpers compile everywhere.
- Tests that exercise `LinuxBackend` with `dunstify`/`notify-send` stubs run on **all** platforms (they only talk to stubs).
- Tests that exercise `WindowsBackend` with `snoretoast`/`burnttoast` stubs run on **all** platforms.
- Tests that exercise `MacOsBackend` with `terminal-notifier`/`alerter` stubs run on **all** platforms.

**Validation**: `cargo test -p messenger-lib --test desktop_helpers` passes on macOS.

### Checkpoint 2

- All six stub binaries build.
- Integration tests cover success, interactive, replace, timeout, non-zero exit, and parse-error paths.
- Tests run on macOS using stubs for Linux and Windows helpers.

---

## Phase 3 — Linux Functional Completeness

**Goal**: Prove that the Linux backend delivers every `Dispatch` feature reliably when helpers are present, and degrades gracefully when they are not.

### Step 3.1 — Daemon-mismatch edge-case test

Add a unit test to `dunstify.rs` (or `desktop_helpers.rs`) that:
- Constructs `DunstifyHelper` with `daemon_is_dunst: false`.
- Sends a request with actions.
- Asserts that `score()` returns `0` and the helper is skipped in election.
- Asserts fallback to `notify-send` or native.

### Step 3.2 — Old libnotify graceful degradation test

Add a unit test to `notify_send.rs` that:
- Constructs `NotifySendHelper` with `libnotify_version: Some((0, 7, 7))` (below the 0.7.8 action threshold).
- Sends a request with actions.
- Asserts `score()` returns `40` (still usable, actions dropped).
- Asserts that `build_args()` does **not** emit `-A` flags.
- Asserts `metadata["dropped"] == "actions_libnotify_old"` on the receipt.

### Step 3.3 — Native fallback test on non-Linux platforms

Because `linux.rs` now compiles on macOS (Phase 1), add an integration test that:
- Builds `LinuxBackend::with_helpers(..., vec![])` (no helpers).
- Calls `send()` on macOS.
- Asserts the native path returns the expected transport error ("native Linux notifications only available on Linux").
- This proves the backend does not panic and correctly reports when it cannot deliver.

### Step 3.4 — `messenger info` output for Linux

Add a CLI snapshot test (or extend existing) that verifies `messenger info` renders:
- `Active daemon` row (e.g., `dunst`, `GNOME Shell`, `mako`).
- Both Linux helpers with correct install hints.
- Election order showing `dunstify` first when daemon is dunst, else `notify-send` first.

**Validation**: `cargo test -p messenger-lib --lib` and `cargo test -p messenger-lib --test desktop_helpers` both pass.

### Checkpoint 3

- Linux backend has full unit-test, integration-test, and edge-case coverage.
- `cargo test -p messenger-lib` passes on Linux runners.

---

## Phase 4 — Windows Functional Completeness

**Goal**: Prove that the Windows backend delivers every `Dispatch` feature reliably when helpers are present, handles image/AppID edge cases, and degrades gracefully.

### Step 4.1 — PNG oversize graceful-degradation test

Add a unit test to `snoretoast.rs` that:
- Creates a 2048×2048 PNG in a temp file.
- Sends a request with that image.
- Asserts the image is dropped from argv.
- Asserts `metadata["dropped"] == "image_too_large"`.
- Asserts the send still succeeds (helper does not fail).

### Step 4.2 — Duplicate action-label election guard test

Add a unit test to `snoretoast.rs` that:
- Builds a request with two actions that have identical labels (`label: "OK".into()` for both).
- Asserts `score()` returns `0` (ambiguous label→id mapping).
- Asserts election skips `snoretoast` and falls through to `burnttoast` or native.

### Step 4.3 — AppID registration failure test

Add a unit test to `windows.rs` that:
- Constructs `WindowsBackend` with `app_id: Some("Test.App")`.
- Mocks `shortcut_bootstrap_state` to return `Missing`.
- Sends a request with no helpers.
- Asserts `MissingConfiguration` error with the exact `WINDOWS_SETUP_REQUIRED` field.

(If mocking `shortcut_bootstrap_state` is hard, restructure it to accept an optional override in test builds, or test at the `WindowsBackend::check_bootstrap()` level.)

### Step 4.4 — BurntToast activation JSON parsing test

Add an integration test in `desktop_helpers.rs` that:
- Uses `stub_burnttoast` to emit a custom `__MESSENGER_ACTIVATION__` JSON line.
- Asserts the `BurntToastHelper` correctly parses `activation_type`, `activation_key`, and `reply_text`.

### Step 4.5 — Windows native fallback on non-Windows platforms

Similar to Step 3.3, add an integration test that:
- Builds `WindowsBackend::with_helpers(..., vec![])` on macOS.
- Calls `send()`.
- Asserts the expected transport error ("Windows toast delivery is only available on Windows hosts").

### Checkpoint 4

- Windows backend has full unit-test, integration-test, and edge-case coverage.
- `cargo test -p messenger-lib` passes on Windows runners.

---

## Phase 5 — CI, Documentation & Final Verification

**Goal**: Ensure the new tests run automatically and the feature is documented as functionally complete.

### Step 5.1 — Update GitHub Actions workflow

In `.github/workflows/` (or the equivalent CI config):

- Ensure the **Linux** test job runs `cargo test -p messenger-lib --features desktop --lib` and `cargo test -p messenger-lib --features desktop --test desktop_helpers`.
- Ensure the **Windows** test job runs the same commands.
- Ensure the **macOS** test job runs the same commands (it now covers all six helpers thanks to Phase 1).

Add a matrix entry or separate job that verifies `cargo check --all-features` on each OS.

### Step 5.2 — Update `messenger/docs/user-guide.md`

Replace language that implies helpers are "opportunistic" or "best effort" with language that states:

> On Linux and Windows, `messenger` uses installed helper utilities (`dunstify`, `notify-send`, `snoretoast`, `burnttoast`) as the **primary** delivery path. When helpers are present, interactive actions, inline replies, image attachments, and reliable notification replacement are fully supported. The native API (`notify-rust` on Linux, `winrt-notification` on Windows) remains as a fallback for simple notifications when no helper is installed.

### Step 5.3 — Update `AGENTS.md` or per-area docs

If the messenger area has a `docs/` file describing test conventions, note that:
- All desktop backend tests run on every OS.
- Stub binaries live in `messenger/lib/tests/bin/`.
- New helpers must include a stub binary and an integration test in `desktop_helpers.rs`.

### Step 5.4 — Final verification

Run the full test matrix:

```bash
# macOS
cargo test -p messenger-lib --features desktop --lib
cargo test -p messenger-lib --features desktop --test desktop_helpers
cargo test -p messenger-cli

# Linux (via Docker or CI)
cargo test -p messenger-lib --features desktop --lib
cargo test -p messenger-lib --features desktop --test desktop_helpers

# Windows (via CI)
cargo test -p messenger-lib --features desktop --lib
cargo test -p messenger-lib --features desktop --test desktop_helpers
```

### Checkpoint 5

- Zero test failures on macOS, Linux, and Windows.
- CI runs the full desktop helper test suite on all three platforms.
- Documentation reflects functionally complete status.

---

## Dependency Graph

```
Phase 1 (compilation fixes + naming bug)
    │
    ▼
Phase 2 (stub binaries + integration test harness)
    │
    ├── Phase 3 (Linux edge-case tests) ── can start after Phase 2 stub harness works
    ├── Phase 4 (Windows edge-case tests) ── can start after Phase 2 stub harness works
    │
    ▼
Phase 5 (CI + docs + verification)
```

**Parallelizable groups**:
- Steps 1.1, 1.2, 1.3, 1.4 are all independent and can land in any order (but merge together for a green build).
- Phase 3 and Phase 4 are fully parallelizable once Phase 2 is done.
- Phase 5 depends only on CI config being available.

---

## Success Criteria

| Criterion | Before Plan 2 | After Plan 2 |
|---|---|---|
| `linux.rs` compiles on macOS | ❌ | ✅ |
| `windows.rs` tests visible on macOS | ❌ (module not compiled) | ✅ |
| `helper_used` test failures | 3 on macOS | 0 everywhere |
| Stub binary integration tests | None | 6 stubs, 1 integration test file |
| Linux edge cases tested (daemon mismatch, old libnotify) | Partial | Full |
| Windows edge cases tested (PNG oversize, duplicate labels, AppID) | Partial | Full |
| CI runs Linux desktop helper tests | Unknown / manual | Automated |
| CI runs Windows desktop helper tests | Unknown / manual | Automated |
| Docs call Linux/Windows "functionally complete" | No | Yes |
