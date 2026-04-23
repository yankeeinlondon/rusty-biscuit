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

`ProviderKind` enum: `Discord`, `DiscordWebhook`, `Slack`, `SlackWebhook`, `Signal`, `WhatsApp`, `Telegram`, `Desktop`.

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
**Markdown renderer**: Shares the same `markdown/discord.rs` renderer as the bot adapter.

**Reply enforcement**: `validate::normalize_dispatch` returns `MessengerError::UnsupportedFeature { provider: DiscordWebhook, feature: "replies" }` before any network call when `reply_to` is set — this hard-error fires in both strict and best-effort modes.

**URL parsing**: `try_new` parses `/webhooks/{id}/{token}` segments out of the full URL, validates the webhook id, and returns `MessengerError::InvalidMessage` for malformed input. Optional `thread_id` values are validated as numeric Discord snowflakes before a request is sent.

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

## Slack-Webhook

**Feature**: `slack` (default, same flag as the bot adapter)
**HTTP crate**: `reqwest`
**Module**: `provider/slack_webhook.rs`

```rust
pub struct SlackWebhookConfig {
    pub webhook_url: SecretString,
}
```

**Capabilities**: markdown_rendering (mrkdwn), reply, location (text fallback), link_preview_control — **no attachments**, no silent delivery. File uploads require the Web API; incoming webhooks do not accept multipart.
**API**: `POST` to the incoming webhook URL (`https://hooks.slack.com/services/{T}/{B}/{token}`) with a JSON body. The same mrkdwn renderer is shared with the bot adapter; `disable_link_preview` sets `unfurl_links` and `unfurl_media` to `false`.
**Target**: `Target::slack_webhook()` — the webhook URL binds the channel, so the target carries no channel id.
**MessageRef**: `MessageRef::SlackWebhook { thread_ts: Option<String> }`. Webhook responses never return a `thread_ts`, so a webhook receipt always carries `thread_ts: None`; reply threading requires a `thread_ts` sourced from a Slack bot receipt (or any external channel that surfaces Slack thread timestamps).

**Receipt semantics**: Successful webhook sends produce `SendReceipt { provider: SlackWebhook, raw_id: "", message_ref: MessageRef::SlackWebhook { thread_ts: None }, metadata: {"delivery_confirmed": "true"} }`. Because the Slack response does not carry a message id, webhook receipts cannot be used on their own as the `reply_to` target of a later send.

**URL parsing**: `try_new` enforces `https` scheme, case-insensitive host `hooks.slack.com` (no subdomains), path prefix `/services/` followed by exactly three non-empty decoded segments, and rejects trailing slash, query strings, fragments, and whitespace-only segments. Malformed URLs produce `MessengerError::InvalidMessage`; absent configuration surfaces as `MessengerError::MissingConfiguration` from the CLI resolution layer.

**Error mapping**: Slack webhook `ok: false` responses map `invalid_token`/`action_prohibited` to `MessengerError::Authentication`, `invalid_payload`/`channel_is_archived` to `MessengerError::InvalidMessage`, and any other response code to `MessengerError::Provider`. HTTP `429` with `Retry-After` maps to `MessengerError::RateLimited`; HTTP `5xx` maps to `MessengerError::Transport`.

**Plan-time validation**: `Messenger::plan_send()` with `Target::SlackWebhook` and `reply_to = MessageRef::Slack { .. }` returns `MessengerError::InvalidMessage` (provider mismatch) before transport execution.

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

## Desktop

**Feature**: `desktop`
**Backend crates**: `notify-rust` (Linux), `winrt-notification` (Windows), `objc2-user-notifications` / `osascript` (macOS)
**Module**: `provider/desktop/mod.rs`

```rust
pub struct DesktopConfig {
    pub app_name: String,
    pub default_title: Option<String>,
    pub icon: Option<NotificationIcon>,
    pub category: Option<String>,
    pub urgency: Option<NotificationUrgency>,
    pub timeout_ms: Option<u64>,
    pub windows: Option<WindowsDesktopConfig>,
    pub macos: Option<MacOsDesktopConfig>,
    pub linux: Option<LinuxDesktopConfig>,
}
```

**Capabilities**: `supported_attachment_kinds: { Image }` only, `silent_delivery: true` — no markdown, no replies, no location, no link preview.
**Target**: `Target::desktop()` — no credentials, no destination identifier.
**MessageRef**: `MessageRef::Desktop { platform: DesktopPlatform, notification_id: String }`.

Title-only messages are valid. `Message::title(...)` is accepted without a body when the resolved provider is `Desktop`.

Platform behavior:
- **Linux**: D-Bus (freedesktop.org Notifications). No setup required.
- **macOS**: `strategy: auto` (default) uses AppleScript — no authorization prompt. `strategy: native_user_notifications` uses `UserNotifications.framework` and requires a bundled app identity.
- **Windows**: requires `messenger setup desktop` to create the Start Menu shortcut and register the AUMID. `send` never writes outside `~/.messenger/`. If the shortcut is missing, `send` returns `MessengerError::MissingConfiguration`.

## CapabilitySet

```rust
pub struct CapabilitySet {
    pub markdown_rendering: bool,
    pub reply: bool,
    pub supported_attachment_kinds: BTreeSet<AttachmentKind>,
    pub location: bool,
    pub silent_delivery: bool,
    pub link_preview_control: bool,
}
```

`supported_attachment_kinds` declares exactly which kinds (`Image`, `Audio`, `Video`, `Document`, `Binary`) a provider accepts. Location can be `true` without native API support (Discord, Slack, Signal append formatted text).

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
