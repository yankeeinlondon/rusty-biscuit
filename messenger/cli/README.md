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

# Send a local desktop notification (no --channel needed)
messenger send --provider desktop --title "Build" "Green across the board"
messenger send --provider desktop --title "Deploy" --urgency critical --timeout-ms 10000 "Rollback required"

# Reply using a saved receipt
messenger send \
  --route slack.ops \
  --reply-to ~/.messenger/receipts/1712345678000-slack.json \
  "Acknowledged"
```

Supported `send` flags:

- `--provider <discord|discord-webhook|slack|slack-webhook|signal|whatsapp|telegram|desktop>`
- `--channel <target>` (required for every provider except `desktop`)
- `--route <name>`
- `--reply-to <path-or-json>`
- `--image <path>`
- `--file <path>`
- `--silent`
- `--strict`
- `--plain`
- `--location <LAT,LON>`
- `--title <text>` (desktop notifications)
- `--subtitle <text>` (desktop notifications)
- `--icon <name-or-path>` (desktop notifications)
- `--category <name>` (desktop notifications)
- `--urgency <low|normal|critical>` (desktop notifications)
- `--timeout-ms <ms>` (desktop notifications)

## Route Resolution

The CLI resolves the delivery target in this order:

1. `--provider` plus `--channel`
2. `--route`
3. `default_route` from `~/.messenger.json`

If you use `--provider`, you must also pass `--channel` for every provider except `desktop`. Desktop notifications deliver to the host OS notification center, so no target identifier is needed — the CLI rejects `--provider desktop --channel <anything>` with a validation error.

Two provider-specific route behaviors are worth calling out:

- Signal targets starting with `+` are treated as direct recipients. Any other value is treated as a Signal group ID.
- Telegram targets are parsed as numeric chat IDs when possible, otherwise they are treated as usernames such as `@ops`.

For clarity, the CLI is routing against the provider API models implemented in the library. Discord (bot) and Slack (bot) routes identify a channel plus bot token; Discord-webhook and Slack-webhook routes carry only a webhook URL (no `channel_id`); Signal and WhatsApp routes identify a recipient plus the provider-specific API credentials.

For ad-hoc webhook sends, pass the full webhook URL as `--channel`:

```bash
messenger send --provider discord-webhook \
  --channel "https://discord.com/api/v10/webhooks/123456789012345678/abc-token" \
  "Deploy succeeded"

messenger send --provider slack-webhook \
  --channel "https://hooks.slack.com/services/T000/B000/XXXXX" \
  "Deploy succeeded"
```

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

- Discord (bot): `channel_id`, optional `bot_token`, `bot_token_env`
- Discord (webhook): `provider: "discord-webhook"`, optional `webhook_url`, `webhook_url_env`
- Slack (bot): `channel_id`, optional `bot_token`, `bot_token_env`
- Slack (webhook): `provider: "slack-webhook"`, optional `webhook_url`, `webhook_url_env`
- Signal: `recipient`, optional `rpc_url`, `rpc_url_env`, optional `account`, `account_env`
- WhatsApp: `recipient`, optional `access_token`, `access_token_env`, optional `phone_number_id`, `phone_number_id_env`
- Telegram: `chat_id`, optional `bot_token`, `bot_token_env`
- Desktop: `provider: "desktop"`, `app_name`, optional `default_title`, `icon`, `category`, `urgency` (`low|normal|critical`), `timeout_ms`, and nested `windows`, `macos`, `linux` blocks

Example desktop route:

```json
{
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
```

Desktop routes store no secrets. All three platform blocks are optional; unset fields fall back to the library defaults.

When both an inline secret and an environment variable name are present, the inline secret wins.

Default environment variable names:

- `DISCORD_BOT_TOKEN`
- `DISCORD_WEBHOOK_URL`
- `SLACK_BOT_TOKEN`
- `SLACK_WEBHOOK_URL`
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

### Desktop Setup

`messenger setup desktop` walks through the portable fields (app name, default title, icon, category, urgency, timeout) followed by platform-specific branches:

- **Windows** — optional App User Model ID (defaults to `RustyBiscuit.Messenger`). On completion, the CLI registers the Start Menu shortcut under `%APPDATA%\Microsoft\Windows\Start Menu\Programs\<app-id>.lnk`, pointing at the currently running `messenger` executable, and prints the absolute shortcut path. If shortcut creation fails, setup exits with a remediation message; `send` will continue to return `MessengerError::MissingConfiguration` until the shortcut exists.
- **macOS** — optional bundle ID plus a strategy choice. `auto` (the default) maps to AppleScript delivery and never triggers a system notification authorization prompt; `native_user_notifications` opts in to the native framework and requires a bundled, signed app identity.
- **Linux** — optional `desktop_entry` value used as the D-Bus `desktop-entry` hint.

`send` never modifies the host filesystem outside `~/.messenger/`. All shortcut/AUMID registration is strictly setup-time work.

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

- The Discord adapters (bot and webhook) accept the full set of attachment kinds. The Desktop adapter accepts image attachments only; multiple images collapse to the first in best-effort mode. Slack (bot and webhook), Signal, WhatsApp, and Telegram accept no attachments — they drop them in best-effort mode or fail in strict mode.
- The Discord-webhook adapter does not support replies. `--reply-to` against a `discord-webhook` route returns `MessengerError::UnsupportedFeature { feature: "replies" }` at plan time — before any network call.
- The Slack-webhook adapter supports replies via `thread_ts`, but webhook sends do not return a message ID of their own. Webhook receipts have an empty `raw_id` and `message_ref.thread_ts == None`, so a webhook receipt on its own cannot be used to reply to a webhook send.
- Signal, WhatsApp, and Desktop fall back to plain text when you send Markdown.
- Telegram and WhatsApp send native location messages. If you pass both body text and `--location`, the location is sent and the text body is ignored.
- Telegram and Desktop support silent delivery via `--silent`.
- Desktop notifications accept a title-only send (`--title "..."` with no message body). Locations and non-image attachments are dropped in best-effort mode and error in strict mode.

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
