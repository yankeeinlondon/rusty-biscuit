---
phases: 6
created: 2026-04-18
start_phase: 2
source_files_during_phase_1:
  - messenger/lib/src/receipt.rs
  - messenger/lib/src/target.rs
  - messenger/lib/src/validate.rs
  - messenger/lib/src/dispatch.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - messenger/lib/Cargo.toml
  - messenger/lib/src/provider/mod.rs
  - messenger/lib/src/provider/discord_webhook.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
packages:
  - messenger
---

# Discord Webhook Provider — Execution Plan

Source spec: [spec.md](spec.md)

## Conventions & Key Decisions

- Authoritative error variant: the spec mentions `MessengerError::UnsupportedCapability`; the codebase's equivalent is `MessengerError::UnsupportedFeature { provider, feature: &'static str }`. Use `UnsupportedFeature` with `feature: "replies"` (matches the wording already used in `validate::normalize_dispatch`).
- Target shape: introduce a dedicated `DiscordWebhookTarget { thread_id: Option<String> }` struct (parallel to `TelegramTarget`) and wrap it in `Target::DiscordWebhook(DiscordWebhookTarget)`. This preserves the enum-variant-per-provider pattern.
- serde tag for `RouteConfig::DiscordWebhook` must serialize as `"discord-webhook"` — the parent enum uses `rename_all = "lowercase"`, so the new variant needs an explicit `#[serde(rename = "discord-webhook")]`. Clap `value_name = "discord-webhook"`.
- HTTP client: recommend `twilight-http::Client::execute_webhook` (already a dep behind the `discord` feature; supports attachments via `twilight-model::http::attachment::Attachment`). The spec leaves this open — if `execute_webhook` proves awkward for unauthenticated URL parsing, fall back to `reqwest` (multipart for attachments). Record the final choice in code comments at implementation time; the capability contract is identical either way.
- Hard-error for reply-on-webhook: enforce in `validate::normalize_dispatch` as an up-front, mode-independent check keyed on `ProviderKind::DiscordWebhook`. This guarantees `plan_send()` never issues a network call for webhook+reply and sidesteps the "best-effort drops replies" branch for this provider kind.

---

## Phase 1 — Library core types

Goal: extend `ProviderKind`, `Target`, and `MessageRef` with their DiscordWebhook variants. No provider logic yet. After this phase, the workspace still builds (with unimplemented-provider tests skipped or stubbed).

Files touched:
- `messenger/lib/src/receipt.rs`
- `messenger/lib/src/target.rs`
- `messenger/lib/src/validate.rs`
- `messenger/lib/src/dispatch.rs` (new `DiscordWebhookOverrides` stub, for symmetry)
- `messenger/lib/src/lib.rs` (no changes expected; re-exports already cover new variants)

Steps:

1.1 `receipt.rs`: add `ProviderKind::DiscordWebhook` variant.
    - Extend `ProviderKind::ALL` from `[Self; 5]` → `[Self; 6]`; include new variant.
    - Update `as_str` to return `"discord-webhook"`.
    - Update `fmt::Display` to return `"Discord-Webhook"` (matches the form the spec asks for in the SKILL.md provider table).
    - Add `MessageRef::DiscordWebhook { webhook_id: String, channel_id: String, message_id: String, thread_id: Option<String> }`. Rationale: webhook execute responses return `id` (message_id), `channel_id`, `webhook_id`; `thread_id` is preserved so future threaded receipts remain routable.
    - Extend `MessageRef::provider_kind()` match.
    - Update the existing `ProviderKind::Discord` variant's feature-gate comments where relevant (no runtime change).

1.2 `target.rs`: add `DiscordWebhookTarget { thread_id: Option<String> }` and `Target::DiscordWebhook(DiscordWebhookTarget)`.
    - Gate both on `#[cfg(feature = "discord")]` (webhook ships with the existing discord feature — no new feature flag).
    - Add convenience constructor `Target::discord_webhook()` (no args) that sets `thread_id = None`, and `Target::discord_webhook_thread(thread_id: impl Into<String>)`.
    - Note: no `channel_id` field; the URL binds the channel at the provider layer.

1.3 `validate.rs::target_provider_kind`: add `Target::DiscordWebhook(_) => ProviderKind::DiscordWebhook` arm.

1.4 `validate.rs::normalize_dispatch`: insert a pre-normalization guard immediately after the existing `reply_to` provider-match check:

    ```
    if provider == ProviderKind::DiscordWebhook && dispatch.reply_to.is_some() {
        return Err(MessengerError::UnsupportedFeature {
            provider,
            feature: "replies",
        });
    }
    ```

    This runs unconditionally (ignores `CompatibilityMode`) and fires before any network call. Add a `tracing::warn!` for observability.

1.5 `dispatch.rs`: add `DiscordWebhookOverrides {}` + `ProviderOverrides::DiscordWebhook(DiscordWebhookOverrides)` — kept as an empty struct for now to preserve the per-provider overrides pattern. Gate on `#[cfg(feature = "discord")]`.

Validation checkpoint:
- `cargo check -p messenger` passes.
- `cargo build -p messenger --all-features` passes.
- `cargo test -p messenger --lib` — existing tests unchanged, still green.

Parallelization: all edits are in four small files; implement sequentially in one pass.

---

## Phase 2 — DiscordWebhookProvider implementation

Goal: a working `DiscordWebhookProvider` that sends to Discord's webhook endpoint, including attachment and `thread_id` support. Depends on Phase 1.

Files touched:
- `messenger/lib/src/provider/mod.rs` (re-export + feature gate)
- `messenger/lib/src/provider/discord_webhook.rs` (new)

Steps:

2.1 Create `messenger/lib/src/provider/discord_webhook.rs`:
    - `pub struct DiscordWebhookConfig { pub webhook_url: SecretString }`.
    - `pub struct DiscordWebhookProvider { /* parsed id + token + http client */ }`.
    - Constructor: parse `/webhooks/{id}/{token}` segments from `webhook_url`; reject malformed URLs with `MessengerError::InvalidMessage` via a separate `try_new()`; provide `new()` as a panicking convenience with a clear panic message (matches the existing `DiscordProvider::new` pattern that doesn't fallibly parse).
    - Reuse `build_attachment` / `build_attachments` helpers by making them either `pub(super)` on `DiscordProvider` or by lifting them into a shared module (prefer the latter only if needed — otherwise copy-paste the small function to avoid coupling).

2.2 Implement `Provider` trait:
    - `kind()` → `ProviderKind::DiscordWebhook`.
    - `capabilities()`:
      ```
      CapabilitySet {
          supports_markdown_rendering: true,
          supports_reply: false,
          supports_attachments: true,
          supports_location: true, // text fallback, same as bot
          supports_silent_delivery: false,
          supports_link_preview_control: false,
      }
      ```
    - `send_prepared(dispatch, message)`:
      - Match `dispatch.target` → `Target::DiscordWebhook(DiscordWebhookTarget { thread_id })`; return `MessengerError::InvalidMessage("expected DiscordWebhook target")` on mismatch.
      - Render body via `message.render_body_with_location(ProviderKind::DiscordWebhook)`.
      - Build attachments (reusing bot path's converter).
      - Use `twilight-http::Client::new(token.clone()).execute_webhook(webhook_id, &token)` — or, if twilight's typing turns out to require a bot-token prefix, switch to a direct `reqwest::Client` POST to `https://discord.com/api/v10/webhooks/{id}/{token}` with `?thread_id={thread_id}` when present. Decide during implementation; document the choice inline.
      - On success, parse response → `SendReceipt` with `MessageRef::DiscordWebhook { webhook_id, channel_id, message_id, thread_id }` and `raw_id = message_id`.
      - Map error kinds: 401/403 → `Authentication`; 429 (honoring `Retry-After`) → `RateLimited`; 5xx / transport → `Transport`; other 4xx → `Provider`.

2.3 `provider/mod.rs`:
    - `#[cfg(feature = "discord")] pub mod discord_webhook;`.

Validation checkpoint:
- `cargo build -p messenger --features discord` passes.
- `cargo clippy -p messenger --all-features -- -D warnings` passes.
- Library still green on existing tests.

Parallelization: single-threaded within the phase (one new file + one mod decl).

---

## Phase 3 — Library tests

Goal: prove capability set, plan-time reply hard-error, wiremock send path, and existing-bot regression. Depends on Phases 1 and 2.

Files touched:
- `messenger/lib/src/tests/mod.rs` (register new module behind `discord` feature)
- `messenger/lib/src/tests/discord_webhook_integration.rs` (new)
- `messenger/lib/src/tests/validation.rs` (extend — or add new tests in the new file if cleaner)

Steps (steps 3.1–3.3 can be authored in parallel; 3.4 must run last):

3.1 Capability + provider-kind unit tests in `discord_webhook_integration.rs`:
    - `DiscordWebhookProvider::new(valid_url).capabilities().supports_reply == false`.
    - Other capabilities match the table above.
    - `provider.kind() == ProviderKind::DiscordWebhook`.

3.2 Plan-time reply hard-error test (may live in `validation.rs` or the new file):
    - Build a `Messenger` with `DiscordWebhookProvider` registered.
    - `plan_send(dispatch_with_reply_to, &message)` returns `MessengerError::UnsupportedFeature { provider: ProviderKind::DiscordWebhook, feature: "replies" }`.
    - Assert this fires in `BestEffort` mode (not just strict) — i.e. the hard-error path is mode-independent.
    - Assert no wiremock request was recorded (use a `MockServer` that expects zero POSTs; `MockServer::verify` on drop).

3.3 Wiremock integration test for successful send (mirrors `slack_integration.rs`):
    - Spin up `MockServer::start().await`; construct webhook URL as `{server.uri()}/api/v10/webhooks/123456789012345678/fake-token`.
    - `Mock::given(method("POST")).and(path("/api/v10/webhooks/123456789012345678/fake-token")).and(body_json(json!({ "content": "hello" })))` → respond with a realistic message JSON (include `id`, `channel_id`, `webhook_id`).
    - Assert `receipt.provider == ProviderKind::DiscordWebhook`, `receipt.message_ref` matches expected `DiscordWebhook { .. }`, and `raw_id` equals the mocked `id`.
    - Add a second test that sets `thread_id` → expect `path_regex` or query matcher with `?thread_id=…`.
    - Add a rate-limit test: 429 with `Retry-After: 15` → `MessengerError::RateLimited { retry_after_ms: Some(15_000) }`.

3.4 Regression sanity pass:
    - Re-run the entire `messenger` test suite, all features: `cargo test -p messenger --all-features`.
    - Confirm every existing Discord-bot test (`provider::discord::tests`, wiremock bot tests if any) still passes with no source modifications.

Validation checkpoint:
- `cargo test -p messenger --all-features` green.
- `cargo test -p messenger --no-default-features --features discord` green (isolates the webhook + bot path under the discord feature).

Parallelization: 3.1, 3.2, and 3.3 can be written concurrently by one or more agents; 3.4 is the serial gate.

---

## Phase 4 — CLI config + provider registration

Goal: `RouteConfig::DiscordWebhook` + `RouteProvider::DiscordWebhook` + send-path wiring. Depends on Phase 1 (types) and Phase 2 (provider constructor).

Files touched:
- `messenger/cli/src/config.rs`
- `messenger/cli/src/main.rs`

Steps:

4.1 `config.rs::RouteProvider` enum:
    - Add variant `DiscordWebhook` with `#[value(name = "discord-webhook")]`.
    - Extend `RouteProvider::ALL` from `[Self; 5]` → `[Self; 6]`.
    - Update `as_str()` → `"discord-webhook"`.
    - Update `fmt::Display` (covered by `as_str()`).

4.2 `config.rs::RouteConfig` enum: add variant

    ```
    DiscordWebhook {
        webhook_url: Option<String>,
        webhook_url_env: String,
    }
    ```

    - No `channel_id` / `thread_id`. Thread ID is a dispatch-time concern, not a config one (per spec §Library-Level Requirements).

4.3 `config.rs::RouteConfigRepr` mirror variant with serde attrs:

    ```
    #[serde(rename = "discord-webhook")]
    DiscordWebhook {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        #[serde(default = "default_discord_webhook_url_env")]
        webhook_url_env: String,
    }
    ```

    Add `fn default_discord_webhook_url_env() -> String { "DISCORD_WEBHOOK_URL".into() }`.

4.4 Update the `From<RouteConfigRepr>` and `From<RouteConfig> for RouteConfigRepr` impls to cover the new variant.

4.5 `RouteConfig::provider()` match arm: `DiscordWebhook { .. } => RouteProvider::DiscordWebhook`.

4.6 `RouteConfig::from_provider_and_target`: add arm for `RouteProvider::DiscordWebhook`. The `target_id` arg carries the webhook URL for ad-hoc CLI routes (`--provider discord-webhook --channel <url>`). Build a `RouteConfig::DiscordWebhook { webhook_url: Some(target_id), webhook_url_env: default_discord_webhook_url_env() }`.

4.7 `LegacyRouteConfig::From` conversion: add arm mapping `RouteProvider::DiscordWebhook` → `RouteConfig::DiscordWebhook { webhook_url: None, webhook_url_env: token_env.unwrap_or_else(default_discord_webhook_url_env) }`. (Unlikely to be hit but required for the exhaustive match.)

4.8 `main.rs::register_provider`: add arm for `RouteConfig::DiscordWebhook { webhook_url, webhook_url_env, .. }`:
    - `let url = resolve_secret(webhook_url.as_deref(), webhook_url_env)?;`
    - `messenger.register(Box::new(DiscordWebhookProvider::new(DiscordWebhookConfig { webhook_url: SecretString::from(url) })));`.

4.9 `main.rs::build_target`: add arm returning `Target::discord_webhook()` (no thread id from the route config). Thread-id support on the CLI is out of scope for this feature spec; add a TODO comment referencing future work.

Validation checkpoint:
- `cargo check -p messenger-cli` passes.
- `cargo test -p messenger-cli --lib` — existing tests still green.
- `messenger --help` shows `discord-webhook` in `--provider` value options.

Parallelization: sequential file edits.

---

## Phase 5 — CLI setup flow + CLI tests

Goal: interactive `setup` prompts for webhook URL; config round-trip and URL-resolution unit tests; setup-flow smoke test. Depends on Phase 4.

Files touched:
- `messenger/cli/src/setup.rs`
- `messenger/cli/src/config.rs` (tests)
- `messenger/cli/src/main.rs` (tests)

Steps:

5.1 `setup.rs::configure_provider`: add arm `RouteProvider::DiscordWebhook => configure_discord_webhook()`.

5.2 Implement `fn configure_discord_webhook() -> Result<RouteConfig>`:
    - Leading explainer blocks via `styled()` and `Prose` per existing pattern:
      - "Discord webhooks require the full webhook URL (https://discord.com/api/v10/webhooks/<id>/<token>)."
      - "Create one under Server Settings → Integrations → Webhooks."
      - "The URL binds both the channel and authentication — treat it as a secret."
    - `Text::new("Webhook URL:")` with `with_help_message("Full webhook URL (leave empty to use env var instead)")`.
    - If empty → prompt `Text::new("Environment variable for webhook URL:").with_default("DISCORD_WEBHOOK_URL")`.
    - Return `RouteConfig::DiscordWebhook { webhook_url, webhook_url_env }`.

5.3 Config tests in `config.rs` (add to existing `#[cfg(test)] mod tests`):

    Test A — round-trips with both fields:
    ```
    let cfg = RouteConfig::DiscordWebhook {
        webhook_url: Some("https://discord.com/api/v10/webhooks/1/abc".into()),
        webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
    };
    assert_eq!(serde_json::from_str::<RouteConfig>(&serde_json::to_string(&cfg)?)?, cfg);
    ```

    Test B — round-trips with only `webhook_url_env`:
    ```
    let cfg = RouteConfig::DiscordWebhook { webhook_url: None, webhook_url_env: "CUSTOM_WEBHOOK_ENV".into() };
    // assert round-trip preserves missing webhook_url.
    ```

    Test C — default env var applied on deserialize when `webhook_url_env` absent from JSON:
    ```
    let raw = r#"{"provider":"discord-webhook","webhook_url":"https://.../webhooks/1/abc"}"#;
    let parsed: RouteConfig = serde_json::from_str(raw)?;
    assert_eq!(parsed, RouteConfig::DiscordWebhook {
        webhook_url: Some("https://.../webhooks/1/abc".into()),
        webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
    });
    ```

    Test D — neither field provided (degenerate but valid):
    ```
    let raw = r#"{"provider":"discord-webhook"}"#;
    let parsed: RouteConfig = serde_json::from_str(raw)?;
    assert_eq!(parsed, RouteConfig::DiscordWebhook {
        webhook_url: None,
        webhook_url_env: "DISCORD_WEBHOOK_URL".into(),
    });
    ```

5.4 URL-resolution test in `main.rs` tests (mirrors existing Signal/Telegram tests):
    - With `DISCORD_WEBHOOK_URL` unset and `webhook_url = Some("…")`: resolved secret equals the direct value.
    - With `webhook_url = None` and `webhook_url_env = "DISCORD_WEBHOOK_URL"` and `std::env::set_var("DISCORD_WEBHOOK_URL", "…")` (use `serial_test::serial` to avoid env-var races): resolved secret equals the env-var value.
    - Both unset and no direct value → `resolve_secret` returns the structured error. (Add `use serial_test::serial;` as an existing dev-dep.)

5.5 Setup-flow smoke test:
    - This test cannot run the interactive `inquire` path unattended. Instead, validate the non-interactive postcondition: given a pre-built `RouteConfig::DiscordWebhook` in a temp `.messenger.json`, `Config::load_from_path` reads it back intact and `RouteConfig::provider()` returns `RouteProvider::DiscordWebhook`. Add a note in the test file explaining that the interactive prompt flow is covered by the setup walkthrough in the user guide.

Validation checkpoint:
- `cargo test -p messenger-cli` green (including new tests).
- `cargo clippy -p messenger-cli -- -D warnings` green.
- Manual: `messenger setup discord-webhook` lists the new provider in the interactive selector (local sanity only — not part of CI gates).

Parallelization: 5.1 + 5.2 are one task; 5.3, 5.4, 5.5 are independent test files and can be authored in parallel once 5.2 lands.

---

## Phase 6 — Documentation

Goal: user-guide walkthrough, SKILL.md provider table refresh, README provider list, CLI help reflects the new variant automatically (clap does this for free, no explicit doc change needed beyond verifying). Depends on Phases 1–5 being semantically final (types + capabilities + CLI config shape stable).

Files touched:
- `messenger/docs/user-guide.md`
- `.claude/skills/messenger/SKILL.md`
- `messenger/README.md`

Steps (all three may be authored in parallel):

6.1 `messenger/docs/user-guide.md`: add a "Discord (Webhook)" section alongside the existing "Discord (Bot)" section. Content:
    - When to prefer webhooks over bots (notification-only, no gateway, no Discord app required beyond server perms).
    - How to create a webhook (Server Settings → Integrations → Webhooks → New Webhook → Copy URL).
    - `webhook_url` vs `webhook_url_env` choice; default `DISCORD_WEBHOOK_URL` env var.
    - Clarify: the URL binds the channel — no separate `channel_id` field.
    - `thread_id` is a dispatch-time field on `Target::DiscordWebhook`, not a route config field; show a short library example.
    - Note capability differences: **no replies**, markdown + attachments supported.
    - CLI usage sample: `messenger send "hello" --provider discord-webhook --channel https://discord.com/api/v10/webhooks/…`.

6.2 `.claude/skills/messenger/SKILL.md`: extend the Provider Support table so Discord and Discord-Webhook are two rows. The Discord-Webhook row must show `Replies: No`. Update any narrative that implies a single Discord entry.

6.3 `messenger/README.md`: update the supported-providers list (or equivalent table) to include `Discord-Webhook` alongside `Discord`, if such a list exists. If the README only links to the skill or user-guide, no change is needed — confirm during this step.

Validation checkpoint:
- `pnpm test` (top-level markdown/link tests, if any) green. Otherwise visual review only.
- Skim pass: capability rows consistent between SKILL.md table and the library `capabilities()` impl in Phase 2.

Parallelization: 6.1, 6.2, 6.3 can run in parallel.

---

## Final Acceptance Matrix (traces back to spec §Acceptance Criteria)

| Spec criterion | Covered in |
|----------------|------------|
| Config round-trip: both/one/neither field | Phase 5 step 5.3 (Tests A, B, C, D) |
| URL resolution: direct > env > default `DISCORD_WEBHOOK_URL` | Phase 5 step 5.4 |
| Wiremock send with expected payload | Phase 3 step 3.3 |
| Plan-time reply_to hard error, no network call | Phase 3 step 3.2 + Phase 1 step 1.4 enforcement |
| Setup flow produces a round-trippable `RouteConfig::DiscordWebhook` | Phase 5 step 5.5 (non-interactive postcondition) |
| Existing bot tests unchanged and green | Phase 3 step 3.4 |

## Out-of-Plan Reminders (from spec §Out of Scope)

- No per-message `username`/`avatar_url` override.
- No webhook edit/delete endpoints.
- No migration of existing `RouteConfig::Discord` users.
- Rate-limit policy stays passthrough — structured `MessengerError::RateLimited` only; no retry loop.

## Rollback Notes

All new code paths are additive:
- New `ProviderKind`, `Target`, `MessageRef`, `RouteConfig`, and `RouteProvider` variants.
- New `discord_webhook.rs` module.
- New tests.

Rolling back is a clean revert of Phases 1–5 source edits + Phase 6 docs; no in-place migration is introduced.
