# Code Review: Type Safety, DRY, and Test Coverage

**Date:** 2026-04-03
**Scope:** `messenger/lib` + `messenger/cli`
**Lines reviewed:** ~5,500 Rust source + ~1,300 test lines

---

## 1. Type Safety Opportunities

### 1.1 Dual provider-kind enums (High Impact, Low Effort)

`ProviderKind` (`lib/src/receipt.rs:9-15`) and `RouteProvider` (`cli/src/config.rs:19-32`) represent the same five providers but as separate types. They share `Display` impls, serde naming, and iteration logic, and must be kept in sync manually.

**Recommendation:** Unify into a single shared enum. Either re-export `ProviderKind` from the library (preferred) or extract both into a shared crate-level type. The CLI's `RouteProvider` adds `ValueEnum` (clap) — handle this with a newtype or a `#[cfg(feature = "cli")]` impl block. This eliminates an entire class of "added a provider to the library but forgot the CLI enum" bugs.

**Ergonomic cost:** None — the types are isomorphic.

### 1.2 Body-rendering match is repeated per provider and not exhaustive (High Impact, Medium Effort)

Four providers (Discord, Slack, Signal, WhatsApp) repeat this body rendering block almost verbatim:

```rust
let text = match message.body() {
    Some(MessageBody::Plain(_)) | Some(MessageBody::Markdown(_)) => {
        message.render_body_with_location(ProviderKind::X)
    }
    None if message.location().is_some() => {
        message.render_body_with_location(ProviderKind::X)
    }
    None => String::new(),
};
```

If a new `MessageBody` variant is added (e.g., `Html(String)`), the compiler won't flag that every provider needs updating because `_` patterns or `Some(_)` catch-alls silently absorb new variants.

**Recommendation:** Add a method on `PreparedMessage` that encapsulates this logic:

```rust
pub fn render_body_or_location(&self, provider: ProviderKind) -> String
```

Make the internal match exhaustive (no wildcard `_`) so the compiler forces every provider to handle new body variants. The providers then become a one-liner.

**Ergonomic cost:** Minor — providers call `message.render_body_or_location(kind)` instead of a 10-line match.

### 1.3 Stringly-typed identifiers validate too late (Medium Impact, Medium Effort)

Discord channel IDs and message IDs are `u64` but stored as `String` in `DiscordTarget::channel_id` (`target.rs:20-21`) and `MessageRef::Discord::message_id` (`receipt.rs:33-36`). Parsing happens in `DiscordProvider::parse_channel_id` / `parse_message_id` at send time.

This means a caller can construct `Target::discord_channel("not-a-number")` and only discover the error after validation, normalization, and markdown rendering are all complete.

**Recommendation:** Either:

- (A) Store channel/message IDs as `u64` in `DiscordTarget` and `MessageRef::Discord`, with `From<u64>` + `FromStr` for construction. Or
- (B) Keep `String` in the public API but validate eagerly in `validate_dispatch` using a new `validate_target_consistency` function, before any rendering work begins.

Option A is more type-safe but requires changes to the CLI config layer (which reads strings from JSON). Option B is lower-risk and aligns with the existing validation architecture.

### 1.4 Empty `ProviderOverrides` structs provide no type-safety (Low Impact, Low Effort)

All five override structs (`DiscordOverrides`, `SlackOverrides`, etc.) are empty (`dispatch.rs:54-72`). The `ProviderOverrides` enum dispatches on provider kind but carries no actual data.

**Recommendation:** This is fine for forward-compatibility. Just add a `// TODO: populate when provider-specific options are needed` comment to each, and a single test that confirms `ProviderOverrides::None` round-trips through `Dispatch`.

### 1.5 `AttachmentSource` is not validated per-provider in the type system (Medium Impact, Medium Effort)

Discord rejects `Url` and `ProviderFileId` sources, but this is only checked inside `DiscordProvider::build_attachment` (`provider/discord.rs:61-69`). Other providers silently ignore unsupported sources or fail at send time.

**Recommendation:** Add an `allowed_sources()` method to `CapabilitySet` (or a new field) that returns a `&[AttachmentSourceKind]`. Validate during `normalize_dispatch` in best-effort mode, dropping disallowed sources with a warning, or erroring in strict mode. This turns a runtime discovery into a validation-phase guarantee.

### 1.6 `Message::body` as `Option<MessageBody>` leaks into every consumer (Low Impact)

Every call site does `match message.body() { Some(...) => ..., None => ... }`. The `None` case is only reached for location-only messages, yet it's handled as a first-class branch everywhere.

**Recommendation:** Not worth changing now. The `Option` accurately models the domain (a message can be body-less). The real improvement is #1.2 above, which consolidates the pattern.

---

## 2. DRY Opportunities

### 2.1 Transport error mapping macro/helper (High Impact, Low Effort)

The pattern `map_err(|e| MessengerError::Transport { provider: ProviderKind::X, message: e.to_string() })` appears **23 times** across the five providers (discord.rs, slack.rs, signal.rs, whatsapp.rs, telegram.rs). Telegram alone has 5 occurrences.

**Recommendation:** Add a private helper method or macro:

```rust
macro_rules! transport_err {
    ($provider:expr, $e:expr) => {
        MessengerError::Transport {
            provider: $provider,
            message: $e.to_string(),
        }
    };
}
```

Or a method on `ProviderKind`:

```rust
impl ProviderKind {
    fn transport_error(self, message: impl fmt::Display) -> MessengerError { ... }
}
```

### 2.2 Server error / rate-limit response handling (Medium Impact, Low Effort)

Slack, WhatsApp, and Signal all repeat the same HTTP response handling pattern:

```rust
if status.as_u16() == 429 {
    return Err(MessengerError::RateLimited { provider, ... });
}
if status.is_server_error() {
    return Err(MessengerError::Transport { provider, ... });
}
let resp: T = response.json().await.map_err(|e| MessengerError::Transport { ... })?;
```

**Recommendation:** Extract a `fn handle_http_response<T: DeserializeOwned>(response: reqwest::Response, provider: ProviderKind) -> Result<T, MessengerError>` helper. Telegram already partially does this with `post_request` — generalize and share it.

### 2.3 Markdown renderer `render_inline` + paragraph spacing (Medium Impact, Medium Effort)

All four markdown renderers (`discord.rs`, `slack_mrkdwn.rs`, `telegram_html.rs`, `plain_text.rs`) duplicate:

- The `render_inline(nodes) -> String` helper (identical logic in all four)
- The `top_level && i + 1 < nodes.len()` paragraph-spacing guard

**Recommendation:** Extract `render_inline` into a shared function in `markdown/mod.rs`:

```rust
fn render_inline(nodes: &[RichNode], render_fn: impl Fn(&mut String, &[RichNode], bool)) -> String { ... }
```

For paragraph spacing, add a `should_add_paragraph_spacing(index: usize, total: usize, top_level: bool)` helper.

### 2.4 `RouteConfig` ↔ `RouteConfigRepr` conversions (Medium Impact, Medium Effort)

`config.rs` has three `From` impls (`RouteConfigRepr -> RouteConfig`, `RouteConfig -> RouteConfigRepr`, `LegacyRouteConfig -> RouteConfig`) totaling **~100 lines** of mechanical field-by-field mapping. Adding a provider requires touching all three.

**Recommendation:** Consider using a derive macro or a `#[serde(from/into)]` attribute to reduce this. Alternatively, just use `RouteConfigRepr` directly (with `#[serde(tag = "provider")]`) and add the `provider()` method there, eliminating `RouteConfig` entirely. The legacy migration can be a single `TryFrom<serde_json::Value>`.

### 2.5 Provider `capabilities()` as hardcoded struct literals (Low Impact, Low Effort)

Each provider has a `capabilities()` method returning a struct literal. These are stable and don't change at runtime.

**Recommendation:** Use `const` blocks or a `const fn` constructor. Example:

```rust
const SLACK_CAPABILITIES: CapabilitySet = CapabilitySet {
    supports_markdown_rendering: true,
    supports_reply: true,
    supports_attachments: false,
    supports_location: true,
    supports_silent_delivery: false,
    supports_link_preview_control: true,
};
```

This makes it clear capabilities are compile-time constants and enables future `const` assertions.

---

## 3. Test Coverage Gaps

### 3.1 Missing library tests

| Area | File(s) | Gap |
|------|---------|-----|
| `Location::format_text_line` | `message.rs:38-46` | None of the 4 branches are tested directly. Only implicitly covered if at all. |
| `PreparedMessage::render_body_with_location` | `prepared.rs:51-59` | No direct unit test. Tested only indirectly via integration tests. |
| `PreparedMessage::render_body_for_provider` | `prepared.rs:63-74` | The `(Some(Markdown(_)), None)` fallback branch is untested. |
| `MessageRef::provider_kind` | `receipt.rs:58-66` | No direct test for each variant. |
| `SendReceipt` / `MessageRef` JSON round-trip | `receipt.rs:69-76, 114-121` | `from_json_str` / `to_pretty_json` are not tested for any variant. |
| `CompatibilityWarning::Display` | `validate.rs:29-36` | Only tested in CLI, not in lib. Should have a lib-level test. |
| `target_provider_kind` | `validate.rs:297-311` | No direct test for each target kind. |
| `validate_attachment_source` edge cases | `validate.rs:233-293` | URL with whitespace, empty bytes filename, ProviderFileId with newlines — only the last is tested. |
| `Messenger::plan_send` with warnings | `provider/mod.rs` | `plan_send` is not tested directly — only `send` and `send_many` are. |
| `Messenger::send_planned` | `provider/mod.rs:125-139` | Not tested directly (always reached via `send`). |
| `send_many` with mixed success/failure | `provider/mod.rs:264-278` | Current test uses a failing provider for all dispatches. No test for mixed results. |
| Markdown edge cases | `markdown/` | No tests for: nested formatting (`***bold italic***`), empty input, consecutive hard breaks, deeply nested lists, heading levels > 2, heading in Discord renderer. |
| `DiscordProvider` send flow | `provider/discord.rs` | No integration test with mocked HTTP (unlike Slack/Telegram/Signal/WhatsApp which all have wiremock tests). Discord uses twilight-http which can't use wiremock directly — but a trait-based abstraction could enable testing. |
| `ProviderOverrides` | `dispatch.rs` | Zero test coverage. Empty structs, but the enum dispatch in `send_prepared` is untested. |
| `CapabilitySet::all()` / `CapabilitySet::none()` | `capabilities.rs` | No test confirming all-true / all-false. |

### 3.2 Missing CLI tests

| Area | File | Gap |
|------|------|-----|
| `parse_location` | `main.rs:466-482` | Not tested at all. Edge cases: extra whitespace, negative values, trailing comma. |
| `resolve_secret` | `main.rs:451-463` | Only the happy path is tested indirectly. No test for when env var is missing. |
| `register_provider` | `main.rs:338-418` | Not tested. The `match` on `RouteConfig` variants is large and could silently miss a new provider. |
| `build_target` edge cases | `main.rs:420-448` | Only Signal phone and Telegram username are tested. Missing: WhatsApp, Discord, Telegram numeric ID, Signal group. |
| `receipt_store::save_receipt` | `receipt_store.rs:13-42` | Not tested. `load_message_ref` has one test but only for the MessageRef JSON path. |
| `receipt_store::load_message_ref` from file | `receipt_store.rs:44-54` | The file-path branch (line 47-51) is not tested. |
| `receipt_store::load_message_ref` from `StoredReceipt` | `receipt_store.rs:57-60` | Not tested. |
| `receipt_store::load_message_ref` from `SendReceipt` | `receipt_store.rs:62-65` | Not tested. |
| Config loading from missing file | `config.rs:428-449` | Not tested — should return default. |
| Config `routes_for_provider` | `config.rs:481-487` | Not tested. |
| Setup flow | `setup.rs` | Entirely untested (interactive — understandable, but validation functions like `suggest_route_name` and `non_empty` could be unit tested). |

### 3.3 Integration test gaps

| Gap | Detail |
|-----|--------|
| Signal note-to-self | No wiremock test for `SignalTarget::NoteToSelf` (signal_integration.rs). |
| Telegram with thread_id | No test for `TelegramTarget { thread_id: Some(...) }` (telegram_integration.rs). |
| Telegram location + reply | No test for sending a location with reply_to (telegram_integration.rs covers them separately). |
| WhatsApp location with name/address | Only bare lat/lon is tested, not named locations (whatsapp_integration.rs). |
| WhatsApp non-190 provider errors | Only auth (code 190) is tested. A generic error should also be tested. |
| Discord smoke test | Only smoke test for Discord exists (`lib/tests/integration.rs`), no wiremock unit tests at all. |
| Cross-provider reply mismatch | Only one combination (Discord target + Slack reply) is tested. No test for e.g. Telegram target + WhatsApp reply. |

---

## Summary Priority Matrix

| # | Item | Category | Impact | Effort | Priority |
|---|------|----------|--------|--------|----------|
| 1.1 | Unify `ProviderKind` / `RouteProvider` | Types | High | Low | **P0** |
| 2.1 | Transport error helper/macro | DRY | High | Low | **P0** |
| 1.2 | Exhaustive body rendering method | Types | High | Medium | **P0** |
| 2.2 | HTTP response handling helper | DRY | Medium | Low | **P1** |
| 1.3 | Eager target validation | Types | Medium | Medium | **P1** |
| 3.1 | `PreparedMessage` + `Location` unit tests | Tests | Medium | Low | **P1** |
| 3.1 | `MessageRef` / `SendReceipt` JSON round-trip | Tests | Medium | Low | **P1** |
| 3.2 | `parse_location` + `resolve_secret` CLI tests | Tests | Medium | Low | **P1** |
| 3.3 | Discord wiremock integration tests | Tests | Medium | Medium | **P1** |
| 1.5 | Per-provider attachment source validation | Types | Medium | Medium | **P2** |
| 2.3 | Markdown renderer dedup | DRY | Medium | Medium | **P2** |
| 2.4 | `RouteConfig` conversion dedup | DRY | Medium | Medium | **P2** |
| 2.5 | Const capability sets | DRY | Low | Low | **P3** |
| 1.4 | Empty overrides comment + test | Types | Low | Low | **P3** |
| 1.6 | `Option<MessageBody>` reconsideration | Types | Low | High | **P3** |
