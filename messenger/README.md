# Messenger

A unified outbound messaging library and CLI for sending messages across chat platforms.

## Library (`messenger`)

Build a `Message` once, dispatch it to any provider.

```rust
let message = Message::markdown("**Deploy succeeded**")
    .attachment(Attachment::image("/tmp/chart.png").caption("Latency chart"))
    .metadata("service", "api");

messenger.send(Dispatch::to(Target::slack_channel("C012345")), &message).await?;
messenger.send(Dispatch::to(Target::discord_channel("123456")), &message).await?;
```

### Supported Providers

| Provider | Feature Flag | Transport | Capabilities |
|----------|-------------|-----------|-------------|
| Discord | `discord` (default) | `twilight-http` | Markdown, replies, attachments |
| Slack | `slack` (default) | `reqwest` → Web API | Markdown (mrkdwn), replies, link preview control |
| Telegram | `telegram` | `reqwest` → Bot API | Markdown (HTML), replies, attachments, location, silent, link preview |
| WhatsApp | `whatsapp` | `reqwest` → Cloud API | Replies, attachments, location |
| Signal | `signal` | `reqwest` → JSON-RPC (`signal-cli`) | Replies, attachments |

### Key Design Decisions

- **`Message` is reusable content** — destination and reply context live in `Dispatch`, not in the message itself
- **Markdown rendering pipeline** — parsed once into an internal AST, then rendered per-provider (Discord Markdown, Slack mrkdwn, Telegram HTML, plain text)
- **Best-effort by default** — formatting downgrades silently; use `Dispatch::strict()` to error on unsupported features
- **Provider-typed receipts** — `SendReceipt` contains a `MessageRef` for replies (Slack `thread_ts`, Discord `message_reference`, etc.)
- **Feature-gated providers** — only `discord` + `slack` compile by default; Stage 2 providers are opt-in

### Core Types

| Type | Purpose |
|------|---------|
| `Message` | Portable content (body, attachments, location, metadata) |
| `Dispatch` | Target + reply context + delivery options |
| `Target` | Provider-specific destination (channel, recipient, chat) |
| `MessageRef` | Provider-typed reference for replies |
| `SendReceipt` | Proof of delivery with provider-native identifiers |
| `Messenger` | Coordinator that routes dispatches to registered providers |
| `Provider` | Trait implemented by each adapter |
| `CapabilitySet` | What a provider supports (markdown, replies, attachments, etc.) |

### Error Handling

All errors use `MessengerError` with provider context:

- `InvalidMessage` — empty message or mismatched reply/target provider
- `UnsupportedFeature` — strict mode: provider lacks a requested capability
- `MissingConfiguration` — no provider registered for target
- `Authentication` / `RateLimited` / `Transport` / `Provider` — API-level errors

## CLI (`messenger-cli`)

Binary name: `messenger`

```bash
# Send to default route
messenger "hello world"

# Send to a specific provider and channel
messenger --provider slack --channel C012345 "Deploy complete"

# Send to a named route
messenger --route discord.alerts "Server down"

# With options
messenger --silent --image /tmp/chart.png "See attached"
messenger --plain "No **markdown** rendering"
messenger --strict "Error if provider can't handle this"
```

### Configuration

Config file: `~/.messenger.json`

```json
{
  "default_route": "slack.ops",
  "routes": {
    "slack.ops": {
      "provider": "slack",
      "channel_id": "C012345",
      "token_env": "SLACK_BOT_TOKEN"
    },
    "discord.alerts": {
      "provider": "discord",
      "channel_id": "123456789012345678",
      "token_env": "DISCORD_BOT_TOKEN"
    }
  }
}
```

### Environment Variables

| Provider | Variables |
|----------|----------|
| Discord | `DISCORD_BOT_TOKEN` |
| Slack | `SLACK_BOT_TOKEN` |
| Signal | `SIGNAL_RPC_URL`, `SIGNAL_ACCOUNT` |
| WhatsApp | `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID` |
| Telegram | `TELEGRAM_BOT_TOKEN` |

## Staged Delivery

### Stage 1 (complete)

- Discord
- Slack

### Stage 2 (complete)

- Signal
- WhatsApp
- Telegram

### Stage 3 (planned)

- SMS
- Email
- Home Assistant
