---
phases: 7
created: 2026-04-19
start_phase: 7
packages:
  - messenger
source_files_during_phase_1:
  - messenger/lib/src/attachment.rs
  - messenger/lib/src/capabilities.rs
  - messenger/lib/src/message.rs
  - messenger/lib/src/prepared.rs
  - messenger/lib/src/provider/discord.rs
  - messenger/lib/src/provider/discord_webhook.rs
  - messenger/lib/src/provider/signal.rs
  - messenger/lib/src/provider/slack.rs
  - messenger/lib/src/provider/slack_webhook.rs
  - messenger/lib/src/provider/telegram.rs
  - messenger/lib/src/provider/whatsapp.rs
  - messenger/lib/src/tests/builders.rs
  - messenger/lib/src/tests/discord_webhook_integration.rs
  - messenger/lib/src/tests/validation.rs
  - messenger/lib/src/validate.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - messenger/lib/Cargo.toml
  - messenger/lib/src/lib.rs
  - messenger/lib/src/prelude.rs
  - messenger/lib/src/receipt.rs
  - messenger/lib/src/target.rs
  - messenger/lib/src/dispatch.rs
  - messenger/lib/src/validate.rs
  - messenger/lib/src/markdown/mod.rs
  - messenger/lib/src/provider/mod.rs
  - messenger/lib/src/provider/desktop/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - messenger/lib/src/lib.rs
  - messenger/lib/src/prelude.rs
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/lib/src/provider/desktop/backend.rs
  - messenger/lib/src/provider/desktop/request.rs
  - messenger/lib/src/provider/mod.rs
  - messenger/lib/src/tests/validation.rs
  - messenger/lib/src/validate.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - messenger/lib/Cargo.toml
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/lib/src/provider/desktop/linux.rs
  - messenger/lib/src/provider/desktop/macos.rs
  - messenger/lib/src/provider/desktop/windows.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - messenger/cli/Cargo.toml
  - messenger/cli/src/config.rs
  - messenger/cli/src/main.rs
  - messenger/cli/src/setup.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
source_files_during_phase_6:
  - messenger/cli/src/desktop_setup.rs
  - messenger/cli/src/main.rs
  - messenger/cli/src/setup.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - messenger/README.md
  - messenger/lib/README.md
  - messenger/cli/README.md
  - messenger/docs/user-guide.md
  - docs/dependencies.md
docs_created_during_phase_7:
  - messenger/docs/platforms/desktop.md
skills_files_updated_during_phase_7:
  - .claude/skills/messenger/SKILL.md
  - .claude/skills/messenger/providers.md
packages:
  - messenger
---

# Desktop Notifications Execution Plan

Source spec: [spec.md](spec.md)

## Conventions And Key Decisions

- Add a new optional library feature flag, `desktop`, and enable it from `messenger-cli` by default.
- Keep the public provider model consistent with the rest of the crate: `ProviderKind::Desktop`, `Target::desktop()`, `ProviderOverrides::Desktop`, and typed desktop receipts.
- Use a dedicated desktop submodule under `messenger/lib/src/provider/desktop/` so OS-specific code does not sprawl across the existing flat provider modules.
- Do not make `send` mutate host state outside `~/.messenger/`. Windows Start Menu shortcut and AUMID registration happen only during `messenger setup desktop`.
- Treat `Message::title` as portable content, but do not silently inject it into non-desktop providers. Make title-only sends valid only when the resolved provider is `desktop`.

## Phase 1 - Portable Message And Capability Migration

Goal: land the shared model changes that the desktop provider depends on, including the breaking `CapabilitySet` attachment migration.

Primary files:
- `messenger/lib/src/message.rs`
- `messenger/lib/src/prepared.rs`
- `messenger/lib/src/capabilities.rs`
- `messenger/lib/src/validate.rs`
- `messenger/lib/src/provider/discord.rs`
- `messenger/lib/src/provider/discord_webhook.rs`
- `messenger/lib/src/provider/slack.rs`
- `messenger/lib/src/provider/signal.rs`
- `messenger/lib/src/provider/whatsapp.rs`
- `messenger/lib/src/provider/telegram.rs`
- `messenger/lib/src/tests/builders.rs`
- `messenger/lib/src/tests/validation.rs`

Steps:

1.1 Add `title: Option<String>` to `Message`, initialize it in all constructors, and add `Message::title(...)`.

1.2 Extend `PreparedMessage` with read access to `title` so provider code can consume title/body independently instead of treating everything as one rendered string.

1.3 Replace `CapabilitySet::supports_attachments: bool` with `supported_attachment_kinds: BTreeSet<AttachmentKind>`.
    - Update `CapabilitySet::all()` and `CapabilitySet::none()`.
    - Keep the rest of the capability API unchanged for this phase.

1.4 Update `validate.rs::normalize_dispatch` to filter attachments by kind instead of using a single boolean.
    - Best-effort mode drops unsupported attachment kinds and emits a warning.
    - Strict mode fails with `MessengerError::UnsupportedFeature { feature: "attachments" }`.
    - Do not make title-only messages valid yet; provider-aware validation lands in Phase 3.

1.5 Update every existing provider capability constant to explicit attachment kind sets.
    - Discord and Discord-Webhook: full set.
    - Slack, Signal, WhatsApp, Telegram: empty set for now.

Validation checkpoint:
- `cargo check -p messenger --all-features`
- `cargo test -p messenger --lib --all-features`
- Existing provider tests still pass with the new capability field and no desktop code present.

Parallelization:
- Steps `1.1` to `1.4` are serial.
- Step `1.5` and the related test fixture updates can be done in parallel once the shared types compile.

## Phase 2 - Desktop Public API Surface And Crate Wiring

Goal: add the new desktop types, feature flag, and module registration without yet implementing platform sends.

Primary files:
- `messenger/lib/Cargo.toml`
- `messenger/lib/src/lib.rs`
- `messenger/lib/src/target.rs`
- `messenger/lib/src/receipt.rs`
- `messenger/lib/src/dispatch.rs`
- `messenger/lib/src/provider/mod.rs`

Steps:

2.1 Add the `desktop` feature to `messenger/lib/Cargo.toml` and wire in the platform dependencies needed for Phase 4.
    - Linux: `notify-rust`
    - Windows: `winrt-notification`
    - macOS native path: `objc2-user-notifications`
    - Shared helpers as needed, for example `uuid`

2.2 Add `ProviderKind::Desktop`, `DesktopPlatform`, and `MessageRef::Desktop { platform, notification_id }` in `receipt.rs`.

2.3 Add a zero-data desktop target shape in `target.rs` and expose `Target::desktop()`.
    - Prefer `Target::Desktop(DesktopTarget)` to match the enum-variant-plus-typed-struct pattern already used elsewhere.

2.4 Add `DesktopOverrides`, `NotificationUrgency`, and `NotificationIcon` in `dispatch.rs`, then extend `ProviderOverrides` with a desktop variant.

2.5 Register the new provider module in `provider/mod.rs` and re-export the new public desktop types from `lib.rs`.

Validation checkpoint:
- `cargo check -p messenger --features desktop`
- `cargo check -p messenger --all-features`
- Desktop types serialize and compile cleanly without any backend implementation.

Parallelization:
- Steps `2.2`, `2.3`, and `2.4` can proceed in parallel after `2.1` lands.
- `2.5` is the final integration step for the phase.

## Phase 3 - Desktop Provider Core, Request Mapping, And Provider-Aware Validation

Goal: build the internal desktop provider abstraction and make `plan_send()` capable of validating title-only desktop notifications without loosening validation for other providers.

Primary files:
- `messenger/lib/src/provider/desktop/mod.rs`
- `messenger/lib/src/provider/desktop/request.rs`
- `messenger/lib/src/provider/desktop/backend.rs`
- `messenger/lib/src/provider/mod.rs`
- `messenger/lib/src/validate.rs`
- `messenger/lib/src/prepared.rs`

Steps:

3.1 Create the desktop provider scaffolding:
    - `DesktopNotificationProvider`
    - `DesktopConfig`
    - `WindowsDesktopConfig`
    - `MacOsDesktopConfig`
    - `LinuxDesktopConfig`
    - `MacOsNotificationStrategy`
    - internal `DesktopBackend` trait

3.2 Add a normalized internal request model, for example `DesktopNotificationRequest`, that preserves title, body, subtitle, image, silent flag, category, urgency, timeout, icon, and replacement metadata separately.

3.3 Make validation provider-aware.
    - Resolve `ProviderKind` before the final "message has content" gate.
    - Keep the current body/attachment/location rule for all existing providers.
    - Permit title-only messages only when the resolved provider is `desktop`.

3.4 Implement desktop request construction from `Message + Dispatch + DesktopConfig + DesktopOverrides`.
    - Title precedence: `message.title` -> `config.default_title` -> `config.app_name`
    - Markdown: render to plain text
    - Attachments: accept only images, reduce multiple images to the first image in best-effort mode
    - Location: drop in best-effort, fail in strict
    - Non-image attachments: drop in best-effort, fail in strict

3.5 Add provider tests using a fake backend to prove request construction, title defaulting, strict-vs-best-effort behavior, and typed receipt generation.

Validation checkpoint:
- `cargo test -p messenger desktop_request --all-features` or equivalent targeted desktop unit tests
- `cargo test -p messenger --lib --all-features`
- A title-only desktop send plans successfully, while a title-only Slack or Signal send still fails before transport.

Parallelization:
- `3.1` and test scaffolding for `3.5` can start together.
- `3.3` and `3.4` are serial because request construction depends on the provider-aware validation rule.

## Phase 4 - Platform Backend Implementations

Goal: implement Linux, macOS, and Windows backends behind the shared desktop provider API.

Primary files:
- `messenger/lib/src/provider/desktop/linux.rs`
- `messenger/lib/src/provider/desktop/macos.rs`
- `messenger/lib/src/provider/desktop/windows.rs`
- `messenger/lib/src/provider/desktop/mod.rs`

Steps:

4.1 Implement the Linux backend with `notify-rust`.
    - Map title/body/app name/icon/urgency/category/timeout/silent hints.
    - Return the daemon notification ID in `MessageRef::Desktop`.

4.2 Implement the macOS backend with two explicit strategies.
    - `Auto` maps to AppleScript in Phase 1 delivery.
    - `NativeUserNotifications` uses `objc2-user-notifications` only when explicitly configured.
    - Mark AppleScript fallback delivery in receipt metadata.

4.3 Implement the Windows backend with `winrt-notification`.
    - Use configured or default AUMID.
    - Detect missing Start Menu shortcut/bootstrap state and return `MessengerError::MissingConfiguration` with the remediation text from the spec.
    - Do not create or repair the shortcut during send.

4.4 Wire runtime backend selection in `DesktopNotificationProvider::new(...)` or equivalent constructor so the active host OS picks the correct backend behind the same public provider.

Validation checkpoint:
- Host-platform unit tests pass.
- Ignored or manual smoke tests exist for Linux, macOS, and Windows without becoming CI requirements.
- On macOS `strategy = auto` does not invoke native authorization.
- On Windows an unbootstrapped send fails before any toast attempt.

Parallelization:
- `4.1`, `4.2`, and `4.3` are parallelizable once Phase 3 is stable.
- `4.4` is the serial merge point.

## Phase 5 - CLI Route, Send Flags, And Provider Registration

Goal: wire desktop into the existing `messenger send` command and config model.

Primary files:
- `messenger/cli/Cargo.toml`
- `messenger/cli/src/config.rs`
- `messenger/cli/src/main.rs`

Steps:

5.1 Enable the library `desktop` feature from `messenger-cli`.

5.2 Add `RouteProvider::Desktop` and a typed `RouteConfig::Desktop` that matches the spec's portable fields plus nested `windows`, `macos`, and `linux` blocks.

5.3 Add `RouteProvider::requires_target()` in `config.rs` and update route resolution in `main.rs` so `--provider desktop` does not require `--channel`, while all existing chat providers still do.

5.4 Extend the `Send` command arguments in `main.rs` with:
    - `--title`
    - `--subtitle`
    - `--icon`
    - `--category`
    - `--urgency`
    - `--timeout-ms`

5.5 Map the new CLI flags into `Message::title(...)` and `ProviderOverrides::Desktop(...)`, register `DesktopNotificationProvider`, and make `build_target(...)` return `Target::desktop()` for desktop routes.

5.6 Add CLI tests for:
    - desktop config round-trip
    - `--provider desktop` without `--channel`
    - flag parsing for `--title`, `--urgency`, and `--timeout-ms`

Validation checkpoint:
- `cargo test -p messenger-cli`
- `cargo check -p messenger-cli`
- `cargo run -p messenger-cli -- --help` shows `desktop` in provider choices and the new desktop-only flags in `send` help.

Parallelization:
- `5.2` and the serde tests from `5.6` can proceed together once the config shape is settled.
- `5.3`, `5.4`, and `5.5` are best done serially in `main.rs`.

## Phase 6 - Interactive Setup And Windows Bootstrap

Goal: implement `messenger setup desktop`, including Windows Start Menu shortcut registration, without violating the send-path side-effect constraint.

Primary files:
- `messenger/cli/src/setup.rs`
- `messenger/cli/src/config.rs`
- `messenger/cli/src/main.rs`
- optional new helper module if Windows setup logic needs isolation

Steps:

6.1 Add a desktop setup flow in `setup.rs` that prompts for:
    - route name
    - app name
    - default title
    - icon name or path
    - urgency
    - timeout
    - platform-specific optional values

6.2 Add platform-specific prompt branches.
    - Windows: optional app ID, with clear explanation that setup writes the Start Menu shortcut.
    - macOS: optional bundle ID and strategy choice, with explicit explanation of `auto` versus `native_user_notifications`.
    - Linux: optional desktop entry.

6.3 Implement Windows shortcut/AUMID registration during setup completion only.
    - Report the written shortcut path on success.
    - Fail setup cleanly with remediation guidance if registration fails.

6.4 Add setup-path tests that verify persisted config, and add Windows-specific tests for shortcut path generation or registration helper behavior where the OS APIs can be isolated.

6.5 Verify the acceptance constraint that `messenger send --provider desktop` never writes outside `~/.messenger/`.

Validation checkpoint:
- `cargo test -p messenger-cli`
- Manual Windows run: `messenger setup desktop` creates the shortcut, then `messenger send --route <desktop-route>` succeeds.
- Manual Windows negative case: removing the shortcut makes `send` return the expected `MissingConfiguration` remediation.

Parallelization:
- `6.1` and `6.2` are serial.
- `6.4` can be developed alongside `6.3` once the helper boundary is defined.

## Phase 7 - Documentation, Dependency Drift, And Release Notes

Goal: ship the feature with the repo maintenance updates required by the spec and this workspace.

Primary files:
- `messenger/README.md`
- `messenger/lib/README.md`
- `messenger/cli/README.md`
- `messenger/docs/user-guide.md`
- `messenger/docs/platforms/desktop.md` (new)
- `docs/dependencies.md`
- `.claude/skills/messenger/SKILL.md`
- optional `.claude/skills/messenger/providers.md` or `cli-reference.md` if the workflow descriptions need refresh

Steps:

7.1 Update the three README files plus `messenger/docs/user-guide.md` with desktop provider usage, config examples, and platform caveats.

7.2 Add `messenger/docs/platforms/desktop.md` covering:
    - Linux D-Bus path
    - Windows AUMID and Start Menu shortcut requirement
    - macOS AppleScript versus native strategy

7.3 Update `docs/dependencies.md` for any new crates added in Phase 2.

7.4 Update the messenger skill docs if the public provider matrix or CLI workflow changed materially.

7.5 Add release-note text for the breaking `CapabilitySet` change from `supports_attachments` to `supported_attachment_kinds`.

Validation checkpoint:
- All referenced commands and config snippets match the final implementation.
- Documentation explicitly calls out the Windows setup prerequisite and the macOS `auto` behavior.
- Breaking change note is present in the release materials.

Parallelization:
- `7.1`, `7.2`, and `7.3` can run in parallel after code freeze.
- `7.4` and `7.5` are short serial cleanup tasks.

## Acceptance Matrix

| Spec requirement | Phase coverage |
| --- | --- |
| `messenger send --provider desktop` does not write outside `~/.messenger/` | Phases 4, 6 |
| Windows requires prior `setup desktop` and returns `MissingConfiguration` otherwise | Phases 4, 6 |
| `setup desktop` on Windows creates the Start Menu shortcut or fails cleanly | Phase 6 |
| macOS `strategy: auto` does not trigger native authorization | Phase 4 |
| `--provider desktop` works without `--channel` | Phase 5 |
| Desktop config round-trips cleanly | Phase 5 |
| Breaking `CapabilitySet` change is called out in release notes | Phases 1, 7 |
