---
phases: 4
created: 2026-04-21
start_phase: 1
packages:
  - messenger
  - messenger-cli
source_files_during_phase_1:
  - messenger/lib/Cargo.toml
  - messenger/lib/src/capabilities.rs
  - messenger/lib/src/dispatch.rs
  - messenger/lib/src/lib.rs
  - messenger/lib/src/markdown/mod.rs
  - messenger/lib/src/message.rs
  - messenger/lib/src/prepared.rs
  - messenger/lib/src/prelude.rs
  - messenger/lib/src/provider/desktop/backend.rs
  - messenger/lib/src/provider/desktop/linux.rs
  - messenger/lib/src/provider/desktop/macos.rs
  - messenger/lib/src/provider/desktop/mod.rs
  - messenger/lib/src/provider/desktop/request.rs
  - messenger/lib/src/provider/desktop/windows.rs
  - messenger/lib/src/provider/discord.rs
  - messenger/lib/src/provider/discord_webhook.rs
  - messenger/lib/src/provider/mod.rs
  - messenger/lib/src/provider/signal.rs
  - messenger/lib/src/provider/slack.rs
  - messenger/lib/src/provider/slack_webhook.rs
  - messenger/lib/src/provider/telegram.rs
  - messenger/lib/src/provider/whatsapp.rs
  - messenger/lib/src/receipt.rs
  - messenger/lib/src/target.rs
  - messenger/lib/src/tests/builders.rs
  - messenger/lib/src/tests/discord_webhook_integration.rs
  - messenger/lib/src/tests/validation.rs
  - messenger/lib/src/validate.rs
  - messenger/cli/Cargo.toml
  - messenger/cli/src/config.rs
  - messenger/cli/src/desktop_setup.rs
  - messenger/cli/src/main.rs
  - messenger/cli/src/setup.rs
docs_updated_during_phase_1:
  - messenger/README.md
  - messenger/lib/README.md
  - messenger/cli/README.md
  - messenger/docs/user-guide.md
  - docs/dependencies.md
docs_created_during_phase_1:
  - messenger/docs/platforms/desktop.md
skills_files_updated_during_phase_1:
  - .claude/skills/messenger/SKILL.md
  - .claude/skills/messenger/providers.md
---

# OS Notifications Execution Plan

## Phase 1: Core Implementation

### Library Changes
- [x] Add `ProviderKind::Desktop` enum variant
- [x] Add `Target::desktop()` helper method
- [x] Add `Message::title` field and `title()` builder method
- [x] Replace `CapabilitySet::supports_attachments: bool` with `supported_attachment_kinds: BTreeSet<AttachmentKind>` (breaking change)
- [x] Add `DesktopPlatform` enum with macOS, Linux, Windows variants
- [x] Add `MessageRef::Desktop` receipt variant
- [x] Add `ProviderOverrides::Desktop(DesktopOverrides)` variant
- [x] Define `NotificationUrgency` and `NotificationIcon` enums
- [x] Create `DesktopNotificationProvider` struct with config and backend
- [x] Define `DesktopBackend` internal trait with async send method
- [x] Implement `DesktopConfig` with platform-specific nested configs

### Platform Backend Implementation
- [x] Implement Linux backend using `notify-rust` with zbus
- [x] Implement Windows backend using `winrt-notification`
- [x] Implement macOS backends: AppleScript fallback and native `UserNotifications.framework`
- [x] Add `MacOsNotificationStrategy` enum with Auto/AppleScript/Native options
- [x] Wire platform detection to select appropriate backend

### CLI Integration
- [x] Add `desktop` provider to route resolution
- [x] Add `RouteProvider::requires_target()` method (returns false for desktop)
- [x] Add new CLI flags: `--title`, `--subtitle`, `--icon`, `--category`, `--urgency`, `--timeout-ms`
- [x] Implement desktop route config serialization/deserialization
- [x] Add `messenger setup desktop` interactive flow with platform-specific prompts
- [x] Implement Windows Start Menu shortcut creation in setup (not in send)

### Validation Checkpoints
- [x] Verify library compiles with new breaking change to `CapabilitySet`
- [x] Test title defaulting logic (message → config → app name)
- [x] Validate attachment normalization (images supported, files rejected)
- [x] Confirm `messenger send --provider desktop` works without `--channel`
- [x] Verify Windows setup creates Start Menu shortcut and send fails without setup
- [x] Confirm macOS default strategy uses AppleScript (no auth prompt)

## Phase 2: Enhanced Desktop Features

### Parallelizable Work
- [ ] Improve macOS native delivery via `UserNotifications.framework`
- [ ] Add notification replacement APIs using `MessageRef::Desktop`
- [ ] Add notification dismissal APIs
- [ ] Expose richer categories and grouping where portable

### Validation Checkpoints
- [ ] Test native macOS notifications with explicit strategy config
- [ ] Verify replacement/dismissal functionality works across platforms
- [ ] Confirm backward compatibility with Phase 1 implementations

## Phase 3: Advanced Desktop Capabilities

### Parallelizable Work
- [ ] Support notification actions and callbacks for packaged apps
- [ ] Implement notification replacement/update in CLI
- [ ] Add support for progress notifications and badge counts

### Validation Checkpoints
- [ ] Test action callbacks in packaged application context
- [ ] Verify CLI replacement/update commands function correctly
- [ ] Confirm advanced notification types work on supported platforms

## Phase 4: Mobile Push Providers

### Parallelizable Work
- [ ] Implement separate `apns` provider for iOS push notifications
- [ ] Implement separate `fcm` provider for Android push notifications
- [ ] Add mobile-specific authentication and targeting models
- [ ] Create mobile-specific documentation and examples

### Validation Checkpoints
- [ ] Verify APNs provider integrates with Apple Push Notification service
- [ ] Confirm FCM provider works with Firebase Cloud Messaging
- [ ] Test mobile providers independently from desktop provider
- [ ] Validate mobile-specific capabilities and constraints

## Cross-Cutting Validation

### Documentation
- [x] Update `messenger/README.md` with desktop provider info
- [x] Update `messenger/lib/README.md` with API changes
- [x] Update `messenger/cli/README.md` with new flags and setup
- [x] Create `messenger/docs/platforms/desktop.md` platform guide

### Testing
- [x] Unit tests for all new library types and methods
- [x] Integration tests for CLI provider resolution and config
- [x] Platform-specific smoke tests (manual verification)
- [x] Breaking change verification for library consumers

### Release Preparation
- [x] Update release notes highlighting breaking change to `CapabilitySet`
- [x] Verify all acceptance criteria from specification are met
- [x] Confirm no filesystem mutations outside `~/.messenger/` during send
- [x] Validate error handling for all expected failure scenarios
