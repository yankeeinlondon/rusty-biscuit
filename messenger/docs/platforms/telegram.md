# Telegram Platform Guide

The `telegram` provider delivers messages via the **Telegram Bot API**. It is a single adapter (`TelegramProvider`) that sends text and location messages to chats, with support for reply threading, silent delivery, and link preview control.

| Host | Backend | Delivery path |
|------|---------|---------------|
| Telegram Bot API | `reqwest` | `POST https://api.telegram.org/bot{token}/sendMessage` |

Telegram uses **chat-based** targeting: every send specifies a `chat_id` (numeric ID or `@username`). Messages can also be routed to a specific forum topic via `thread_id`.

## Capability Summary

- `supports_markdown_rendering`: `true` — Telegram HTML subset
- `supports_reply`: `true`
- `supported_attachment_kinds`: `{}` — attachments drop in best-effort and error in strict
- `supports_location`: `true` — native `sendLocation` API
- `supports_silent_delivery`: `true` — `disable_notification`
- `supports_link_preview_control`: `true` — `disable_web_page_preview`

## Enabling

Library:

```toml
[dependencies]
messenger = { version = "0.1", features = ["telegram"] }
```

CLI: `messenger-cli` enables `telegram` by default.

## Quick Test

```bash
export TELEGRAM_BOT_TOKEN="123456789:ABCdefGhIJKlmNoPQRsTUVwxyz"
messenger send --provider telegram --chat-id "12345678" "Hello from messenger"
```

## Authentication

Telegram Bot API uses **token-based** authentication. No OAuth flow is required.

1. Message [@BotFather](https://t.me/BotFather) on Telegram
2. Send `/newbot` and follow the prompts
3. Receive a bot token in the format `123456789:ABCdefGhIJKlmNoPQRsTUVwxyz`
4. The token is the only credential — anyone with it has full control of the bot

Library usage:

```rust
use messenger::prelude::*;
use secrecy::SecretString;

let provider = TelegramProvider::new(TelegramConfig {
    bot_token: SecretString::from(std::env::var("TELEGRAM_BOT_TOKEN").unwrap()),
    api_base_url: None, // optional: override for testing with wiremock
});
```

## Field Mapping

| Portable | Telegram Bot API |
|----------|-----------------|
| `body` (Markdown) | `text` with `parse_mode: "HTML"` |
| `body` (Plain) | `text` with no `parse_mode` |
| `location` | `sendLocation` with `latitude`, `longitude` |
| `reply_to` (`MessageRef::Telegram { message_id }`) | `reply_parameters.message_id` |
| `silent` | `disable_notification: true` |
| `disable_link_preview` | `disable_web_page_preview: true` |
| `target.thread_id` | `message_thread_id` |

When a location is present, the provider uses `sendLocation` instead of `sendMessage`. Text and location are mutually exclusive in a single send.

## Chat Identifiers

The `TelegramChatId` enum accepts two forms:

| Form | Example | Notes |
|------|---------|-------|
| Numeric ID | `12345678` | Positive for users, negative for groups |
| Username | `"@channelname"` | Only for public channels/groups; the `@` is optional |

Group IDs for supergroups are **negative** numbers starting with `-100`. Private chat IDs are positive.

## Markdown Rendering

Telegram uses a **subset of HTML** for rich text formatting:

| Construct | Telegram HTML |
|-----------|--------------|
| `**bold**` | `<b>bold</b>` |
| `_italic_` | `<i>italic</i>` |
| `~~strike~~` | `<s>strike</s>` |
| `` `code` `` | `<code>code</code>` |
| "```lang\ncode\n```" | "<pre><code class=\"language-lang\">code</code></pre>" |
| `[text](url)` | `<a href="url">text</a>` |
| Lists | `• ` or `1. ` prefix (no native list tags) |
| Headings | rendered as `<b>heading</b>` |

HTML entities (`<`, `>`, `&`, `"`) are escaped automatically.

## Receipts

```json
{
  "provider": "Telegram",
  "message_ref": {
    "Telegram": {
      "chat_id": {
        "Id": 12345678
      },
      "message_id": 42,
      "thread_id": null
    }
  },
  "raw_id": "42"
}
```

The `message_id` is an integer used for replies, edits, and deletion. Preserve it along with the `chat_id` for follow-up operations.

## Error Handling

| Response | Meaning | Action |
|----------|---------|--------|
| `ok: false` with `retry_after` | Rate limited | Wait `retry_after` seconds |
| `Unauthorized` or `bot token` in description | Invalid token | Regenerate token via @BotFather |
| `403 Forbidden` | User blocked bot or hasn't `/start`-ed | Remove user from recipient list |
| `400 Bad Request` | Invalid chat ID, malformed message | Verify chat ID format and message content |

## Rate Limits

Telegram does not publish exact rate limits. Community-observed thresholds:

| Scope | Approximate Limit |
|-------|-------------------|
| Private chat | ~1 msg/second |
| Group chat | ~20 msgs/minute |
| Bulk broadcast | ~30 msgs/second (blocks ALL calls if exceeded) |

When rate limited, the `retry_after` period blocks **every** API call to your bot, not just the offending chat. Implement a global rate limiter for broadcast scenarios.

## Troubleshooting

- **`Authentication` error** — The bot token is invalid or revoked. Check the token with @BotFather and regenerate if necessary.
- **`RateLimited` with long `retry_after`** — You're sending too fast. Back off and respect the `retry_after` header. For broadcasts, stay under 25 msgs/second with jitter.
- **Message silently fails to deliver** — The user may have blocked the bot or never sent `/start`. Handle 403 responses by removing dead recipients from your list.
- **Messages land in "General" topic instead of intended topic** — Supergroups with forum topics require `message_thread_id`. Pass the topic ID via `target.thread_id`.
- **`InvalidMessage: expected Telegram target`** — The `Target` enum variant must be `Telegram`, not another provider type.
- **HTML formatting not working** — Ensure the body is `MessageBody::Markdown`. Plain text does not set `parse_mode`. If using MarkdownV2 manually, remember aggressive escaping rules; the provider uses HTML mode for safety.

## Related Documents

- [User Guide](../user-guide.md) — platform setup, CLI configuration, library usage.
- [messenger README](../../README.md) — high-level package overview.
- [messenger-cli README](../../cli/README.md) — CLI flags, route shapes, setup flow.
- [Research: Telegram API Deep Dive](../../docs/research/platforms/telegram.md) — full API research notes.
