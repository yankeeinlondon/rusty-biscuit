---
phases: 6
created: 2026-04-27
start_phase: 2
source_files_during_phase_1:
  - sniff/lib/Cargo.toml
  - sniff/lib/src/programs/enums/categories.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/enums/mod.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/notification_helpers.rs
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/notification_helpers.rs
  - sniff/cli/tests/snapshots/snapshots__help_output.snap
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - messenger/lib/Cargo.toml
  - messenger/lib/src/provider/desktop/helpers/mod.rs
  - messenger/lib/src/provider/desktop/helpers/process.rs
  - messenger/lib/src/provider/desktop/helpers/dunstify.rs
  - messenger/lib/src/provider/desktop/helpers/notify_send.rs
  - messenger/lib/src/provider/desktop/helpers/election.rs
  - messenger/lib/src/provider/desktop/linux.rs
  - messenger/lib/src/provider/desktop/macos.rs
  - messenger/lib/src/provider/desktop/windows.rs
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/lib/src/provider/desktop/request.rs
  - messenger/cli/src/main.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - messenger/lib/src/provider/desktop/helpers/mod.rs
  - messenger/lib/src/provider/desktop/helpers/election.rs
  - messenger/lib/src/provider/desktop/helpers/terminal_notifier.rs
  - messenger/lib/src/provider/desktop/helpers/alerter.rs
  - messenger/lib/src/provider/desktop/macos.rs
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/cli/src/main.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - messenger/lib/src/provider/desktop/helpers/mod.rs
  - messenger/lib/src/provider/desktop/helpers/snoretoast.rs
  - messenger/lib/src/provider/desktop/helpers/burnttoast.rs
  - messenger/lib/src/provider/desktop/windows.rs
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/cli/src/main.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
source_files_during_phase_5:
  - messenger/cli/Cargo.toml
  - messenger/cli/src/config.rs
  - messenger/cli/src/info.rs
  - messenger/cli/src/install.rs
  - messenger/cli/src/main.rs
  - messenger/cli/src/setup.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .opencode/skill/messenger/SKILL.md
source_files_during_phase_6:
  - messenger/lib/src/lib.rs
  - messenger/lib/src/prelude.rs
  - messenger/lib/src/receipt.rs
docs_updated_during_phase_6:
  - messenger/docs/user-guide.md
docs_created_during_phase_6: []
skills_files_updated_during_phase6: []
packages:
  - sniff
  - sniff-cli
  - messenger
  - messenger-cli
---

# Execution Plan: Leveraging Desktop Notification Helpers

## Summary

Wire six third-party notification helper CLIs (2 per OS) into messenger's desktop provider via a new `HelperBackend` trait in messenger and a new `notification_helpers` detection category in sniff. The work is phased so each phase is independently mergeable.

---

## Phase 1 — Sniff Detection Layer

**Goal**: Add `notification_helpers` as a new program category in the `sniff` library and CLI.

### Step 1.1 — Define `NotificationHelper` enum and metadata

- **Files**: `sniff/lib/src/programs/enums/categories.rs`
- **Action**: Add a `NotificationHelper` enum with six variants: `TerminalNotifier`, `Alerter`, `SnoreToast`, `BurntToast`, `Dunstify`, `NotifySend`. Derive the standard strum traits (`Display`, `EnumString`, `EnumIter`, `EnumCount`, `IntoStaticStr`). Implement `ProgramMetadata` with `binary_name()`, `display_name()`, `description()`, `website()`, platform filters, version flags, and install hints per helper.
- **Validation**: `cargo check -p sniff-lib` compiles. Unit test: each variant returns correct `binary_name()`.

### Step 1.2 — Implement `InstalledNotificationHelpers` detector

- **Files**: New `sniff/lib/src/programs/notification_helpers.rs`
- **Action**: Create `InstalledNotificationHelpers` struct following the pattern of `InstalledEditors` / `InstalledTtsClients`. Use the generic `CategoryDetector<NotificationHelper>`. Handle:
  - Standard PATH probes for 5 CLI binaries (all except BurntToast).
  - BurntToast: PowerShell module probe (`pwsh -NoProfile -Command "if (Get-Module -ListAvailable BurntToast) { 'yes' } else { 'no' }"`). Cache per process.
  - Version probing per helper (per tech design §4.2 table).
  - `active_daemon: Option<NotificationDaemon>` field (Linux only) — zbus call to `org.freedesktop.Notifications.GetServerInformation`.
- **Parallelizable with**: Step 1.1 (can write the struct while the enum lands, but merge depends on 1.1).
- **Validation**: Unit test with mocked `ExecutableIndex` returns correct installed/version/path for each helper. Linux-only daemon test gated behind `#[cfg(target_os = "linux")]`.

### Step 1.3 — Wire into `ProgramsInfo` and module re-exports

- **Files**: `sniff/lib/src/programs/mod.rs`, `sniff/lib/src/lib.rs`
- **Action**: Add `pub mod notification_helpers;` to programs mod. Add `pub notification_helpers: InstalledNotificationHelpers` field to `ProgramsInfo`. Add parallel detection call in `ProgramsInfo::detect()` alongside the existing 8 categories. Re-export `NotificationHelper`, `InstalledNotificationHelpers` from `sniff::programs`.
- **Depends on**: Step 1.1, Step 1.2.
- **Validation**: `sniff::programs::ProgramsInfo::detect()` includes `notification_helpers` field. Existing tests still pass.

### Step 1.4 — Add `sniff notification-helpers` CLI subcommand

- **Files**: `sniff/cli/src/commands.rs`, `sniff/cli/src/args.rs`, `sniff/cli/src/main.rs`, new `sniff/cli/src/output/notification_helpers.rs`
- **Action**: Add `NotificationHelpers` variant to the CLI command enum. Wire subcommand `sniff notification-helpers [--json]`. Render text output (table of helpers with install hints) and JSON output.
- **Depends on**: Step 1.3.
- **Validation**: `sniff notification-helpers` prints a table. `sniff notification-helpers --json` produces valid JSON. `sniff programs` output now includes notification helpers section.

### Checkpoint 1

- `cargo test -p sniff-lib` passes.
- `cargo test -p sniff-cli` passes.
- `sniff notification-helpers` runs on macOS (detects terminal-notifier/alerter presence).
- No changes to messenger crate yet.

---

## Phase 2 — Helper Trait + Linux Helpers

**Goal**: Introduce the `HelperBackend` abstraction in messenger and implement the two Linux helpers (`dunstify`, `notify-send`) with full backend integration.

### Step 2.1 — Define `HelperBackend` trait and supporting types

- **Files**: New `messenger/lib/src/provider/desktop/helpers/mod.rs`
- **Action**: Define:
  - `HelperName` — re-export from sniff's `NotificationHelper` (or a newtype if API boundary requires it), exposed as `messenger::desktop::HelperName`.
  - `HelperCapabilities` struct (boolean flags: `actions`, `reply`, `image`, `sound`, `replace`, `group`, `blocking`).
  - `HelperError` enum (`NotPresent`, `Unsupported`, `Exited`, `Timeout`, `Parse`, `Io`).
  - `HelperBackend` async trait with `name()`, `capabilities()`, `score()`, `send()`, `replace()`.
  - `HelperAttempt` struct for fallback tracking.
- **Validation**: `cargo check -p messenger-lib` compiles.

### Step 2.2 — Implement `process.rs` helper utilities

- **Files**: New `messenger/lib/src/provider/desktop/helpers/process.rs`
- **Action**: Implement `spawn_helper()` — a wrapper around `tokio::process::Command` with timeout handling, stderr capture, and exit-code classification. Shared by all helper implementations.
- **Parallelizable with**: Step 2.1.
- **Validation**: Unit test: spawn `echo hello` and capture stdout; spawn `sleep 10` with 100ms timeout returns `HelperError::Timeout`.

### Step 2.3 — Implement `dunstify` helper

- **Files**: New `messenger/lib/src/provider/desktop/helpers/dunstify.rs`
- **Action**: Implement `DunstifyHelper` struct with `HelperBackend`. Include `build_args()` method (unit-testable seam) and stdout parser for `--printid` + `--wait` action key. Score: `90` interactive (if `daemon_is_dunst`), `70` notice-only, `0` if daemon is not dunst.
- **Depends on**: Step 2.1.
- **Validation**: Unit tests for `build_args()` — snapshot argv for notice-only, interactive with actions, replace, urgency variants. Unit tests for `score()` — table-driven. Unit tests for `parse_output()` with canned stdout.

### Step 2.4 — Implement `notify-send` helper

- **Files**: New `messenger/lib/src/provider/desktop/helpers/notify_send.rs`
- **Action**: Implement `NotifySendHelper` struct with `HelperBackend`. Handle libnotify version check for actions support (`>= 0.7.8`). Score: `60` universal, `40` with actions on old libnotify.
- **Depends on**: Step 2.1.
- **Parallelizable with**: Step 2.3.
- **Validation**: Same test matrix as dunstify: `build_args()` snapshots, `score()` table, `parse_output()`.

### Step 2.5 — Implement election algorithm

- **Files**: New `messenger/lib/src/provider/desktop/helpers/election.rs`
- **Action**: Implement `elect_helper()` — given `Vec<Arc<dyn HelperBackend>>`, `DesktopNotificationRequest`, and optional `prefer_helpers`, iterate helpers sorted by preference, filter by `score > 0`, return ordered attempt list.
- **Depends on**: Step 2.1.
- **Parallelizable with**: Steps 2.3, 2.4.
- **Validation**: Unit tests: correct ordering for notice-only vs interactive dispatches; `prefer_helpers` reorder; score-0 filtering; empty vec → native fallback.

### Step 2.6 — Extend `LinuxBackend` with helper integration

- **Files**: `messenger/lib/src/provider/desktop/linux.rs`
- **Action**: Add `helpers: Vec<Arc<dyn HelperBackend>>` field to `LinuxBackend`. In `new()`: call `sniff::detect_notification_helpers()`, filter to Linux-relevant helpers (`Dunstify`, `NotifySend`), construct helper structs, sort by config `prefer_helpers` then default order. Update `send()` to iterate helpers before native fallback. Update `replace()` to route via `replace_helper_hint`.
- **Depends on**: Steps 2.1–2.5.
- **Validation**: Integration test with stub `dunstify`/`notify-send` binaries on `PATH`: send notice-only → elected helper used; send with actions → dunstify elected (when daemon==dunst); helper failure → fallback to next helper → native.

### Step 2.7 — Add `replace_helper_hint` to `DesktopNotificationRequest`

- **Files**: `messenger/lib/src/provider/desktop/request.rs`
- **Action**: Add `pub replace_helper_hint: Option<HelperName>` field to `DesktopNotificationRequest`.
- **Parallelizable with**: Steps 2.3–2.5 (independent of them, but needed by 2.6).
- **Validation**: Existing tests compile (new field is `Option` with default `None`).

### Checkpoint 2

- `cargo test -p messenger-lib` passes.
- Linux stub-helper integration test passes.
- `cargo test -p sniff-lib` still passes (no regressions).
- Feature is live on Linux only; macOS/Windows unchanged.

---

## Phase 3 — macOS Helpers

**Goal**: Implement `terminal-notifier` and `alerter` helpers and wire them into `MacOsBackend`.

### Step 3.1 — Implement `terminal-notifier` helper

- **Files**: New `messenger/lib/src/provider/desktop/helpers/terminal_notifier.rs`
- **Action**: Implement `TerminalNotifierHelper`. Score: `80` notice-only, `0` when actions/reply present. Build args: `-title`, `-subtitle`, `-message`, `-contentImage`, `-sound`, `-group`, `-remove` (replace), `-ignoreDnD`, `-open`. Sound mapping: urgency `Critical` → `Basso`, `Low` → suppress. Timeout: 5s.
- **Parallelizable with**: Step 3.2.
- **Validation**: `build_args()` snapshot tests. `score()` table tests. `parse_output()` tests.

### Step 3.2 — Implement `alerter` helper

- **Files**: New `messenger/lib/src/provider/desktop/helpers/alerter.rs`
- **Action**: Implement `AlerterHelper`. Score: `90` interactive (actions/reply), `30` notice-only. Build args: `--title`, `--subtitle`, `--message`, `--contentImage`, `--sound`, `--actions "id1|Label1,id2|Label2"`, `--reply`, `--closeLabel`, `--json`. Parse JSON output for activation. No timeout for interactive; 60s ceiling for notice-only.
- **Parallelizable with**: Step 3.1.
- **Validation**: Same test matrix. Additional: JSON parse test for each `activationType` variant.

### Step 3.3 — Extend `MacOsBackend` with helper integration

- **Files**: `messenger/lib/src/provider/desktop/macos.rs`
- **Action**: Add `helpers: Vec<Arc<dyn HelperBackend>>` field. In `new()`: sniff detection, filter to macOS helpers, construct, sort. Update `send()` with helper election + fallback. Update `replace()` to route via hint (alerter returns `Unsupported` for replace).
- **Depends on**: Steps 3.1, 3.2.
- **Validation**: macOS-flagged stub-helper integration test (`#[cfg(target_os = "macos")]`). Manual test: send notice → terminal-notifier elected; send with actions → alerter elected.

### Checkpoint 3

- `cargo test -p messenger-lib` passes.
- macOS stub-helper integration test passes (or is correctly skipped on non-macOS CI).
- No regressions on Linux helpers.

---

## Phase 4 — Windows Helpers

**Goal**: Implement `snoretoast` and `burnttoast` helpers with AppID auto-registration and wire into `WindowsBackend`.

### Step 4.1 — Implement `snoretoast` helper

- **Files**: New `messenger/lib/src/provider/desktop/helpers/snoretoast.rs`
- **Action**: Implement `SnoreToastHelper` with `app_id` field. Score: `90` always (default Windows choice). Build args: `-appID`, `-t`, `-m`, `-p` (PNG, pre-validate ≤1024×1024, ≤200KB), `-s`, `-b "Label1;Label2"` (build `id_by_label` map), `-tb` (reply), `-id`. Parse exit codes (0–5, -1). Map stdout label back to action id via the label table. AppID registration in constructor.
- **Validation**: `build_args()` snapshots. Exit-code mapping tests. PNG size validation test (oversize → `dropped: image_too_large`). Duplicate-label score→0 test.

### Step 4.2 — Implement `burnttoast` helper

- **Files**: New `messenger/lib/src/provider/desktop/helpers/burnttoast.rs`
- **Action**: Implement `BurntToastHelper` with `pwsh_path` and `app_id`. Score: `40` (secondary to snoretoast). Pipe PowerShell script over stdin. Template includes `New-BurntToastNotification` + `OnActivated` handler that writes `__MESSENGER_ACTIVATION__\t<json>` to stdout. AppID registration via `New-BTAppId`.
- **Parallelizable with**: Step 4.1.
- **Validation**: `build_args()` / script template tests. Parse activation marker from stdout.

### Step 4.3 — Extend `WindowsBackend` with helper integration

- **Files**: `messenger/lib/src/provider/desktop/windows.rs`
- **Action**: Add `helpers` field. In `new()`: sniff detection, filter to Windows helpers, construct with `app_id` from `WindowsDesktopConfig`. AppID auto-registration with `OnceCell` caching. Update `send()` / `replace()`.
- **Depends on**: Steps 4.1, 4.2.
- **Validation**: Windows-flagged stub-helper integration test (`#[cfg(target_os = "windows")]`).

### Checkpoint 4

- `cargo test -p messenger-lib` passes.
- Windows stub-helper integration test passes (or correctly skipped on non-Windows CI).
- All three OS backends now support helpers. Feature is functionally complete at the library level.

---

## Phase 5 — CLI Surface

**Goal**: Add `messenger info` and `messenger install` commands, TOML config for `prefer_helpers`, and env var override.

### Step 5.1 — Parse `prefer_helpers` in CLI config

- **Files**: `messenger/cli/src/config.rs`
- **Action**: Add `prefer_helpers: Vec<String>` field to each per-OS desktop config section in the TOML schema. Parse `NotificationHelper` from string using strum's `FromStr`. Merge `MESSENGER_DESKTOP_PREFER_HELPERS` env var (comma-separated) ahead of config file values.
- **Validation**: Unit test: parse TOML with `prefer_helpers = ["dunstify"]` → correct vec. Unit test: env var override.

### Step 5.2 — Implement `messenger info` command

- **Files**: New `messenger/cli/src/info.rs`, `messenger/cli/src/main.rs`
- **Action**: Implement `messenger info [--json | --plain]`. Render:
  - Host OS, active daemon (Linux), bundle_id/app_id.
  - Notification helpers table (name, installed, version, install hint) via `biscuit-terminal` `Prose`.
  - Election order for this host.
  - Configured routes.
  - JSON mode: flat record matching text layout.
- **Depends on**: Phase 1 (sniff detection).
- **Parallelizable with**: Step 5.1.
- **Validation**: Snapshot test with `insta` using fixed sniff JSON fixture. Both `--plain` and `--json` modes.

### Step 5.3 — Implement `messenger install` command

- **Files**: New `messenger/cli/src/install.rs`, `messenger/cli/src/main.rs`
- **Action**: Implement `messenger install [--yes] [--helper <name>]…`. Steps:
  1. Run sniff detection.
  2. Filter to uninstalled helpers (or `--helper` restricted list).
  3. Present `inquire::MultiSelect` (skipped with `--yes`).
  4. Print install plan with elevation badges.
  5. Confirm via `inquire::Confirm`.
  6. Execute via sniff's `execute_install` pipeline.
  7. Re-detect and print updated `messenger info` table.
- **Depends on**: Phase 1 (sniff detection + install infrastructure).
- **Parallelizable with**: Steps 5.1, 5.2.
- **Validation**: Snapshot test with mocked sniff result. Verify command wiring in `messenger --help`.

### Step 5.4 — Wire into `messenger setup desktop`

- **Files**: `messenger/cli/src/desktop_setup.rs`
- **Action**: At the end of `messenger setup desktop`, invoke the `messenger info` renderer so the user sees what helpers they gained.
- **Depends on**: Step 5.2.
- **Validation**: Manual test of `messenger setup desktop` shows helpers table.

### Checkpoint 5

- `cargo test -p messenger-cli` passes.
- `messenger info` renders correctly on macOS.
- `messenger install` dry-run (with `--yes` and mocked sniff) produces expected plan.
- Snapshot tests stable.

---

## Phase 6 — Receipt Convenience API + Documentation

**Goal**: Add typed accessors on `SendReceipt` and update documentation.

### Step 6.1 — Add `SendReceipt` convenience methods

- **Files**: `messenger/lib/src/receipt.rs`
- **Action**: Add:
  - `helper_used() -> Option<&str>`
  - `activation() -> Option<Activation>` — parse `activation_type` + value into typed enum.
  - `reply_text() -> Option<&str>`
  - `Activation` enum: `Action(&str)`, `Reply(&str)`, `Dismissed`, `Timeout`, `ContentClicked`.
- **Validation**: Unit tests: receipt with full helper metadata parses correctly; receipt without helper metadata returns `None`.

### Step 6.2 — Documentation pass

- **Files**: `messenger/docs/user-guide.md`, module-level rustdoc on new public types.
- **Action**: Document:
  - Helper detection and election behavior.
  - `messenger info` and `messenger install` CLI usage.
  - `prefer_helpers` TOML config and env var.
  - `SendReceipt` activation accessors.
  - Per-helper capability notes and limitations.
- **Depends on**: Step 6.1.
- **Validation**: `cargo doc -p messenger-lib --no-deps` succeeds without warnings on public items.

### Checkpoint 6

- `cargo test -p messenger-lib` passes.
- `cargo doc -p messenger-lib --no-deps` clean.
- `cargo test -p messenger-cli` passes.
- Full feature is complete and documented.

---

## Dependency Graph

```
Phase 1 (sniff detection)
    │
    ▼
Phase 2 (trait + Linux helpers)
    │
    ├── Phase 3 (macOS helpers)   ── can start after Phase 2.1
    ├── Phase 4 (Windows helpers) ── can start after Phase 2.1
    │
    ▼
Phase 5 (CLI surface)            ── needs Phase 1 + Phase 2 election logic
    │
    ▼
Phase 6 (receipt API + docs)
```

**Parallelizable groups**:
- Within Phase 1: Steps 1.1 and 1.2 can be developed in parallel (merge 1.1 first).
- Within Phase 2: Steps 2.3, 2.4, 2.5, 2.7 are all parallelizable after 2.1 lands.
- Phases 3 and 4 are fully parallelizable with each other after Phase 2.1.
- Within Phase 5: Steps 5.1, 5.2, 5.3 are parallelizable.
