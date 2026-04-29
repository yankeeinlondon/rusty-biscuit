# Discord Platform Guide

The `discord` provider delivers messages to Discord channels via two adapters behind a single feature flag:

- **`DiscordProvider`** — bot token, full capability (replies, attachments, rich text)
- **`DiscordWebhookProvider`** — webhook URL, notification-only (no replies, but supports attachments and rich text)

Both adapters share the same Markdown renderer; the practical difference is transport capability, not formatting syntax.

| Adapter | Auth | Channel binding | Replies | Attachments |
|---------|------|-----------------|---------|-------------|
| Bot (`DiscordProvider`) | Bot token | Per-send `channel_id` | Yes | Yes |
| Webhook (`DiscordWebhookProvider`) | Webhook URL | Bound by URL | No | Yes |

## Capability Summary

### Discord Bot

- `supports_markdown_rendering`: `true` — Discord-flavoured Markdown
- `supports_reply`: `true`
- `supported_attachment_kinds`: `{ Image, Audio, Video, Document, Binary }`
- `supports_location`: `true` — rendered as text fallback
- `supports_silent_delivery`: `false`
- `supports_link_preview_control`: `false`

### Discord Webhook

- `supports_markdown_rendering`: `true` — same renderer as bot
- `supports_reply`: `false` — `plan_send()` and `send()` reject `reply_to` with `UnsupportedFeature`
- `supported_attachment_kinds`: `{ Image, Audio, Video, Document, Binary }`
- `supports_location`: `true` — rendered as text fallback
- `supports_silent_delivery`: `false`
- `supports_link_preview_control`: `false`

## Enabling

Library:

```toml
[dependencies]
messenger = { version = "0.1", features = ["discord"] }
```

The `discord` feature is enabled by default. CLI: `messenger-cli` enables it by default.

## Quick Test

### Bot adapter

```bash
messenger send --provider discord --channel 123456789012345678 "Hello from messenger"
```

### Webhook adapter

```bash
messenger send --provider discord-webhook --webhook-url https://discord.com/api/v10/webhooks/123/token "Hello from webhook"
```

## Authentication

### Bot Token

1. Create an application at the [Discord Developer Portal](https://discord.com/developers/applications)
2. Navigate to **Bot** → **Add Bot**
3. Copy the token
4. Invite the bot to your server with the **Send Messages** permission

Library usage:

```rust
use messenger::prelude::*;
use secrecy::SecretString;

let provider = DiscordProvider::new(DiscordConfig {
    bot_token: SecretString::from(std::env::var("DISCORD_BOT_TOKEN").unwrap()),
});
```

### Webhook URL

1. In your Discord server, open **Server Settings** → **Integrations** → **Webhooks**
2. **New Webhook** → choose channel → copy URL

The URL must match the form `https://discord.com/api/v{version}/webhooks/{id}/{token}`. The provider validates this at construction time and rejects malformed URLs with `MessengerError::InvalidMessage`.

Library usage:

```rust
let provider = DiscordWebhookProvider::try_new(DiscordWebhookConfig {
    webhook_url: SecretString::from("https://discord.com/api/v10/webhooks/1234567890/abc-token"),
})?;
```

## Field Mapping

### Bot Adapter

| Portable | Discord REST API |
|----------|------------------|
| `body` (Markdown) | `content` — rendered to Discord Markdown |
| `attachments` | `attachments` multipart upload |
| `attachment.alt_text` / `caption` | `description` on Discord attachment |
| `location` | appended to `content` as text fallback |
| `reply_to` (`MessageRef::Discord { message_id }`) | `message_reference.message_id` |

The bot adapter uses `twilight-http` for transport. Channel IDs and message IDs are parsed as `u64`; invalid IDs produce `MessengerError::InvalidMessage`.

### Webhook Adapter

| Portable | Discord Webhook Execute |
|----------|------------------------|
| `body` (Markdown) | `content` — rendered to Discord Markdown |
| `attachments` | multipart `files[{index}]` + `payload_json` |
| `attachment.alt_text` / `caption` | `attachments[].description` in payload JSON |
| `location` | appended to `content` as text fallback |
| `target.thread_id` | `thread_id` query parameter |

The webhook adapter uses `reqwest` directly (not `twilight-http`) to keep integration tests simple — substituting a wiremock server URL is straightforward when the transport is plain HTTP.

Requests include `?wait=true` so Discord returns the created message object instead of an empty `204`. On success the receipt contains:

- `raw_id` — the Discord message snowflake
- `message_ref` — `DiscordWebhook { webhook_id, channel_id, message_id, thread_id }`

## Markdown Rendering

Discord Markdown is a near pass-through from CommonMark:

| Construct | Discord output |
|-----------|---------------|
| `**bold**` | `**bold**` |
| `_italic_` | `*italic*` |
| `~~strike~~` | `~~strike~~` |
| `` `code` `` | `` `code` `` |
| "```lang\ncode\n```" | "```lang\ncode\n```" |
| `[text](url)` | `[text](url)` or bare URL when text == URL |
| Lists | `- ` or `1. ` prefix |
| Headings | `# ` repeated prefix |

## Receipts

### Bot

```json
{
  "provider": "Discord",
  "message_ref": {
    "Discord": {
      "channel_id": "123456789012345678",
      "message_id": "987654321098765432"
    }
  },
  "raw_id": "987654321098765432"
}
```

### Webhook

```json
{
  "provider": "DiscordWebhook",
  "message_ref": {
    "DiscordWebhook": {
      "webhook_id": "1234567890",
      "channel_id": "123456789012345678",
      "message_id": "987654321098765432",
      "thread_id": null
    }
  },
  "raw_id": "987654321098765432"
}
```

## Troubleshooting

- **`InvalidMessage: invalid Discord channel ID`** — Channel IDs must be numeric snowflakes. Discord channel names (`#general`) or mentions (`<#123>`) are not accepted; use the raw numeric ID.
- **`Authentication` error from Discord** — The bot token is invalid or the bot has been removed from the server. Check the token and re-invite the bot with correct permissions.
- **Webhook URL rejected at construction** — Ensure the URL contains exactly `/webhooks/{numeric_id}/{token}` with no extra path segments. Query strings and fragments are stripped, but the path must be clean.
- **Attachments rejected** — Discord only accepts local file paths (`AttachmentSource::Path`) or in-memory bytes (`AttachmentSource::Bytes`). URL attachments and provider file IDs are rejected with `InvalidMessage`.
- **Empty `content` with attachments** — Discord allows messages with no text content if attachments are present. The provider sets `content` to an empty string when the body is empty and attachments exist.

## Related Documents

- [User Guide](../user-guide.md) — platform setup, CLI configuration, library usage.
- [messenger README](../../README.md) — high-level package overview.
- [messenger-cli README](../../cli/README.md) — CLI flags, route shapes, setup flow.
- [Research: Discord API Deep Dive](../../docs/research/platforms/discord.md) — full API research notes.
