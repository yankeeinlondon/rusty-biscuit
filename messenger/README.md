# Messenger

Unified outbound messaging for Rust applications and shell workflows.

`messenger/` contains two packages:

- [`messenger`](./lib/README.md): a reusable Rust library for building one message payload and sending it to multiple chat providers.
- [`messenger-cli`](./cli/README.md): a `messenger` binary for one-off sends, saved routes, interactive setup, and receipt-backed replies.

## Supported Providers

| Provider | Library feature | Library default | Rich text | Replies | Attachments | Location | Silent | Link preview control |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Discord | `discord` | Yes | Yes | Yes | Yes | Text fallback | No | No |
| Slack | `slack` | Yes | Yes | Yes | No | Text fallback | No | Yes |
| Signal | `signal` | No | Plain text fallback | Yes | No | Text fallback | No | No |
| WhatsApp | `whatsapp` | No | Plain text fallback | Yes | No | Native | No | No |
| Telegram | `telegram` | No | Yes | Yes | No | Native | Yes | Yes |

The CLI enables all five providers. The library enables `discord` and `slack` by default, with the other providers behind opt-in Cargo features.

For outbound sends, `messenger` uses provider API credentials plus an explicit destination identifier. In practice that means:

- Discord: bot token + channel ID
- Slack: bot token + channel ID
- Signal: JSON-RPC account + recipient/group ID
- WhatsApp: Cloud API credentials + recipient phone number
- Telegram: bot token + chat ID

The current implementation does not use Slack or Discord incoming webhook URLs as its primary send model.

## How It Works

`messenger` separates portable content from provider-specific delivery details:

- `Message` holds the reusable body, attachments, location payload, and metadata.
- `Dispatch` holds the destination, reply context, and delivery options.
- `SendReceipt` captures the provider-native identifier needed for future replies.

The library validates the message, normalizes it against provider capabilities, renders Markdown into provider-specific output, and returns compatibility warnings when best-effort delivery drops unsupported features.

## Quick Example

```rust
use messenger::prelude::*;
use secrecy::SecretString;

#[tokio::main]
async fn main() -> Result<(), messenger::MessengerError> {
    let mut messenger = Messenger::new();
    messenger.register(Box::new(SlackProvider::new(SlackConfig {
        bot_token: SecretString::from(std::env::var("SLACK_BOT_TOKEN").unwrap()),
        api_base_url: None,
    })));

    let message = Message::markdown("**Deploy succeeded**")
        .metadata("service", "api");
    let dispatch = Dispatch::to(Target::slack_channel("C01234567"));

    let receipt = messenger.send(dispatch, &message).await?;
    println!("{}", receipt.to_pretty_json().unwrap());
    Ok(())
}
```

For a full library walkthrough, see [`messenger/lib/README.md`](./lib/README.md).

## CLI Quick Start

```bash
# Configure a route interactively
messenger setup

# Send to the default route
messenger send "Deploy succeeded"

# Send to an ad-hoc route
messenger send --provider slack --channel C01234567 "Deploy succeeded"

# Reply using a saved receipt
messenger send --route slack.ops --reply-to ~/.messenger/receipts/1712345678000-slack.json "Acknowledged"
```

The CLI stores routes in `~/.messenger.json` and delivery receipts in `~/.messenger/receipts/`.

Route configs mirror the implemented send model: bot-token-plus-channel for Discord and Slack, recipient-based API configuration for Signal and WhatsApp, and bot-token-plus-chat for Telegram.

For command details and config examples, see [`messenger/cli/README.md`](./cli/README.md).

For provider setup instructions, CLI configuration, and library usage examples, see the [User Guide](./docs/user-guide.md).

## Package Layout

- `lib/`: library crate and provider adapters
- `cli/`: command-line interface
- `docs/research/platforms/`: provider-specific implementation research
- `docs/research/notifications/`: API notes and design references

## Development

Run commands from [`messenger/`](./):

```bash
just build
just test
just lint
```
