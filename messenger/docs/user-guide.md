# Messenger User Guide

This guide walks through setting up each messaging provider, configuring the CLI, and using the library from Rust code.

## Platform Setup

Each provider requires credentials and a destination identifier. The sections below explain what you need, where to get it, and what the setup looks like for both the CLI and the library.

The outbound send model in `messenger` is provider API credentials plus an explicit destination identifier:

- Discord (bot): bot token + channel ID
- Discord (webhook): full webhook URL (binds channel + authentication)
- Slack (bot): bot token + channel ID
- Slack (webhook): full webhook URL (binds channel + authentication)
- Signal: JSON-RPC URL + account + recipient/group ID
- WhatsApp: Cloud API credentials + recipient phone number
- Telegram: bot token + chat ID
- Desktop: host OS notification center (no credentials, no destination identifier)

Discord and Slack each ship with two distinct adapters: a bot-token adapter (full capability, replies supported) and a webhook adapter (notification-only, no attachments, receipts are not addressable).

Desktop is the odd provider out: it targets the running host OS rather than a remote API, so it needs no credentials and no target identifier. Runtime behavior varies by platform — see [`docs/platforms/desktop.md`](./platforms/desktop.md) for Linux D-Bus, Windows WinRT toast, and macOS AppleScript/native specifics.

### Discord (Bot)

**What you need:**

- A **Bot Token** to authenticate API calls
- A **Channel ID** to identify where messages are sent

**Account setup:**

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications) and create a new application.
2. Under the **Bot** tab, click **Add Bot** (or **Reset Token** if one already exists) and copy the bot token.
3. Under **OAuth2 > URL Generator**, select the `bot` scope and the `Send Messages` and `Attach Files` permissions. Copy the generated URL and open it to invite the bot to your server.
4. In Discord, enable **Developer Mode** (User Settings > Advanced > Developer Mode). Right-click the target channel and select **Copy Channel ID**.

**Helpful resources:**

- [Discord Developer Documentation](https://discord.com/developers/docs/intro)
- [Bot Permissions Calculator](https://discord.com/developers/docs/topics/permissions)

**Environment variable:** `DISCORD_BOT_TOKEN`

---

### Discord (Webhook)

Prefer a Discord **webhook** over a bot when you only need one-way notifications into a specific channel: no Discord application is required, there is no gateway to run, and the per-server permission model is simpler (just "Manage Webhooks"). The tradeoff is capability — webhooks cannot reply to an existing message and cannot be modeled as a conversational agent.

**What you need:**

- A **Webhook URL** of the form `https://discord.com/api/v10/webhooks/{id}/{token}`. This single URL binds both the target channel and the authentication token — there is no separate channel ID field.

**Account setup:**

1. Open the target Discord server and go to **Server Settings > Integrations > Webhooks**.
2. Click **New Webhook**, pick a name and channel, and click **Copy Webhook URL**.
3. Treat the URL as a secret: anyone with it can post to the channel.

**Helpful resources:**

- [Discord Webhooks Guide](https://support.discord.com/hc/en-us/articles/228383668-Intro-to-Webhooks)
- [Execute Webhook API Reference](https://discord.com/developers/docs/resources/webhook#execute-webhook)

**Capability notes:**

- Markdown rendering: supported (same as the bot adapter).
- Attachments: supported (local path or in-memory bytes).
- Replies: **not supported** — `plan_send()` and `send()` return `MessengerError::UnsupportedFeature { provider: DiscordWebhook, feature: "replies" }` before any network call. Use the bot adapter if you need threaded replies.
- Thread routing: optional, supplied at dispatch time via `Target::discord_webhook_thread(thread_id)`. Thread IDs are not part of the route config.

**Environment variable:** `DISCORD_WEBHOOK_URL`

---

### Slack (Bot)

**What you need:**

- A **Bot Token** (`xoxb-...`) to authenticate API calls
- A **Channel ID** to identify where messages are sent

**Account setup:**

1. Go to [Slack API: Your Apps](https://api.slack.com/apps) and click **Create New App** > **From scratch**.
2. Under **OAuth & Permissions**, add the `chat:write` bot token scope. If you want to send to channels the bot hasn't been invited to, also add `chat:write.public`.
3. Click **Install to Workspace** and authorize. Copy the **Bot User OAuth Token** (starts with `xoxb-`).
4. In Slack, right-click the target channel, select **View channel details**, and scroll to the bottom to find the **Channel ID** (starts with `C`).
5. Invite the bot to the channel: `/invite @YourBotName`.

**Helpful resources:**

- [Slack API Documentation](https://api.slack.com/docs)
- [chat.postMessage Reference](https://api.slack.com/methods/chat.postMessage)

**Environment variable:** `SLACK_BOT_TOKEN`

---

### Slack (Webhook)

Prefer a Slack **incoming webhook** over a bot when you only need one-way notifications into a specific channel: no Slack app token management is required, and the permission model is simpler (just "Incoming Webhooks"). The tradeoff is capability — webhooks cannot upload files, and the Slack response does not include a message ID, so webhook receipts cannot be used for richer downstream follow-up.

**What you need:**

- A **Webhook URL** of the form `https://hooks.slack.com/services/T.../B.../<token>`. This single URL binds both the target channel and the authentication token — there is no separate channel ID field.

**Account setup:**

1. Go to [Slack API: Your Apps](https://api.slack.com/apps), open your app (or create one), and enable **Incoming Webhooks**.
2. Click **Add New Webhook to Workspace**, pick the target channel, and copy the webhook URL.
3. Treat the URL as a secret: anyone with it can post to the channel.

**Helpful resources:**

- [Slack Incoming Webhooks Guide](https://api.slack.com/messaging/webhooks)

**Capability notes:**

- Markdown rendering: supported (uses the same Slack mrkdwn renderer as the bot adapter).
- Attachments: **not supported** — file uploads require the Web API.
- Replies: supported. The reply thread is driven by `MessageRef::SlackWebhook { thread_ts: Some(...) }` on the dispatch's `reply_to`. Webhook responses do not return a `thread_ts`, so a webhook send cannot be replied to using only its own receipt — the thread timestamp has to come from a Slack bot receipt or be supplied externally.
- Link preview suppression: supported via `dispatch.options.disable_link_preview`.
- Receipts: `raw_id` is always empty and `message_ref.thread_ts` is `None`. Successful sends are recorded with `metadata["delivery_confirmed"] = "true"`, but there is no message ID to address later.

**Environment variable:** `SLACK_WEBHOOK_URL`

---

### Signal

**What you need:**

- A running **signal-cli** daemon with JSON-RPC enabled
- The **RPC URL** of the daemon (e.g., `http://localhost:7583`)
- A registered **Signal account** (phone number) for the daemon
- A **recipient** phone number or group ID

**Account setup:**

1. Install [signal-cli](https://github.com/AsamK/signal-cli) (available via Homebrew, AUR, or manual download).
2. Register or link an account:

   ```bash
   # Register a new number (requires SMS verification)
   signal-cli -a +1234567890 register
   signal-cli -a +1234567890 verify 123456

   # Or link to an existing Signal account
   signal-cli link -n "messenger-cli"
   ```

3. Start the JSON-RPC daemon:

   ```bash
   signal-cli -a +1234567890 daemon --json-rpc
   ```

   This listens on `http://localhost:7583` by default.
4. The recipient is either a phone number with country code (e.g., `+15551234567`) for direct messages, or a base64-encoded group ID for group messages. Find group IDs with `signal-cli -a +1234567890 listGroups`.

**Helpful resources:**

- [signal-cli GitHub](https://github.com/AsamK/signal-cli)
- [signal-cli man page](https://github.com/AsamK/signal-cli/blob/master/man/signal-cli.1.adoc)

**Environment variables:** `SIGNAL_RPC_URL`, `SIGNAL_ACCOUNT`

---

### WhatsApp

**What you need:**

- A **Cloud API Access Token** to authenticate API calls
- A **Phone Number ID** identifying the WhatsApp Business number that sends messages
- A **recipient phone number** for the person or group receiving messages

**Account setup:**

1. Go to [Meta for Developers](https://developers.facebook.com/) and create an app with the **WhatsApp** product.
2. In the app dashboard, navigate to **WhatsApp > API Setup**.
3. Copy the **Temporary access token** (for testing) or generate a permanent System User token (for production).
4. Copy the **Phone number ID** shown in the API Setup section. This identifies which WhatsApp Business number sends your messages.
5. Add recipient phone numbers to the **allowed list** (required during testing with the temporary token). In production with an approved Business account, this restriction is lifted.

**Helpful resources:**

- [WhatsApp Cloud API Getting Started](https://developers.facebook.com/docs/whatsapp/cloud-api/get-started)
- [WhatsApp Message API Reference](https://developers.facebook.com/docs/whatsapp/cloud-api/reference/messages)

**Environment variables:** `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`

---

### Telegram

**What you need:**

- A **Bot Token** to authenticate API calls
- A **Chat ID** for the target conversation, group, or channel

**Account setup:**

1. Open Telegram and message [@BotFather](https://t.me/BotFather).
2. Send `/newbot`, follow the prompts to name your bot, and copy the bot token (looks like `123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11`).
3. Find the target Chat ID using one of these methods:
   - Message [@userinfobot](https://t.me/userinfobot) to get your personal chat ID.
   - For groups: add the bot to the group, send a message, then call `https://api.telegram.org/bot<TOKEN>/getUpdates` and look for the `chat.id` field.
   - For channels: use the `@channelname` format (e.g., `@ops_channel`).
4. For groups and channels, make sure the bot is added as a member (or admin for channels).

**Helpful resources:**

- [Telegram Bot API Documentation](https://core.telegram.org/bots/api)
- [BotFather Commands](https://core.telegram.org/bots/features#botfather)

**Environment variable:** `TELEGRAM_BOT_TOKEN`

---

### Desktop

The desktop provider delivers a local notification to the host OS notification center. It is unlike every other provider in two ways:

- there are no credentials to provision — the target is the running desktop session
- there is no destination identifier — `--channel` is not used, and `Target::desktop()` carries no fields

**What you need:**

- Linux: a running notification daemon that speaks freedesktop.org notifications over D-Bus (typical on GNOME, KDE Plasma, XFCE, MATE, Cinnamon). No permission prompt.
- macOS: nothing extra for the default `strategy: auto`, which uses `osascript display notification`. The native path (`strategy: native_user_notifications`) requires a bundled and signed app identity and will trigger an authorization prompt on first use.
- Windows: run `messenger setup desktop` once. Setup registers a Start Menu shortcut tied to an App User Model ID (AUMID) — WinRT toasts require this pairing for unpackaged desktop applications. `send` never creates, repairs, or removes the shortcut; if the shortcut is missing, `send` returns `MessengerError::MissingConfiguration` with remediation text pointing back to `setup desktop`.

**Helpful resources:**

- [freedesktop.org Notification Spec](https://specifications.freedesktop.org/notification-spec/latest/)
- [WinRT toast notifications overview](https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/toast-ux-guidance)
- [Apple `UserNotifications` framework](https://developer.apple.com/documentation/usernotifications)

**Environment variables:** none — desktop routes persist all configuration in `~/.messenger.json`.

**Capability notes:**

- Title: `Message::title(...)` (library) or `--title` (CLI). Title-only messages are valid.
- Body: rendered as plain text (Markdown is downgraded with a warning in best-effort mode).
- Attachments: image attachments only. Multiple images collapse to the first image in best-effort mode; non-image attachments are dropped in best-effort and fail in strict mode.
- Locations: dropped in best-effort, fail in strict.
- Replies: unsupported.
- Silent delivery: supported (maps to `suppress-sound` on Linux, `sound(None)` on Windows, omitted sound on macOS).

---

## CLI Configuration

### Config File Location

The CLI stores its configuration at `~/.messenger.json`. The `messenger setup` command creates and updates this file interactively.

The config examples below intentionally follow the implemented transport models: bot-token-plus-channel for Discord (bot) and Slack (bot), webhook-URL-only for Discord (webhook) and Slack (webhook), recipient-based API sends for Signal and WhatsApp, and bot-token-plus-chat for Telegram.

### Config Schema

```json
{
  "default_route": "slack.ops",
  "routes": {
    "slack.ops": {
      "provider": "slack",
      "channel_id": "C012345ABC",
      "bot_token_env": "SLACK_BOT_TOKEN"
    },
    "slack.deploys": {
      "provider": "slack-webhook",
      "webhook_url_env": "SLACK_WEBHOOK_URL"
    },
    "discord.alerts": {
      "provider": "discord",
      "channel_id": "123456789012345678",
      "bot_token_env": "DISCORD_BOT_TOKEN"
    },
    "discord.deploys": {
      "provider": "discord-webhook",
      "webhook_url_env": "DISCORD_WEBHOOK_URL"
    },
    "telegram.ops": {
      "provider": "telegram",
      "chat_id": "@ops_channel",
      "bot_token": "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
    },
    "signal.ops": {
      "provider": "signal",
      "recipient": "+15551234567",
      "rpc_url_env": "SIGNAL_RPC_URL",
      "account_env": "SIGNAL_ACCOUNT"
    },
    "whatsapp.support": {
      "provider": "whatsapp",
      "recipient": "+15559876543",
      "access_token_env": "WHATSAPP_ACCESS_TOKEN",
      "phone_number_id_env": "WHATSAPP_PHONE_NUMBER_ID"
    },
    "desktop.local": {
      "provider": "desktop",
      "app_name": "Messenger",
      "default_title": "Messenger",
      "icon": "dialog-information",
      "urgency": "normal",
      "timeout_ms": 5000,
      "windows": { "app_id": "RustyBiscuit.Messenger" },
      "macos": { "bundle_id": "com.rustybiscuit.messenger", "strategy": "auto" },
      "linux": { "desktop_entry": "messenger" }
    }
  }
}
```

**Top-level fields:**

| Field | Type | Description |
|-------|------|-------------|
| `default_route` | `string?` | Route used when no `--route` or `--provider` is specified |
| `routes` | `object` | Named routes keyed by route name |

**Route fields by provider:**

| Provider | Required | Optional |
|----------|----------|----------|
| Discord | `provider`, `channel_id` | `bot_token`, `bot_token_env` |
| Discord (webhook) | `provider` (`"discord-webhook"`) | `webhook_url`, `webhook_url_env` |
| Slack | `provider`, `channel_id` | `bot_token`, `bot_token_env` |
| Slack (webhook) | `provider` (`"slack-webhook"`) | `webhook_url`, `webhook_url_env` |
| Signal | `provider`, `recipient` | `rpc_url`, `rpc_url_env`, `account`, `account_env` |
| WhatsApp | `provider`, `recipient` | `access_token`, `access_token_env`, `phone_number_id`, `phone_number_id_env` |
| Telegram | `provider`, `chat_id` | `bot_token`, `bot_token_env` |
| Desktop | `provider`, `app_name` | `default_title`, `icon`, `category`, `urgency`, `timeout_ms`, `windows.app_id`, `macos.bundle_id`, `macos.strategy`, `linux.desktop_entry` |

For `discord-webhook` routes, at least one of `webhook_url` or a resolvable `webhook_url_env` must be present at send time. The webhook URL is the single credential; there is no `channel_id` field because the URL already binds the channel.

For `slack-webhook` routes, the same rule applies: at least one of `webhook_url` or a resolvable `webhook_url_env` must be present at send time. An empty or whitespace-only `webhook_url` is treated as absent and the CLI falls back to `webhook_url_env` (defaults to `SLACK_WEBHOOK_URL` when omitted).

### Secret Resolution

Each secret can be provided in two ways:

1. **Inline** — stored directly in the config (e.g., `"bot_token": "xoxb-..."`)
2. **Environment variable** — referenced by name (e.g., `"bot_token_env": "SLACK_BOT_TOKEN"`)

When both are present, the inline value takes priority. If neither is set, the CLI falls back to a default environment variable name per provider (see the table in [Platform Setup](#platform-setup)).

### Discord Webhook Route Setup

`messenger setup discord-webhook` follows the same secret-resolution model as the bot-token routes:

1. Choose whether to store the webhook URL directly in `~/.messenger.json` or reference it via an environment variable.
2. If you choose the env-var path, the default suggested variable is `DISCORD_WEBHOOK_URL`.
3. The saved route contains only `webhook_url` or `webhook_url_env`; it never stores a `channel_id` because the webhook URL already selects the channel.

Discord webhook thread routing is a dispatch-time concern, not a route field. CLI route config does not persist `thread_id`; library callers provide it with `Target::discord_webhook_thread(...)` when needed.

### Slack Webhook Route Setup

`messenger setup slack-webhook` follows the same secret-resolution model as the bot-token routes:

1. Choose whether to store the webhook URL directly in `~/.messenger.json` or reference it via an environment variable.
2. If you choose the direct-value path, the prompt uses masked input so the URL is not echoed and an empty value is rejected before the route is written.
3. If you choose the env-var path, the default suggested variable is `SLACK_WEBHOOK_URL`.
4. The saved route contains only `webhook_url` or `webhook_url_env`; it never stores a `channel_id` because the webhook URL already selects the channel.

Key behavioral differences from the Slack bot route:

- No file uploads: attachments are rejected or dropped per the usual compatibility rules.
- Receipts have no message ID: `raw_id` is always empty and `message_ref.thread_ts` is `None` on success. A webhook receipt cannot be used on its own as the reply target of a later send.
- Replies are still possible by supplying `--reply-to` from a Slack bot receipt or any other source that carries a Slack `thread_ts`.

### Desktop Route Setup

`messenger setup desktop` collects the portable fields first (app name, default title, icon, category, urgency, timeout) and then branches by platform:

- **Windows** — prompts for an optional `app_id` (defaults to `RustyBiscuit.Messenger`) and then writes a Start Menu shortcut keyed on that AUMID. The shortcut path is printed on success. A shortcut must exist before any Windows desktop send will succeed; `send` never creates or repairs it.
- **macOS** — prompts for an optional `bundle_id` and a strategy. `auto` (default) uses AppleScript and does not trigger a notification authorization prompt. `native_user_notifications` uses `UserNotifications.framework` and requires a bundled, signed app identity. `applescript` is an explicit synonym for the default.
- **Linux** — prompts for an optional `desktop_entry` value used as the D-Bus `desktop-entry` hint.

Desktop routes persist configuration only — no secrets. Running `messenger send --provider desktop` ad-hoc (without a route) uses the library defaults for every field, which is enough for a simple `--title "..."` notification.

### Desktop Notification Helpers

On Linux and Windows, `messenger` uses installed helper utilities (`dunstify`, `notify-send`, `snoretoast`, `BurntToast`) as the **primary** delivery path. When helpers are present, interactive actions, inline replies, image attachments, and reliable notification replacement are fully supported. The native API (`notify-rust` on Linux, `winrt-notification` on Windows) remains as a fallback for simple notifications when no helper is installed.

Each desktop backend delegates delivery to a third-party CLI helper before falling back to its native API. The library detects which helpers are present at startup, scores them per dispatch, and uses the highest-scoring one. If a helper fails (missing binary, exited non-zero, timed out), the backend tries the next helper and finally the native path.

**Helpers per OS:**

| OS      | Helpers                            | Best for                             |
|---------|------------------------------------|--------------------------------------|
| Linux   | `dunstify`, `notify-send`          | Interactive actions on dunst; universal notice-only |
| macOS   | `terminal-notifier`, `alerter`     | Notice-only on macOS; interactive actions/replies   |
| Windows | `snoretoast`, `BurntToast`         | Default Windows toasts; PowerShell module fallback  |

**Inspecting helper availability:**

```bash
messenger info            # human-readable table
messenger info --json     # machine-readable record
messenger info --plain    # uncolored text (for scripts)
```

The output lists every helper in the catalog, whether it is installed, the detected version, the active notification daemon (Linux only), and the election order the backend will use on this host.

**Installing missing helpers:**

```bash
messenger install                     # interactively pick from missing helpers
messenger install --yes               # install everything that applies to the host
messenger install --helper dunstify   # install a specific helper
messenger install --dry-run           # print the install plan without executing
```

`messenger install` reuses sniff's install pipeline (Homebrew, apt, dnf, pacman, scoop, winget, PowerShellGet for `BurntToast`) and prints elevation badges for steps that need `sudo` or admin.

**Configuring helper preference:**

Each per-OS desktop config section accepts a `prefer_helpers` array that reorders the election. Names use the snake_case form of the helper (`dunstify`, `notify_send`, `terminal_notifier`, `alerter`, `snore_toast`, `burnt_toast`). Unknown names are ignored.

```json
{
  "routes": {
    "desk": {
      "provider": "desktop",
      "linux": { "prefer_helpers": ["dunstify", "notify_send"] },
      "macos": { "prefer_helpers": ["alerter"] },
      "windows": { "prefer_helpers": ["snore_toast"] }
    }
  }
}
```

The `MESSENGER_DESKTOP_PREFER_HELPERS` environment variable (comma-separated) overrides any value in the config file. Any helper not listed in `prefer_helpers` keeps its default order behind the listed entries.

**Capability notes:**

- `terminal-notifier` is notice-only; supplying `actions` or `reply` gives it a score of 0 so the backend picks `alerter` (or the native path) instead.
- `alerter` blocks until the user dismisses or activates the toast — only elected for interactive sends.
- `dunstify` only scores above 0 when the active D-Bus daemon is dunst.
- `snoretoast` requires a registered AppID; the backend auto-registers it via the configured Start Menu shortcut.
- `BurntToast` requires PowerShell and the `BurntToast` module; the backend installs nothing automatically — use `messenger install --helper burnt_toast`.

### Receipt Storage

Every successful send writes a JSON receipt to:

```
~/.messenger/receipts/<unix_ms>-<provider>.json
```

These receipts are used for reply threading via the `--reply-to` flag. Each receipt contains the route name and the full `SendReceipt` from the library, including the provider-typed `MessageRef` needed for replies.

---

## Library Configuration

The library does not read config files or environment variables. Callers construct provider configs directly and register the providers they need.

### Single Provider

```rust
use messenger::prelude::*;
use secrecy::SecretString;

#[tokio::main]
async fn main() -> Result<(), messenger::MessengerError> {
    let mut messenger = Messenger::new();

    // Register a Slack provider
    messenger.register(Box::new(SlackProvider::new(SlackConfig {
        bot_token: SecretString::from(std::env::var("SLACK_BOT_TOKEN").unwrap()),
        api_base_url: None,
    })));

    // Build a message and send it
    let message = Message::markdown("**Deploy succeeded**");
    let dispatch = Dispatch::to(Target::slack_channel("C012345ABC"));
    let receipt = messenger.send(dispatch, &message).await?;

    println!("Sent: {}", receipt.raw_id);
    Ok(())
}
```

This example is using the implemented Slack send path: authenticate with a bot token and select the destination with `Target::slack_channel(...)`.

### Multiple Providers

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

    messenger.register(Box::new(TelegramProvider::new(TelegramConfig {
        bot_token: SecretString::from(std::env::var("TELEGRAM_BOT_TOKEN").unwrap()),
        api_base_url: None,
    })));

    let message = Message::markdown("**Deploy succeeded**");

    // Fan out to multiple destinations
    let dispatches = vec![
        Dispatch::to(Target::slack_channel("C012345ABC")),
        Dispatch::to(Target::telegram_chat(
            messenger::target::TelegramChatId::Username("@ops".into()),
        )),
    ];

    let results = messenger.send_many(dispatches, &message).await;
    for result in results {
        match result {
            Ok(receipt) => println!("Sent to {:?}: {}", receipt.provider, receipt.raw_id),
            Err(err) => eprintln!("Failed: {err}"),
        }
    }

    Ok(())
}
```

### Inspecting Compatibility Before Sending

```rust
use messenger::prelude::*;

// plan_send validates and normalizes without sending
let plan = messenger.plan_send(dispatch, &message)?;

for warning in &plan.warnings {
    eprintln!("Warning: {warning}");
}

// Only send if acceptable
if plan.warnings.is_empty() {
    let receipt = messenger.send_planned(plan).await?;
}
```

### Discord Webhook Example

```rust
use messenger::prelude::*;
use secrecy::SecretString;

#[tokio::main]
async fn main() -> Result<(), messenger::MessengerError> {
    let mut messenger = Messenger::new();

    messenger.register(Box::new(DiscordWebhookProvider::try_new(DiscordWebhookConfig {
        webhook_url: SecretString::from(std::env::var("DISCORD_WEBHOOK_URL").unwrap()),
    })?));

    let message = Message::markdown("**Deploy succeeded**");

    // Post to the channel bound by the webhook URL.
    let dispatch = Dispatch::to(Target::discord_webhook());

    // Or route into a specific thread within that channel:
    // let dispatch = Dispatch::to(Target::discord_webhook_thread("1122334455"));

    let receipt = messenger.send(dispatch, &message).await?;
    println!("Sent: {}", receipt.raw_id);
    Ok(())
}
```

The webhook adapter does not support replies. Attempting `Dispatch::reply_to(...)` against a `Target::DiscordWebhook` causes `plan_send()`/`send()` to return `MessengerError::UnsupportedFeature` before any network call.
The webhook adapter uses the same Discord markdown renderer as the bot adapter, so Markdown formatting is identical between the two Discord transports.

### Ad-hoc CLI Usage

```bash
# Pass the full webhook URL as the target — it binds the channel and auth.
messenger send --provider discord-webhook \
  --channel "https://discord.com/api/v10/webhooks/123456789012345678/abc-token" \
  "Deploy succeeded"
```

### Provider Config Reference

| Provider | Config struct | Required fields |
|----------|-------------|-----------------|
| Discord | `DiscordConfig` | `bot_token: SecretString` |
| Discord (webhook) | `DiscordWebhookConfig` | `webhook_url: SecretString` |
| Slack | `SlackConfig` | `bot_token: SecretString` |
| Slack (webhook) | `SlackWebhookConfig` | `webhook_url: SecretString` |
| Signal | `SignalConfig` | `rpc_url: String`, `account: String` |
| WhatsApp | `WhatsAppConfig` | `access_token: SecretString`, `phone_number_id: String` |
| Telegram | `TelegramConfig` | `bot_token: SecretString` |
| Desktop | `DesktopConfig` | `app_name: String` (plus optional per-platform nested config) |

All config structs except `DiscordConfig`, `DiscordWebhookConfig`, `SlackWebhookConfig`, `SignalConfig`, and `DesktopConfig` have an optional `api_base_url: Option<String>` field for testing or proxy use. `WhatsAppConfig` also has an optional `api_version: Option<String>` (defaults to `v23.0`). `DiscordWebhookConfig` and `SlackWebhookConfig` bind their endpoints through the `webhook_url` itself; integration tests construct them through provider-specific test-only helpers that relax the runtime host validation so a wiremock server can stand in for the production host. `DesktopConfig` has no network endpoint — it captures `app_name`, default title/icon/category/urgency/timeout, and nested `WindowsDesktopConfig`, `MacOsDesktopConfig`, `LinuxDesktopConfig` blocks.

### Slack Webhook Library Example

```rust
use messenger::prelude::*;
use messenger::provider::slack_webhook::{SlackWebhookConfig, SlackWebhookProvider};
use secrecy::SecretString;

#[tokio::main]
async fn main() -> Result<(), messenger::MessengerError> {
    let mut messenger = Messenger::new();

    messenger.register(Box::new(SlackWebhookProvider::try_new(SlackWebhookConfig {
        webhook_url: SecretString::from(std::env::var("SLACK_WEBHOOK_URL").unwrap()),
    })?));

    let message = Message::markdown("**Deploy succeeded**");
    let dispatch = Dispatch::to(Target::slack_webhook());

    let receipt = messenger.send(dispatch, &message).await?;
    // raw_id is "" and message_ref.thread_ts is None for webhook sends;
    // successful delivery is confirmed via metadata["delivery_confirmed"] = "true".
    println!("Delivered: {}", receipt.metadata.get("delivery_confirmed").cloned().unwrap_or_default());
    Ok(())
}
```

`SlackWebhookProvider::try_new` enforces production URL rules (https scheme, `hooks.slack.com` host, `/services/{T}/{B}/{token}` path). Malformed URLs return `MessengerError::InvalidMessage`; missing configuration is surfaced by the CLI resolution layer as `MessengerError::MissingConfiguration`.

### Desktop Notification Library Example

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

    // Title-only sends are valid for desktop notifications.
    let message = Message::text("Build finished in 42s").title("Build");
    let dispatch = Dispatch::to(Target::desktop());

    let receipt = messenger.send(dispatch, &message).await?;
    println!(
        "delivered via {:?}: id={}",
        receipt.message_ref,
        receipt.raw_id,
    );
    Ok(())
}
```

`DesktopNotificationProvider::new` picks the runtime backend based on the compile target: `notify-rust` on Linux, AppleScript or native `UserNotifications.framework` on macOS, and `winrt-notification` on Windows. Platform-specific behavior — including the Windows setup prerequisite and the macOS strategy choice — is covered in [`docs/platforms/desktop.md`](./platforms/desktop.md).

Per-dispatch overrides (subtitle, category, urgency, timeout, icon, replace ID, app name) are supplied via `ProviderOverrides::Desktop(DesktopOverrides { .. })` on the `Dispatch`.

### Reading Helper Activations

When a desktop helper captures a user activation (action click, inline reply, dismissal, timeout, content click), the result is recorded on the `SendReceipt`. Decode it with [`SendReceipt::activation`] instead of reading metadata strings:

```rust
use messenger::prelude::*;

# fn _example(receipt: SendReceipt) {
match receipt.activation() {
    Some(Activation::Action(id))      => println!("user clicked action {id}"),
    Some(Activation::Reply(text))     => println!("user replied: {text}"),
    Some(Activation::Dismissed)       => println!("user dismissed"),
    Some(Activation::Timeout)         => println!("notification timed out"),
    Some(Activation::ContentClicked)  => println!("user clicked the body"),
    None                              => {} // notice-only or non-desktop send
}

if let Some(name) = receipt.helper_used() {
    println!("delivered via helper {name}");
}

if let Some(text) = receipt.reply_text() {
    println!("inline reply: {text}");
}
# }
```

`helper_used()` returns `None` when the native backend handled the send — callers can use this to detect whether a fallback occurred without inspecting metadata directly. `reply_text()` is a convenience for the `Activation::Reply` arm and returns `None` for any other activation type.
