---
ready: true
---

# Review: OS Notifications Feature

This review covers the implementation of the OS notifications feature as defined in the Phase 1 specification.

## Summary

The implementation is very high quality and actually exceeds the Phase 1 requirements by delivering several Phase 2 features (notification replacement and dismissal) ahead of schedule. The architecture is clean, with a solid separation between the portable provider logic and platform-specific backends.

## Gaps & Findings

### 1. Incomplete Linux Dismissal
The `LinuxBackend` implements `send` and `replace` but currently returns `UnsupportedFeature` for `dismiss`. Since `notify-rust` (v4) supports closing notifications by ID, this is a small but notable gap in an otherwise feature-complete Linux implementation.

### 2. Windows Backend Limitations
The `WindowsBackend` currently only supports `send`. While this aligns with the Phase 1 goal of "best-effort" support for Windows, it means `replace` and `dismiss` are unavailable even if the user provides a receipt. This should be a priority for Phase 2.

### 3. Missing Integration Tests
While the `desktop` provider has excellent unit tests in `messenger/lib/src/provider/desktop/mod.rs`, it lacks a dedicated integration test file in `messenger/lib/src/tests/` (e.g., `desktop_integration.rs`). Although there are some integration-style tests in the provider's `mod.rs`, adding a top-level integration test that exercises the full `Messenger` registry would bring it in line with other providers like Slack and Discord.

### 4. Breaking Changes Verified
The breaking change to `CapabilitySet` (renaming `supports_attachments` to `supported_attachment_kinds`) has been successfully propagated to all existing providers (Discord, Slack, Telegram, etc.). This ensures the library remains consistent.

## Ergonomics & Performance

- **Title Precedence:** The logic for title precedence (Override > Message > Config) is correctly implemented and well-tested.
- **macOS Fallback:** The choice to default to AppleScript for CLI compatibility while keeping the native framework as an opt-in is a pragmatically sound decision for v1.
- **Windows Bootstrap:** The enforcement of the Start Menu shortcut registration during `setup` ensures that `send` remains side-effect-free against the host OS, which is a strong architectural win.

## Recommendation

The feature is **ready for production**. The gaps identified are minor and either align with the phased rollout plan or can be addressed as follow-up polish tasks.

### Suggested Improvements (Post-Phase 1)
- Implement `dismiss` in `LinuxBackend` using `notify_rust::close_notification`.
- Add `messenger/lib/src/tests/desktop_integration.rs` to maintain testing conventions.
- Investigate notification replacement for Windows in Phase 2.
