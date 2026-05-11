# Messenger

Unified outbound messaging for Rust applications and shell workflows.

`messenger/` contains two packages:

- [`messenger`](./lib/README.md): a reusable Rust library for building one message payload and sending it to multiple chat providers.
- [`messenger-cli`](./cli/README.md): a `messenger` binary for one-off sends, saved routes, interactive setup, and receipt-backed replies.

## Supported Providers

| Provider | Library feature | Library default | Rich text | Replies | Attachments | Location | Silent | Link preview control |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Discord | `discord` | Yes | Yes | Yes | Yes | Text fallback | No | No |
| Discord-Webhook | `discord` | Yes | Yes | No | Yes | Text fallback | No | No |
| Slack | `slack` | Yes | Yes | Yes | No | Text fallback | No | Yes |
| Slack-Webhook | `slack` | Yes | Yes | Yes | No | Text fallback | No | Yes |
| Signal | `signal` | No | Plain text fallback | Yes | No | Text fallback | No | No |
| WhatsApp | `whatsapp` | No | Plain text fallback | Yes | No | Native | No | No |
| Telegram | `telegram` | No | Yes | Yes | No | Native | Yes | Yes |
| Desktop | `desktop` | No | Plain text fallback | No | Images only | No | Yes | No |

The CLI enables every provider listed above. The library enables `discord` (both bot and webhook adapters) and `slack` (both bot and webhook adapters) by default, with the other providers behind opt-in Cargo features.

For outbound sends, `messenger` uses provider API credentials plus an explicit destination identifier. In practice that means:

- Discord (bot): bot token + channel ID
- Discord (webhook): webhook URL (binds channel + authentication)
- Slack (bot): bot token + channel ID
- Slack (webhook): webhook URL (binds channel + authentication)
- Signal: JSON-RPC account + recipient/group ID
- WhatsApp: Cloud API credentials + recipient phone number
- Telegram: bot token + chat ID
- Desktop: host OS notification center (no credentials or destination identifier; see the [desktop platform guide](./docs/platforms/desktop.md))

Both Slack adapters share the Slack mrkdwn renderer and link-preview controls. Receipts from the webhook adapter have an empty `raw_id` and no `thread_ts`, because Slack incoming webhooks do not return a message identifier; successful delivery is confirmed via `metadata["delivery_confirmed"] = "true"`.

The Discord-Webhook adapter is notification-only: `plan_send()` and `send()` reject `reply_to` with `MessengerError::UnsupportedFeature` before any network call. Discord bot sends and Discord webhook sends share the same Markdown renderer; the practical difference is transport capability, not formatting syntax.

The Desktop adapter delivers local OS notifications via D-Bus on Linux, AppleScript (or native `UserNotifications.framework` when explicitly opted in) on macOS, and WinRT toasts on Windows. Desktop is the only provider that does not require a destination identifier, so `--provider desktop` is valid without `--channel`. On Windows, `messenger setup desktop` must run first to create a Start Menu shortcut and register the App User Model ID; `send` never writes outside `~/.messenger/`.

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

# Send a local desktop notification (no --channel required)
messenger send --provider desktop --title "Build" "Green across the board"

# Reply using a saved receipt
messenger send --route slack.ops --reply-to ~/.messenger/receipts/1712345678000-slack.json "Acknowledged"
```

The CLI stores routes in `~/.messenger.json` and delivery receipts in `~/.messenger/receipts/`.

Route configs mirror the implemented send model: bot-token-plus-channel for Discord (bot) and Slack, webhook-URL-only for Discord (webhook), recipient-based API configuration for Signal and WhatsApp, and bot-token-plus-chat for Telegram.

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

## Release Notes

### Unreleased — Desktop Notifications

**New**

- `desktop` provider for local OS notifications (Linux D-Bus, Windows WinRT toast, macOS AppleScript or native `UserNotifications.framework`). Opt-in via the `desktop` library feature; the CLI enables it by default.
- `Message::title(...)` builder plus `--title`, `--subtitle`, `--icon`, `--category`, `--urgency`, and `--timeout-ms` CLI flags.
- `messenger setup desktop` registers the Windows Start Menu shortcut and AUMID. `send` never writes outside `~/.messenger/`.

**Breaking**

- `CapabilitySet::supports_attachments: bool` has been replaced with `supported_attachment_kinds: BTreeSet<AttachmentKind>`. Providers can now advertise exactly which attachment kinds (`Image`, `Audio`, `Video`, `Document`, `Binary`) they accept. Library consumers who build or inspect a `CapabilitySet` directly must migrate from the boolean to the set.
  - Migration (typical): replace `supports_attachments: true` with `supported_attachment_kinds: BTreeSet::from([AttachmentKind::Image, AttachmentKind::Audio, AttachmentKind::Video, AttachmentKind::Document, AttachmentKind::Binary])`, and `supports_attachments: false` with `supported_attachment_kinds: BTreeSet::new()`. Use `CapabilitySet::all()` or `CapabilitySet::none()` for the two extremes.
  - Best-effort normalization now drops attachments whose kind is missing from the set instead of dropping all attachments when the boolean was `false`. Strict mode still fails with `MessengerError::UnsupportedFeature { feature: "attachments" }` when any unsupported attachment is present.
