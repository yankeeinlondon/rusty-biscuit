---
phases: 5
created: 2026-04-19
start_phase: 1
packages:
  - messenger
---

# Slack Webhook Provider - Execution Plan

Source spec: [spec.md](spec.md)

## Conventions And Key Decisions

- Follow the existing Discord webhook split already present in the codebase: add a separate `ProviderKind`, `Target`, `MessageRef`, `RouteProvider`, and `RouteConfig` for Slack webhooks instead of extending the existing Slack bot path.
- Implement the webhook transport in a new `messenger/lib/src/provider/slack_webhook.rs` using direct `reqwest`, matching the current Slack bot adapter and avoiding a new dependency.
- Keep production URL validation strict in `SlackWebhookProvider::try_new()`. Because the acceptance suite requires wiremock tests, add a test-only transport seam or internal request URL override so integration tests can target a mock server without relaxing the runtime `hooks.slack.com` validation rules.
- Reuse the existing Slack mrkdwn renderer and `dispatch.options.disable_link_preview` behavior. No new `Message`, `Dispatch`, or `ProviderOverrides` API is needed for this feature.
- Treat successful webhook sends as "delivered but unaddressable": `raw_id = ""`, `MessageRef::SlackWebhook { thread_ts: None }`, and `metadata["delivery_confirmed"] = "true"`.
- Keep the observable CLI secret-resolution semantics from the spec even if the internal route shape stays aligned with current code patterns: omitted or empty direct values must fall back to the configured env var, which itself defaults to `SLACK_WEBHOOK_URL`.

## Phase 1 - Library Type Surface And Routing

Goal: land the new Slack webhook identity types and route them through validation and markdown rendering before any HTTP implementation work.

Primary files:
- `messenger/lib/src/receipt.rs`
- `messenger/lib/src/target.rs`
- `messenger/lib/src/validate.rs`
- `messenger/lib/src/markdown/mod.rs`
- `messenger/lib/src/tests/receipts.rs`
- `messenger/lib/src/tests/validation.rs`

Steps:

1.1 Add `ProviderKind::SlackWebhook` in `receipt.rs`.
    - Extend `ProviderKind::ALL`, `as_str()`, and `fmt::Display`.
    - Add `MessageRef::SlackWebhook { thread_ts: Option<String> }`.
    - Extend `MessageRef::provider_kind()` and any receipt serialization tests.

1.2 Add `Target::SlackWebhook(SlackWebhookTarget)` in `target.rs`.
    - Make `SlackWebhookTarget` an empty struct.
    - Add `Target::slack_webhook()` as the convenience constructor.
    - Do not add channel or thread fields to the target; the webhook URL binds the channel and reply threading comes from `dispatch.reply_to`.

1.3 Extend provider routing and rendering.
    - Update `validate.rs::target_provider_kind()` to return `ProviderKind::SlackWebhook`.
    - Update `markdown/mod.rs` so `ProviderKind::SlackWebhook` renders through `slack_mrkdwn::render_slack_mrkdwn(...)`, matching the existing Slack bot path.

1.4 Add plan-time validation coverage.
    - Add or update tests proving that `Target::SlackWebhook` combined with `MessageRef::Slack` fails via the existing provider-mismatch check before any provider send occurs.
    - Add a receipt serialization test for `MessageRef::SlackWebhook { thread_ts: None }` so the new reply reference shape is locked down early.

Validation checkpoint:
- `cargo check -p messenger --features slack`
- `cargo test -p messenger --features slack --lib`

Parallelization:
- Step `1.1` should land first because the enum additions drive the rest of the compile fixes.
- Steps `1.2` and `1.3` can proceed in parallel once `ProviderKind::SlackWebhook` exists.
- Step `1.4` runs after the type surface is stable.

## Phase 2 - SlackWebhookProvider Implementation

Goal: add the actual provider, URL validation, payload construction, response mapping, and receipt semantics.

Primary files:
- `messenger/lib/src/provider/mod.rs`
- `messenger/lib/src/provider/slack_webhook.rs`
- `messenger/lib/src/provider/http_helpers.rs`

Steps:

2.1 Create `messenger/lib/src/provider/slack_webhook.rs`.
    - Add `SlackWebhookConfig { webhook_url: SecretString }`.
    - Add `SlackWebhookProvider` with a `reqwest::Client` and a validated request URL representation.
    - Register the module from `provider/mod.rs`.

2.2 Implement constructor-time webhook URL validation in `SlackWebhookProvider::try_new(...)`.
    - Enforce `https` only.
    - Enforce host `hooks.slack.com` with case-insensitive host comparison and no subdomains.
    - Enforce path prefix `/services/` followed by exactly three non-empty decoded segments.
    - Reject trailing slash, query string, fragment, empty segments, and whitespace-only decoded segments.
    - Return `MessengerError::InvalidMessage` for every malformed case and `MessengerError::MissingConfiguration` only for absent values at the CLI resolution layer.

2.3 Implement the provider trait.
    - `kind()` returns `ProviderKind::SlackWebhook`.
    - `capabilities()` matches the spec table exactly.
    - `send_prepared(...)` accepts only `Target::SlackWebhook`.
    - Render text with `message.render_body_with_location(ProviderKind::SlackWebhook)`.
    - Read `thread_ts` only from `MessageRef::SlackWebhook { thread_ts: Some(...) }`.
    - When `disable_link_preview` is true, include `unfurl_links: false` and `unfurl_media: false`; otherwise omit both fields.

2.4 Implement success and error mapping.
    - Success returns `SendReceipt { provider: ProviderKind::SlackWebhook, message_ref: MessageRef::SlackWebhook { thread_ts: None }, raw_id: "", metadata: {"delivery_confirmed": "true"} }`.
    - Slack webhook `ok: false` error codes map to `Authentication`, `InvalidMessage`, or `Provider` exactly as specified.
    - HTTP `429` uses the shared rate-limit handling and surfaces `Retry-After`.
    - HTTP `5xx` surfaces `MessengerError::Transport`.

2.5 Add the test transport seam needed for wiremock.
    - Keep `try_new()` strict for production URLs.
    - Expose an internal constructor, request builder override, or equivalent test-only hook so integration tests can verify request shape and error mapping against a local mock server without weakening runtime validation.

Validation checkpoint:
- `cargo check -p messenger --features slack`
- `cargo test -p messenger --features slack slack_webhook --lib`

Parallelization:
- Steps `2.1` and `2.2` are serial because the validated URL representation shapes the provider struct.
- Step `2.5` can be developed in parallel with `2.3` once the provider skeleton exists.
- Step `2.4` should land after the request path is stable.

## Phase 3 - Library Acceptance Tests And Slack Bot Regression

Goal: lock in all acceptance criteria at the library layer and prove the existing Slack bot path remains unchanged.

Primary files:
- `messenger/lib/src/tests/mod.rs`
- `messenger/lib/src/tests/slack_webhook_integration.rs`
- `messenger/lib/src/tests/slack_integration.rs`
- `messenger/lib/src/tests/validation.rs`
- `messenger/lib/src/provider/slack_webhook.rs`

Steps:

3.1 Add unit tests for constructor behavior and capability reporting.
    - URL validation matrix covering wrong scheme, wrong host, bad prefix, too few segments, too many segments, empty segment, whitespace-only segment, trailing slash, query string, and fragment.
    - CapabilitySet assertion matching the spec table.

3.2 Add wiremock integration tests for successful sends.
    - Plain text payload shape.
    - Markdown rendering to Slack mrkdwn.
    - Reply threading via `MessageRef::SlackWebhook { thread_ts: Some(...) }`.
    - Link preview suppression when `disable_link_preview()` is set.
    - Successful receipt contents: `raw_id == ""`, `thread_ts == None`, and `metadata["delivery_confirmed"] == "true"`.

3.3 Add wiremock integration tests for error handling.
    - `invalid_token` and `action_prohibited` map to `MessengerError::Authentication`.
    - `invalid_payload` and `channel_is_archived` map to `MessengerError::InvalidMessage`.
    - Unknown Slack error strings map to `MessengerError::Provider`.
    - `429` with `Retry-After` maps to `MessengerError::RateLimited`.
    - `500/502/503` map to `MessengerError::Transport`.

3.4 Add plan-time mismatch coverage.
    - `Messenger::plan_send()` with `Target::SlackWebhook` and `reply_to = MessageRef::Slack { .. }` returns `MessengerError::InvalidMessage`.
    - Assert the failure happens before transport execution.

3.5 Re-run the existing Slack bot suite unchanged.
    - Existing `messenger/lib/src/provider/slack.rs` behavior and current `slack_integration.rs` tests stay green without semantic changes.

Validation checkpoint:
- `cargo test -p messenger --features slack --lib`
- `cargo test -p messenger --all-features`

Parallelization:
- Steps `3.1`, `3.2`, and `3.3` can be authored in parallel once Phase 2 compiles.
- Step `3.4` can run alongside them because it exercises the planning path, not the transport path.
- Step `3.5` is the final serial regression gate.

## Phase 4 - CLI Route, Secret Resolution, And Setup Flow

Goal: expose Slack webhooks as a first-class CLI route and make setup and provider registration follow the spec's secret-handling rules.

Primary files:
- `messenger/cli/src/config.rs`
- `messenger/cli/src/main.rs`
- `messenger/cli/src/setup.rs`

Steps:

4.1 Add `RouteProvider::SlackWebhook` and `RouteConfig::SlackWebhook`.
    - Add clap and serde naming as `slack-webhook`.
    - Add default env-var handling for `SLACK_WEBHOOK_URL`.
    - Extend `RouteProvider::ALL`, `RouteConfig::provider()`, `RouteConfig::from_provider_and_target()`, and serde round-trip tests.

4.2 Wire Slack webhook routes through CLI route resolution and target building.
    - Update `register_provider()` to construct `SlackWebhookProvider`.
    - Update `build_target()` to return `Target::slack_webhook()`.
    - Preserve the current ad-hoc CLI pattern by allowing `--provider slack-webhook --channel <webhook-url>` to build a temporary route, mirroring the existing Discord webhook behavior.

4.3 Fix secret resolution semantics for empty direct values.
    - Update `resolve_secret(...)` or add a Slack-webhook-specific wrapper so `Some("")` and whitespace-only direct values fall back to the env var instead of winning over it.
    - Add tests for explicit-value precedence, env fallback, default env name fallback, and missing-configuration failure.

4.4 Extend the interactive `setup` flow.
    - Add a `SlackWebhook` option to provider selection and route configuration.
    - Prompt for the webhook URL and support storing either the literal secret or an env-var name.
    - Use masked input for the webhook URL prompt and avoid echoing the resolved URL in confirmations or logs.
    - Reject empty input before writing config when the user chooses the direct-value path.

4.5 Add CLI coverage.
    - Config serde round-trip tests for direct URL, env-var-only, both set, and neither set.
    - Ad-hoc route resolution test for `RouteProvider::SlackWebhook`.
    - Setup-flow smoke coverage at the same level already used by the CLI: save/load round-trip for a `RouteConfig::SlackWebhook` entry and any new non-interactive helpers introduced for masked secret prompting.

Validation checkpoint:
- `cargo test -p messenger-cli`
- `cargo check -p messenger-cli`

Parallelization:
- Step `4.1` should land before the rest because the new route variant is the shared dependency.
- Steps `4.2` and `4.3` can proceed in parallel after `4.1`.
- Steps `4.4` and `4.5` follow once the route shape is stable.

## Phase 5 - Documentation And End-To-End Acceptance Sweep

Goal: update user-facing docs and close the feature with a full validation pass.

Primary files:
- `messenger/docs/user-guide.md`
- `messenger/README.md`
- `.claude/skills/messenger/SKILL.md`

Steps:

5.1 Update the user guide.
    - Add a Slack webhook setup walkthrough.
    - Document `webhook_url` vs `webhook_url_env`.
    - Document the default `SLACK_WEBHOOK_URL` env var.
    - Call out key behavioral differences from the bot path: no file uploads, no message ID in receipts, limited downstream utility of webhook receipts.

5.2 Update repository and skill docs.
    - Add `Slack-Webhook` to the supported-provider list in `messenger/README.md` if that list is present.
    - Update `.claude/skills/messenger/SKILL.md` so the provider table lists separate Slack bot and Slack webhook entries with the correct capability rows and receipt semantics.

5.3 Verify generated CLI help and observable setup text.
    - Confirm `slack-webhook` appears in clap help and setup provider selection.
    - Confirm help text does not imply file uploads or message IDs for webhook routes.

5.4 Run the full acceptance sweep.
    - `cargo test -p messenger --features slack --lib`
    - `cargo test -p messenger-cli`
    - `cargo test -p messenger --all-features`
    - Manual smoke test: run `messenger setup`, select `SlackWebhook`, enter a test route, reload the config, and confirm the saved route deserializes correctly.

Validation checkpoint:
- All commands in step `5.4` pass.
- Docs match the shipped behavior and the setup flow wording.

Parallelization:
- Steps `5.1`, `5.2`, and `5.3` can be done in parallel after Phases 3 and 4 are complete.
- Step `5.4` is the release gate and must run last.
