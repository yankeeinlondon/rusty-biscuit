# Spec: Notification-Aware Message Bodies

## Background

When a Markdown message is sent to Discord today, the formatted text goes into the channel as `content`. Discord renders it nicely **in the chat**, but the **desktop notification** is generated from the raw `content` string with no Markdown rendering — the user sees literal `**`, `_`, and backtick characters in the system notification banner. Result: a clean in-channel message paired with an ugly desktop notification.

The cause is structural: Discord's notification body comes from `content`, while rich Markdown renders correctly when placed in an embed's `description`. The two surfaces want different inputs, but our `MessageBody` type can only express one body string.

## Goal

Extend `MessageBody` and the provider rendering pipeline so callers can provide:

1. **One Markdown string** (current default) — placed in `content`. Notification shows raw characters. _No change in behavior; preserves backwards compatibility._
2. **A pair of strings** — a plain "summary" used for the notification surface and a rich Markdown "body" used for the rendered surface. On Discord this maps to `content` (summary) + an embed `description` (rich body).
3. **One Markdown string with auto-strip** — formatting is removed at construction time and the resulting plain string is sent to both surfaces.

Variants 1 and 3 stay single-surface; variant 2 splits surfaces. All three must be expressible from both the library and the CLI.

## Scope

This feature is **not** about adding Discord embeds as a general first-class concept (no titles, fields, colors, thumbnails). It is specifically about giving callers a way to feed two text strings to providers that have a notification surface distinct from a rich-rendering surface.

In scope:

- A new `MessageBody::Summarized { summary, markdown }` enum variant.
- New `Message` constructors: `Message::summarized(summary, markdown)` and `Message::markdown_stripped(md)`.
- New `PreparedMessage` rendering helpers: `render_summary(provider)` and `render_rich(provider)`.
- Discord provider (`provider/discord.rs`): when a `Summarized` body is present, send `summary` as `content` and the rendered Markdown as a single embed's `description`.
- Discord webhook provider (`provider/discord_webhook.rs`): same split via the JSON payload's `embeds` array.
- Match-arm coverage in providers that pattern-match on `MessageBody`: `apns`, `fcm`, `telegram`, `validate`. Each picks the contextually correct half.
- CLI flags: `--summary <TEXT>` to opt into variant 2; `--strip-markdown` to opt into variant 3.
- Unit and integration tests for each variant on Discord and Discord webhook; smoke tests for the new arms in apns/fcm/telegram/validate.
- Documentation update for the user guide and the messenger SKILL file.

Out of scope:

- General-purpose embed authoring (multiple embeds, fields, images, footers, authors).
- Slack rich-block authoring beyond what Slack already does today (Slack's `text` fallback already serves a similar role; we will _not_ rewire it as part of this feature).
- Provider-specific notification overrides (those continue to live in `DesktopOverrides`, `DiscordOverrides`, etc.).
- Changing the default behavior of any existing caller. `Message::text` and `Message::markdown` continue to produce `Plain` and `Markdown` bodies and behave exactly as today.

## Variant semantics, per provider

| Variant                                   | Discord (`content` field)        | Discord (embed `description`)             | Discord Webhook                                  | Telegram                          | APNs / FCM                        | Desktop / Signal / WhatsApp / Slack |
|-------------------------------------------|----------------------------------|-------------------------------------------|--------------------------------------------------|-----------------------------------|-----------------------------------|-------------------------------------|
| `Plain(text)` (existing)                  | `text`                           | _none_                                    | `text`                                           | `text`                            | `text`                            | `text`                              |
| `Markdown(md)` (existing — variant 1)     | rendered MD                      | _none_                                    | rendered MD                                      | rendered HTML                     | raw MD string (today's behavior)  | per-provider rendering              |
| `Summarized { summary, markdown }` (new)  | `summary`                        | rendered MD                               | `summary` + embed                                | rendered MD (no embed concept)    | `summary`                         | per-provider rendering of `markdown`, but `summary` preferred for notification-only providers (apns/fcm) |
| `markdown_stripped(md)` constructor       | plain stripped text              | _none_                                    | plain stripped text                              | plain                             | plain                             | plain                               |

Notes:

- `markdown_stripped` returns a `MessageBody::Plain` — it is sugar, not a new variant.
- For non-Discord providers without an embed/notification split, `Summarized` providers must still emit a sensible single string. The convention: notification-centric providers (apns, fcm) take `summary`; rich-text providers (telegram, slack-mrkdwn) take rendered `markdown`; flat-text providers (signal, whatsapp, desktop) take `summary` since it is already plain.
- Location text (`📍 …`) continues to be appended for providers without a native location API. When an embed is present, the location text is appended to the embed description, not the content.

## CLI

The Send subcommand gains two flags. They are mutually exclusive with each other:

- `--summary <TEXT>` — treat the message argument as Markdown and pair it with `<TEXT>` as the notification summary. Produces `MessageBody::Summarized`. Implies Markdown body (cannot combine with `--plain`).
- `--strip-markdown` — strip Markdown formatting from the message body before sending. Produces `MessageBody::Plain`. Cannot combine with `--plain` (redundant) or `--summary` (incoherent).

The existing `--plain` flag is unchanged. The existing `--subtitle` flag remains a desktop-only override and is unrelated to this feature.

## Acceptance Criteria

1. `Message::summarized("Plain summary", "**Rich** body")` and `Message::markdown_stripped("**hi**")` compile and produce the expected `MessageBody` shapes.
2. `cargo test -p messenger-lib` passes with new tests covering all four variants in `tests/builders.rs`.
3. A Discord wiremock-style integration test verifies that a `Summarized` body produces a request with `content = summary` and an `embeds[0].description = rendered markdown`.
4. A Discord webhook wiremock test verifies the same payload split.
5. Telegram, APNs, FCM, validation tests exercise the new `Summarized` arm.
6. `messenger send "**rich body**" --summary "plain notification text" --route discord:test` results in a Discord message whose desktop notification banner reads "plain notification text" while the in-channel embed renders the rich Markdown.
7. `messenger send "**rich**" --strip-markdown --route discord:test` produces a single-line plain `content = "rich"` (no embed), and the notification banner matches.
8. No existing callers break: `cargo test --workspace` (excluding integration tests that require live providers) passes without modification of any test that doesn't touch `MessageBody` directly.
9. The user guide gains a short section explaining the three calling shapes with examples.
