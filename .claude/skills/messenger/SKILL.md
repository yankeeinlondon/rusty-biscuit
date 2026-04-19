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

## Provider Support

| Provider | Feature flag | Rich text | Replies | Attachments | Location | Silent | Link preview |
|----------|-------------|-----------|---------|-------------|----------|--------|-------------|
| Discord | `discord` (default) | Markdown | Yes | Yes | Text fallback | No | No |
| Discord-Webhook | `discord` (default) | Markdown | No | Yes | Text fallback | No | No |
| Slack | `slack` (default) | mrkdwn | Yes | No | Text fallback | No | Yes |
| Signal | `signal` | Plain text | Yes | No | Text fallback | No | No |
| WhatsApp | `whatsapp` | Plain text | Yes | No | Native | No | No |
| Telegram | `telegram` | HTML | Yes | No | Native | Yes | Yes |

Discord ships with two adapters behind a single `discord` feature: `DiscordProvider` (bot token, full capability) and `DiscordWebhookProvider` (webhook URL, notification-only). The webhook adapter rejects `reply_to` at plan time with `MessengerError::UnsupportedFeature { feature: "replies" }` — no network call is made.

## Key Types

| Type | Module | Role |
|------|--------|------|
| `Message` | `message.rs` | Portable body, attachments, location, metadata |
| `MessageBody` | `message.rs` | `Plain(String)` or `Markdown(String)` |
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
      provider/  # Discord, Discord-Webhook, Slack, Signal, WhatsApp, Telegram adapters
      markdown/  # AST, parser, per-provider renderers
      tests/     # Unit + wiremock integration tests
  cli/           # messenger binary (send, setup, completions)
    src/
      config.rs  # Route config and secret resolution
      setup.rs   # Interactive provider setup
      receipt_store.rs
  docs/research/ # Provider research and API design notes
```

## Detailed Documentation

- [Providers Reference](providers.md) - Provider trait, adapter implementations, config structs, capabilities
- [Markdown Rendering](markdown-rendering.md) - AST, parser, per-provider renderers, supported constructs
- [CLI Reference](cli-reference.md) - Commands, route resolution, config format, receipts, setup flow
- [User Guide](../../../messenger/docs/user-guide.md) - Platform setup walkthroughs, CLI config schema, library usage examples

## Related Packages

- **biscuit-terminal**: Used by CLI for styled setup output
- **Source**: [messenger/lib](../../../messenger/lib/), [messenger/cli](../../../messenger/cli/)
