# messenger

Portable outbound messaging for Rust. Build one `Message`, choose a `Dispatch`, and let a provider adapter translate that payload into the provider-native API call.

## What The Library Owns

The library is responsible for:

- message modeling (`Message`, `Attachment`, `Location`)
- destination and reply modeling (`Dispatch`, `Target`, `MessageRef`)
- provider registration and routing (`Messenger`, `Provider`)
- capability-aware normalization (`plan_send`, compatibility warnings, strict mode)
- provider-specific Markdown rendering and plain-text fallbacks
- provider-typed delivery receipts (`SendReceipt`)

The library does not load secrets or configuration files. Callers construct provider configs directly and register the providers they need.

The built-in providers follow the provider API models currently implemented in this repo:

- Discord (bot): bot token + channel ID
- Discord (webhook): full webhook URL (binds channel + authentication)
- Slack (bot): bot token + channel ID
- Slack (webhook): full webhook URL (binds channel + authentication)
- Signal: JSON-RPC URL + sending account + recipient/group ID
- WhatsApp: Cloud API access token + phone number ID + recipient
- Telegram: bot token + chat ID
- Desktop: host OS notification center (no credentials; runtime backend picks D-Bus, WinRT toast, or AppleScript)

Discord and Slack each ship both a bot-token adapter and a webhook-URL adapter. The Discord webhook adapter is notification-only and does not support replies. The Slack webhook adapter supports mrkdwn and reply threading, but does not support attachments, and successful sends produce receipts with an empty `raw_id` and no `thread_ts` since Slack incoming webhooks do not return a message identifier.

Desktop is a single provider (`DesktopNotificationProvider`) that picks its backend at construction time based on the host OS: `notify-rust` on Linux, AppleScript or native `UserNotifications.framework` on macOS, and `winrt-notification` on Windows. Desktop sends accept only image attachments and accept title-only messages so notifications can carry a summary without a body.

## Prelude

For the common path, import the crate prelude:

```rust
use messenger::prelude::*;
```

The prelude re-exports the core message, dispatch, receipt, error, registry, and built-in provider config/provider types behind their corresponding Cargo features.

## Feature Flags

```toml
[dependencies]
messenger = { path = "messenger/lib", default-features = false, features = ["slack", "telegram"] }
```

Available features:

- `discord`
- `slack`
- `signal`
- `whatsapp`
- `telegram`
- `desktop`

Default features: `discord`, `slack`

The `desktop` feature pulls in platform-specific dependencies per target: `notify-rust` on Linux, `winrt-notification` on Windows, and `objc2-foundation` + `objc2-user-notifications` on macOS.

## Core Types

| Type            | Role                                               |
|-----------------|----------------------------------------------------|
| `Message`       | Portable body, attachments, location, and metadata |
| `MessageBody`   | Plain text or Markdown                             |
| `Attachment`    | Image, document, or other file payload             |
| `Dispatch`      | Target, reply context, and delivery options        |
| `Target`        | Provider-specific destination                      |
| `MessageRef`    | Provider-specific reply reference                  |
| `SendReceipt`   | Provider, raw ID, typed reply reference, metadata  |
| `Messenger`     | Provider registry and send coordinator             |
| `Provider`      | Adapter trait implemented by each provider         |
| `CapabilitySet` | Declares what a provider can actually do           |

## Send Flow

`Messenger::send` performs the same pipeline the CLI relies on:

1. Validate the message is not empty.
2. Resolve the provider from the `Dispatch` target.
3. Normalize the request against provider capabilities.
4. Emit warnings for best-effort drops, or error in strict mode.
5. Parse Markdown once into an internal AST.
6. Render the prepared message into provider-native output.
7. Return a `SendReceipt` with a provider-typed `MessageRef`.

If you want visibility before sending, use `plan_send` and inspect `SendPlan::warnings`.

## Provider Support

| Provider        | Rich text           | Replies | Attachments | Location handling       | Silent | Link previews |
|-----------------|---------------------|---------|-------------|-------------------------|--------|---------------|
| Discord         | Markdown rendering  | Yes     | Yes         | Appends text fallback   | No     | No            |
| Discord-Webhook | Markdown rendering  | No      | Yes         | Appends text fallback   | No     | No            |
| Slack           | mrkdwn rendering    | Yes     | No          | Appends text fallback   | No     | Yes           |
| Slack-Webhook   | mrkdwn rendering    | Yes     | No          | Appends text fallback   | No     | Yes           |
| Signal          | Plain-text fallback | Yes     | No          | Appends text fallback   | No     | No            |
| WhatsApp        | Plain-text fallback | Yes     | No          | Native location payload | No     | No            |
| Telegram        | HTML rendering      | Yes     | No          | Native location payload | Yes    | Yes           |
| Desktop         | Plain-text fallback | No      | Images only | Dropped                 | Yes    | No            |

Two details matter when integrating:

- A provider can report `supports_location = true` even when it does not have a native location API. Discord, Slack, and Signal keep the location by appending a formatted text line to the rendered body.
- Markdown is parsed for all Markdown messages, but Signal, WhatsApp, and Desktop warn that rich rendering is unsupported and fall back to plain text.
- Telegram and WhatsApp send native location requests. If a message contains both text and location, the location is sent and the text body is not.
- Desktop accepts only image attachments; multiple images collapse to the first image in best-effort mode. Locations are dropped in best-effort and fail in strict. A title-only desktop message is valid — `plan_send()` skips the "empty body" check when the resolved provider is `desktop`.

## Example

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
        .metadata("service", "api")
        .metadata("env", "prod");

    let dispatch = Dispatch::to(Target::slack_channel("C01234567"));
    let plan = messenger.plan_send(dispatch, &message)?;

    for warning in &plan.warnings {
        eprintln!("{warning}");
    }

    let receipt = messenger.send_planned(plan).await?;
    println!("{}", receipt.to_pretty_json().unwrap());
    Ok(())
}
```

This example is intentionally using the implemented Slack transport model: authenticate with a bot token and choose the destination with `Target::slack_channel(...)`.

## Message Model

`Message` supports three main entry points:

```rust
use messenger::prelude::*;

let plain = Message::text("hello");
let markdown = Message::markdown("**hello** from `messenger`");
let location = Message::location(34.05, -118.24);

let with_file = Message::markdown("Artifact ready")
    .attachment(Attachment::file("/tmp/build.log"))
    .metadata("job", "deploy");
```

Attachment sources can be local files, URLs, in-memory bytes, or provider-native file identifiers. Validation rejects missing files, unreadable paths, empty URLs, and malformed provider file IDs before a send is attempted.

The Discord adapters (bot and webhook) support the full attachment-kind set today. The Desktop provider accepts image attachments only. Every other provider accepts no attachments. Attachments must come from a local file or in-memory bytes; URL-based and provider-file-ID attachments are rejected by both Discord adapters.

## Desktop Notifications

Register the desktop provider like any other:

```rust
use messenger::prelude::*;
use messenger::provider::desktop::{DesktopConfig, DesktopNotificationProvider};

#[tokio::main]
async fn main() -> Result<(), messenger::MessengerError> {
    let mut messenger = Messenger::new();
    messenger.register(Box::new(DesktopNotificationProvider::new(DesktopConfig {
        app_name: "Messenger".into(),
        ..DesktopConfig::default()
    })));

    // Title-only desktop messages are valid.
    let message = Message::text("Build finished in 42s").title("Build");
    let dispatch = Dispatch::to(Target::desktop());
    let receipt = messenger.send(dispatch, &message).await?;
    println!("delivered: {}", receipt.raw_id);
    Ok(())
}
```

The backend is selected at construction time from the compile target. See [`messenger/docs/platforms/desktop.md`](../docs/platforms/desktop.md) for platform caveats, including the Windows Start Menu shortcut requirement and the macOS AppleScript-vs-native strategy.

Per-dispatch overrides (subtitle, category, urgency, timeout, replace ID, custom icon) ride on `ProviderOverrides::Desktop(DesktopOverrides { .. })`.

## Dispatch And Replies

`Dispatch` carries delivery behavior that should not be baked into the reusable message:

```rust
use messenger::prelude::*;

let dispatch = Dispatch::to(Target::telegram_chat(
    messenger::target::TelegramChatId::Id(-1001234567890),
))
.reply_to(MessageRef::Telegram {
    chat_id: messenger::receipt::TelegramChatRef::Id(-1001234567890),
    message_id: 42,
    thread_id: None,
})
.silent()
.strict();
```

Strict mode turns capability mismatches into `MessengerError::UnsupportedFeature`. Best-effort mode keeps going and returns warnings such as:

```text
⚠️ the attachments feature is not supported on Slack and will be dropped
```

The library also validates that a `reply_to` reference matches the target provider.

## Markdown Rendering

Markdown is parsed with `pulldown-cmark` into an internal AST and then rendered per provider:

- Discord: Discord-flavored Markdown
- Slack: Slack mrkdwn
- Telegram: Telegram Bot API HTML
- Signal / WhatsApp / Desktop: plain text

The parser currently handles the formatting primitives that show up in the codebase and tests: paragraphs, headings, bold, italic, strikethrough, links, inline code, fenced code blocks, and lists. Unsupported Markdown constructs are flattened to their children instead of preserved as provider-specific syntax.

`ProviderOverrides` is public, but the current override structs are empty placeholders and the built-in providers do not read `dispatch.overrides` yet.

## Errors

All public send paths return `Result<_, MessengerError>`.

Common variants:

- `InvalidMessage`
- `UnsupportedFeature`
- `MissingConfiguration`
- `Authentication`
- `RateLimited`
- `Transport`
- `Provider`

Two current limitations are worth documenting explicitly:


- `Message::metadata` is preserved on the message and receipt types, but the built-in providers do not currently consume it.
- Built-in providers currently return empty receipt metadata maps.

## Testing

From [`messenger/`](../):

```bash
just test
```

The crate includes:

- unit tests for message builders, normalization, Markdown rendering, and provider adapters
- `wiremock`-backed provider integration tests
- ignored smoke tests in `tests/integration.rs` for real Slack and Discord sends
- desktop helper integration tests in `tests/desktop_helpers.rs` that exercise real `tokio::process::Command` invocations against stub binaries

### Desktop Helper Test Conventions

All desktop backend modules (`linux.rs`, `macos.rs`, `windows.rs`) and helper modules (`dunstify.rs`, `notify_send.rs`, `terminal_notifier.rs`, `alerter.rs`, `snoretoast.rs`, `burnttoast.rs`) compile on every host. Their unit tests run on macOS, Linux, and Windows alike, so a contributor on one platform can validate the full helper matrix.

Stub helper binaries live under [`tests/bin/`](./tests/bin/). Each stub is a tiny Rust program that mimics the real helper's stdout/exit-code contract under environment-variable control:

- `stub_dunstify`, `stub_notify_send` — Linux helpers
- `stub_snoretoast`, `stub_burnttoast` — Windows helpers
- `stub_terminal_notifier`, `stub_alerter` — macOS helpers

The integration test in `tests/desktop_helpers.rs` exercises every helper against its stub. When you add a new helper, you must:

1. Add a `stub_<helper>` binary under `tests/bin/<name>/main.rs` that reproduces the real helper's exit-code and stdout protocol.
2. Add a corresponding integration test to `tests/desktop_helpers.rs` covering success, interactive (actions/reply), replace, timeout, non-zero exit, and parse-error paths.
3. Make the helper module compile on all platforms (gate native API calls with `#[cfg(target_os = "...")]`, not the entire module).

## Key Crates

- `pulldown-cmark` for Markdown parsing
- `reqwest` for Slack, Signal, WhatsApp, Telegram, and Discord-webhook HTTP calls
- `twilight-http` and `twilight-model` for the Discord bot adapter
- `thiserror` for the public error type
- `serde` / `serde_json` for typed receipts and reply references
- `notify-rust` (Linux), `winrt-notification` (Windows), `objc2-foundation` + `objc2-user-notifications` (macOS), and `uuid` (shared) for the `desktop` feature

## Lessons Learned

- Separating `Message` from `Dispatch` keeps reusable content clean and makes fan-out sends straightforward.
- Provider-typed `MessageRef` values are worth the extra modeling because replies are not structurally equivalent across APIs.
- Best-effort normalization is useful for operator workflows, but `plan_send` is the right escape hatch when callers need to inspect drops before sending.
