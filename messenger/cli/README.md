# messenger-cli

`messenger-cli` installs the `messenger` binary: a thin operator-facing layer over the `messenger` library with route configuration, receipt storage, and interactive setup.

## Commands

The CLI currently exposes four subcommands:

```bash
messenger send <message> [options]
messenger setup [provider]
messenger init [provider]
messenger completions
```

`init` is an alias for `setup`.

`completions` prints setup instructions only. Actual shell completion is driven dynamically through the `COMPLETE` environment variable.

## Send Examples

```bash
# Send to the configured default route
messenger send "Deploy finished"

# Send to a named route
messenger send --route slack.ops "Deploy finished"

# Send to an ad-hoc route
messenger send --provider slack --channel C012345ABC "Deploy finished"

# Force plain text instead of Markdown
messenger send --route telegram.ops --plain "No formatting"

# Attach files from disk
messenger send --route discord.alerts --image ./status.png "Screenshot attached"
messenger send --route discord.alerts --file ./build.log "Build output"

# Add a location payload
messenger send --route telegram.ops --location "34.05,-118.24" "Ignored on Telegram"

# Reply using a saved receipt
messenger send \
  --route slack.ops \
  --reply-to ~/.messenger/receipts/1712345678000-slack.json \
  "Acknowledged"
```

Supported `send` flags:

- `--provider <discord|slack|signal|whatsapp|telegram>`
- `--channel <target>`
- `--route <name>`
- `--reply-to <path-or-json>`
- `--image <path>`
- `--file <path>`
- `--silent`
- `--strict`
- `--plain`
- `--location <LAT,LON>`

## Route Resolution

The CLI resolves the delivery target in this order:

1. `--provider` plus `--channel`
2. `--route`
3. `default_route` from `~/.messenger.json`

If you use `--provider`, you must also pass `--channel`.

Two provider-specific route behaviors are worth calling out:

- Signal targets starting with `+` are treated as direct recipients. Any other value is treated as a Signal group ID.
- Telegram targets are parsed as numeric chat IDs when possible, otherwise they are treated as usernames such as `@ops`.

For clarity, the CLI is routing against the provider API models implemented in the library, not webhook URLs. Discord and Slack routes identify a channel plus bot token, while Signal and WhatsApp routes identify a recipient plus the provider-specific API credentials.

## Configuration

The config file lives at `~/.messenger.json`.

Top-level shape:

```json
{
  "default_route": "slack.ops",
  "routes": {
    "slack.ops": {
      "provider": "slack",
      "channel_id": "C012345ABC",
      "bot_token_env": "SLACK_BOT_TOKEN"
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
    }
  }
}
```

Supported route shapes:

- Discord: `channel_id`, optional `bot_token`, `bot_token_env`
- Slack: `channel_id`, optional `bot_token`, `bot_token_env`
- Signal: `recipient`, optional `rpc_url`, `rpc_url_env`, optional `account`, `account_env`
- WhatsApp: `recipient`, optional `access_token`, `access_token_env`, optional `phone_number_id`, `phone_number_id_env`
- Telegram: `chat_id`, optional `bot_token`, `bot_token_env`

When both an inline secret and an environment variable name are present, the inline secret wins.

Default environment variable names:

- `DISCORD_BOT_TOKEN`
- `SLACK_BOT_TOKEN`
- `SIGNAL_RPC_URL`
- `SIGNAL_ACCOUNT`
- `WHATSAPP_ACCESS_TOKEN`
- `WHATSAPP_PHONE_NUMBER_ID`
- `TELEGRAM_BOT_TOKEN`

The CLI still reads the older legacy route shape `{ "provider": "...", "channel_id": "...", "token_env": "..." }`, but it writes the typed provider-specific format shown above.

## Interactive Setup

`messenger setup` walks through provider-specific prompts and writes `~/.messenger.json`.

Examples:

```bash
messenger setup
messenger setup slack
messenger init telegram
```

Behavior to expect:

- If no provider is passed, setup prompts you to choose one.
- If routes already exist for that provider, setup offers `Exit`, `Add another`, or `Modify "<route>"`.
- Suggested route names start at `provider` and then increment as `provider.2`, `provider.3`, and so on.
- The first configured route defaults to becoming `default_route`; replacing an existing default is opt-in.
- Ctrl+C or prompt cancellation exits cleanly without writing partial state.

## Receipts And Replies

Every successful send writes a JSON receipt to:

```text
~/.messenger/receipts/<unix_ms>-<provider>.json
```

The stored payload includes:

- `route_name`
- the full `messenger::SendReceipt`

After a send, the CLI prints the provider, raw message ID, and receipt path on `stderr`.

`--reply-to` accepts three forms:

- a saved receipt file path
- a raw `SendReceipt` JSON blob
- a raw `MessageRef` JSON blob

That makes follow-up sends easy without manually extracting provider-specific reply identifiers.

## Compatibility Behavior

By default, sends use best-effort compatibility:

- unsupported features are dropped when possible
- compatibility warnings are printed before the send

`--strict` switches the underlying `Dispatch` to strict mode and turns those compatibility problems into errors.

Some practical examples:

- Only Discord supports attachments today. Slack, Signal, WhatsApp, and Telegram drop them in best-effort mode or fail in strict mode.
- Signal and WhatsApp fall back to plain text when you send Markdown.
- Telegram and WhatsApp send native location messages. If you pass both body text and `--location`, the location is sent and the text body is ignored.
- Telegram is the only CLI-exposed provider that supports silent delivery.

## Shell Completions

The binary uses dynamic completions via `clap_complete`.

```bash
messenger completions
COMPLETE=zsh messenger
COMPLETE=fish messenger | source
```

## Development

Run from [`messenger/`](../):

```bash
just build
just test
just lint
```

## Key Crates

- `clap` and `clap_complete` for command parsing and dynamic completions
- `inquire` for interactive setup
- `color-eyre` for CLI-facing error reporting
- `dirs` for config and receipt locations
- `biscuit-terminal` for styled setup output

## Lessons Learned

- Saving typed receipts is simpler than inventing a CLI-only reply abstraction; the library already exposes the right shape.
- Provider-specific config structs keep the JSON explicit and avoid the drift that usually appears around “generic” route schemas.
- A small interactive setup flow is enough to hide most credential and ID boilerplate without obscuring how routes are stored.
