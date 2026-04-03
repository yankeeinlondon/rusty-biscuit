# Messaging in Claudine

When executing long-running non-interactive sessions, being able to communicate progress over multiple channels (not just STDOUT) is a superpower. Fortunately Claudine already leverages the `biscuit-speaks` (TTS) and  `playa` (Audio playback and Effects) libraries to produce TTS messages and Sound Effects.

In this feature we will add messaging to apps like:

- Discord
- Slack
- Signal
- WhatsApp

This will be done via the `messenger` library in this monorepo.

Read these two documents for context on what the `messenger` library provides:

- [User Guide to Messaging](@messenger/docs/user-guide.md) 
- [README](@messenger/README.md)

Also use the 'messenger' skill.

## Configuration

- Configuration supports both **User scope** (`~/.claudine/config.json`) and **Repo scope** (`.claudine/config.json`)
- Uses a **named-config pattern** with an `active` selector:
    - Multiple provider configurations are stored under a `configs` map, keyed by user-chosen names
    - An `active` field selects which named config is currently in use
    - Setting `active` to `null` or omitting it disables messaging for that scope
- Each named config specifies a `provider` field (e.g., `"slack"`, `"discord"`, `"signal"`, `"whatsapp"`) plus provider-specific fields wrapped in a `MessageProvider` enum
- **Scope resolution**: Repo-scope `active` config overrides User-scope when present; User-scope is used as fallback

```json
// Example: settings.messaging in config.json
"messaging": {
  "active": "work-slack",
  "configs": {
    "work-slack": {
      "provider": "slack",
      "channel_id": "C012345ABC",
      "bot_token_env": "SLACK_BOT_TOKEN"
    },
    "personal-discord": {
      "provider": "discord",
      "channel_id": "123456789012345678",
      "bot_token_env": "DISCORD_BOT_TOKEN"
    }
  }
}
```

> Secrets can be provided inline (`bot_token`) or via environment variable name (`bot_token_env`). Inline values take precedence. Default env var names follow the `messenger` CLI conventions.

## Initialization

No initial configuration will be provided by the `init` command. We will change this in the future.

No default message templates are pre-defined per event type. Users manually add `message` actions to their event bindings as needed (unlike TTS, which ships with default templates).

## Usage

We add a new `Message` variant to `HookAction`, following the same pattern as `Speak`:

- Sends a message to Claudine's configured chat/message app (Repo-scope when defined, User-scope as fallback)
- **Fire-and-forget** execution: non-blocking, errors logged as warnings (consistent with `Speak` and `SoundEffect`)
- **Template interpolation**: Reuses the existing `interpolate()` function and `EventMeta` fields (same variables available as TTS: `tool_name`, `event`, `provider`, etc.)
- Format: `{ "type": "message", "message": "{template}", "image": "{path}" }`

  | Field     | Type     | Default    | Description                                       |
  |-------    |------    |---------   |-------------                                      |
  | `message` | `string` | (required) | Template message with `{{variable}}` placeholders (Handlebars-style, interpolated via `EventMeta`) |
  | `image`   | `string` \| omitted | omitted | Filepath to a raster image; sent as attachment if the messaging platform supports it, otherwise ignored |

