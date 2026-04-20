# CLI Reference

## Commands

```bash
messenger send <message> [options]   # Send a message
messenger setup [provider]           # Interactive route configuration
messenger init [provider]            # Alias for setup
messenger completions                # Print shell completion instructions
```

## Send Options

| Flag | Purpose |
|------|---------|
| `--provider <name>` | Target provider (requires `--channel`) |
| `--channel <id>` | Target channel/recipient |
| `--route <name>` | Named route from config |
| `--reply-to <path-or-json>` | Reply to a previous message |
| `--image <path>` | Attach an image (Discord / Discord-Webhook only) |
| `--file <path>` | Attach a file (Discord / Discord-Webhook only) |
| `--silent` | Suppress notification sound (Telegram only) |
| `--strict` | Error on unsupported features |
| `--plain` | Force plain text (disable Markdown) |
| `--location <LAT,LON>` | Attach location payload |

Valid `--provider` values: `discord`, `discord-webhook`, `slack`, `slack-webhook`, `signal`, `whatsapp`, `telegram`.

## Route Resolution Order

1. `--provider` + `--channel` (ad-hoc) — for webhook providers (`discord-webhook`, `slack-webhook`), pass the full webhook URL as `--channel`
2. `--route` (named route from config)
3. `default_route` from `~/.messenger.json`

The CLI does not currently expose a Discord webhook thread-routing flag. If you need `thread_id`, build the dispatch in library code with `Target::discord_webhook_thread(...)`.

## Configuration

**File**: `~/.messenger.json`

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
    }
  }
}
```

### Route Shapes by Provider

**Discord (bot) / Slack (bot)**: `channel_id`, optional `bot_token` / `bot_token_env`
**Discord (webhook)**: `provider: "discord-webhook"`, optional `webhook_url` / `webhook_url_env`
**Slack (webhook)**: `provider: "slack-webhook"`, optional `webhook_url` / `webhook_url_env`
**Signal**: `recipient`, optional `rpc_url` / `rpc_url_env`, optional `account` / `account_env`
**WhatsApp**: `recipient`, optional `access_token` / `access_token_env`, optional `phone_number_id` / `phone_number_id_env`
**Telegram**: `chat_id`, optional `bot_token` / `bot_token_env`

When both an inline secret and env var name are present, the inline value wins. Webhook routes (Discord and Slack) intentionally have no `channel_id` because the webhook URL already binds the destination channel.

For Slack webhook routes, an empty or whitespace-only `webhook_url` is treated as absent: the CLI falls back to the resolved `webhook_url_env` value (defaulting to `SLACK_WEBHOOK_URL` when the env-var name is omitted).

### Default Environment Variables

| Provider | Variable |
|----------|----------|
| Discord | `DISCORD_BOT_TOKEN` |
| Discord-Webhook | `DISCORD_WEBHOOK_URL` |
| Slack | `SLACK_BOT_TOKEN` |
| Slack-Webhook | `SLACK_WEBHOOK_URL` |
| Signal | `SIGNAL_RPC_URL`, `SIGNAL_ACCOUNT` |
| WhatsApp | `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID` |
| Telegram | `TELEGRAM_BOT_TOKEN` |

### Target Parsing

- **Signal**: Targets starting with `+` are direct recipients; others are group IDs
- **Telegram**: Numeric values are chat IDs; others are usernames (e.g., `@ops`)

## Receipts and Replies

Every send writes a JSON receipt to `~/.messenger/receipts/<unix_ms>-<provider>.json`.

`--reply-to` accepts:
- A saved receipt file path
- A raw `SendReceipt` JSON blob
- A raw `MessageRef` JSON blob

Slack webhook receipts carry `raw_id == ""` and `message_ref.thread_ts == None`, so they cannot be used on their own as the `--reply-to` target of a later send. To reply into a Slack thread from a webhook route, point `--reply-to` at a Slack bot receipt (or any other source that surfaces a Slack `thread_ts`).

## Interactive Setup

`messenger setup [provider]` prompts for provider-specific configuration:
- If no provider is passed, prompts for selection
- Offers `Exit`, `Add another`, or `Modify "<route>"` when routes exist
- Route names: `provider`, `provider.2`, `provider.3`, ...
- First route becomes `default_route`; replacing existing default is opt-in
- `discord-webhook` setup prompts for either a literal webhook URL or an environment variable name, defaulting to `DISCORD_WEBHOOK_URL`
- `slack-webhook` setup prompts for either a literal webhook URL (masked input, empty rejected before the route is written) or an environment variable name, defaulting to `SLACK_WEBHOOK_URL`

## Key CLI Crates

- `clap` + `clap_complete` for argument parsing and dynamic completions
- `inquire` for interactive setup prompts
- `color-eyre` for styled error output
- `biscuit-terminal` for styled setup text
- `dirs` for config and receipt directory resolution
