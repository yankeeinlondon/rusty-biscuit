## Phase 0

- **Key finding:** The `desktop` feature must be added to `claudine/lib/Cargo.toml` in Phase 2 before `execute_notification` can be wired up.

## Phase 1

- **Key finding:** `ClaudineMessengerConfig::validate()` intentionally does NOT validate individual bot-token route fields (channel_id, recipient, etc.) because the TUI creates WIP routes with empty fields. Only webhook routes and active_config references are validated at the user-config level. Bot-token route field validation happens later at the `ScopedMessagingSettings` / runtime level.
- **Key finding:** Adding new enum variants to `MessengerProviderConfig` and `MessagingRouteConfig` requires updating all exhaustive `match` expressions across both `claudine/lib` and `claudine/cli`. In Phase 1 this affected `messaging/send.rs` and `config_tui/tabs/messenger.rs`.
- **Key finding:** The `is_none_or` method on `Option` (stabilized in Rust 1.82) works well for checking "either None or empty string" in webhook validation: `webhook_url.as_ref().map(|s| s.trim()).is_none_or(|s| s.is_empty())`.

## Phase 2

- **Key finding:** `DiscordWebhookProvider` and `SlackWebhookProvider` expose `try_new` (not `new`) constructors that validate the URL. Error propagation wraps them with `redact_webhook_urls` before surfacing so the URL leak never reaches user logs or snapshots, even for constructor-level validation failures.
- **Key finding:** Messenger `Target` variants for webhooks are `Target::DiscordWebhook(DiscordWebhookTarget)` and `Target::SlackWebhook(SlackWebhookTarget)` (matched with `matches!(payload.target, Target::DiscordWebhook(_))`). These constructors are behind the `discord`/`slack` feature flags, not behind webhook-specific flags.
- **Key finding:** The image-attachment rule in `build_payload()` previously checked `provider_kind == ProviderKind::Discord` — this must become a `matches!` check over both `Discord` and `DiscordWebhook` so webhooks get the same image-capable behavior.
- **Key finding:** `DesktopNotificationProvider::new(DesktopConfig::default())` is the full public constructor; nothing more is required. Host-OS backend selection happens inside `new()` so Claudine does not need to thread in platform hints. The provider is feature-gated behind `messenger/desktop`, which we added to `claudine/lib/Cargo.toml`.
- **Key finding:** `execute_notification("   ")` unit-tests correctly by calling the helper synchronously from a non-tokio test — the blank-title early return prevents `tokio::spawn` from being reached. If the early return regresses, the test panics with "no reactor running".
- **Flaky test:** `claudine-cli::wrap_commands::explicit_provider_flag_bypasses_chooser` intermittently fails with `Broken pipe (os error 32)` when running as part of the full suite (passes in isolation). Unrelated to Phase 2 changes.

## Phase 4

- **Key finding:** Adding secret-field masking to the TUI required extending `ModalState::MessengerInput` with `is_secret: bool` and `error: Option<String>` because ratatui has no built-in password input widget. The actual masking happens at render time by replacing buffer characters with `●` while preserving the real buffer in state.
- **Key finding:** The `messenger_fields` helper is used in three places (rendering, input handling, config building), so adding a third tuple element `(label, default, is_secret)` required updating all call sites. A type alias `MessengerField = (String, String, bool)` keeps the signature readable.
- **Key finding:** TUI validation should be modal-local: errors are stored in `MessengerInput::error`, cleared on any keystroke, and prevent advancing to the next field without mutating `ClaudineConfig`. This keeps invalid state from ever reaching the config model.
- **Key finding:** `build_messenger_from_fields` must convert empty webhook URL strings to `None` so the resulting `MessengerProviderConfig::DiscordWebhook { webhook_url: None, .. }` matches the config model and passes env-only validation.
- **Key finding:** The config list rendering masks inline webhook URLs by showing `webhook: ********` as a stable masked summary, while env-var names remain visible because they are not secrets.

## Phase 5

- **Key finding:** `tokio::task::block_in_place` requires a multi-threaded tokio runtime. Tests that exercise TUI handlers calling async code must use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`, not the default single-threaded flavor.
- **Key finding:** Building a temporary route config from modal state requires combining `collected_fields` with the current `buffer` at `field_index`, then skipping the config name to produce provider-specific fields for `build_messenger_from_fields`.
- **Key finding:** Test status should be modal-local (like validation errors): stored in `MessengerInput::test_status`, cleared on input changes, and never written to `ClaudineConfig`. This keeps the config model clean and prevents spurious dirty flags.
- **Key finding:** The `test_webhook_connection` helper uses the same provider constructors and `redact_webhook_urls` error wrapping as the normal send path, ensuring test failures never leak webhook secrets even when the failure originates from network-level errors.

## Phase 7

- **Key finding:** The plan frontmatter declared `phases: 7` but the document body only defined phases 0–6. Phase 7 had to be retroactively defined as the final integration and skill-update phase once all product functionality was already complete.
- **Key finding:** Claudine skill updates must be applied to both `.claude/skills/claudine/SKILL.md` and `.opencode/skill/claudine/SKILL.md`. These files are not symlinks; drifting them would cause inconsistent behavior across agent contexts.
- **Key finding:** The `structured_verbose_summary_reports_no_tool_calls_when_absent` integration test exhibits the same `Broken pipe (os error 32)` flakiness as `explicit_provider_flag_bypasses_chooser` — it fails when run as part of the full suite but passes in isolation. This is unrelated to messaging changes.
- **Key finding:** When all feature functionality is complete by the documentation phase (Phase 6), the final phase should focus on skill accuracy, prompt-template improvements, and recording cross-phase lessons learned rather than product code changes.
