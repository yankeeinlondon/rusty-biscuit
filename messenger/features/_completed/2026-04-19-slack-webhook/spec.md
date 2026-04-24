# Slack Webhook Provider

## Summary

Continue supporting the existing Slack bot integration unchanged, and add Slack
Incoming Webhook delivery as a parallel, separately-registered provider in the
messenger library and CLI. Webhook routes carry their own configuration shape,
capability set, target type, and receipt type so callers can route deliberately
between the two transports.

This follows the same architectural split used for Discord bot vs. Discord
webhook: a distinct `ProviderKind`, `Target` variant, `MessageRef` variant, and
CLI `RouteConfig` variant.

## Background and References

- Messenger Slack research:
  `/Users/ken/.claudine/worktrees/rusty-biscuit/messenger/messenger/docs/research/platforms/slack.md`
- Discord webhook spec (design precedent):
  `/Users/ken/.claudine/worktrees/rusty-biscuit/messenger/messenger/features/2026-04-17-discord-webhooks/spec.md`
- Slack Incoming Webhooks documentation:
  <https://api.slack.com/messaging/webhooks>
- Slack Web API OpenAPI spec:
  `https://raw.githubusercontent.com/slackapi/slack-api-specs/master/web-api/slack_web_openapi_v2.json`

### Slack Incoming Webhooks — Key Characteristics

Incoming Webhooks are a one-way URL-based interface for posting messages. Key
properties that inform the design below:

- **URL format**: `https://hooks.slack.com/services/T{workspace}/B{app}/{token}`
- **Authentication**: embedded in the URL — no `Authorization` header required.
- **Channel binding**: the URL is created for a specific channel, but the
  `channel` field in the payload can override it.
- **Payload**: JSON body with `text` (required in practice), plus optional
  `blocks`, `attachments` (structured Slack message attachments, **not** file
  uploads), `thread_ts`, `unfurl_links`, `unfurl_media`, `username`,
  `icon_emoji`, `icon_url`.
- **Response**: `{"ok": true}` on success, `{"ok": false, "error": "..."}` on
  failure. **No message identifier (`ts`) is returned**, unlike the
  `chat.postMessage` Web API.
- **Rate limits**: HTTP 429 with `Retry-After` header. Scoped per-webhook.

## In Scope

1. Slack Incoming Webhooks are added as a **separate provider kind** (not as a
   mode on the existing Slack provider). Routing, capability advertisement,
   target typing, and receipt typing are all distinct from the bot path.
2. The CLI gains a **new `RouteConfig::SlackWebhook` variant**. The existing
   `RouteConfig::Slack` variant is not extended. Webhook URL secrets flow
   through the same direct-or-env-var resolution path used for bot tokens.
3. The webhook provider supports **reply threading** via `thread_ts` extracted
   from `dispatch.reply_to`, but only when the `MessageRef` is a
   `MessageRef::SlackWebhook`. Cross-provider replies (e.g. `MessageRef::Slack`
   used against a `SlackWebhook` target) are a plan-time hard error via the
   existing provider-mismatch check in `validate::normalize_dispatch`.

## Out of Scope

- **File attachment uploads.** Slack Incoming Webhooks do not support multipart
  file uploads. The `attachments` field in the webhook payload represents
  structured Slack message attachments (rich text cards, not files), which is
  a different concept from the messenger library's `Attachment` type.
  `supports_attachments: false`.
- **Channel override via payload `channel` field.** The webhook URL binds the
  default channel. Allowing the payload to override it would require a
  channel-like field on `RouteConfig` or `Target`, which deserves its own
  design discussion. Deferred.
- **Block Kit rendering.** The messenger library's `Message` type carries
  markdown or plain text, not Slack Block Kit structures. The webhook provider
  will send `text` only (rendered to Slack mrkdwn, same as the existing bot
  provider). Block Kit support would require a cross-provider content model
  change. Deferred.
- **Per-message username/icon override.** Webhooks support overriding the
  displayed username and icon per request, but exposing this would require a
  cross-provider change to `Message` (e.g. a `sender_override` field). Deferred
  — same rationale as the Discord webhook spec.
- **Migration of existing `RouteConfig::Slack` users.** The bot path is
  unchanged. No config migration, deprecation, or alias is introduced.
- **Message edit and delete.** Incoming Webhooks are fire-and-forget — there is
  no message identifier returned to use for subsequent edits or deletes.

## Library-Level Requirements

- Add a new variant `ProviderKind::SlackWebhook` alongside the existing
  `ProviderKind::Slack`.

- Add a new variant `Target::SlackWebhook(SlackWebhookTarget)` alongside
  `Target::Slack(SlackTarget)`. `SlackWebhookTarget` is an empty struct (no
  `channel_id` — the webhook URL binds the channel; no `thread_ts` — threading
  is handled via `dispatch.reply_to`). Provide convenience constructors
  `Target::slack_webhook()` (returns `Target::SlackWebhook(SlackWebhookTarget
  {})`) matching the pattern used by `Target::discord_webhook()`.

- Add a new variant `MessageRef::SlackWebhook { thread_ts: Option<String> }`.
  Slack Incoming Webhooks return `{"ok": true}` without a message timestamp or
  channel identifier, so neither `thread_ts` nor `raw_id` can be populated from
  the response. The `thread_ts` field is therefore always `None` on a
  freshly-created receipt; it exists so that callers who obtain a `ts` through
  another channel (e.g. the bot Web API) can construct a `MessageRef` that
  threads into a webhook-bound channel.

- Add a new `SlackWebhookProvider` struct backed by a `SlackWebhookConfig` that
  carries the resolved webhook URL.

- The provider must report an honest `CapabilitySet`:

  | Capability | Value | Rationale |
  |---|---|---|
  | `supports_markdown_rendering` | `true` | Slack mrkdwn via `text` field |
  | `supports_reply` | `true` | `thread_ts` supported in Incoming Webhook payloads |
  | `supports_attachments` | `false` | No multipart upload; webhook `attachments` are Slack message attachments, not files |
  | `supports_location` | `true` | Text fallback appended to body (same as bot) |
  | `supports_silent_delivery` | `false` | No such feature in Incoming Webhooks |
  | `supports_link_preview_control` | `true` | `unfurl_links` / `unfurl_media` supported |

- The existing Slack bot provider's `CapabilitySet` is unchanged.

- **No Discord-style reply hard-error.** Unlike `DiscordWebhook` (which cannot
  thread at all and hard-errors on `reply_to`), `SlackWebhook` supports
  threading via `thread_ts`. The existing provider-mismatch guard in
  `normalize_dispatch` already catches cross-provider `reply_to` misuse (e.g.
  `MessageRef::Slack` used with a `SlackWebhook` target). No additional
  mode-independent hard-error is needed for this provider.

- **Webhook URL validation.** The provider must validate the webhook URL at
  construction time, mirroring `DiscordWebhookProvider::try_new`. Each rule
  below returns `MessengerError::InvalidMessage` on failure (matching the
  existing Discord precedent — `MessengerError` has no `Configuration` or
  `InvalidConfig` variant, and `MissingConfiguration` is reserved for absent
  fields, not malformed values):
    - Scheme must be `https`.
    - Host must equal `hooks.slack.com` exactly (case-insensitive host
      comparison per RFC 3986; no subdomains).
    - Path must begin with the literal prefix `/services/`.
    - Exactly three non-empty path segments must follow `/services/`. A
      segment is non-empty if it contains at least one character after URL
      decoding; whitespace-only segments are rejected.
    - No trailing slash after the third segment.
    - No query string.
    - No fragment.
    - Path segment matching is case-sensitive (Slack path tokens are
      case-sensitive in practice).
    - Validation occurs at provider construction time (mirroring
      `DiscordWebhookProvider::try_new`).

- **Reply threading behavior.** When `dispatch.reply_to` contains a
  `MessageRef::SlackWebhook { thread_ts: Some(ts) }`, the provider includes
  that `ts` as `thread_ts` in the payload. When `reply_to` is `None`, or when
  it is `MessageRef::SlackWebhook { thread_ts: None }`, the message posts as a
  top-level message in the channel bound to the webhook URL.

- **Link preview control.** The provider reads
  `dispatch.options.disable_link_preview` — the same field used by the Slack
  bot and Telegram providers; no new API surface is introduced. When
  `disable_link_preview` is `true`, the provider includes `unfurl_links:
  false` and `unfurl_media: false` in the payload. When `disable_link_preview`
  is `false` (the default), both fields are omitted so Slack applies its
  default unfurl behavior. This matches the existing `SlackProvider` behavior
  exactly (see `messenger/lib/src/provider/slack.rs` lines 107–111), ensuring
  cross-provider consistency and honoring the `supports_link_preview_control:
  true` capability row.

- **Response handling.** The Incoming Webhook response is `{"ok": true}` on
  success with no message identifier. The provider must:
  - Return a `SendReceipt` with `MessageRef::SlackWebhook { thread_ts: None }`.
    Slack webhooks return no `ts`, so `thread_ts` is always `None` on a
    newly-created receipt.
  - Set `raw_id` to an empty string, since Slack provides no message ID. The
    provider adds a `metadata` entry `"delivery_confirmed": "true"` on
    successful sends so consumers can distinguish "delivered but unknown ID"
    from an error.
  - On `{"ok": false, "error": "..."}`:
    - `invalid_token`, `no_service`, `no_team`, `action_prohibited` →
      `MessengerError::Authentication`
    - `invalid_payload`, `channel_is_archived`, `channel_not_found` →
      `MessengerError::InvalidMessage`
    - All other error codes → `MessengerError::Provider`
  - On HTTP 429 (respect `Retry-After` header) → `MessengerError::RateLimited`.
  - On 5xx → `MessengerError::Transport`.

  These error codes are drawn from the Slack Incoming Webhooks surface (not
  the Web API), which returns a different set of strings than
  `chat.postMessage` and other Web API methods.

Rationale for splitting at the provider boundary: `ProviderKind` is already the
routing primitive, `CapabilitySet` is static per provider, and this split
mirrors how Discord vs. Discord-Webhook and Signal vs. Telegram are kept
separate. The differing capability sets (bot: `attachments: false, reply: true,
link_preview: true` vs. webhook: `attachments: false, reply: true,
link_preview: true`) are currently identical, but the authentication model,
target resolution, and receipt semantics are fundamentally different.

## CLI-Level Requirements

- Add a new `RouteConfig::SlackWebhook` variant with fields:
    - `webhook_url: Option<String>` for direct configuration.
    - `webhook_url_env: Option<String>` for env-var indirection. The default
      value is `"SLACK_WEBHOOK_URL"`, matching the repo-wide pattern used by
      `SLACK_BOT_TOKEN`.

- **Secret resolution precedence.** When `webhook_url` is set (non-empty), it
  wins unconditionally. When `webhook_url` is unset (`None`) or empty,
  resolution falls back to the env var named by `webhook_url_env`, defaulting
  to `SLACK_WEBHOOK_URL` when `webhook_url_env` is itself unset. If neither
  the direct value nor the env var yields a non-empty URL, provider
  construction returns `MessengerError::MissingConfiguration`. This mirrors
  the existing bot-token resolution path.

- Do not add a `channel_id` field to this variant. The webhook URL of the form
  `https://hooks.slack.com/services/T.../B.../xxx` already binds the default
  channel; restating it would invite drift between the URL and the configured
  channel.

- Extend `RouteProvider::ALL` to include `SlackWebhook`. The clap `value_name`
  is expected to be `slack-webhook` (final spelling to be confirmed during
  implementation against the existing clap conventions).

- The `setup` command must prompt for a webhook URL when the user selects this
  route kind, and must store either the literal URL or the env-var name
  according to the user's choice — same behavior model as bot-token entry.

- **Secret handling at setup.** The webhook URL is a bearer secret (anyone
  with the URL can post to the channel). The `setup` prompt should use a
  masked input mode (matching how bot tokens are collected today), must not
  echo the URL in confirmation messages or log output, and must reject empty
  input before writing the config. When the user chooses env-var indirection,
  only the env-var *name* is stored in the config file — never the resolved
  URL.

- The webhook URL must flow through the same secret-resolution path that bot
  tokens use today (direct value vs. env-var lookup).

## Documentation Updates Required

The following docs must be updated as part of this feature:

- `messenger/docs/user-guide.md` — add a Slack webhook setup walkthrough
  alongside the existing Slack bot section. Cover obtaining the webhook URL
  from the Slack admin console, the `webhook_url` vs. `webhook_url_env` choice,
  the default `SLACK_WEBHOOK_URL` env var, and the key behavioral differences
  from the bot path (no file uploads, no message ID in receipts, limited
  receipt utility for downstream operations).
- `.claude/skills/messenger/SKILL.md` — update the provider table to list
  `Slack` and `Slack-Webhook` as separate entries, with their distinct
  capability rows (notably `supports_attachments: false` and `supports_reply:
  true` for webhooks, matching the bot's capabilities but with different auth
  and receipt semantics).
- CLI `--help` output and the interactive `setup` walkthrough — must reflect
  the new `SlackWebhook` route option.
- `messenger/README.md` — update if it lists supported providers.

## Acceptance Criteria

The following are the testable items that gate this feature:

- Unit test: `RouteConfig::SlackWebhook` round-trips through the CLI config
  serializer/deserializer with both `webhook_url` and `webhook_url_env` set,
  with only one set, and with neither set (degenerate / error case as
  appropriate).
- Unit test: webhook URL resolution prefers the explicit `webhook_url` over
  `webhook_url_env`, and falls back to `SLACK_WEBHOOK_URL` when
  `webhook_url_env` is unset.
- Integration test (wiremock): a webhook send POSTs to the mocked Slack webhook
  endpoint with the expected payload shape (`text`, optional `thread_ts`,
  optional `unfurl_links`/`unfurl_media`), mirroring the test pattern used by
  other providers in the workspace.
- Unit test: the provider's `CapabilitySet` matches the table in
  §Library-Level Requirements.
- Integration test (wiremock): reply threading — sending with a `reply_to`
  containing a `MessageRef::SlackWebhook { thread_ts: Some(...) }` includes
  `thread_ts` in the posted payload.
- Integration test (wiremock): error mapping — responses with `invalid_token`
  and `action_prohibited` resolve to `MessengerError::Authentication`;
  `invalid_payload` and `channel_is_archived` resolve to
  `MessengerError::InvalidMessage`; unknown error strings resolve to
  `MessengerError::Provider`.
- Integration test (wiremock): link preview control —
  `Dispatch::to(...).disable_link_preview()` produces a payload with
  `unfurl_links: false` and `unfurl_media: false`; the default `Dispatch`
  omits both fields.
- Unit test: `SlackWebhookProvider::try_new` rejects URLs that violate any of
  the validation rules in §Library-Level Requirements > "Webhook URL
  validation" — specifically: wrong scheme (`http://`), wrong host
  (`hooks.slack.example`, subdomain `a.hooks.slack.com`), wrong prefix
  (`/services_v2/...`), fewer than three segments, more than three segments,
  empty segment (e.g. `//`), whitespace-only segment, trailing slash, query
  string, fragment. Each case returns `MessengerError::InvalidMessage`.
- Integration test (wiremock): transport error handling — HTTP 429 with
  `Retry-After: 30` resolves to `MessengerError::RateLimited` with the
  retry-after value surfaced; HTTP 500/502/503 resolve to
  `MessengerError::Transport`.
- Plan-time test: `plan_send()` against a `Target::SlackWebhook` with a
  `reply_to` containing a `MessageRef::Slack` (bot variant) returns
  `MessengerError::InvalidMessage` due to provider mismatch — no network call.
- Setup-flow smoke test: running the CLI `setup` command, selecting the
  `SlackWebhook` route kind, and providing a webhook URL produces a
  `RouteConfig::SlackWebhook` entry that round-trips on read.
- Regression: all existing Slack bot tests continue to pass without
  modification, and the bot path's observable behavior is unchanged.

## Open Questions and Risks

### Resolved Decisions

- **`MessageRef::SlackWebhook` shape.** The variant carries
  `thread_ts: Option<String>` only. Since Incoming Webhooks return no `ts`,
  `thread_ts` is always `None` on a fresh send. Callers wanting to thread into
  a webhook message must obtain the `ts` through another channel (e.g. the
  bot Web API). A future enhancement could use `chat.postMessage` instead of
  Incoming Webhooks for the initial send (which would return `ts`), but that
  requires a bot token and defeats the webhook-only use case.
- **`slack-hook` crate vs. direct `reqwest`.** Use `reqwest` directly. The
  research doc recommends `slack-morphism` for full Slack apps and
  `slack-hook` for simple webhook usage, but the existing Slack bot provider
  already uses `reqwest` directly and the Discord webhook provider followed
  the same pattern. Using `reqwest` directly keeps the dependency footprint
  minimal and maintains consistency with the rest of the codebase. The
  `slack-hook` crate's value-add (type-safe payload construction) does not
  justify pulling in an additional dependency for a single POST endpoint.
- **Empty `raw_id` on receipts.** Documented limitation. Slack Incoming
  Webhooks return no message identifier, so callers that rely on `raw_id` for
  deduplication or downstream operations (edit, delete) cannot use the
  webhook receipt for those purposes. To distinguish "delivered but unknown
  ID" from an error, the provider adds a `metadata` entry
  `"delivery_confirmed": "true"` on successful sends.

### Outstanding Questions and Risks

These items do not block initial implementation but must be addressed
before/during rollout:

- **Rate-limit back-off policy.** Incoming Webhook rate limits are scoped
  per-webhook. The research doc notes Slack uses tiered rate limits. Behavior
  under 429 (back-off, surface to caller, retry policy) should be
  characterized during implementation and may warrant a follow-up spec if it
  grows beyond passthrough.
- **Final clap `value_name`** for the new `RouteProvider` variant — confirmed
  during implementation against existing CLI naming conventions.
