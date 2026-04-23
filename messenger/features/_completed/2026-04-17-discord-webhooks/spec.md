# Discord Webhook Provider

## Summary

Continue supporting the existing Discord bot integration unchanged, and add Discord
webhook delivery as a parallel, separately-registered provider in the messenger
library and CLI. Webhook routes carry their own configuration shape, capability
set, target type, and receipt type so callers can route deliberately between the
two transports.

## Background and References

- Messenger Discord research:
  `/Users/ken/.claudine/worktrees/rusty-biscuit/messenger/messenger/docs/research/platforms/discord.md`
- The original spec referenced `claudine/docs/research/platforms/discord.md`; that
  path does not resolve in the worktree. The authoritative reference is the
  `messenger/docs/research/platforms/discord.md` document above.

## In Scope

The three decisions confirmed during clarification, written as binding
requirements for this feature:

1. Discord webhooks are added as a **separate provider kind** (not as a mode on
   the existing Discord provider). Routing, capability advertisement, target
   typing, and receipt typing are all distinct from the bot path.
2. The CLI gains a **new `RouteConfig::DiscordWebhook` variant**. The existing
   `RouteConfig::Discord` variant is not extended. Webhook URL secrets flow
   through the same direct-or-env-var resolution path used for bot tokens.
3. Attempting to use reply-to semantics against a webhook target is a
   **plan-time hard error**, not a silently dropped field. The error surfaces
   before any network call.

## Out of Scope

- **Per-message username/avatar override.** Webhooks support overriding the
  displayed username and avatar per request, but exposing this would require a
  cross-provider change to `Message` (e.g. a `sender_override` field) and
  deserves its own design discussion. Deferred.
- **Webhook edit and delete endpoints.** Only the basic webhook send path is in
  scope unless edit/delete turn out to be naturally required to implement send.
- **Migration of existing `RouteConfig::Discord` users.** The bot path is
  unchanged. No config migration, deprecation, or alias is introduced.
- **Choice between `twilight-http` and a direct `reqwest` call** for the
  webhook send. Tracked under Open Questions; the spec does not mandate either.

## Library-Level Requirements

- Add a new variant `ProviderKind::DiscordWebhook` alongside the existing
  `ProviderKind::Discord`.
- Add a new variant `Target::DiscordWebhook { thread_id: Option<String> }`
  alongside `Target::Discord`. `thread_id` is a dispatch-time concern that
  mirrors `TelegramTarget.thread_id`; it is not a route-level field.
- Add a new variant `MessageRef::DiscordWebhook` so receipts produced by webhook
  sends are typed distinctly from bot-message receipts.
- Add a new `DiscordWebhookProvider` struct backed by a `DiscordWebhookConfig`
  that carries the resolved webhook URL.
- The provider must report an honest `CapabilitySet`:
    - `supports_reply: false`
    - `supports_attachments: true`
    - `supports_markdown: true`
    - All other capability fields match the existing Discord provider's
      capabilities unless the research doc explicitly contradicts.
- The existing Discord bot provider's `CapabilitySet` is unchanged.
- `plan_send()` must detect `Dispatch.reply_to.is_some()` when the target is
  `Target::DiscordWebhook` and return `MessengerError::UnsupportedCapability`
  (or the equivalently named existing typed error variant) before issuing any
  network request.

Rationale for splitting at the provider boundary: `ProviderKind` is already the
routing primitive, `CapabilitySet` is static per provider, and this split
mirrors how Signal vs. Telegram are kept separate today.

## CLI-Level Requirements

- Add a new `RouteConfig::DiscordWebhook` variant with fields:
    - `webhook_url: Option<String>` for direct configuration.
    - `webhook_url_env: Option<String>` for env-var indirection. The default
      value is `"DISCORD_WEBHOOK_URL"`, matching the repo-wide pattern used by
      `DISCORD_BOT_TOKEN`.
- Do not add a `channel_id` field to this variant. The webhook URL of the form
  `/api/v10/webhooks/{id}/{token}` already binds the channel; restating it
  would invite drift between the URL and the configured channel ID.
- Extend `RouteProvider::ALL` to include `DiscordWebhook`. The clap `value_name`
  is expected to be `discord-webhook` (final spelling to be confirmed during
  implementation against the existing clap conventions).
- The `setup` command must prompt for a webhook URL when the user selects this
  route kind, and must store either the literal URL or the env-var name
  according to the user's choice — same behavior model as bot-token entry.
- The webhook URL must flow through the same secret-resolution path that bot
  tokens use today (direct value vs. env-var lookup).

## Documentation Updates Required

The following docs must be updated as part of this feature:

- `messenger/docs/user-guide.md` — add a Discord webhook setup walkthrough
  alongside the existing Discord bot section. Cover obtaining the webhook URL,
  the `webhook_url` vs. `webhook_url_env` choice, the default
  `DISCORD_WEBHOOK_URL` env var, and how `thread_id` is provided at dispatch
  time rather than in the route config.
- `.claude/skills/messenger/SKILL.md` — update the provider table to list
  `Discord` and `Discord-Webhook` as separate entries, with their distinct
  capability rows (notably `supports_reply: false` for webhooks).
- CLI `--help` output and the interactive `setup` walkthrough — must reflect
  the new `DiscordWebhook` route option.
- `messenger/README.md` — update if it lists supported providers.

## Acceptance Criteria

The following are the testable items that gate this feature:

- Unit test: `RouteConfig::DiscordWebhook` round-trips through the CLI config
  serializer/deserializer with both `webhook_url` and `webhook_url_env` set,
  with only one set, and with neither set (degenerate / error case as
  appropriate).
- Unit test: webhook URL resolution prefers the explicit `webhook_url` over
  `webhook_url_env`, and falls back to `DISCORD_WEBHOOK_URL` when
  `webhook_url_env` is unset.
- Integration test (wiremock): a webhook send POSTs to the mocked Discord
  webhook endpoint with the expected payload shape, mirroring the test pattern
  used by other providers in the workspace.
- Plan-time test: `plan_send()` against a `Target::DiscordWebhook` with a
  non-`None` `Dispatch.reply_to` returns
  `MessengerError::UnsupportedCapability` (or the existing equivalent typed
  variant) and does **not** perform a network call.
- Setup-flow smoke test: running the CLI `setup` command, selecting the
  `DiscordWebhook` route kind, and providing a webhook URL produces a
  `RouteConfig::DiscordWebhook` entry that round-trips on read.
- Regression: all existing Discord bot tests continue to pass without
  modification, and the bot path's observable behavior is unchanged.

## Open Questions and Risks

- **HTTP client choice.** Whether webhook sends should reuse `twilight-http`
  (already pulled in for the bot path) or call the webhook endpoint directly
  with `reqwest` is deferred to the implementation phase.
- **Rate-limit handling.** Webhook rate limits are scoped per webhook and are
  separate from the bot's global buckets; the research doc notes
  `X-RateLimit-Scope: shared`. Behavior under 429 (back-off, surface to caller,
  retry policy) should be characterized during implementation and may warrant a
  follow-up spec if it grows beyond passthrough.
- **Final clap `value_name`** for the new `RouteProvider` variant — confirmed
  during implementation against existing CLI naming conventions.
