---
title: Surface Inventory — current chat adapters
date: 2026-09-17
status: completed
---

# Surface Inventory — current chat adapters

Plan Phase 1, Wave 1, task **Surface Inventory**. This is read-only discovery.
It covers the payload surfaces, operations, interface identities, response
handling, receipts, and tests for the seven chat adapters, as of inspected
revision `d57faf7e8` (branch `feat/better-static-analysis`). Paths are relative
to the repository root. Symbols are cited by name, not line number.

Legend for claims: **code** means read directly from the cited symbol.
**unverified** means an external platform fact that repository evidence does not
establish. Research must confirm these before they become constraint facts.

## 1. Adapter identities

Recommendation: make `adapter_id` **exactly** `ProviderKind::as_str()`
(`messenger/lib/src/receipt.rs`). That value is already persisted in three
places: the `ProviderKind` serde form (receipt JSON), `MessageRef` serde tags,
and the CLI route `provider` tag (`RouteProvider::as_str`,
`RouteConfigRepr` in `messenger/cli/src/config.rs`). The snake_case names
proposed in the task brief suit **interface** IDs, which name external APIs
rather than Messenger adapters. Keeping the two namespaces distinct avoids
re-keying persisted receipts.

| adapter_id (existing) | `ProviderKind` | Rust type | Proposed `interface_id` | Operations emitted | API/SDK version source |
|---|---|---|---|---|---|
| `discord` | `Discord` | `DiscordProvider` (`provider/discord.rs`) | `discord_http_api` (via SDK `twilight-http`) | `POST /channels/{id}/messages` via `Client::create_message` | Twilight chooses the API version. Messenger pins `twilight-http`/`-model`/`-util` `"0.17"` in `messenger/lib/Cargo.toml`, and the truncation spec records `twilight-validate` 0.17.0 in the lockfile. The REST version (v10) is **unverified** |
| `discord-webhook` | `DiscordWebhook` | `DiscordWebhookProvider` (`provider/discord_webhook.rs`) | `discord_http_api` (operation `execute_webhook`, no SDK) | `POST /webhooks/{id}/{token}?wait=true[&thread_id=]` | Taken from the caller's URL (`/api/v{n}/`). `parse_webhook_url` accepts any version or none |
| `slack` | `Slack` | `SlackProvider` (`provider/slack.rs`) | `slack_web_api` | `POST {base}/chat.postMessage` | Unversioned URL (`https://slack.com/api`) |
| `slack-webhook` | `SlackWebhook` | `SlackWebhookProvider` (`provider/slack_webhook.rs`) | `slack_incoming_webhook` | `POST https://hooks.slack.com/services/{t}/{b}/{x}` | Unversioned |
| `telegram` | `Telegram` | `TelegramProvider` (`provider/telegram.rs`) | `telegram_bot_api` | `sendMessage` **or** `sendLocation` (never both) | Unversioned URL. Bot API version is not recorded anywhere |
| `whatsapp` | `WhatsApp` | `WhatsAppProvider` (`provider/whatsapp.rs`) | `whatsapp_cloud_api` | `POST /{version}/{phone_number_id}/messages`, `type` = `text` **or** `location` | `WhatsAppConfig::api_version`, defaulting to `v23.0` (`WhatsAppProvider::new`) |
| `signal` | `Signal` | `SignalProvider` (`provider/signal.rs`) | `signal_cli_jsonrpc` (a bridge, not a service API) | JSON-RPC `send` (user, note-to-self) or `sendGroupMessage` (group) | No bridge version is negotiated or recorded. `SignalConfig::rpc_url` is used verbatim |

Discord bot and Discord webhook share one external API but have different
client-side validators, auth models, and error paths (see §9). The schema needs
**one interface with two operations mapped to two adapters**, and the SDK
validator needs a separate version field (`twilight-validate`).

## 2. Shared pipeline (applies to every adapter)

| Stage | Symbol | Behavior relevant to surfaces |
|---|---|---|
| Content validation | `validate_message_for_provider`, `validate_message` (`lib/src/validate.rs`) | Requires a body, an attachment, or a location. `title` alone is invalid for all seven chat adapters |
| Capability normalization | `normalize_dispatch` (`validate.rs`) | Drops, or rejects under `Strict`, unsupported attachments, location, replies, silent delivery, and link-preview control according to the provider's `CapabilitySet`, and emits a `CompatibilityWarning`. When Markdown is unsupported it only **warns**. The body is not changed here. Discord webhook plus `reply_to` is a hard error in every mode |
| Rendering | `PreparedMessage::render_body_for_provider`, `render_body_with_location`, `render_summary`, `render_rich` (`lib/src/prepared.rs`) | Maps each `MessageBody` variant to a string (table below) |
| Markdown parse | `parse_markdown` (`lib/src/markdown/parse.rs`) | pulldown-cmark with **only** `ENABLE_STRIKETHROUGH`. Block quotes, images, and other unrecognized tags are flattened to their children, so image URLs are lost and alt text is kept. Inline and block HTML events are silently dropped. Tables are not enabled |
| AST | `RichNode` (`lib/src/markdown/ast.rs`) | Text, Bold, Italic, Strikethrough, Code, CodeBlock{language}, Link, List, Paragraph, Heading, SoftBreak, HardBreak. There is **no** underline, spoiler, block quote, image, table, or mention node |
| Dialect dispatch | `render_nodes_for_provider` (`lib/src/markdown/mod.rs`) | discord/discord-webhook → `render_discord`; slack/slack-webhook → `render_slack_mrkdwn`; telegram → `render_telegram_html`; signal/whatsapp → `render_plain_text` |
| Location text fallback | `Location::format_text_line` (`lib/src/message.rs`) | `📍 name (address) — lat, lon` with 4 decimal places, appended after `'\n'` by `render_body_with_location` |
| Attachment bytes | `read_local_or_bytes_attachment` (`provider/attachment_helpers.rs`) | Accepts only `Path` and `Bytes`. Rejects `Url` and `ProviderFileId`. **Ignores `Bytes.mime_type`** |
| HTTP response | `handle_http_response` (`provider/http_helpers.rs`) | 429 → `RateLimited` from the integer-seconds `Retry-After` header only. 5xx → `Transport("server error: {status}")` with the body discarded. Any other status → JSON decode, and a decode failure becomes `Transport` |

### `MessageBody` variant → rendered string

From `PreparedMessage::render_body_for_provider`, `render_summary`, and
`render_rich`:

| Adapter | `Plain(s)` | `Markdown(md)` | `Summarized{summary, markdown}` |
|---|---|---|---|
| discord / discord-webhook | `content` = `s`, **not escaped** (Discord still parses it as Markdown) | `content` = `render_discord(md)` | `content` = `summary` (`render_summary`), embed `description` = `render_discord(markdown)` (`render_rich`) |
| slack / slack-webhook | `text` = `s`, **not escaped** (`&`, `<`, `>`, and mrkdwn characters pass through) | `text` = `render_slack_mrkdwn(md)` | `text` = `render_slack_mrkdwn(markdown)`. **The summary is discarded** |
| telegram | `text` = `s`, `parse_mode` omitted | `text` = `render_telegram_html(md)`, `parse_mode: "HTML"` | `text` = `render_telegram_html(markdown)`, `parse_mode: "HTML"`. **The summary is discarded** |
| whatsapp | `text.body` = `s` | `text.body` = `render_plain_text(md)`, plus a Markdown warning | `text.body` = `summary`. **The whole Markdown half is discarded.** The only warning is the generic "markdown rendering" warning |
| signal | `message` = `s` | `message` = `render_plain_text(md)`, plus a Markdown warning | `message` = `summary`. **The Markdown half is discarded** (same as WhatsApp) |

`Message.title` and `Message.metadata` are ignored by all seven chat adapters.
No chat adapter calls `PreparedMessage::title`.

`CapabilitySet` (`lib/src/capabilities.rs`) has only six booleans. Its
`supports_location` is `true` for all seven adapters, but five of them emit only
the text fallback. The current capability model therefore **conflates native
support with Messenger fallback**, which the spec explicitly forbids for
research values.

## 3. Discord bot — `discord`

| Surface | Native field locator | Source of content | Representation / format | Notes |
|---|---|---|---|---|
| Primary text (Plain/Markdown) | `content` via `CreateMessage::content` | `render_body_with_location(Discord)` | markup (`render_discord`), or raw text for Plain | Location line appended. Empty string → field omitted (`DiscordProvider::build_payload`) |
| Summary (Summarized) | `content` | `render_summary()` → `summary` verbatim | caller text, not escaped | Omitted when empty |
| Rich body (Summarized) | `embeds[0].description` via `EmbedBuilder::new().description(..)` | `render_rich(Discord)` + `'\n'` + location line | markup (`render_discord`) | The embed has **only** a description: no title, color, footer, or fields |
| Attachments | multipart `files[n]` plus `attachments[n]{id, filename, description}` via `CreateMessage::attachments` and `DiscordAttachment::from_bytes` | `DiscordProvider::build_attachment` | bytes | `id` = index. `description` = `alt_text` or, failing that, `caption`. **The caption is never visible** and MIME type is not sent |
| Location | text fallback only | `Location::format_text_line` | text | Goes in `content` (Plain/Markdown) or the embed description (Summarized) |
| Reply | `message_reference` via `CreateMessage::reply(msg_id)` | `MessageRef::Discord.message_id` | id | `MessageRef.channel_id` is ignored. `fail_if_not_exists` is not set by Messenger (twilight default **unverified**) |
| Silent / link preview | not emitted | — | — | `supports_silent_delivery: false` and `supports_link_preview_control: false` in `capabilities()` |
| Mentions | not controlled | — | — | `allowed_mentions` is not set (twilight default **unverified**) |

**Receipt** (`send_prepared`): `MessageRef::Discord{channel_id: target channel, message_id: msg.id}`.
`raw_id` = `msg.id`. `metadata` is empty.

**Tests.** `provider/discord.rs` `mod tests` covers
`build_payload_plain_body_uses_content_only`,
`build_payload_markdown_body_uses_content_only`,
`build_payload_summarized_body_splits_into_content_and_embed`,
`build_payload_summarized_with_empty_summary_omits_content`,
`build_payload_summarized_appends_location_to_embed_description`,
`build_payload_legacy_appends_location_to_content`,
`converts_path_attachments_to_discord_uploads`,
`converts_byte_attachments_to_discord_uploads`, `rejects_url_attachments`,
`rejects_provider_file_id_attachments`, `rejects_invalid_channel_id`,
`rejects_invalid_message_id`, and `supports_attachments`.
`lib/tests/integration.rs::smoke_test_discord_send` is a live smoke test.
There is **no** wire-level test: twilight has no mock transport, so request
serialization, replies, and error mapping are untested.

### Fingerprint inputs — `discord`

| Path | Symbols |
|---|---|
| `messenger/lib/src/provider/discord.rs` | `DiscordProvider::build_payload`, `DiscordPayload`, `build_attachment`, `build_attachments`, `parse_channel_id`, `parse_message_id`, `capabilities`, `send_prepared`, `mod tests` |
| `messenger/lib/src/provider/attachment_helpers.rs` | `read_local_or_bytes_attachment` |
| `messenger/lib/src/markdown/discord.rs` | `render_discord`, `render_nodes`, `render_inline` |
| `messenger/lib/Cargo.toml`, `Cargo.lock` | `twilight-http`, `twilight-model`, `twilight-util`, and transitive `twilight-validate` versions (the SDK validator is an assessed input) |
| shared set (§10) | — |

## 4. Discord webhook — `discord-webhook`

| Surface | Native field locator | Source of content | Representation / format | Notes |
|---|---|---|---|---|
| Primary text (Plain/Markdown) | JSON `/content`, or `payload_json` → `/content` when attachments are present (`WebhookJsonBody`) | `render_body_with_location(DiscordWebhook)` | markup (`render_discord`) | Same shape as the bot, built inline in `send_prepared` rather than through a shared `build_payload` |
| Summary (Summarized) | `/content` | `render_summary()` | text | Omitted when empty |
| Rich body (Summarized) | `/embeds/0/description` (`EmbedBody`) | `render_rich` + location line | markup | Single-field embed |
| Attachments | multipart `files[{i}]` plus `payload_json` → `/attachments/{i}{id, filename, description}` (`AttachmentMeta`, `DiscordWebhookProvider::build_part`) | alt text, then caption | bytes | `Part::bytes(..).file_name(..)` sets no MIME type |
| Thread | query `thread_id` (`build_request_url`) | `DiscordWebhookTarget.thread_id` | numeric string | Validated as `u64` before the request |
| Wait for message | query `wait=true` | constant | — | Required to get a message object back |
| Location | text fallback | as for the bot | text | — |
| Reply | **rejected at plan time** (`normalize_dispatch` → `UnsupportedFeature`) | — | — | `supports_reply: false` |
| Username / avatar / silent / link preview / mentions | not emitted | — | — | See §12 |

**Receipt**: `MessageRef::DiscordWebhook{webhook_id: response webhook_id or the configured id, channel_id, message_id, thread_id: target thread}`.
`raw_id` = `id`. `metadata` is empty. The response is decoded as
`WebhookMessageResponse`.

**Tests.** `lib/src/tests/discord_webhook_integration.rs` (wiremock) covers
`sends_text_message_with_expected_payload`,
`sends_markdown_message_with_expected_payload`,
`sends_with_thread_id_query_parameter`,
`invalid_thread_id_errors_before_network_call`,
`sends_attachments_as_multipart_with_payload_json_and_file_part`,
`rate_limit_maps_to_rate_limited_with_retry_after`,
`generic_client_error_maps_to_provider_error`,
`auth_error_maps_to_authentication`, `server_error_maps_to_transport`,
`falls_back_to_provider_webhook_id_when_response_omits_it`,
`plan_send_with_reply_to_errors_before_network_call`,
`plan_send_with_reply_hard_errors_in_best_effort_mode`,
`webhook_summarized_body_splits_into_content_and_embed`,
`webhook_markdown_body_does_not_include_embeds`,
`capabilities_match_webhook_contract`, and
`messenger_routes_to_discord_webhook_provider`. `provider/discord_webhook.rs`
`mod tests` covers URL parsing and `build_request_url`.

### Fingerprint inputs — `discord-webhook`

| Path | Symbols |
|---|---|
| `messenger/lib/src/provider/discord_webhook.rs` | `DiscordWebhookProvider::send_prepared`, `build_request_url`, `build_part`, `parse_thread_id`, `parse_webhook_url`, `WebhookJsonBody`, `EmbedBody`, `AttachmentMeta`, `WebhookMessageResponse`, `capabilities`, `mod tests` |
| `messenger/lib/src/provider/attachment_helpers.rs` | `read_local_or_bytes_attachment` |
| `messenger/lib/src/provider/http_helpers.rs` | `handle_http_response` |
| `messenger/lib/src/markdown/discord.rs` | `render_discord` |
| `messenger/lib/src/validate.rs` | `normalize_dispatch` (the webhook reply rejection branch) |
| `messenger/lib/src/tests/discord_webhook_integration.rs` | whole file |
| shared set (§10) | — |

## 5. Slack Web API — `slack`

| Surface | Native field locator | Source of content | Representation / format | Notes |
|---|---|---|---|---|
| Primary text | `/text` (`ChatPostMessageRequest`) | `render_body_with_location(Slack)` | mrkdwn (`render_slack_mrkdwn`), or raw text for Plain | `mrkdwn` is not set, so the Slack default applies. `blocks` are never sent. With `blocks`, `text` would become the notification fallback, which is a native summary/rich split that Messenger does not use |
| Destination | `/channel` | `SlackTarget.channel_id` | id | — |
| Thread reply | `/thread_ts` | `MessageRef::Slack.thread_ts` | ts string | `MessageRef.channel_id` is ignored. The receipt stores the **new message's `ts`** as `thread_ts`, so replying to a threaded reply passes that reply's ts, not the parent's. Slack behavior in that case is **unverified** |
| Link preview | `/unfurl_links=false`, `/unfurl_media=false` | `DeliveryOptions.disable_link_preview` | bool | Both are set together and omitted otherwise |
| Attachments | none | — | — | Empty `supported_attachment_kinds`, so they are dropped with a warning or rejected under `Strict` |
| Location | text fallback | `format_text_line` | text | — |
| Silent | not emitted | — | — | `supports_silent_delivery: false` |

**Receipt**: `MessageRef::Slack{channel_id: response channel (empty if absent), thread_ts: response ts (empty if absent)}`.
`raw_id` = `ts`. The `unwrap_or_default` calls mean a malformed success
response yields an **empty, unusable reference** without an error.

**Tests.** `lib/src/tests/slack_integration.rs` covers
`sends_text_message_with_correct_payload`, `sends_markdown_as_mrkdwn`,
`includes_thread_ts_for_reply`, `auth_error_maps_to_authentication`,
`rate_limit_maps_to_rate_limited`, `disables_link_preview_in_payload`,
`server_error_maps_to_transport`, and `messenger_routes_to_slack_provider`.
`lib/tests/integration.rs::smoke_test_slack_send` is a live smoke test.
`markdown/slack_mrkdwn.rs` `mod tests` covers the renderer.

### Fingerprint inputs — `slack`

| Path | Symbols |
|---|---|
| `messenger/lib/src/provider/slack.rs` | `SlackProvider::send_prepared`, `ChatPostMessageRequest`, `ChatPostMessageResponse`, `capabilities` |
| `messenger/lib/src/provider/http_helpers.rs` | `handle_http_response` |
| `messenger/lib/src/markdown/slack_mrkdwn.rs` | `render_slack_mrkdwn`, `render_nodes`, `render_inline` |
| `messenger/lib/src/tests/slack_integration.rs` | whole file |
| shared set (§10) | — |

## 6. Slack incoming webhook — `slack-webhook`

| Surface | Native field locator | Source of content | Representation / format | Notes |
|---|---|---|---|---|
| Primary text | `/text` (`WebhookRequest`) | `render_body_with_location(SlackWebhook)` | mrkdwn | Same renderer as `slack` |
| Thread reply | `/thread_ts` | `MessageRef::SlackWebhook{thread_ts: Some}` | ts | The adapter never produces a `thread_ts` (receipt is `thread_ts: None`), so callers must build the ref by hand. Whether incoming webhooks honor `thread_ts` at all is **unverified** |
| Link preview | `/unfurl_links`, `/unfurl_media` | as for `slack` | bool | — |
| Destination | bound in the URL | `SlackWebhookConfig.webhook_url` | — | `SlackWebhookTarget` is empty. `validate_webhook_url` enforces the host and path |
| Attachments / silent | none | — | — | As for `slack` |

**Receipt**: `MessageRef::SlackWebhook{thread_ts: None}`, `raw_id` = `""`,
and `metadata{"delivery_confirmed": "true"}`. That key overstates the
evidence: the webhook confirms acceptance, not delivery.

**Response parsing** (inline in `send_prepared`, not the shared helper):
429 → `RateLimited` from the header. 5xx → `Transport`. Anything else is
decoded as the JSON `WebhookResponse{ok, error}`, and `!ok` goes through
`map_webhook_error`. **Conflict to research:** the completed spec
`messenger/features/_completed/2026-04-19-slack-webhook/spec.md` asserts a JSON
`{"ok":...}` response, and the tests mock JSON (`webhook_ok_response`,
`send_with_webhook_error`). Slack's public docs describe incoming webhooks as
returning a **plain-text** body: `ok` on success, or an HTTP 4xx status with a
text token such as `invalid_payload` or `no_service` (from memory,
**unverified**). If that is right, real successes fail JSON decoding and become
`Transport` errors, and every 4xx error token is lost. The spec's required
"plain-text webhook error" fixture shape applies here, so this should be a
priority research question.

**Tests.** `lib/src/tests/slack_webhook_integration.rs` covers the payload,
thread, unfurl, error-code mapping, 429, and 5xx cases (20 tests).
`provider/slack_webhook.rs` `mod tests` covers URL validation and
`map_webhook_error`.

### Fingerprint inputs — `slack-webhook`

| Path | Symbols |
|---|---|
| `messenger/lib/src/provider/slack_webhook.rs` | `SlackWebhookProvider::send_prepared`, `WebhookRequest`, `WebhookResponse`, `map_webhook_error`, `validate_webhook_url`, `capabilities`, `mod tests` |
| `messenger/lib/src/markdown/slack_mrkdwn.rs` | `render_slack_mrkdwn` |
| `messenger/lib/src/tests/slack_webhook_integration.rs` | whole file |
| shared set (§10) | — |

## 7. Telegram Bot API — `telegram`

| Surface | Native field locator | Source of content | Representation / format | Notes |
|---|---|---|---|---|
| Primary text | `sendMessage` `/text` (`SendMessageRequest`) | `render_body_for_provider(Telegram)` | Plain → text with no `parse_mode`. Markdown/Summarized → HTML markup | No location line is ever appended |
| Parse mode | `/parse_mode` | variant match in `send_prepared` | `"HTML"` or omitted | Selector field. MarkdownV2 and `entities` are never used |
| Location | `sendLocation` `/latitude`, `/longitude` (`SendLocationRequest`) | `Message.location` | native | **Early return**: when a location is present, only `sendLocation` is called. **The body is silently dropped**, as are `Location.name`/`address` (`sendVenue` is not used). There is no warning, and the tests only cover location-only messages (`sends_location`) |
| Reply | `/reply_parameters/message_id` | `MessageRef::Telegram.message_id` | int | The ref's `chat_id` and `thread_id` are ignored |
| Forum topic | `/message_thread_id` | `TelegramTarget.thread_id` | int | On both operations |
| Silent | `/disable_notification=true` | `DeliveryOptions.silent` | bool | On both operations |
| Link preview | `/disable_web_page_preview=true` | `disable_link_preview` | bool | A **deprecated** field. The research doc lists `link_preview_options` (`docs/research/platforms/telegram.md`) |
| Attachments | none | — | — | Empty set, although the Bot API has `sendPhoto`, `sendDocument`, and similar methods |

**Receipt**: `MessageRef::Telegram{chat_id: TelegramChatRef from the target, message_id: result.message_id, thread_id: target thread}`.
`raw_id` = `message_id`. `metadata` is empty.

**Tests.** `lib/src/tests/telegram_integration.rs` covers `sends_text_message`,
`sends_markdown_as_html`, `sends_location`, `includes_reply_parameters`,
`auth_error_maps_correctly`, `rate_limit_maps_correctly`, `silent_delivery`,
and `disables_link_preview_in_payload`. The rate-limit fixture is **HTTP 200**
with `parameters.retry_after` in the body. It was constructed by hand, and the
real Telegram 429 status would take a different code path (§9).
`markdown/telegram_html.rs` `mod tests` covers the renderer.

### Fingerprint inputs — `telegram`

| Path | Symbols |
|---|---|
| `messenger/lib/src/provider/telegram.rs` | `TelegramProvider::send_prepared`, `post_request`, `handle_error`, `SendMessageRequest`, `SendLocationRequest`, `ReplyParameters`, `TelegramResponse`, `TelegramErrorParameters`, `chat_id_to_string`, `chat_id_to_ref`, `capabilities` |
| `messenger/lib/src/provider/http_helpers.rs` | `handle_http_response` |
| `messenger/lib/src/markdown/telegram_html.rs` | `render_telegram_html`, `render_nodes`, `escape_html` |
| `messenger/lib/src/target.rs` | `TelegramTarget`, `TelegramChatId` |
| `messenger/lib/src/tests/telegram_integration.rs` | whole file |
| shared set (§10) | — |

## 8. WhatsApp Cloud API and signal-cli JSON-RPC

### WhatsApp — `whatsapp`

| Surface | Native field locator | Source of content | Representation / format | Notes |
|---|---|---|---|---|
| Primary text | `/text/body` (`TextPayload`), `/type="text"` | `render_body_for_provider(WhatsApp)` | plain text (`render_plain_text`) | WhatsApp's native formatting (`*bold*`, `_italic_`, and so on) is not used. `supports_markdown_rendering: false` |
| Link preview | `/text/preview_url` | hard-coded `None` | — | The native control exists but is never emitted, and `supports_link_preview_control: false` |
| Location | `/location{latitude, longitude, name, address}`, `/type="location"` | `Message.location` | native | **The body is silently dropped** when a location is present (`send_prepared` picks one type). No warning, and the test (`sends_location`) is location-only |
| Reply | `/context/message_id` | `MessageRef::WhatsApp.message_id` | wamid | — |
| Attachments / silent / templates / interactive | none | — | — | Empty attachment set |

**Receipt**: `MessageRef::WhatsApp{message_id: last messages[].id, or "" if absent}`,
`raw_id` = the same value, and `metadata` is empty. `messages[].message_status`
and `contacts[].wa_id` are not deserialized (`WhatsAppResponse`).

**Tests.** `lib/src/tests/whatsapp_integration.rs` covers `sends_text_message`,
`sends_location`, `sends_reply`, `auth_error_maps_correctly`,
`rate_limit_returns_error` (bare HTTP 429), `server_error_maps_to_transport`,
and `markdown_rendered_as_plain_text`.

### Signal — `signal`

| Surface | Native field locator | Source of content | Representation / format | Notes |
|---|---|---|---|---|
| Primary text | `params.message` | `render_body_with_location(Signal)` | plain text | `textStyle` / body ranges are not used |
| Destination | `params.recipient[]` (user), `params.groupId` (group), or `params.recipient=[account]` plus `noteToSelf: true` | `SignalTarget` | — | **Group uses the method `sendGroupMessage`.** signal-cli's JSON-RPC documentation (man page `signal-cli-jsonrpc.5`) mirrors CLI commands, where groups go through `send` with `groupId`. `sendGroupMessage` looks like signal-cli's **D-Bus** method name. Possible bug, **unverified**; the test `sends_group_message` only mocks it |
| Reply | `params.quoteAuthor`, `params.quoteTimestamp` | `MessageRef::Signal{author, timestamp_ms}` | — | `quoteMessage` is not sent |
| Location | text fallback | `format_text_line` | text | — |
| Attachments / mentions / preview / edit / silent | none | — | — | Empty attachment set |

**Receipt**: `MessageRef::Signal{thread from the target, author derived from the configured account (a leading '+' means Phone), timestamp_ms: result.timestamp or 0}`.
`raw_id` = the timestamp. A missing timestamp yields a **zero-timestamp
reference that cannot be quoted**, with no error.

**Tests.** `lib/src/tests/signal_integration.rs` covers `sends_direct_message`,
`sends_group_message`, `sends_reply_with_quote`,
`jsonrpc_error_maps_to_provider_error`, `server_error_maps_to_transport`, and
`markdown_rendered_as_plain_text`. The fixture's `result` holds only
`timestamp` (`jsonrpc_ok_response`).

### Fingerprint inputs — `whatsapp`, `signal`

| Adapter | Path | Symbols |
|---|---|---|
| whatsapp | `messenger/lib/src/provider/whatsapp.rs` | `WhatsAppProvider::new` (default version), `send_prepared`, `WhatsAppMessageRequest`, `TextPayload`, `LocationPayload`, `ContextPayload`, `WhatsAppResponse`, `WhatsAppError`, `capabilities` |
| whatsapp | `messenger/lib/src/markdown/plain_text.rs` | `render_plain_text` |
| whatsapp | `messenger/lib/src/tests/whatsapp_integration.rs` | whole file |
| signal | `messenger/lib/src/provider/signal.rs` | `SignalProvider::send_prepared`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `address_to_string`, `capabilities` |
| signal | `messenger/lib/src/target.rs`, `receipt.rs` | `SignalTarget`, `SignalAddress`, `SignalThreadKey`, `SignalAuthor` |
| signal | `messenger/lib/src/markdown/plain_text.rs` | `render_plain_text` |
| signal | `messenger/lib/src/tests/signal_integration.rs` | whole file |
| both | `messenger/lib/src/provider/http_helpers.rs` | `handle_http_response` |

## 9. Error handling loss

Final variants come from `MessengerError` (`lib/src/error.rs`). Its only
structured slot is `Provider.code: Option<String>`. No variant carries field
errors, warnings, correlation IDs, a delivery-certainty value, or the HTTP
status separately from `code`.

| Adapter | Where lost | What is lost | Evidence |
|---|---|---|---|
| all using `handle_http_response` | 5xx branch | The whole response body, including any native code and correlation ID. The result is `Transport("server error: {status}")` | `handle_http_response` |
| all using `handle_http_response` | 429 branch | Body-level retry fields and scope/global flags. `Retry-After` is parsed only as integer seconds (the HTTP-date form becomes `None`) | `handle_http_response` |
| all using `handle_http_response` | JSON decode failure | The body is replaced by the serde error text in `Transport` | `handle_http_response` |
| all seven | response headers | No correlation or rate-limit headers are captured (for example Slack `x-slack-req-id` or Discord `X-RateLimit-*`; header names **unverified**) | no header reads except `Retry-After` |
| discord | every failure | The twilight error is stringified into `Transport`. Client-side `ErrorType::Validation` (before submission), HTTP 4xx `Response` errors with Discord JSON `code`/`errors`, and auth failures are all indistinguishable. `RateLimited` is never produced (twilight's internal ratelimiter behavior is **unverified**) | `send_prepared`: `map_err(.. transport_error(e))` |
| discord-webhook | 4xx | Discord's JSON `code` (for example 50035) and nested `errors` field paths. `code` is set to the **HTTP status**, and the raw body becomes `message` verbatim. It is also logged at `warn` with `body = %body`, which conflicts with the spec's "raw bodies are not default logs" | `send_prepared` client-error branch |
| discord-webhook | 401/403 | 403 (a permission problem) is classified as `Authentication`. 404 Unknown Webhook becomes `Provider{code:"404"}` | `send_prepared` |
| slack | `!ok` | `warning`, `response_metadata.warnings`/`messages`, and `needed`/`provided` scopes. The error string is kept as `code` | `ChatPostMessageResponse`, `send_prepared` |
| slack | success with a warning | `ok: true` plus `warning` is treated as a clean success | `ChatPostMessageResponse` |
| slack-webhook | 4xx plain-text body (if plain text; see §6) | The native error token, lost to a `Transport` decode error | `send_prepared` |
| slack-webhook | known tokens | `invalid_payload`, `channel_not_found`, and `channel_is_archived` become `InvalidMessage(String)`, which has no `code` and no provider | `map_webhook_error` |
| telegram | `ok: false` | `error_code` is not deserialized (`code: None`). Auth is detected by **free-form substring** (`"Unauthorized"`, `"bot token"`), which is outside the spec's executable contract. `parameters.migrate_to_chat_id` is lost | `TelegramResponse`, `handle_error` |
| telegram | real HTTP 429 | The shared helper returns before the body is parsed, so `parameters.retry_after` is lost unless a `Retry-After` header is present (**unverified** for Telegram) | `post_request` → `handle_http_response` |
| whatsapp | `error` object | `type`, `error_subcode`, `error_data.details` (field-level detail), and `fbtrace_id` (correlation). Only code 190 maps to `Authentication`, and throttling codes returned with HTTP 400 become `Provider`, not `RateLimited` (code values **unverified**) | `WhatsAppError`, `send_prepared` |
| whatsapp | success | `messages[].message_status` (acceptance nuance) | `WhatsAppResponse` |
| signal | JSON-RPC error | `error.data`. `code` is kept as a string | `JsonRpcError` |
| signal | success result | Per-recipient `results[]` outcomes (signal-cli reports per-recipient success or failure types; **unverified**). Partial or undelivered sends inside a successful RPC become an `Ok` receipt | `send_prepared` reads only `result.timestamp` |
| all | successful send | No warning channel exists besides pre-send `CompatibilityWarning` (`SendPlan.warnings`). Silent body drops (Telegram/WhatsApp location, Summarized halves) produce **no** warning at all | `normalize_dispatch`, adapters |

## 10. Shared fingerprint inputs (include in every adapter's set)

| Path | Symbols |
|---|---|
| `messenger/lib/src/message.rs` | `Message`, `MessageBody`, `Location`, `Location::format_text_line` |
| `messenger/lib/src/prepared.rs` | `PreparedMessage::new`, `render_body_for_provider`, `render_body_with_location`, `render_summary`, `render_rich` |
| `messenger/lib/src/markdown/parse.rs` | `parse_markdown`, `parse_events`, `extract_text` |
| `messenger/lib/src/markdown/ast.rs` | `RichNode` |
| `messenger/lib/src/markdown/mod.rs` | `render_for_provider`, `render_nodes_for_provider` |
| `messenger/lib/src/markdown/plain_text.rs` | `render_plain_text` (also used by `render_summary`, so Discord depends on it too) |
| `messenger/lib/src/validate.rs` | `normalize_dispatch`, `validate_message_for_provider`, `validate_attachment_source`, `is_undeliverable_after_normalization` |
| `messenger/lib/src/capabilities.rs` | `CapabilitySet` |
| `messenger/lib/src/dispatch.rs` | `Dispatch`, `DeliveryOptions`, `CompatibilityMode` |
| `messenger/lib/src/receipt.rs` | `ProviderKind::as_str`, `MessageRef` (the adapter's variant), `SendReceipt` |
| `messenger/lib/src/error.rs` | `MessengerError` |
| `messenger/lib/src/attachment.rs` | `Attachment`, `AttachmentKind`, `AttachmentSource` |
| `messenger/lib/src/target.rs` | the adapter's `Target` variant and struct |
| `messenger/lib/src/tests/validation.rs`, `builders.rs`, `receipts.rs` | whole files (shared behavior tests) |

Adding every shared file to every adapter's set means any shared edit marks all
seven assessments `unassessed`. Symbol-level hashing (for example, only the
adapter's `MessageRef` variant or its `render_nodes_for_provider` arm) would
limit that churn, but it requires a symbol extractor. The spec explicitly does
not promise that level of dependency analysis.

## 11. Discord truncation regression inputs

Source: `messenger/fixes/2026-09-06-truncation/spec.md` (status `draft`,
reviewed; plan at phase 1). **Not implemented at HEAD**: there is no
`provider/limits.rs` and no `DISCORD_MAX_*` constants in `messenger/lib/src`.

Field mapping to preserve as regression cases (spec R4 table, R4.1, and R4.2):

| # | Adapter | Native field | Body shape | Source | Limit (chars) | Location reserved |
|---|---|---|---|---|---|---|
| 1 | discord | `content` | Summarized | `render_summary()` | 2000 | no |
| 2 | discord | `embeds[0].description` | Summarized | `render_rich()` + location | 4096 | yes |
| 3 | discord | `content` | Plain / Markdown | `render_body_with_location()` | 2000 | yes |
| 4 | discord-webhook | `content` | Summarized | `render_summary()` | 2000 | no |
| 5 | discord-webhook | `embeds[0].description` | Summarized | `render_rich()` + location | 4096 | yes |
| 6 | discord-webhook | `content` | Plain / Markdown | `render_body_with_location()` | 2000 | yes |
| 7 | discord | `attachments[n].description` | any | `alt_text` or, failing that, `caption` | 1024 | n/a |
| 8 | discord-webhook | `payload_json` `/attachments/{n}/description` | any | `alt_text` or, failing that, `caption` | 1024 | n/a |

SDK-validation findings (spec "Root Cause" and R4.2):

- The bot path fails **closed on the client**. `twilight_validate::message::content()` (`chars().count() <= 2000`), `embeds()` (description 4096 plus `EMBED_TOTAL_LENGTH` 6000), and `attachment_description` (`ATTACHMENT_DESCIPTION_LENGTH_MAX` = 1024, twilight's own spelling) run at `req.content`, `req.embeds`, and `req.attachments`. They surface at `.await` as `ErrorType::Validation` → `MessengerError::Transport`, and no request is sent. `EmbedBuilder::build()` does not validate.
- The webhook path has **no validator**. The overflow reaches Discord and returns HTTP 400 `{"content": ["Must be 2000 or fewer in length."]}`, which becomes `Provider{code:"400"}` with the raw body as the message. That was the observed failure.
- Counting unit: Unicode scalar values (`chars()`), matching twilight. Both adapters' `content_len` debug fields log **bytes** (`str::len`).
- Embed-shape invariant: the embed carries only `description`, so the 6000-character total and the 256-character title limit are unreachable. That stops being true if a title, footer, or fields are ever added.

Keep these as regression inputs only. Do **not** generalize the following
Discord-fix-specific policies, which the spec itself scopes to Discord: scalar
counting, the truncation marker, the `MAX_LOCATION_LINE_CHARS` = 256
reservation, markup-unaware cutting (the spec warns it is **unsafe for Telegram
HTML**), and the receipt `truncated` metadata token set.

Related drift: `docs/research/platforms/discord.md` ("Embed Limits and
Validation") still contains the byte-slicing `truncate_for_embed` example that
the fix's R8 replaces. It is secondary evidence and must not be treated as a
constraint source.

## 12. Researched-but-not-emitted surfaces

From `messenger/docs/research/platforms/*.md`. (`email.md` is an empty file.)

| Platform | Researched surface (doc section) | Emitted by Messenger? |
|---|---|---|
| Discord | `tts`, `allowed_mentions`, `components`, `sticker_ids`, up to 10 embeds, embed `title`/`color`/`fields`/`footer`/`author` ("REST API Message Creation", "Embed Limits") | No. One description-only embed |
| Discord | webhook `username` and `avatar_url` ("Webhook-Based Messaging"), relevant to attribution | No |
| Discord | `message_reference.fail_if_not_exists` and cross-channel `channel_id`/`guild_id` ("Message Reply") | No (only the `message_id` reply) |
| Discord | interaction responses, ephemeral `flags: 64`, modals ("Interaction Responses"), relevant to interactivity | No (send-only) |
| Discord | silent delivery and embed suppression via message `flags` | **Not in the research doc**, and not emitted. A research gap |
| Slack | `blocks` (Block Kit) as the rich alternative to `text` ("Originating a Message", "Data Model") | No |
| Slack | Events API / Socket Mode receive, `app_mention` ("Responding to Messages") | No |
| Telegram | `entities`, `MarkdownV2`, legacy `Markdown` parse modes; `link_preview_options`; `reply_markup` (keyboards) ("Originating a Message") | No. HTML only, with the deprecated preview field |
| Telegram | `sendPhoto`/`Video`/`Document`/`Audio`/`Voice`, `sendMediaGroup`, `sendContact`, `sendPoll` | No. Only `sendLocation` |
| Telegram | `reply_parameters.chat_id`/`quote`/`allow_sending_without_reply`; edit/forward/copy/delete ("Replying to Messages") | No. Only `message_id` |
| WhatsApp | template, media, interactive, contacts, sticker, reaction, and mark-as-read messages; status webhooks ("Cloud API capabilities") | No. Only `text` and `location` |
| WhatsApp | `preview_url` (in the research data model) | No (hard-coded `None`) |
| WhatsApp | 24-hour customer-service window ("Important behavioral rule"), relevant to delivery eligibility | Not modeled. Surfaces only as a runtime `Provider` error |
| Signal | attachments, view-once, stickers, link previews, mentions, styled text (UTF-16 offsets), edits, reactions, receipts, typing, remote delete ("Originate a message", "Respond to another message", gotchas) | No. Only plain text plus a quote |

## 13. Implications for schema design

1. **Operations are content-dependent and mutually exclusive.** Telegram
   (`sendMessage` vs `sendLocation`) and WhatsApp (`type` text vs location) pick
   one operation per message and silently drop the rest. Bindings need an
   operation-selection rule, and the implementation projection must be able to
   record a "dropped without warning" outcome, not just implemented or missing.
2. **Summarized has three different fates**: split across two fields (Discord),
   the rich half kept and the summary dropped (Slack, Telegram), or the summary
   kept and the rich half dropped (WhatsApp, Signal). Slack natively supports
   the split (`text` as notification fallback plus `blocks`), but Messenger does
   not use it. The `fallback_for` relationship must be expressible per adapter
   binding, not per platform.
3. **Plain is not escaped.** The Discord and Slack Plain bindings submit text
   into a field that the platform parses as markup. A binding needs a flag like
   "raw text into a markup-parsing field" so that "plain" does not imply
   literal. Telegram Plain really is literal, because `parse_mode` is omitted.
4. **Native versus fallback location**: five adapters report
   `supports_location: true` for a text fallback. The implementation projection
   needs separate `native` and `messenger_fallback` values.
5. **Webhook response envelopes** (Slack plain text or JSON, Discord `wait=true`)
   and **bridge method names** (signal-cli `sendGroupMessage`) need
   operation-level envelope records with explicit version applicability.
   Bridge version is currently unrecorded.
6. **Receipts can be empty or zero** without an error (Slack `ts`/`channel`,
   WhatsApp `id`, Signal `timestamp`, and the Slack webhook `raw_id` is always
   empty). The receipt-semantics category should record whether the send
   returns an addressable ID, and whether Messenger validates that it is
   present.
7. **Warnings after sending do not exist in Messenger.** Slack `warning` and
   WhatsApp `message_status` have no destination. The diagnostic handoff should
   treat "successful response with a warning" as currently **lost everywhere**.

## Unverified items for research

- The Slack incoming-webhook response body format (plain text vs JSON) and whether it honors `thread_ts`.
- Whether signal-cli JSON-RPC exposes a `sendGroupMessage` method, and the shape of per-recipient `results`.
- Telegram's 429 transport: an HTTP 429 status and whether a `Retry-After` header accompanies the body's `parameters.retry_after`.
- The Discord API version used by twilight-http 0.17, and twilight's defaults for `allowed_mentions`, `fail_if_not_exists`, and 429 handling.
- Slack reply semantics when `thread_ts` is a reply's `ts` rather than the parent's.
- WhatsApp throttling error codes and their HTTP status.
