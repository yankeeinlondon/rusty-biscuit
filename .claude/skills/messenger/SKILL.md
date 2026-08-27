---
name: messenger
description: Unified outbound messaging library and CLI for Rust. Use when building or modifying messaging features, adding providers, working with Message/Dispatch/SendReceipt types, Markdown rendering, provider capabilities, CLI routes, or the messenger package in the rusty-biscuit monorepo.
---

## Purpose

`messenger` separates portable content from provider-specific delivery:

- **Message** holds reusable body, attachments, location, and metadata
- **Dispatch** holds destination, reply context, and delivery options
- **SendReceipt** captures provider-native identifiers for future replies

The library validates, normalizes against provider capabilities, renders Markdown per-provider, and returns compatibility warnings for best-effort drops.

## Quick Start

```rust
use messenger::prelude::*;
use secrecy::SecretString;

let mut messenger = Messenger::new();
messenger.register(Box::new(SlackProvider::new(SlackConfig {
    bot_token: SecretString::from(std::env::var("SLACK_BOT_TOKEN").unwrap()),
    api_base_url: None,
})));

let message = Message::markdown("**Deploy succeeded**")
    .metadata("service", "api");
let dispatch = Dispatch::to(Target::slack_channel("C01234567"));

let receipt = messenger.send(dispatch, &message).await?;
```

### Notification-Aware Messages

Providers like Discord render Markdown in chat but generate desktop notifications from raw text. Three options:

1. **Single Markdown string** (default): `Message::markdown("**bold**")` — rendered in chat, raw in notifications
2. **Plain summary + rich body**: `Message::summarized("plain", "**rich**")` — clean notification, rich embed (Discord)
3. **Strip formatting**: `Message::markdown_stripped("**bold**")` — plain text everywhere

CLI equivalents:

```bash
messenger send "**rich**" --route discord.channel                        # default
messenger send "**rich**" --summary "plain" --route discord.channel      # split
messenger send "**rich**" --strip-markdown --route discord.channel       # stripped
```

## Provider Support

| Provider | Feature flag | Rich text | Replies | Attachments | Location | Silent | Link preview |
|----------|-------------|-----------|---------|-------------|----------|--------|-------------|
| Discord | `discord` (default) | Markdown | Yes | Yes | Text fallback | No | No |
| Discord-Webhook | `discord` (default) | Markdown | No | Yes | Text fallback | No | No |
| Slack | `slack` (default) | mrkdwn | Yes | No | Text fallback | No | Yes |
| Slack-Webhook | `slack` (default) | mrkdwn | Yes | No | Text fallback | No | Yes |
| Signal | `signal` | Plain text | Yes | No | Text fallback | No | No |
| WhatsApp | `whatsapp` | Plain text | Yes | No | Native | No | No |
| Telegram | `telegram` | HTML | Yes | No | Native | Yes | Yes |
| Desktop | `desktop` | Plain text | No | Images only | No | Yes | No |
| APNs | `apns` | Plain text | No | No | No | Yes | No |
| FCM | `fcm` | Plain text | No | No | No | Yes | No |

### Desktop Provider

### Mobile Push Providers

| Provider | Feature flag | Rich text | Replies | Attachments | Location | Silent | Link preview |
|----------|-------------|-----------|---------|-------------|----------|--------|-------------|
| APNs | `apns` | Plain text | No | No | No | Yes | No |
| FCM | `fcm` | Plain text | No | No | No | Yes | No |

**APNs** (Apple Push Notification service) sends iOS push notifications via Apple's HTTP/2 API. Authentication uses JWT (ES256) signed with a `.p8` private key. Requires: team ID, key ID, private key, bundle ID. Supports sandbox and production environments.

**FCM** (Firebase Cloud Messaging) sends Android push notifications via the FCM HTTP v1 API. Authentication uses OAuth2 access tokens with `https://www.googleapis.com/auth/firebase.messaging` scope. Requires: project ID, access token.

Both mobile providers support title-only messages (like desktop) and silent delivery. They do not support rich text rendering, replies, attachments, location, or link preview control.

### Desktop Provider

The desktop provider delivers notifications to the host OS notification center (Linux D-Bus, macOS native/AppleScript, Windows WinRT). It opportunistically uses third-party helper CLIs when available to unlock richer features:

| Helper | Platform | Score | Actions | Reply | Replace | Notes |
|--------|----------|-------|---------|-------|---------|-------|
| `dunstify` | Linux | 90/70 | Yes | No | Yes | Preferred when dunst daemon is active |
| `notify-send` | Linux | 60 | Yes* | No | No | *Actions require libnotify >= 0.7.8 |
| `terminal-notifier` | macOS | 80 | No | No | Yes | Notice-only; drops actions/reply |
| `alerter` | macOS | 90/30 | Yes | Yes | No | Preferred for interactive dispatches |
| `snoretoast` | Windows | 90 | Yes | Yes | Yes | Default choice; requires AppID registration |
| `burnttoast` | Windows | 40 | Yes | Yes | No | PowerShell-based fallback |

**Helper election** runs at send time: helpers are scored per-dispatch, filtered by `score > 0`, and tried in preference order (env var `MESSENGER_DESKTOP_PREFER_HELPERS` > per-OS config `prefer_helpers` > default OS order). A failed helper falls through to the next candidate; if all helpers fail, the native backend is used.

Beyond basic send, the desktop provider exposes:

- **`DesktopNotificationProvider::replace(receipt, dispatch, message)`** — update an existing notification by its `SendReceipt` (supported on Linux D-Bus and macOS native; returns `UnsupportedFeature` on AppleScript and Windows)
- **`DesktopNotificationProvider::dismiss(receipt)`** — remove a delivered notification (supported on macOS native; returns `UnsupportedFeature` on other backends)

Desktop-specific overrides on `Dispatch`:

- `subtitle`, `app_name`, `category`, `urgency`, `timeout_ms`, `icon` — per-send overrides
- `replace_id` — replace a notification at send time (Linux/macOS native)
- `group_id` — grouping hint (best-effort custom hint on Linux)
- `actions` — interactive action buttons (best-effort; callback handling requires a packaged app)
- `progress` — progress indicator with `current` and `total` values (best-effort hint on Linux)
- `badge_count` — app icon badge count (best-effort hint on Linux/macOS native)

CLI commands for desktop notifications:

- `messenger replace <receipt> [message]` — replace an existing desktop notification using a saved receipt
- `messenger dismiss <receipt>` — dismiss a delivered desktop notification using a saved receipt
- `messenger info [--json]` — show host OS, detected helpers, election order, and configured routes
- `messenger install [--yes] [--helper <name>…] [--dry-run]` — install missing notification helpers via the host package manager

Discord ships with two adapters behind a single `discord` feature: `DiscordProvider` (bot token, full capability) and `DiscordWebhookProvider` (webhook URL, notification-only). The webhook adapter rejects `reply_to` at plan time with `MessengerError::UnsupportedFeature { feature: "replies" }` — no network call is made.
Both Discord adapters render Markdown through the same Discord renderer; the transport and capability differences live in the provider layer, not in a second markup dialect.

Slack also ships two adapters behind the `slack` feature: `SlackProvider` (bot token, full capability with addressable receipts) and `SlackWebhookProvider` (webhook URL, notification-only). The webhook adapter reuses the same Slack mrkdwn renderer and supports reply threading via `MessageRef::SlackWebhook { thread_ts: Some(...) }`, but rejects attachments (file uploads require the Web API). Successful webhook sends produce receipts with `raw_id == ""`, `message_ref.thread_ts == None`, and `metadata["delivery_confirmed"] = "true"` because Slack incoming webhooks do not return a message identifier.

## Key Types

| Type | Module | Role |
|------|--------|------|
| `Message` | `message.rs` | Portable body, attachments, location, metadata |
| `MessageBody` | `message.rs` | `Plain(String)`, `Markdown(String)`, or `Summarized { summary, markdown }` |
| `Attachment` | `attachment.rs` | File payload with kind, source, caption |
| `Dispatch` | `dispatch.rs` | Target + reply context + delivery options |
| `Target` | `target.rs` | Provider-specific destination enum |
| `MessageRef` | `receipt.rs` | Provider-typed reply reference |
| `SendReceipt` | `receipt.rs` | Delivery proof with raw ID and typed ref |
| `Messenger` | `provider/mod.rs` | Provider registry and send coordinator |
| `Provider` | `provider/mod.rs` | Async trait for provider adapters |
| `CapabilitySet` | `capabilities.rs` | Boolean feature flags per provider |
| `MessengerError` | `error.rs` | Typed error enum for all send paths |

## Send Flow

1. Validate message is not empty
2. Resolve provider from `Target`
3. Normalize against `CapabilitySet` (drop unsupported or error in strict mode)
4. Parse Markdown into `RichNode` AST (once)
5. Render into provider-native output
6. Execute provider send
7. Return `SendReceipt` with typed `MessageRef`

Use `plan_send()` to inspect `SendPlan::warnings` before sending. Use `send_many()` for fan-out.

## Package Layout

```
messenger/
  lib/           # Reusable library crate
    src/
      provider/  # Discord, Discord-Webhook, Slack, Slack-Webhook, Signal, WhatsApp, Telegram, Desktop, APNs, FCM adapters
      markdown/  # AST, parser, per-provider renderers
      tests/     # Unit + wiremock integration tests
  cli/           # messenger binary (send, setup, completions)
    src/
      config.rs  # Route config, secret resolution, prefer_helpers parsing
      info.rs    # Host detection, helper election, route table
      install.rs # Interactive helper installation via sniff
      setup.rs   # Interactive provider setup
      receipt_store.rs
  docs/research/ # Provider research and API design notes
```

Local L1 enables only the `desktop` feature used by the package-area contract.
CI retains `all-features` coverage for every provider; the separate
`local-features` metadata prevents root local testing from inheriting that
provider-wide graph.

## Detailed Documentation

- [Providers Reference](providers.md) - Provider trait, adapter implementations, config structs, capabilities
- [Markdown Rendering](markdown-rendering.md) - AST, parser, per-provider renderers, supported constructs
- [CLI Reference](cli-reference.md) - Commands, route resolution, config format, receipts, setup flow
- [User Guide](../../../messenger/docs/user-guide.md) - Platform setup walkthroughs, CLI config schema, library usage examples

## Related Packages

- **biscuit-terminal**: Used by CLI for styled setup output
- **Source**: [messenger/lib](../../../messenger/lib/), [messenger/cli](../../../messenger/cli/)
