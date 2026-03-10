# Messenger Review

## Summary

The shared library shape is largely aligned with the design in [messenger-design.md](./messenger-design.md): `Message` stays destination-independent, `Dispatch` carries target/reply/options, and the provider-specific Markdown renderers follow the intended downgrade strategy well. The main gaps are in validation semantics, attachment delivery, and the CLI surface, where several advertised capabilities are either not implemented or not fully test-covered.

## Findings

### 1. Best-effort validation is looser than the design allows

The design says only formatting may downgrade in `BestEffort`; unsupported core content kinds should still fail, and attachment sources should be validated before send. The current `validate_dispatch()` only enforces capability checks in `Strict` mode and never validates attachment sources at all. See [messenger-design.md](./messenger-design.md#capability-and-validation-rules), [`../lib/src/validate.rs:19`](../lib/src/validate.rs), and [`../lib/src/tests/validation.rs:123`](../lib/src/tests/validation.rs).

Suggestions:

- Keep Markdown downgrade as the only best-effort relaxation.
- Always reject unsupported core content kinds such as attachments and location when a provider cannot actually deliver them.
- Add attachment source validation for at least `AttachmentSource::Path` existence/readability and obvious malformed provider/file-id cases before provider dispatch.

### 2. Attachment support is advertised but not implemented for the providers that claim it

This is the largest design drift. Discord, Signal, WhatsApp, and Telegram all advertise `supports_attachments: true`, but their send paths never consume `message.attachments`. Discord explicitly leaves uploads as a TODO. That means attachment-only messages can validate successfully and still be dropped or reduced to empty sends. See [messenger-design.md](./messenger-design.md#stage-1-deliverables), [messenger-design.md](./messenger-design.md#stage-2-deliverables), [`../lib/src/provider/discord.rs:51`](../lib/src/provider/discord.rs), [`../lib/src/provider/signal.rs:83`](../lib/src/provider/signal.rs), [`../lib/src/provider/whatsapp.rs:113`](../lib/src/provider/whatsapp.rs), and [`../lib/src/provider/telegram.rs:153`](../lib/src/provider/telegram.rs).

Suggestions:

- Either implement attachment sends per provider now, or set `supports_attachments` to `false` until the transport is real.
- Add provider-level tests that send `Message` values with actual attachments and assert the outbound multipart/JSON shape.
- Prevent empty fallback sends when the only user content is an ignored attachment.

### 3. The CLI exposes dead flags and is missing the receipt-based reply flow from the design

`--reply-to` and `--file` are accepted by clap and then explicitly discarded before `send_message()` is called. The CLI also prints only `raw_id` and does not persist typed `SendReceipt` data for later replies, which the design called out as part of the recommended implementation order. See [messenger-design.md](./messenger-design.md#recommended-first-implementation-order), [`../cli/src/main.rs:40`](../cli/src/main.rs), [`../cli/src/main.rs:48`](../cli/src/main.rs), [`../cli/src/main.rs:83`](../cli/src/main.rs), and [`../cli/src/main.rs:155`](../cli/src/main.rs).

Suggestions:

- Remove `--reply-to` and `--file` until implemented, or wire them end-to-end.
- Add a persisted receipt format so replies can round-trip through `MessageRef` instead of a raw string.
- If `--reply-to` stays provider-specific, make that explicit with parseable formats and provider-aware validation.

### 4. Signal and WhatsApp route configuration is internally inconsistent

The setup flow asks the user for extra env var names for Signal and WhatsApp, but `RouteConfig` can only store one env var field, and runtime ignores the extra answers anyway by hardcoding `SIGNAL_ACCOUNT` and `WHATSAPP_PHONE_NUMBER_ID`. See [messenger-design.md](./messenger-design.md#auth-and-configuration-model), [`../cli/src/config.rs:17`](../cli/src/config.rs), [`../cli/src/setup.rs:247`](../cli/src/setup.rs), and [`../cli/src/main.rs:242`](../cli/src/main.rs).

Suggestions:

- Expand `RouteConfig` so providers that need multiple secrets/config values can store them honestly.
- Do not ask setup questions whose answers are discarded.
- Normalize provider config so runtime resolution comes entirely from saved route config plus env lookup.

### 5. Test coverage is decent in the library, but it is not strong enough yet for the claimed surface

What is covered:

- `just test` passes for the default-feature package area.
- `cargo test -p messenger --all-features` passes and exercises the feature-gated Signal/WhatsApp/Telegram tests.

What is still weak:

- `messenger-cli` currently has zero tests.
- The default package-area test command does not cover the Stage 2 providers because `just test` runs `cargo test -p messenger` with default features only.
- Most provider wiremock tests assert method/path/status but not the actual serialized payload, so the tests do not prove that reply IDs, silent flags, link-preview flags, or Markdown transformations are actually being transmitted. See [`../justfile:146`](../justfile), [`../lib/Cargo.toml:25`](../lib/Cargo.toml), [`../lib/src/tests/slack_integration.rs:31`](../lib/src/tests/slack_integration.rs), [`../lib/src/tests/signal_integration.rs:28`](../lib/src/tests/signal_integration.rs), [`../lib/src/tests/telegram_integration.rs:31`](../lib/src/tests/telegram_integration.rs), and [`../lib/src/tests/whatsapp_integration.rs:29`](../lib/src/tests/whatsapp_integration.rs).

Suggestions:

- Change the package `just test` recipe to exercise `messenger` with `--all-features`.
- Add CLI tests around `resolve_route()`, `default_token_env()`, `build_target()`, and the setup/config round-trip.
- Strengthen provider tests with JSON body matchers for `thread_ts`, `reply_parameters`, quote fields, `disable_notification`, and link-preview controls.
- Add explicit tests for attachment validation and attachment sends once implemented.
- Add non-ignored unit-level coverage for Discord adapter behavior if possible; right now Discord is mostly covered only by generic coordinator tests and ignored smoke tests.

## Idiomatic API Suggestions

### Replace stringly provider handling in the CLI with a typed enum

`VALID_PROVIDERS`, `RouteConfig.provider: String`, and repeated `match route.provider.as_str()` branches make the CLI more error-prone than it needs to be. A typed provider enum backed by `clap::ValueEnum` and `serde` would remove duplicated string tables and make config/runtime wiring more idiomatic. See [`../cli/src/main.rs:12`](../cli/src/main.rs) and [`../cli/src/config.rs:17`](../cli/src/config.rs).

### Prefer typed config over overloaded `token_env`

For Signal in particular, `token_env` is not a token at all; it is the RPC URL env var. That is a sign the config model is carrying provider-specific meaning in a misleading generic field. A typed route config per provider, or at least a provider-specific config enum, would be clearer and more idiomatic than overloading one `String`.

### Tighten the public data model for comparison/persistence use cases

If `SendReceipt` and `MessageRef` are meant to be stored and replayed, consider deriving additional comparison traits where practical and adding parse/format helpers for provider refs. The current types are mostly `Debug + Clone`, which is workable, but not especially ergonomic for persistence or test assertions. See [`../lib/src/receipt.rs:26`](../lib/src/receipt.rs), [`../lib/src/target.rs:1`](../lib/src/target.rs), and [`../lib/src/message.rs:5`](../lib/src/message.rs).

## Performance Opportunities

### Parse Markdown once per send fan-out

The design proposed a parse-once internal rich-text pipeline, but `render_for_provider()` reparses Markdown every time it is called. If the same `Message` is sent to multiple providers or multiple targets on the same provider, that work repeats unnecessarily. See [messenger-design.md](./messenger-design.md#internal-render-pipeline) and [`../lib/src/markdown/mod.rs:11`](../lib/src/markdown/mod.rs).

Suggestions:

- Parse Markdown once into the internal AST and reuse it across provider renders.
- If you want to preserve the current public API, consider lazy parsing/caching inside an internal send context instead of changing `MessageBody`.

### Make `send_many()` concurrent with provider-aware limits

`Messenger::send_many()` currently awaits each send serially. For fan-out sends across independent targets, that turns latency into a linear sum of network round trips. See [`../lib/src/provider/mod.rs:73`](../lib/src/provider/mod.rs).

Suggestions:

- Use bounded concurrency with `FuturesUnordered` or `join_all`.
- Keep throttling at the provider layer so Slack/Telegram-specific rate limits still remain explicit and centralized.

## Recommended Next Steps

1. Fix validation semantics so best-effort only downgrades formatting, not core content.
2. Implement attachment sends or lower the advertised capabilities until they exist.
3. Either implement or remove the CLI’s `--reply-to` and `--file` options.
4. Make route configuration honest for Signal and WhatsApp.
5. Upgrade test coverage by running all features in `just test`, adding CLI tests, and asserting outbound payload bodies instead of only endpoints/status codes.
