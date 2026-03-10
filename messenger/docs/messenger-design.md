# Messenger Stage 1 and 2 Design

## Scope

This document defines the implementation plan for Stage 1 and Stage 2 from [messenger/README.md](../README.md).

- Stage 1 providers: Discord, Slack
- Stage 2 providers: Signal, WhatsApp, Telegram

The design is intentionally centered on an ergonomic `Message` type for outbound sends. The message should be easy to compose once and dispatch to one or many provider targets without leaking provider SDK types into the public API.

## Goals

- Provide a single outbound messaging library API for Discord, Slack, Signal, WhatsApp, and Telegram.
- Let callers author prose as Markdown and have each provider adapter render the best supported output.
- Support the common outbound use cases across Stage 1 and 2:
  - text messages
  - markdown-authored messages
  - replies / thread targeting
  - attachments / images
  - optional location payloads
- Keep provider-specific differences explicit where the platforms genuinely diverge.
- Make the core library reusable by the CLI and other crates.

## Non-Goals

- Inbound event handling, webhooks, slash commands, or bot frameworks
- Interactive provider-native payloads in the shared API:
  - Slack Block Kit
  - Discord embeds / components
  - Telegram keyboards
  - WhatsApp templates / interactive flows
  - Signal reactions / receipts / typing events
- A fake universal message identifier
- Full edit / delete / reaction APIs in Stage 1 or 2

## Design Principles

### 1. `Message` is reusable content, not a destination-bound request

The same `Message` should be sendable to multiple providers. Because of that, destination and reply identity do not live directly inside the base `Message`; they live in a `Dispatch` wrapper.

This is the main choice that keeps the core API ergonomic:

- build a `Message` once
- dispatch it to one or many `Target`s
- attach provider-specific reply metadata at dispatch time

### 2. Keep the shared model narrow

The research docs show that the platforms differ most in:

- destination identity
- reply / thread identity
- formatting syntax
- rich payload shape

The shared model should therefore stay text-first and attachment-first. Rich provider-native payloads belong behind provider-specific extension types.

### 3. Prefer best-effort rendering, but never lie

The README explicitly says Markdown should be stripped when a provider cannot render rich text. That means:

- formatting downgrade is allowed by default
- unsupported core content kinds should return errors
- provider-native extras should only be available through provider-specific overrides

### 4. Persist provider-native references

Replies are not portable:

- Slack replies use `thread_ts`
- Discord replies use `message_reference`
- Signal follow-up actions often need author plus timestamp
- WhatsApp replies use `context.message_id`
- Telegram replies use `message_id`, with `message_thread_id` handled separately

The library must return a provider-typed receipt that can be stored and used later for replies.

## Proposed Public API

### Core Types

```rust
use bytes::Bytes;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct Message {
    pub body: Option<MessageBody>,
    pub attachments: Vec<Attachment>,
    pub location: Option<Location>,
    pub metadata: BTreeMap<String, String>,
}

pub enum MessageBody {
    Plain(String),
    Markdown(String),
}

pub struct Attachment {
    pub kind: AttachmentKind,
    pub source: AttachmentSource,
    pub caption: Option<String>,
    pub alt_text: Option<String>,
}

pub enum AttachmentKind {
    Image,
    Audio,
    Video,
    Document,
    Binary,
}

pub enum AttachmentSource {
    Path(PathBuf),
    Url(String),
    Bytes {
        filename: String,
        mime_type: String,
        data: Bytes,
    },
    ProviderFileId(String),
}

pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub name: Option<String>,
    pub address: Option<String>,
}

pub struct Dispatch {
    pub target: Target,
    pub reply_to: Option<MessageRef>,
    pub options: DeliveryOptions,
    pub overrides: ProviderOverrides,
}

pub struct DeliveryOptions {
    pub silent: bool,
    pub disable_link_preview: bool,
    pub compatibility: CompatibilityMode,
}

pub enum CompatibilityMode {
    BestEffort,
    Strict,
}

pub enum Target {
    Discord(DiscordTarget),
    Slack(SlackTarget),
    Signal(SignalTarget),
    WhatsApp(WhatsAppTarget),
    Telegram(TelegramTarget),
}

pub enum MessageRef {
    Discord {
        channel_id: String,
        message_id: String,
    },
    Slack {
        channel_id: String,
        thread_ts: String,
    },
    Signal {
        thread: SignalThreadKey,
        author: SignalAddress,
        timestamp_ms: i64,
    },
    WhatsApp {
        message_id: String,
    },
    Telegram {
        chat_id: TelegramChatId,
        message_id: i64,
        thread_id: Option<i64>,
    },
}

pub enum ProviderOverrides {
    None,
    Discord(DiscordOverrides),
    Slack(SlackOverrides),
    Signal(SignalOverrides),
    WhatsApp(WhatsAppOverrides),
    Telegram(TelegramOverrides),
}

pub struct SendReceipt {
    pub provider: ProviderKind,
    pub message_ref: MessageRef,
    pub raw_id: String,
    pub metadata: BTreeMap<String, String>,
}
```

### Why this shape

- `Message` holds only portable content.
- `Dispatch` carries the provider target and provider-specific reply context.
- `MessageRef` is explicitly provider-typed because the research docs show there is no honest universal reply identifier.
- `ProviderOverrides` gives the design an escape hatch without polluting the common case.

## Ergonomic Builder API

The common case should feel like this:

```rust
let message = Message::markdown("**Deploy succeeded**")
    .attachment(Attachment::image("/tmp/chart.png").caption("Latency chart"))
    .metadata("service", "api");

messenger
    .send(
        Dispatch::to(Target::slack_channel("C012345"))
            .reply_to(MessageRef::Slack {
                channel_id: "C012345".into(),
                thread_ts: "1712345678.000100".into(),
            }),
        &message,
    )
    .await?;

messenger
    .send(Dispatch::to(Target::discord_channel("123456789012345678")), &message)
    .await?;
```

Recommended constructors and fluent methods:

- `Message::text(...)`
- `Message::markdown(...)`
- `Message::location(...)`
- `message.attachment(...)`
- `message.image(...)`
- `message.metadata(...)`
- `Dispatch::to(...)`
- `dispatch.reply_to(...)`
- `dispatch.silent()`
- `dispatch.disable_link_preview()`
- `dispatch.strict()`
- `dispatch.with_overrides(...)`
- `Target::discord_channel(...)`
- `Target::slack_channel(...)`
- `Target::signal_user(...)`
- `Target::signal_group(...)`
- `Target::whatsapp_recipient(...)`
- `Target::telegram_chat(...)`

## Provider Trait and Runtime

```rust
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn capabilities(&self) -> CapabilitySet;
    async fn send(&self, dispatch: &Dispatch, message: &Message)
        -> Result<SendReceipt, MessengerError>;
}

pub struct Messenger {
    providers: std::collections::HashMap<ProviderKind, Box<dyn Provider>>,
}

impl Messenger {
    pub async fn send(
        &self,
        dispatch: Dispatch,
        message: &Message,
    ) -> Result<SendReceipt, MessengerError> {
        // resolve provider from dispatch.target and delegate
    }
}
```

`Messenger` should only coordinate dispatch. Each provider adapter is responsible for:

- auth and client setup
- target validation
- markdown rendering for that provider
- request serialization
- rate limiting / retry behavior
- mapping provider responses into `SendReceipt`

## Formatting Model

### Public model

The public model should support only:

- `Plain`
- `Markdown`

That is enough to satisfy the README while staying honest about platform differences.

### Internal render pipeline

Internally, the library should parse Markdown once into a normalized intermediate representation, then let each provider adapter render from that representation.

Suggested pipeline:

1. `MessageBody::Markdown` is parsed into a small internal rich-text AST
2. providers render from the AST to native text format
3. unsupported formatting is dropped in `BestEffort` mode
4. unsupported formatting returns `UnsupportedFeature` in `Strict` mode

This avoids reusing a single Markdown dialect across all targets.

### Provider rendering strategy

#### Discord

- Treat Markdown as near-pass-through text
- Use plain message content first
- Do not include embeds/components in the shared API

#### Slack

- Render shared text into Slack `text`
- Keep Slack-specific blocks behind `SlackOverrides`
- Always retain plain fallback text for accessibility and notifications if rich Slack formatting is introduced later

#### Signal

- Stage 2 initial implementation should render shared Markdown to plain text
- Reserve UTF-16 body ranges and richer styling for `SignalOverrides` or a later stage

#### WhatsApp

- Stage 2 initial implementation should render shared Markdown to plain text
- Template / interactive messages are provider-specific and out of the shared API

#### Telegram

- Render shared Markdown to Telegram HTML, not MarkdownV2
- The platform research makes it clear that MarkdownV2 escaping is too brittle for the shared path

## Target and Reply Modeling

The target types should be typed enough to prevent obviously invalid calls, but not so elaborate that users fight the API.

```rust
pub struct DiscordTarget {
    pub channel_id: String,
}

pub struct SlackTarget {
    pub channel_id: String,
}

pub enum SignalTarget {
    User(SignalAddress),
    Group { group_id_base64: String },
    NoteToSelf,
}

pub struct WhatsAppTarget {
    pub recipient: String,
}

pub struct TelegramTarget {
    pub chat_id: TelegramChatId,
    pub thread_id: Option<i64>,
}
```

Important rule: `reply_to` must always be expressed using the same provider as `target`. Cross-provider replies should be rejected at validation time.

## Provider-Specific Implementation Choices

The platform research points to different implementation strategies per provider. The library should not force one SDK strategy across all providers.

| Provider | Stage | Transport choice | Rationale |
| --- | --- | --- | --- |
| Discord | 1 | `twilight-http` + `twilight-model` | The Discord research strongly favors mature crates because of rate limits and API complexity. Using the modular HTTP client is the best fit for outbound-only sending. This is an inference from the research, not a direct quote. |
| Slack | 1 | `slack-morphism` | Recommended by the platform research and well aligned with app-grade Slack messaging. |
| Signal | 2 | `signal-cli-jsonrpc-client` | The research recommends a `signal-cli`-based approach as the most practical integration path. |
| WhatsApp | 2 | `reqwest` + `serde` | The Cloud API is straightforward HTTP/JSON, and the research calls out SDK drift risk. |
| Telegram | 2 | `frankenstein` | The research recommends it as a thin, current Bot API client. |

## Auth and Configuration Model

The core library should accept typed runtime configs with resolved secret values.

The CLI should own env var lookup and `~/.messenger.json` storage.

Example runtime configs:

```rust
pub struct DiscordConfig {
    pub bot_token: secrecy::SecretString,
}

pub struct SlackConfig {
    pub bot_token: secrecy::SecretString,
}

pub struct SignalConfig {
    pub rpc_url: String,
    pub account: String,
}

pub struct WhatsAppConfig {
    pub access_token: secrecy::SecretString,
    pub phone_number_id: String,
    pub api_version: String,
}

pub struct TelegramConfig {
    pub bot_token: secrecy::SecretString,
}
```

Recommended CLI env var names:

| Provider | Env vars |
| --- | --- |
| Discord | `DISCORD_BOT_TOKEN` |
| Slack | `SLACK_BOT_TOKEN` |
| Signal | `SIGNAL_RPC_URL`, `SIGNAL_ACCOUNT` |
| WhatsApp | `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID` |
| Telegram | `TELEGRAM_BOT_TOKEN` |

Example CLI route config shape:

```json
{
  "default_route": "slack.ops",
  "routes": {
    "slack.ops": {
      "provider": "slack",
      "channel_id": "C012345",
      "bot_token_env": "SLACK_BOT_TOKEN"
    },
    "discord.alerts": {
      "provider": "discord",
      "channel_id": "123456789012345678",
      "bot_token_env": "DISCORD_BOT_TOKEN"
    },
    "signal.ops": {
      "provider": "signal",
      "recipient": "+15551234567",
      "rpc_url_env": "SIGNAL_RPC_URL",
      "account_env": "SIGNAL_ACCOUNT"
    }
  }
}
```

## Capability and Validation Rules

Each provider should publish a `CapabilitySet` used by validation and the CLI.

Minimum capability flags:

- `supports_markdown_rendering`
- `supports_reply`
- `supports_attachments`
- `supports_location`
- `supports_silent_delivery`
- `supports_link_preview_control`

Validation rules:

- empty `Message` is invalid
- `reply_to` provider must match `target`
- attachment source must be resolvable before send
- in `Strict` mode, the provider must support every requested content element
- in `BestEffort` mode, formatting may downgrade, but core content kinds may not silently disappear

The CLI should persist each `SendReceipt` so later reply commands can load the typed `MessageRef` back from disk rather than requiring a raw provider-specific ID string.

## Error Model

```rust
pub enum MessengerError {
    InvalidMessage(String),
    UnsupportedFeature {
        provider: ProviderKind,
        feature: &'static str,
    },
    MissingConfiguration {
        provider: ProviderKind,
        field: &'static str,
    },
    Authentication {
        provider: ProviderKind,
        message: String,
    },
    RateLimited {
        provider: ProviderKind,
        retry_after_ms: Option<u64>,
    },
    Transport {
        provider: ProviderKind,
        message: String,
    },
    Provider {
        provider: ProviderKind,
        code: Option<String>,
        message: String,
    },
}
```

This keeps the error surface consistent while still preserving provider context.

## Rate Limiting and Retry Strategy

The provider adapters should own rate limits because the semantics differ per platform.

### Discord

- rely on the Discord client library’s rate-limit handling
- do not hand-roll naive request loops

### Slack

- respect `Retry-After`
- queue sends per token / route as needed

### Signal

- serialize sends through the JSON-RPC client until real throughput requirements appear

### WhatsApp

- back off on 429 / 5xx
- keep the 24-hour messaging-window logic inside the adapter

### Telegram

- use provider-level throttling
- respect `retry_after`

## Module Layout

Suggested `messenger/lib` layout:

```txt
src/
  lib.rs
  message.rs
  attachment.rs
  target.rs
  dispatch.rs
  receipt.rs
  error.rs
  capabilities.rs
  markdown/
    mod.rs
    ast.rs
    plain_text.rs
    telegram_html.rs
    slack_text.rs
  provider/
    mod.rs
    discord.rs
    slack.rs
    signal.rs
    whatsapp.rs
    telegram.rs
```

Feature flags should be provider-based so Stage 1 can land without pulling all Stage 2 dependencies:

```toml
[features]
default = ["discord", "slack"]
discord = []
slack = []
signal = []
whatsapp = []
telegram = []
```

## Stage 1 Deliverables

### Core

- `Message`, `Attachment`, `Location`, `Dispatch`, `Target`, `MessageRef`, `SendReceipt`
- builder ergonomics for plain text, markdown, attachments, and replies
- markdown parsing and downgrade behavior
- provider trait, validation, and shared error model

### Discord

- send plain and Markdown-authored text
- send file / image attachments
- send replies using `message_reference`
- return `MessageRef::Discord`

### Slack

- send plain and Markdown-authored text
- send replies using `thread_ts`
- return `MessageRef::Slack`
- keep Block Kit out of the shared API

## Stage 2 Deliverables

### Signal

- send text and attachments through the `signal-cli` JSON-RPC client
- support `SignalTarget`
- support replies using author plus timestamp
- return `MessageRef::Signal`

### WhatsApp

- send text, attachments, and location through Cloud API
- support replies via `context.message_id`
- return `MessageRef::WhatsApp`
- enforce provider-side validation for the 24-hour window

### Telegram

- send text, attachments, and location through Bot API
- render shared Markdown to HTML
- support replies and optional `thread_id`
- return `MessageRef::Telegram`

## Testing Strategy

### Unit tests

- builder validation
- Markdown rendering to provider-specific output
- reply target validation
- attachment source validation

### Integration-style tests

- HTTP provider request serialization with `wiremock`
- env var loading with `serial_test`
- attachment filesystem handling with `tempfile`
- Signal JSON-RPC request / response fixtures

### Acceptance tests

For each provider, add a small acceptance matrix:

- plain text send
- markdown send
- reply send
- attachment send
- location send where supported

## Why this design will scale to Stage 3

Stage 3 adds SMS, email, and Home Assistant. Those platforms reinforce the same design choice:

- keep `Message` small
- keep target and reply modeling provider-scoped
- let adapters decide how to degrade formatting
- avoid leaking provider-native request types into the public API

Email will likely introduce `subject`, and SMS will reinforce the need for aggressive downgrade. Both can be added without breaking the Stage 1 and 2 structure.

## Recommended First Implementation Order

1. Build the shared core types and validation layer.
2. Implement Markdown parsing and provider render helpers.
3. Implement Discord and Slack adapters.
4. Add `SendReceipt` persistence hooks in the CLI so replies have the metadata they need later.
5. Implement Telegram, then WhatsApp, then Signal.

This order keeps Stage 1 small, lands the main public API early, and avoids letting Signal or WhatsApp complexity distort the core model.
