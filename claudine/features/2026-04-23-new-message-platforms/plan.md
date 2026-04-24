---
phases: 7
created: 2026-04-24
start_phase: 0
packages:
  - claudine
  - claudine-cli
  - messenger
source_documents:
  - spec.md
  - tech-design.md
source_files_during_phase_0: []
docs_updated_during_phase_0: []
docs_created_during_phase_0: []
skills_files_updated_during_phase_0: []
---

# New Message and Notification Platforms - Execution Plan

Source documents:
- Functional specification: [spec.md](spec.md)
- Technical design: [tech-design.md](tech-design.md)

## Phase 0 - Preflight and Scope Confirmation

Goal: confirm the messenger crate already exposes the provider capabilities Claudine will delegate to, then establish the exact Claudine files to change. This phase has no intended product behavior change.

Steps:

0.1 Confirm upstream messenger support is present.
- Verify `messenger::ProviderKind::{DiscordWebhook, SlackWebhook, Desktop}` exists.
- Verify `messenger::Target::{discord_webhook, slack_webhook, desktop}` exists.
- Verify `messenger::provider::{discord_webhook, slack_webhook}` modules and `messenger::DesktopNotificationProvider` are exported.
- Observable result: `rg` or code inspection shows all provider types are available without adding new messenger provider code.

0.2 Confirm Claudine's messenger dependency features.
- Inspect `claudine/lib/Cargo.toml`.
- If `messenger` lacks the `desktop` feature, add it when Phase 2 starts.
- Observable result: dependency feature delta is known before wiring `execute_notification`.

0.3 Inventory Claudine integration points.
- Runtime/config files: `claudine/lib/src/config/claudine_config.rs`, `claudine/lib/src/messaging/config.rs`, `claudine/lib/src/messaging/resolve.rs`, `claudine/lib/src/messaging/send.rs`, `claudine/lib/src/messaging/mod.rs`, `claudine/lib/src/dispatch/loader.rs`.
- Lifecycle file: `claudine/lib/src/composition/lifecycle.rs`.
- TUI files: `claudine/cli/src/commands/config_tui/app.rs`, `reducers.rs`, `tabs/messenger.rs`, `widgets/modal.rs`.
- Observable result: implementation owner can point each design requirement to a file.

Validation checkpoint:
- `cargo check -p messenger --all-features`
- `cargo check -p claudine`

Parallelization:
- Steps 0.1 and 0.3 can run in parallel. Step 0.2 is independent but should complete before Phase 2.

## Phase 1 - Config Models and Validation

Goal: make Claudine's persisted config and runtime route config understand `discord_webhook` and `slack_webhook` without changing existing bot-token route behavior.

Steps:

1.1 Add webhook defaults to config models.
- Add `default_discord_webhook_url()` / `default_slack_webhook_url()` helpers returning `DISCORD_WEBHOOK_URL` and `SLACK_WEBHOOK_URL` in both config layers that need serde defaults.
- Observable result: defaults are used when webhook env fields are omitted from hand-authored config.

1.2 Extend user-facing config.
- Add `MessengerProviderConfig::DiscordWebhook { webhook_url: Option<String>, webhook_url_env: String }`.
- Add `MessengerProviderConfig::SlackWebhook { webhook_url: Option<String>, webhook_url_env: String }`.
- Keep serde tag as `provider` and serialized names as `discord_webhook` and `slack_webhook`.
- Observable result: `~/.claudine/config.json` can deserialize named webhook routes.

1.3 Extend runtime route config.
- Add matching variants to `MessagingRouteConfig`.
- Because this enum uses `rename_all = "lowercase"`, explicitly rename the new variants to `discord_webhook` and `slack_webhook`.
- Observable result: runtime route JSON round trips with snake_case webhook provider names.

1.4 Add URL validators.
- Implement shared functions for conservative regex validation:
  - Discord: `^https://(discord\.com|discordapp\.com)/api/webhooks/[0-9]+/[A-Za-z0-9._-]+$`
  - Slack: `^https://hooks\.slack\.com/services/[A-Z0-9]+/[A-Z0-9]+/[A-Za-z0-9]+$`
- Keep these as early validation only; messenger provider constructors remain the production source of truth.
- Observable result: validators accept the documented examples and reject wrong schemes, hosts, blank values, and malformed paths.

1.5 Extend semantic validation.
- Update `ClaudineMessengerConfig::validate()` and `ScopedMessagingSettings::validate()`.
- Webhook routes require either non-empty inline `webhook_url` or non-empty `webhook_url_env`.
- If inline `webhook_url` is present, validate provider URL format.
- Existing Discord, Slack, Signal, and WhatsApp validation remains unchanged.
- Observable result: invalid webhook configs fail during config load; env-only webhook configs remain valid.

1.6 Add schema and validation tests.
- Cover serde round trips for both webhook variants.
- Cover missing env/default behavior.
- Cover inline URL validation success/failure.
- Cover regression cases for existing bot-token routes.

Validation checkpoint:
- `cargo test -p claudine messaging::config`
- `cargo test -p claudine claudine_config`
- `cargo check -p claudine`

Parallelization:
- Steps 1.2 and 1.3 can proceed in parallel after 1.1.
- Step 1.4 can proceed in parallel with enum work.
- Steps 1.5 and 1.6 depend on the variants and validators.

## Phase 2 - Runtime Bridge, Send Path, and Desktop Helper

Goal: route configured webhooks through messenger transports and expose a zero-config desktop notification helper for lifecycle use.

Steps:

2.1 Enable messenger desktop support in Claudine if needed.
- Add `desktop` to `claudine/lib/Cargo.toml` messenger features.
- If this changes transitive dependencies, record the docs update in Phase 6.
- Observable result: `claudine` can import `DesktopNotificationProvider` behind enabled features.

2.2 Bridge user config to runtime config.
- Update `dispatch::loader::bridge_provider_config()` for both webhook variants.
- Preserve inline `webhook_url` and `webhook_url_env`.
- Observable result: `bridge_messaging_settings()` returns a runtime route with the same route name and provider fields.

2.3 Extend payload building.
- In `messaging/send.rs::build_payload()`, map:
  - `DiscordWebhook` to `Target::discord_webhook()` and `ProviderKind::DiscordWebhook`.
  - `SlackWebhook` to `Target::slack_webhook()` and `ProviderKind::SlackWebhook`.
- Preserve image behavior:
  - Discord bot and Discord webhook may carry image attachments.
  - Slack bot and Slack webhook ignore images when text exists.
  - Slack image-only payloads are skipped.
- Observable result: payload tests prove provider kind, target, and image handling for both webhook routes.

2.4 Extend provider registration.
- Import `DiscordWebhookConfig`, `DiscordWebhookProvider`, `SlackWebhookConfig`, and `SlackWebhookProvider`.
- Resolve `webhook_url` through the existing inline/env `resolve_secret()` behavior.
- Construct providers through their validating constructors.
- Register only the active route's provider before planning the send.
- Observable result: malformed webhook URLs fail as non-fatal send warnings, not as composition failures.

2.5 Harden labels, hints, and redaction.
- Extend provider label helpers with `discord_webhook` and `slack_webhook`.
- Extend failure hints for webhook URL validation failures, Discord 404, and Slack `no_service`.
- Redact webhook URLs before rendering user-facing errors, status warnings, logs, or test snapshots.
- Observable result: tests prove warning text does not contain configured webhook tokens.

2.6 Add `execute_notification(title: &str)`.
- Put the helper in `messaging/send.rs` and re-export it from `messaging/mod.rs`.
- Trim title and no-op on blank input.
- Spawn a fire-and-forget async task.
- Build a `messenger::Messenger`, register `DesktopNotificationProvider::new(DesktopConfig::default())`, send a title-capable desktop message to `Target::desktop()`.
- Render or log warnings on failure; never return errors to lifecycle execution.
- Observable result: notification helper is callable without Claudine messenger config.

2.7 Add send-path tests.
- Webhook payload target/provider mapping.
- Discord webhook image-only payload retained.
- Slack webhook image-only payload skipped.
- Secret resolution uses inline value first, then env var.
- `execute_notification("   ")` is a no-op.
- Redaction tests for webhook-bearing error strings.

Validation checkpoint:
- `cargo test -p claudine messaging`
- `cargo test -p claudine dispatch::loader`
- `cargo check -p claudine`

Parallelization:
- Step 2.1 is independent.
- Steps 2.2, 2.3, and 2.4 depend on Phase 1 variants.
- Step 2.5 can proceed in parallel with 2.4 after labels exist.
- Step 2.6 depends on 2.1.
- Step 2.7 follows the implementation steps it tests.

## Phase 3 - Lifecycle `notify` Parsing and Emission

Goal: add local desktop notification fan-out to composition lifecycle frontmatter while preserving existing lifecycle ordering and failure behavior.

Steps:

3.1 Extend `LifecycleNotification`.
- Add `notify: Option<String>`.
- Ensure default/serde behavior remains compatible with existing lifecycle frontmatter.
- Observable result: old frontmatter still parses; frontmatter with `notify` stores the value.

3.2 Normalize `notify` in parsing.
- Update `parse_lifecycle_config()` so blank `notify` strings become `None`, matching other optional string fields.
- Observable result: `notify: ""` and whitespace-only values are no-ops.

3.3 Extend `LifecycleEmitter`.
- Add `emit_notification(&self, title: &str)`.
- Implement it in `DefaultLifecycleEmitter` by calling `crate::messaging::execute_notification(title)`.
- Update test emitters to record notification calls.
- Observable result: lifecycle tests can observe notification emission without hitting the OS.

3.4 Update signal emission order.
- In `LifecycleRunGuard::emit_signal()`, immediate fan-out order must be:
  1. `stderr`
  2. `message`
  3. `notify`
  4. audio phases
- Notification failures remain non-fatal because the default emitter delegates to a warning-only helper.
- Observable result: ordering tests prove `notify` runs before `say`, `say_first`, and `effect`.

3.5 Add lifecycle tests.
- Parse `notify` for `start`, `success`, `blocked`, and `failure`.
- Parse combined `message` + `notify` fields as independent outputs.
- Verify blank `notify` does not emit.
- Verify `notify` emits even when remote messaging has no active route.
- Verify existing lifecycle `message`, `stderr`, and audio tests still pass.

Validation checkpoint:
- `cargo test -p claudine composition::lifecycle`
- `cargo test -p claudine composition::prepare`

Parallelization:
- Steps 3.1 and 3.2 are serial.
- Step 3.3 can start after 3.1.
- Step 3.4 depends on 3.3.
- Step 3.5 follows all lifecycle changes.

## Phase 4 - Config TUI Webhook Editing and Masking

Goal: expose webhook routes in `claudine config` while preventing accidental secret display and adding early URL feedback. Do not add any desktop notification controls.

Steps:

4.1 Add provider choices.
- Update messenger provider lists to include `discord_webhook` and `slack_webhook`.
- Render labels as `Discord Webhook` and `Slack Webhook`.
- Leave desktop notifications out of all config UI.
- Observable result: add-provider modal contains six remote messaging choices and no desktop option.

4.2 Create webhook route skeletons.
- Update reducer/helper logic that creates `MessengerProviderConfig` values.
- Webhook skeletons should default `webhook_url` to `None` and env names to `DISCORD_WEBHOOK_URL` / `SLACK_WEBHOOK_URL`.
- Observable result: selecting a webhook provider creates a route that can be completed through modal input.

4.3 Define webhook input fields.
- Add fields for route name, webhook URL, and webhook env var according to current modal flow.
- Treat webhook URL as a secret field.
- Preserve env var visibility.
- Observable result: modal state distinguishes secret URL fields from ordinary fields.

4.4 Mask secret rendering.
- Extend modal rendering so secret fields display bullets or asterisks while preserving the real buffer in state.
- Ensure configuration list rendering never displays inline webhook URLs; show `webhook: ********` or an equivalent stable masked summary.
- Observable result: snapshots and manual TUI inspection never show full webhook URLs.

4.5 Add input validation.
- Run provider-specific URL validation before advancing from webhook URL input when the user supplied an inline URL.
- Allow env-only routes when the URL is blank and env var is non-empty.
- Show modal-local validation status without saving config.
- Observable result: bad inline URLs keep the user on the field with a visible error; env-only routes can proceed.

4.6 Keep existing messenger and repo override behavior intact.
- Active messenger selection continues to use route names.
- Repo override selection can select webhook route names without a new schema.
- Existing bot-token providers keep their current input flow.
- Observable result: no regression in active route selection, repo inherit, repo disabled, or existing provider editing tests.

4.7 Add TUI tests.
- Provider list contains webhook choices.
- Provider labels render correctly.
- Webhook route skeletons use default env names.
- Secret fields are masked in modal snapshots.
- Invalid webhook URLs fail reducer/modal validation.
- No desktop notification controls are present.

Validation checkpoint:
- `cargo test -p claudine-cli config_tui`
- Run `claudine config` manually enough to verify add-provider, masked input, validation, and save behavior.

Parallelization:
- Steps 4.1 and 4.2 can proceed in parallel after Phase 1 config variants.
- Step 4.4 can proceed in parallel with 4.3 once modal state supports secret metadata.
- Step 4.5 depends on the validators from Phase 1 and the field definitions from 4.3.
- Step 4.7 follows implementation.

## Phase 5 - Webhook Test Connection Workflow

Goal: allow users to validate webhook settings from the config TUI without saving them.

Steps:

5.1 Add a Claudine test-send helper.
- Add a focused helper near `messaging/send.rs` that accepts an in-progress webhook route and sends a short test message through messenger.
- Use the same provider constructors and redaction rules as normal sends.
- Return a small success/failure result for TUI display.
- Observable result: helper can test Discord and Slack webhook routes without requiring the route to be active or persisted.

5.2 Add modal-local status.
- Extend `ModalState::MessengerInput` or a related TUI state to hold test status text.
- Status must be scoped to the current modal and must not mutate `ClaudineConfig`.
- Observable result: pressing test updates only modal UI state.

5.3 Add hotkey/control.
- For webhook routes only, expose `T` as "Test Connection" once enough fields exist to build the route.
- Do not show the test control for Discord bot, Slack bot, Signal, or WhatsApp routes unless a future feature adds it.
- Observable result: hotkey bar shows `T: Test` only for webhook input flow.

5.4 Wire async execution safely.
- Keep the TUI responsive while the test runs.
- Convert helper errors into redacted status messages.
- Avoid saving config as a side effect.
- Observable result: success and failure statuses are visible; malformed or rejected URLs do not leak secrets.

5.5 Add tests.
- Build in-progress Discord and Slack webhook routes from modal fields.
- Test connection success status.
- Failure status redacts URLs.
- Test action does not mark config dirty or save files.

Validation checkpoint:
- `cargo test -p claudine-cli config_tui`
- `cargo test -p claudine messaging`
- Manual test with a known-invalid webhook URL confirms redaction and no save side effect.

Parallelization:
- Step 5.1 can proceed in parallel with 5.2.
- Steps 5.3 and 5.4 depend on both helper and modal status state.
- Step 5.5 follows all workflow wiring.

## Phase 6 - Documentation, Dependency Records, and Final Validation

Goal: update public docs and local knowledge only where behavior changed, then run the full relevant validation suite.

Steps:

6.1 Update user-facing docs.
- Update Claudine messaging/config docs or README sections that describe messenger providers.
- Add examples for:
  - `discord_webhook` and `slack_webhook` named routes.
  - env-only webhook configuration.
  - lifecycle `message` and `notify` used together.
- Explicitly state desktop notifications are zero-config and not a `claudine config` setting.
- Observable result: docs match the implemented schema and lifecycle behavior.

6.2 Update dependency docs if needed.
- If Phase 2 added the `messenger/desktop` feature and this changed transitive dependencies, update `docs/dependencies.md` and any relevant per-area dependency docs.
- Observable result: dependency docs do not drift from Cargo manifests.

6.3 Update local skills only if warranted.
- Update `.claude/skills/claudine/SKILL.md` if webhook/desktop notification support changes the architecture summary or common workflows.
- Do not edit skill docs for trivial internal-only implementation details.
- Observable result: skill catalog remains authoritative for future Claudine work.

6.4 Run focused validation.
- `cargo test -p claudine messaging`
- `cargo test -p claudine composition::lifecycle`
- `cargo test -p claudine dispatch::loader`
- `cargo test -p claudine-cli config_tui`
- Observable result: all new and adjacent regression tests pass.

6.5 Run package validation.
- `cargo check -p claudine`
- `cargo check -p claudine-cli`
- `cargo clippy -p claudine --all-targets -- -D warnings`
- `cargo clippy -p claudine-cli --all-targets -- -D warnings`
- If root `just lint` covers the touched area and is practical, run it as the final lint gate.
- Observable result: code compiles and lint gates are clean.

6.6 Manual smoke checks.
- Hand-authored config with `slack_webhook` env-only route loads.
- Hand-authored config with malformed inline webhook URL fails validation.
- Composition frontmatter with `message` + `notify` parses and emits both side effects without blocking audio phases.
- `claudine config` can add webhook route, mask URL input, reject malformed URL, and save valid/env-only route.
- Observable result: the feature works through both file config and TUI surfaces.

Parallelization:
- Steps 6.1, 6.2, and 6.3 can proceed in parallel after implementation stabilizes.
- Steps 6.4 through 6.6 are serial release gates.

## Cross-Phase Invariants

- Webhook URLs are secrets. Do not render full URLs in config lists, input echo, errors, logs, snapshots, or test failure messages.
- Desktop notifications are a separate local notification concern, not a messenger route in Claudine config.
- Runtime message and notification failures are operational warnings and must not fail hooks, composition runs, or lifecycle transitions.
- Existing Discord bot, Slack bot, Signal, WhatsApp, `HookAction::Message`, and lifecycle `message` behavior must remain compatible.
- Repo override behavior remains name-based and does not need a new schema.
