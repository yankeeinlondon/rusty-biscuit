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
| `--image <path>` | Attach an image (Discord only) |
| `--file <path>` | Attach a file (Discord only) |
| `--silent` | Suppress notification sound (Telegram only) |
| `--strict` | Error on unsupported features |
| `--plain` | Force plain text (disable Markdown) |
| `--location <LAT,LON>` | Attach location payload |

## Route Resolution Order

1. `--provider` + `--channel` (ad-hoc)
2. `--route` (named route from config)
3. `default_route` from `~/.messenger.json`

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
    }
  }
}
```

### Route Shapes by Provider

**Discord / Slack**: `channel_id`, optional `bot_token` / `bot_token_env`
**Signal**: `recipient`, optional `rpc_url` / `rpc_url_env`, optional `account` / `account_env`
**WhatsApp**: `recipient`, optional `access_token` / `access_token_env`, optional `phone_number_id` / `phone_number_id_env`
**Telegram**: `chat_id`, optional `bot_token` / `bot_token_env`

When both an inline secret and env var name are present, the inline value wins.

### Default Environment Variables

| Provider | Variable |
|----------|----------|
| Discord | `DISCORD_BOT_TOKEN` |
| Slack | `SLACK_BOT_TOKEN` |
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

## Interactive Setup

`messenger setup [provider]` prompts for provider-specific configuration:
- If no provider is passed, prompts for selection
- Offers `Exit`, `Add another`, or `Modify "<route>"` when routes exist
- Route names: `provider`, `provider.2`, `provider.3`, ...
- First route becomes `default_route`; replacing existing default is opt-in

## Key CLI Crates

- `clap` + `clap_complete` for argument parsing and dynamic completions
- `inquire` for interactive setup prompts
- `color-eyre` for styled error output
- `biscuit-terminal` for styled setup text
- `dirs` for config and receipt directory resolution
