# Messenger User Guide

This guide walks through setting up each messaging provider, configuring the CLI, and using the library from Rust code.

## Platform Setup

Each provider requires credentials and a destination identifier. The sections below explain what you need, where to get it, and what the setup looks like for both the CLI and the library.

### Discord

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

### Slack

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

## CLI Configuration

### Config File Location

The CLI stores its configuration at `~/.messenger.json`. The `messenger setup` command creates and updates this file interactively.

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
    "discord.alerts": {
      "provider": "discord",
      "channel_id": "123456789012345678",
      "bot_token_env": "DISCORD_BOT_TOKEN"
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
| Slack | `provider`, `channel_id` | `bot_token`, `bot_token_env` |
| Signal | `provider`, `recipient` | `rpc_url`, `rpc_url_env`, `account`, `account_env` |
| WhatsApp | `provider`, `recipient` | `access_token`, `access_token_env`, `phone_number_id`, `phone_number_id_env` |
| Telegram | `provider`, `chat_id` | `bot_token`, `bot_token_env` |

### Secret Resolution

Each secret can be provided in two ways:

1. **Inline** — stored directly in the config (e.g., `"bot_token": "xoxb-..."`)
2. **Environment variable** — referenced by name (e.g., `"bot_token_env": "SLACK_BOT_TOKEN"`)

When both are present, the inline value takes priority. If neither is set, the CLI falls back to a default environment variable name per provider (see the table in [Platform Setup](#platform-setup)).

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

### Provider Config Reference

| Provider | Config struct | Required fields |
|----------|-------------|-----------------|
| Discord | `DiscordConfig` | `bot_token: SecretString` |
| Slack | `SlackConfig` | `bot_token: SecretString` |
| Signal | `SignalConfig` | `rpc_url: String`, `account: String` |
| WhatsApp | `WhatsAppConfig` | `access_token: SecretString`, `phone_number_id: String` |
| Telegram | `TelegramConfig` | `bot_token: SecretString` |

All config structs except `DiscordConfig` and `SignalConfig` have an optional `api_base_url: Option<String>` field for testing or proxy use. `WhatsAppConfig` also has an optional `api_version: Option<String>` (defaults to `v23.0`).
