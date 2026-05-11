# Slack Platform Guide

The `slack` provider delivers messages to Slack channels via two adapters behind a single feature flag:

- **`SlackProvider`** — bot token, full capability with addressable receipts
- **`SlackWebhookProvider`** — webhook URL, notification-only (no file uploads, simplified receipts)

Both adapters share the same **mrkdwn** renderer and link-preview controls. The webhook adapter supports reply threading via `MessageRef::SlackWebhook { thread_ts: Some(...) }`, but rejects attachments (file uploads require the Web API).

| Adapter | Auth | Channel binding | Replies | Attachments | Link preview control |
|---------|------|-----------------|---------|-------------|---------------------|
| Bot (`SlackProvider`) | Bot token (`xoxb-...`) | Per-send `channel_id` | Yes | No | Yes |
| Webhook (`SlackWebhookProvider`) | Webhook URL | Bound by URL | Yes (thread_ts only) | No | Yes |

## Capability Summary

### Slack Bot

- `supports_markdown_rendering`: `true` — Slack mrkdwn format
- `supports_reply`: `true` — thread replies via `thread_ts`
- `supported_attachment_kinds`: `{}` — attachments drop in best-effort and error in strict
- `supports_location`: `true` — rendered as text fallback
- `supports_silent_delivery`: `false`
- `supports_link_preview_control`: `true` — `unfurl_links` + `unfurl_media`

### Slack Webhook

- `supports_markdown_rendering`: `true` — same mrkdwn renderer as bot
- `supports_reply`: `true` — via `thread_ts` in reply_to
- `supported_attachment_kinds`: `{}` — attachments drop in best-effort and error in strict
- `supports_location`: `true` — rendered as text fallback
- `supports_silent_delivery`: `false`
- `supports_link_preview_control`: `true` — `unfurl_links` + `unfurl_media`

## Enabling

Library:

```toml
[dependencies]
messenger = { version = "0.1", features = ["slack"] }
```

The `slack` feature is enabled by default. CLI: `messenger-cli` enables it by default.

## Quick Test

### Bot adapter

```bash
export SLACK_BOT_TOKEN="xoxb-your-bot-token"
messenger send --provider slack --channel C01234567 "Hello from messenger"
```

### Webhook adapter

```bash
messenger send --provider slack-webhook --webhook-url https://hooks.slack.com/services/T000/B000/XXXXX "Hello from webhook"
```

## Authentication

### Bot Token

Slack uses OAuth 2.0 for bot apps. The token prefix is `xoxb-...`.

1. Create an app at [api.slack.com/apps](https://api.slack.com/apps)
2. Navigate to **OAuth & Permissions**
3. Add scopes: `chat:write`, `chat:write.public` (for public channels)
4. **Install to Workspace** and copy the **Bot User OAuth Token**

Library usage:

```rust
use messenger::prelude::*;
use secrecy::SecretString;

let provider = SlackProvider::new(SlackConfig {
    bot_token: SecretString::from(std::env::var("SLACK_BOT_TOKEN").unwrap()),
    api_base_url: None, // optional: override for testing with wiremock
});
```

### Webhook URL

1. In your Slack app settings, go to **Incoming Webhooks**
2. **Add New Webhook to Workspace** → choose channel → copy URL

The URL must match `https://hooks.slack.com/services/{team}/{app}/{token}`. The provider validates this strictly at construction time:

- Must use `https` scheme
- Host must be `hooks.slack.com` (case-insensitive)
- Path must begin with `/services/`
- Must have exactly 3 non-empty segments after `/services/`
- No query strings, fragments, or trailing slashes

Library usage:

```rust
let provider = SlackWebhookProvider::try_new(SlackWebhookConfig {
    webhook_url: SecretString::from("https://hooks.slack.com/services/T000/B000/XXXXX"),
})?;
```

## Field Mapping

### Bot Adapter

| Portable | Slack Web API (`chat.postMessage`) |
|----------|-----------------------------------|
| `body` (Markdown) | `text` — rendered to Slack mrkdwn |
| `location` | appended to `text` as text fallback |
| `reply_to` (`MessageRef::Slack { thread_ts }`) | `thread_ts` |
| `disable_link_preview` | `unfurl_links: false`, `unfurl_media: false` |

### Webhook Adapter

| Portable | Slack Incoming Webhook |
|----------|------------------------|
| `body` (Markdown) | `text` — rendered to Slack mrkdwn |
| `location` | appended to `text` as text fallback |
| `reply_to` (`MessageRef::SlackWebhook { thread_ts: Some(ts) }`) | `thread_ts` |
| `disable_link_preview` | `unfurl_links: false`, `unfurl_media: false` |

## Markdown Rendering

Slack mrkdwn differs from standard Markdown:

| Construct | Slack mrkdwn |
|-----------|-------------|
| `**bold**` | `*bold*` |
| `_italic_` | `_italic_` |
| `~~strike~~` | `~strike~` |
| `` `code` `` | `` `code` `` |
| "```lang\ncode\n```" | "```\ncode\n```" (no language tag) |
| `[text](url)` | `<url\|text>` or `<url>` when text == URL |
| Lists | `• ` or `1. ` prefix |
| Headings | rendered as `*heading*` (bold) |

## Receipts

### Bot

```json
{
  "provider": "Slack",
  "message_ref": {
    "Slack": {
      "channel_id": "C01234567",
      "thread_ts": "1712345678.000100"
    }
  },
  "raw_id": "1712345678.000100"
}
```

Slack message identifiers are **timestamps** (e.g. `1712345678.000100`), not UUIDs. The `thread_ts` is both the message ID and the thread identifier.

### Webhook

```json
{
  "provider": "SlackWebhook",
  "message_ref": {
    "SlackWebhook": {
      "thread_ts": null
    }
  },
  "raw_id": "",
  "metadata": {
    "delivery_confirmed": "true"
  }
}
```

Slack incoming webhooks do not return a message identifier. The receipt has an empty `raw_id` and `thread_ts: None`. Successful delivery is confirmed via `metadata["delivery_confirmed"] = "true"`.

## Error Handling

### Bot Adapter

| Slack error | Messenger error | Action |
|-------------|-----------------|--------|
| `invalid_auth`, `not_authed`, `token_revoked` | `Authentication` | Regenerate bot token |
| `channel_not_found` | `Provider` | Check channel ID; bot may not be in channel |
| `is_archived` | `Provider` | Channel is archived |
| `rate_limited` | `RateLimited` | Back off and retry |

### Webhook Adapter

| Slack error | Messenger error | Action |
|-------------|-----------------|--------|
| `invalid_token`, `no_service`, `no_team`, `action_prohibited` | `Authentication` | Regenerate webhook URL |
| `invalid_payload`, `channel_is_archived`, `channel_not_found` | `InvalidMessage` | Check payload and channel |
| `429` response | `RateLimited` | Back off using `Retry-After` header |

## Troubleshooting

- **`Authentication` error** — Bot token is invalid or the app was uninstalled. Check the token in Slack app settings and re-install if needed.
- **`InvalidMessage: expected Slack target`** — The `Target` enum variant must be `Slack` or `SlackWebhook`, not another provider type.
- **Webhook URL rejected at construction** — Ensure the URL exactly matches `https://hooks.slack.com/services/T000/B000/XXXXX` with no query strings, fragments, or trailing slashes.
- **Messages not appearing in channel** — The bot may not be a member of the channel. For private channels, invite the bot. For public channels, ensure `chat:write.public` scope is granted.
- **Thread replies not threading** — Pass the parent message's `ts` (timestamp) as `reply_to`, not the message's ID. Slack threads are identified by the parent timestamp.
- **Link previews still showing** — `disable_link_preview` sets `unfurl_links: false` and `unfurl_media: false`. Some Slack workspaces override this at the workspace level.
- **Rich formatting not working** — Ensure the body is `MessageBody::Markdown`. Plain text does not trigger the mrkdwn renderer. Remember Slack uses `*bold*` not `**bold**`.

## Related Documents

- [User Guide](../user-guide.md) — platform setup, CLI configuration, library usage.
- [messenger README](../../README.md) — high-level package overview.
- [messenger-cli README](../../cli/README.md) — CLI flags, route shapes, setup flow.
- [Research: Slack API Deep Dive](../../docs/research/platforms/slack.md) — full API research notes.
