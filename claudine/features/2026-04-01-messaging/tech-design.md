# Messaging Tech Design

This document defines the implementation-ready technical design for the messaging feature described in:

- `claudine/features/2026-04-01-messaging/spec.md`
- `messenger/README.md`
- `messenger/docs/user-guide.md`
- the current hook action/config/dispatch flow in `claudine/lib/src/actions/`, `claudine/lib/src/events/config.rs`, `claudine/lib/src/dispatch/loader.rs`, and `claudine/lib/src/dispatch/runner.rs`

The goal is to add outbound chat notifications to Claudine's hook system without breaking the current fire-and-forget action model or the existing user-scope/repo-scope configuration strategy.

## Summary

Claudine already supports fire-and-forget side effects through `Speak`, `SoundEffect`, `Log`, and `FireAndForget`. This feature adds a seventh action, `Message`, which sends an interpolated message to one configured messaging destination through the `messenger` library.

This design explicitly adopts the transport models already implemented in `messenger`:

- Discord: bot token + channel ID
- Slack: bot token + channel ID
- Signal: JSON-RPC URL + account + recipient/group ID
- WhatsApp: access token + phone number ID + recipient

Slack and Discord incoming webhook URLs are out of scope for this implementation.

The recommended implementation shape is:

1. add a `Message` variant to `HookAction`
2. add scope-local messaging settings to `GlobalSettings`
3. preserve user/repo messaging scope separately in `RuntimeConfig` so repo fallback semantics remain exact
4. add a small `claudine::messaging` module that:
   - resolves the effective active config
   - resolves inline-vs-env secrets
   - builds a `messenger::Messenger` with exactly one registered provider
   - builds the `Dispatch` and `Message`
   - sends asynchronously and logs warnings/errors
5. invoke that helper from `dispatch::runner` using the same fire-and-forget behavior as `Speak`

## Goals

1. Add a `message` hook action that can be attached to any event binding.
2. Support user-scope and repo-scope messaging configuration in `.claudine/config.json`.
3. Use the existing `interpolate()` template engine and `EventMeta` variables.
4. Keep send behavior non-blocking and warning-only on failure.
5. Support the four providers named in the spec: Discord, Slack, Signal, and WhatsApp.
6. Support an optional raster image attachment field on the action.

## Non-Goals

1. No `claudine init` wizard support in v1.
2. No default message templates per event.
3. No inbound messaging, reply threading, receipt persistence, or retries.
4. No Telegram support in Claudine v1, even though `messenger` supports it.
5. No multi-route fan-out; one active route is used at a time.
6. No new Markdown/template language beyond the existing string interpolation system.

## Spec Review And Clarifications

The current spec is directionally correct, but a few details need to be resolved before implementation.

### 1. Slack/Discord webhook examples do not match `messenger`

This is the largest mismatch.

The spec examples currently use:

```json
{
  "provider": "slack",
  "webhook_url": "https://hooks.slack.com/..."
}
```

and:

```json
{
  "provider": "discord",
  "webhook_url": "https://discord.com/api/webhooks/..."
}
```

That does not match the current `messenger` library. Today:

- Slack uses bot token + channel ID
- Discord uses bot token + channel ID
- Signal uses JSON-RPC URL + account + recipient
- WhatsApp uses access token + phone number ID + recipient

Recommendation:

1. Claudine v1 should follow the existing `messenger` library model rather than inventing a second transport model.
2. The spec examples should be updated accordingly.
3. Webhook-based Slack/Discord delivery can be a future `messenger` enhancement, not part of this feature.

Decision for this design:

1. implement only the bot-token-plus-channel or credential-plus-recipient models that `messenger` already exposes
2. do not add webhook-specific config fields to Claudine v1

### 2. Repo-scoped configs need env-backed secrets

The spec allows repo-scoped messaging config, but inline secrets in repo config are a bad default. The `messenger` CLI already solves this with direct-value plus `*_env` fallback fields.

Recommendation:

1. Reuse the `messenger` CLI pattern for provider secrets.
2. Allow either an inline secret or an env var name.
3. Keep default env var names aligned with `messenger`:
   - `DISCORD_BOT_TOKEN`
   - `SLACK_BOT_TOKEN`
   - `SIGNAL_RPC_URL`
   - `SIGNAL_ACCOUNT`
   - `WHATSAPP_ACCESS_TOKEN`
   - `WHATSAPP_PHONE_NUMBER_ID`

This is the smallest viable design that keeps repo config usable without committing credentials.

### 3. Scope fallback must preserve user and repo scopes separately

The current loader merges `settings` field-by-field and loses scope provenance. That is fine for `tts`, `linking`, and `protect`, but it is not sufficient for messaging.

Example problem:

1. user scope sets `active = "ops"`
2. repo scope defines another config also named `ops`
3. repo scope leaves `active = null`

The correct runtime behavior is to fall back to the user's active `ops`, not silently select the repo's `ops`.

Recommendation:

1. Keep messaging scope-local in the serialized config.
2. Preserve user and repo messaging settings separately in `RuntimeConfig`.
3. Resolve the effective route at send time using the two preserved scopes.

### 4. Action message content should be treated as Markdown

The spec says `message` is a string template but does not say whether it is plain text or rich text. Since the `messenger` library is designed around Markdown rendering and graceful plain-text fallback, Markdown should be the default.

Recommendation:

1. Treat interpolated `message` text as Markdown.
2. Let `messenger` render rich text on Discord and Slack.
3. Let `messenger` fall back to plain text on Signal and WhatsApp.

### 5. `image` should be provider-aware before validation

Only Discord supports attachments today in `messenger`. If Claudine blindly builds an attachment for every provider, `messenger` path validation may fail even when the target provider would have ignored attachments anyway.

Recommendation:

1. Resolve the target provider first.
2. Only attach the `image` when the target provider supports attachments in Claudine v1.
3. In practice, that means only Discord gets the attachment in v1.
4. For Slack, Signal, and WhatsApp, log a warning that the image is being ignored and send the text message normally.

### 6. Missing active config should be a config validation error

If `active = "work-slack"` but `configs.work-slack` is missing, that is a configuration error, not a runtime warning.

Recommendation:

1. Validate this in `HookerConfig::validate()` on each raw scope config before merging.

## Current Baseline

Relevant existing behavior:

1. `HookAction` currently supports `SoundEffect`, `Speak`, `Log`, `FireAndForget`, `Call`, and `Report`.
2. `dispatch::runner::execute_actions()` executes actions in declaration order and already treats `Speak` and `SoundEffect` as fire-and-forget.
3. `GlobalSettings` currently holds `default_log_target`, `tts`, `linking`, and `protect`.
4. `dispatch::loader::merge_configs()` merges settings field-by-field and repo provider bindings replace user provider bindings.
5. `messenger` already provides:
   - portable `Message` and `Attachment`
   - provider-specific `Target`
   - `Messenger::plan_send()` plus `send_planned()`
   - warning-based compatibility normalization

Current gaps:

1. no `message` action type
2. no messaging config in Claudine
3. no scope-aware messaging resolution in the runtime loader
4. no helper that maps Claudine config into `messenger` provider configs and targets

## Recommended Module Layout

Add a new module:

```txt
claudine/lib/src/
├── messaging/
│   ├── mod.rs
│   ├── config.rs
│   ├── resolve.rs
│   └── send.rs
```

Responsibilities:

1. `config.rs`
   - provider config enums and secret reference fields
   - semantic validation helpers
2. `resolve.rs`
   - user/repo effective route resolution
   - env secret resolution
   - Signal recipient parsing
3. `send.rs`
   - build `messenger::Messenger`
   - build `messenger::Dispatch`
   - build `messenger::Message`
   - fire-and-forget async send helper
4. `mod.rs`
   - public re-exports and top-level entry points used by the dispatcher

Existing modules that change:

```txt
claudine/lib/src/
├── actions/hook_action.rs
├── actions/mod.rs
├── dispatch/loader.rs
├── dispatch/runner.rs
├── events/config.rs
└── lib.rs
```

## Data Model

### Hook action

Add a new variant:

```rust
pub enum HookAction {
    // existing variants...
    Message {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<String>,
    },
}
```

Behavior:

1. `message` is required.
2. `image` is optional.
3. both fields support existing Handlebars-style interpolation.
4. empty interpolated `message` plus absent/empty `image` means the action is skipped.

### Scope-local serialized settings

Add a messaging field to `GlobalSettings`:

```rust
pub struct GlobalSettings {
    pub default_log_target: Option<LogTarget>,
    pub tts: Option<TtsSettings>,
    pub linking: Option<LinkingSettings>,
    pub protect: Option<ProtectConfig>,
    pub messaging: Option<ScopedMessagingSettings>,
}
```

`ScopedMessagingSettings`:

```rust
pub struct ScopedMessagingSettings {
    pub active: Option<String>,
    #[serde(default)]
    pub configs: HashMap<String, MessagingRouteConfig>,
}
```

Provider configs:

```rust
#[serde(tag = "provider", rename_all = "lowercase", deny_unknown_fields)]
pub enum MessagingRouteConfig {
    Discord {
        channel_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        #[serde(default = "default_discord_token_env")]
        bot_token_env: String,
    },
    Slack {
        channel_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        #[serde(default = "default_slack_token_env")]
        bot_token_env: String,
    },
    Signal {
        recipient: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rpc_url: Option<String>,
        #[serde(default = "default_signal_rpc_url_env")]
        rpc_url_env: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account: Option<String>,
        #[serde(default = "default_signal_account_env")]
        account_env: String,
    },
    WhatsApp {
        recipient: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        access_token: Option<String>,
        #[serde(default = "default_whatsapp_access_token_env")]
        access_token_env: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phone_number_id: Option<String>,
        #[serde(default = "default_whatsapp_phone_number_id_env")]
        phone_number_id_env: String,
    },
}
```

This deliberately mirrors the `messenger` CLI route model, minus Telegram.

### Runtime messaging settings

Do not try to encode exact fallback behavior in merged `GlobalSettings`. Preserve scopes separately in `RuntimeConfig`:

```rust
pub struct RuntimeMessagingSettings {
    pub user: Option<ScopedMessagingSettings>,
    pub repo: Option<ScopedMessagingSettings>,
}

pub struct RuntimeConfig {
    settings: GlobalSettings,
    messaging: RuntimeMessagingSettings,
    providers: HashMap<Provider, RuntimeProviderConfig>,
}
```

Expose:

```rust
impl RuntimeConfig {
    pub fn messaging(&self) -> &RuntimeMessagingSettings;
}
```

`settings()` can remain unchanged for the existing fields.

## Effective Resolution Rules

Route selection should follow this exact order:

1. if repo scope exists and `repo.active` is `Some(name)`, use `repo.configs[name]`
2. otherwise, if user scope exists and `user.active` is `Some(name)`, use `user.configs[name]`
3. otherwise, messaging is disabled for this event

Notes:

1. repo `active = null` does not erase user fallback; it disables only repo selection
2. invalid active names are rejected at config validation time
3. repo and user config maps are never merged for route selection

The resolved route should carry scope metadata for diagnostics:

```rust
pub enum MessagingScope {
    User,
    Repo,
}

pub struct ResolvedMessagingRoute {
    pub scope: MessagingScope,
    pub name: String,
    pub config: MessagingRouteConfig,
}
```

## Provider Mapping

### Discord

Config:

```json
{
  "provider": "discord",
  "channel_id": "123456789012345678",
  "bot_token_env": "DISCORD_BOT_TOKEN"
}
```

Maps to:

- `messenger::provider::discord::DiscordConfig`
- `messenger::Target::discord_channel(channel_id)`

### Slack

Config:

```json
{
  "provider": "slack",
  "channel_id": "C012345ABC",
  "bot_token_env": "SLACK_BOT_TOKEN"
}
```

Maps to:

- `messenger::provider::slack::SlackConfig`
- `messenger::Target::slack_channel(channel_id)`

### Signal

Config:

```json
{
  "provider": "signal",
  "recipient": "+15551234567",
  "rpc_url_env": "SIGNAL_RPC_URL",
  "account_env": "SIGNAL_ACCOUNT"
}
```

Maps to:

- `messenger::provider::signal::SignalConfig`
- `messenger::Target`

Recipient resolution should follow the existing `messenger` CLI behavior:

1. if `recipient` starts with `+`, treat it as a direct phone recipient
2. otherwise, treat it as a Signal group ID

This keeps v1 aligned with current repo conventions and avoids inventing a second Signal route grammar.

### WhatsApp

Config:

```json
{
  "provider": "whatsapp",
  "recipient": "+15559876543",
  "access_token_env": "WHATSAPP_ACCESS_TOKEN",
  "phone_number_id_env": "WHATSAPP_PHONE_NUMBER_ID"
}
```

Maps to:

- `messenger::provider::whatsapp::WhatsAppConfig`
- `messenger::Target::whatsapp_recipient(recipient)`

## Secret Resolution

Secret resolution should exactly follow the `messenger` CLI rule:

1. if the inline field is present, use it
2. otherwise, look up the configured env var name
3. if neither produces a value, log a warning and drop the send

Runtime send failure should not crash hook dispatch, so missing env vars become warning logs at send time, not fatal dispatch errors.

Example helper:

```rust
fn resolve_secret(value: Option<&str>, env_name: &str) -> std::result::Result<String, String>
```

The error string should say both what is missing and which env var Claudine tried to read.

## Dispatch Execution Flow

Recommended action flow inside `dispatch::runner`:

```rust
match action {
    HookAction::Message { message, image } => {
        execute_message(message, image.as_deref(), meta, runtime_messaging)
    }
    // existing variants...
}
```

`execute_actions()` should receive one more input:

```rust
pub async fn execute_actions(
    actions: &[HookAction],
    compiled_mappers: Option<&[Option<CompiledMapper>]>,
    meta: &EventMeta,
    settings: &GlobalSettings,
    messaging: &RuntimeMessagingSettings,
    can_block: bool,
    protect_decision: Option<&ProtectDecision>,
) -> Result<Option<HookResponse>>
```

`dispatch/mod.rs` should pass `config.messaging()` alongside `config.settings()`.

## Send Helper Behavior

`execute_message()` should:

1. interpolate `message`
2. interpolate `image` when present
3. resolve the effective messaging route
4. if no route is active, return immediately
5. build the `messenger::Message`
6. spawn a `tokio::spawn` task that:
   - builds the provider registry
   - calls `plan_send()`
   - logs compatibility warnings
   - sends with `send_planned()`
   - logs any send error as a warning

Recommended pseudocode:

```rust
fn execute_message(
    message_template: &str,
    image_template: Option<&str>,
    meta: &EventMeta,
    messaging: &RuntimeMessagingSettings,
) {
    let text = interpolate(message_template, meta);
    let image = image_template
        .map(|raw| interpolate(raw, meta))
        .filter(|value| !value.trim().is_empty());

    let Some(route) = resolve_effective_route(messaging) else {
        return;
    };

    let payload = match build_hook_message(&route, &text, image.as_deref(), meta) {
        Some(payload) => payload,
        None => return,
    };

    tokio::spawn(async move {
        if let Err(error) = send_hook_message(route, payload).await {
            warn!(%error, "Messaging send failed");
        }
    });
}
```

## Message Construction Rules

### Body

If interpolated `message` is non-empty, build:

```rust
messenger::Message::markdown(text)
```

If the text is empty but the provider-supported image attachment is present, construct a `Message` with `body = None` and one attachment.

If both text and image are absent after interpolation, skip the action.

### Image handling

Absolute behavior for v1:

1. Discord:
   - attach the image using `Attachment::image(path)`
   - missing/unreadable paths become warning logs when send is attempted
2. Slack, Signal, WhatsApp:
   - do not attach the image at all
   - log one warning that image attachments are ignored for that provider

Path resolution:

1. absolute paths remain absolute
2. paths beginning with `~/` expand through `dirs::home_dir()`
3. other relative paths resolve from `meta.cwd` when available
4. otherwise resolve relative to `meta.env.repo.root`, then current working directory as final fallback

This is more intuitive for event-driven config than resolving relative to the config file location.

## Logging And Error Handling

Message delivery is non-blocking and best-effort. No messaging error should alter the hook response or fail the dispatch pipeline.

Use `tracing::warn!` for:

1. no active route could be resolved because of missing env-backed secret
2. image ignored because the selected provider does not support attachments
3. `messenger::plan_send()` compatibility warnings
4. provider send failure

Use `tracing::debug!` for:

1. no messaging scope is configured
2. no active route is selected in either scope
3. empty interpolated payload skipped

## Config Validation

`HookerConfig::validate()` should gain messaging validation for the scope-local `settings.messaging` object.

Validation rules:

1. if `active` is `Some(name)`, `configs` must contain that key
2. config names must not be empty after trimming
3. provider target identifiers must not be blank
4. env var name fields must not be blank when present
5. inline-or-env requirements:
   - Discord/Slack: inline bot token or env var name must be available by schema default
   - Signal: rpc URL and account must be satisfiable through inline or env name fields
   - WhatsApp: access token and phone number ID must be satisfiable through inline or env name fields

Important loader change:

1. validate raw user config before merging
2. validate raw repo config before merging

That preserves accurate scope-local error messages.

## Loader Changes

`load_runtime_config()` should stop relying only on the merged `HookerConfig` for messaging.

Recommended flow:

1. load raw user config
2. load raw repo config
3. validate each raw config independently
4. build the merged provider/action config exactly as today
5. separately capture:
   - `user.settings.messaging.clone()`
   - `repo.settings.messaging.clone()`
6. compile runtime regexes and return `RuntimeConfig { settings, messaging, providers }`

`load_config()` can keep current merged behavior for existing callers. For v1 messaging dispatch, the scope-aware runtime view is the source of truth.

## Cargo And Dependency Changes

`claudine/lib/Cargo.toml` should add:

```toml
messenger = { path = "../../messenger/lib", default-features = false, features = ["discord", "slack", "signal", "whatsapp"] }
secrecy = "0.10"
```

Rationale:

1. only enable the four providers named in the spec
2. `messenger` provider configs use `SecretString`

## Documentation Updates In The Same Change

When implementation lands, update:

1. `claudine/docs/topics/configuring-actions.md`
   - add the `message` action
   - document provider config schema under `settings.messaging`
2. `claudine/lib/README.md`
   - mention messaging as a supported action type
3. `claudine/features/2026-04-01-messaging/spec.md`
   - replace webhook examples with `messenger`-compatible provider configs
   - mention env-backed secrets explicitly

## Testing Strategy

### Unit tests

Add unit tests for:

1. `HookAction::Message` serde round-trip
2. `type_slug()` and `type_pascal_case()` include `message`
3. scope-local messaging config serde with defaults
4. config validation rejects:
   - missing active route
   - blank channel/recipient values
   - blank env var names
5. effective route resolution:
   - repo active beats user active
   - repo inactive falls back to user active
   - both inactive yields `None`
6. secret resolution:
   - inline overrides env
   - env fallback works
   - missing env produces the expected warning/error string
7. Signal recipient parsing:
   - `+1555...` -> user phone
   - non-`+` string -> group
8. image path resolution

### Dispatcher tests

Add tests around `dispatch::runner` that verify:

1. `Message` actions are skipped when no route is configured
2. empty interpolated messages are skipped
3. non-blocking message actions do not alter selected `HookResponse`

These tests should stay local and avoid real network sends.

### Integration boundary

Do not duplicate `messenger` provider transport tests inside Claudine. The `messenger` crate already owns provider-specific integration coverage. Claudine should focus on:

1. config resolution
2. route selection
3. message construction
4. fire-and-forget dispatch behavior

## Example Final Config Shape

User scope:

```json
{
  "version": "1.0",
  "settings": {
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
  },
  "providers": {}
}
```

Repo scope:

```json
{
  "version": "1.0",
  "settings": {
    "messaging": {
      "active": "build-alerts",
      "configs": {
        "build-alerts": {
          "provider": "signal",
          "recipient": "+15551234567",
          "rpc_url_env": "SIGNAL_RPC_URL",
          "account_env": "SIGNAL_ACCOUNT"
        }
      }
    }
  },
  "providers": {}
}
```

Action example:

```json
{
  "type": "message",
  "message": "**{{provider}}** `{{event}}` in `{{cwd}}`",
  "image": "{{cwd}}/.claudine/artifacts/last-run.png"
}
```

## Implementation Order

Recommended order:

1. add config types and validation
2. extend runtime loader to preserve messaging scopes
3. add `HookAction::Message`
4. add the `claudine::messaging` helper module
5. wire `dispatch::runner` to call the helper
6. add docs and tests

This order keeps the integration risk low and makes it easy to test the feature layer-by-layer.
