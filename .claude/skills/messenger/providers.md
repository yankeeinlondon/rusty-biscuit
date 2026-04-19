# Providers Reference

## Provider Trait

All providers implement the async `Provider` trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn capabilities(&self) -> CapabilitySet;
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError>;

    // Default impl: validate, normalize, prepare, send_prepared
    async fn send(&self, dispatch: &Dispatch, message: &Message)
        -> Result<SendReceipt, MessengerError>;
}
```

`ProviderKind` enum: `Discord`, `DiscordWebhook`, `Slack`, `Signal`, `WhatsApp`, `Telegram`.

## Discord

**Feature**: `discord` (default)
**HTTP crate**: `twilight-http` + `twilight-model`
**Module**: `provider/discord.rs`

```rust
pub struct DiscordConfig {
    pub bot_token: SecretString,
}
```

**Capabilities**: markdown_rendering, reply, attachments, location (text fallback)
**Attachment support**: Local paths or in-memory bytes only (no URLs or provider file IDs)
**Target**: `Target::discord_channel("channel_id")`
**MessageRef**: `MessageRef::Discord { channel_id, message_id }`

## Discord-Webhook

**Feature**: `discord` (default, same flag as the bot adapter)
**HTTP crate**: `reqwest` (direct `multipart`/`json` POST, not `twilight-http`)
**Module**: `provider/discord_webhook.rs`

```rust
pub struct DiscordWebhookConfig {
    pub webhook_url: SecretString,
}
```

**Capabilities**: markdown_rendering, attachments, location (text fallback) — **no reply**, no silent delivery, no link-preview control.
**Attachment support**: Local paths or in-memory bytes only (no URLs or provider file IDs).
**Target**: `Target::discord_webhook()` (no thread) or `Target::discord_webhook_thread("thread_id")`.
**MessageRef**: `MessageRef::DiscordWebhook { webhook_id, channel_id, message_id, thread_id }`.

**Reply enforcement**: `validate::normalize_dispatch` returns `MessengerError::UnsupportedFeature { provider: DiscordWebhook, feature: "replies" }` before any network call when `reply_to` is set — this hard-error fires in both strict and best-effort modes.

**URL parsing**: The constructor parses `/webhooks/{id}/{token}` segments out of the full URL. `try_new` returns `MessengerError::InvalidMessage` for malformed input; `new` panics with the same message (mirrors `DiscordProvider::new`).

## Slack

**Feature**: `slack` (default)
**HTTP crate**: `reqwest`
**Module**: `provider/slack.rs`

```rust
pub struct SlackConfig {
    pub bot_token: SecretString,
    pub api_base_url: Option<String>,
}
```

**Capabilities**: markdown_rendering (mrkdwn), reply, location (text fallback), link_preview_control
**API**: `chat.postMessage` with `thread_ts` for replies, `unfurl_links`/`unfurl_media` for link previews
**Target**: `Target::slack_channel("C01234567")`
**MessageRef**: `MessageRef::Slack { channel_id, thread_ts }`

## Signal

**Feature**: `signal`
**HTTP crate**: `reqwest`
**Module**: `provider/signal.rs`

```rust
pub struct SignalConfig {
    pub rpc_url: String,
    pub account: String,
}
```

**Capabilities**: reply, location (text fallback)
**Note**: Plain text only (Markdown falls back with warning)
**Backend**: signal-cli JSON-RPC daemon
**Target**: `Target::signal_direct("+15551234567")` or `Target::signal_group("base64_group_id")`
**MessageRef**: `MessageRef::Signal { thread, author, timestamp_ms }`

## WhatsApp

**Feature**: `whatsapp`
**HTTP crate**: `reqwest`
**Module**: `provider/whatsapp.rs`

```rust
pub struct WhatsAppConfig {
    pub access_token: SecretString,
    pub phone_number_id: String,
    pub api_version: Option<String>,
    pub api_base_url: Option<String>,
}
```

**Capabilities**: reply, location (native)
**Note**: Plain text only (Markdown falls back with warning). Text+location sends location only.
**API**: WhatsApp Cloud API (`cloud.facebook.com`)
**Target**: `Target::whatsapp("recipient_phone")`
**MessageRef**: `MessageRef::WhatsApp { message_id }`

## Telegram

**Feature**: `telegram`
**HTTP crate**: `reqwest`
**Module**: `provider/telegram.rs`

```rust
pub struct TelegramConfig {
    pub bot_token: SecretString,
    pub api_base_url: Option<String>,
}
```

**Capabilities**: markdown_rendering (HTML), reply, location (native), silent_delivery, link_preview_control
**Note**: Text+location sends location only. Supports thread_id for forum topics.
**API**: Telegram Bot API (`api.telegram.org/bot<token>/`)
**Target**: `Target::telegram_chat(TelegramChatId::Id(-1001234567890))` or `TelegramChatId::Username("@ops")`
**MessageRef**: `MessageRef::Telegram { chat_id, message_id, thread_id }`

## CapabilitySet

```rust
pub struct CapabilitySet {
    pub markdown_rendering: bool,
    pub reply: bool,
    pub attachments: bool,
    pub location: bool,
    pub silent_delivery: bool,
    pub link_preview_control: bool,
}
```

Location can be `true` without native API support (Discord, Slack, Signal append formatted text).

## Capability Normalization

**Best-effort mode** (default): Unsupported features are silently dropped with `CompatibilityWarning`.
**Strict mode**: Unsupported features produce `MessengerError::UnsupportedFeature`.

Use `plan_send()` to inspect warnings before committing:

```rust
let plan = messenger.plan_send(dispatch, &message)?;
for warning in &plan.warnings {
    eprintln!("{warning}");
}
let receipt = messenger.send_planned(plan).await?;
```

## Error Types

```rust
pub enum MessengerError {
    InvalidMessage(String),
    UnsupportedFeature { provider: ProviderKind, feature: String },
    MissingConfiguration { provider: ProviderKind, field: String },
    Authentication { provider: ProviderKind, message: String },
    RateLimited { provider: ProviderKind, retry_after_ms: Option<u64> },
    Transport { provider: ProviderKind, message: String },
    Provider { provider: ProviderKind, code: Option<String>, message: String },
}
```

## Adding a Provider

1. Add a feature flag in `lib/Cargo.toml`
2. Create `provider/<name>.rs` implementing `Provider`
3. Declare `CapabilitySet` accurately
4. Add a Markdown renderer in `markdown/` (or use `plain_text.rs`)
5. Add `Target` variant, `MessageRef` variant, config struct
6. Wire into `prelude.rs` behind the feature gate
7. Add wiremock integration tests in `tests/`
8. Enable the feature in `cli/Cargo.toml` and add CLI route support
