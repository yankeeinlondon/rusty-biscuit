# Messaging Review

## Findings

1. [P2] Image-only notifications on non-Discord routes currently degrade into blank sends instead of being skipped.

   `build_payload()` constructs `Message::text("")` whenever the interpolated text is blank, then only attaches the image for Discord and merely warns for Slack/Signal/WhatsApp ([`claudine/lib/src/messaging/send.rs:139`](../../lib/src/messaging/send.rs), [`claudine/lib/src/messaging/send.rs:146`](../../lib/src/messaging/send.rs)). In `messenger`, an empty-string body is still considered a non-empty message because `is_empty()` only checks whether `body` is `None` ([`messenger/lib/src/message.rs:123`](../../../messenger/lib/src/message.rs)), so providers such as Slack will go on to POST an empty `text` field ([`messenger/lib/src/provider/slack.rs:95`](../../../messenger/lib/src/provider/slack.rs)). That does not match the design requirement to ignore unsupported images and "send the text message normally" only when text exists ([`tech-design.md:146`](./tech-design.md)). For image-only actions targeting Slack/Signal/WhatsApp, Claudine should warn and skip rather than emit a blank outbound message or provider-side failure.

2. [P2] Provider route configs do not reject unexpected fields, even though the design requires strict parsing.

   The design explicitly calls for `MessagingRouteConfig` to use `deny_unknown_fields` ([`tech-design.md:278`](./tech-design.md)), but the implementation only uses `#[serde(tag = "provider", rename_all = "lowercase")]` ([`claudine/lib/src/messaging/config.rs:39`](../../lib/src/messaging/config.rs)). That means typos such as `bot_toke_env` or unsupported legacy fields can be silently ignored during deserialization, often falling back to defaults instead of surfacing a config error. For a credentials-heavy feature, that makes misconfiguration much harder to diagnose and is a direct mismatch with the approved data model.

3. [P3] Library documentation is stale after adding the new `Message` action.

   The tech design called out a same-change update to `claudine/lib/README.md`, but the README still says `HookAction` has six variants ([`claudine/lib/README.md:32`](../../lib/README.md)) and the action execution table still omits `Message` entirely ([`claudine/lib/README.md:264`](../../lib/README.md)). The runtime behavior is implemented, so the docs now misrepresent the public action surface for downstream users reading the library README.

## Verification

Ran `cargo test -p claudine messaging:: -- --nocapture`.
