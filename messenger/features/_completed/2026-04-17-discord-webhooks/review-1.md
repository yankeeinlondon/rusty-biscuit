---
feature: discord-webhooks
spec: spec.md
plan: plan.md
reviewer: claude (messenger skill)
review_date: 2026-04-19
ready: false
---

# Discord Webhook Provider — Review 1

## Overall Assessment

Implementation tracks the spec closely and all six phases of the plan have
landed. The full workspace test suite (`cargo test -p messenger --lib`,
`cargo test -p messenger-cli --bins`) is green, including 9 new webhook
integration tests and 5 new CLI config/resolution tests. Capability reporting
matches the spec, the plan-time hard-error for `reply_to` fires in both
strict and best-effort modes, and the CLI-level `RouteConfig::DiscordWebhook`
round-trips through serde with the expected defaults.

That said, there is one correctness concern (thread-id handling) and several
meaningful test-coverage gaps that justify holding this out of production
until a quick follow-up pass.

## Spec → Implementation Coverage

| Spec requirement | Status | Location |
|------------------|--------|----------|
| `ProviderKind::DiscordWebhook` | ✅ | `lib/src/receipt.rs:14,33-34,55` |
| `Target::DiscordWebhook { thread_id: Option<String> }` | ✅ | `lib/src/target.rs:7,30-34,92-104` |
| `MessageRef::DiscordWebhook` | ✅ | `lib/src/receipt.rs:72-78,103` |
| `DiscordWebhookProvider` + `DiscordWebhookConfig` | ✅ | `lib/src/provider/discord_webhook.rs:21-38` |
| Capability set honest (reply:false, attachments:true, markdown:true, location:true, silent:false, link-preview:false) | ✅ | `lib/src/provider/discord_webhook.rs:197-207` |
| Plan-time reject `reply_to` as `UnsupportedFeature{feature:"replies"}` before any network call | ✅ | `lib/src/validate.rs:83-93` |
| Bot provider unchanged | ✅ | `lib/src/provider/discord.rs` untouched |
| `RouteConfig::DiscordWebhook { webhook_url, webhook_url_env }` (no `channel_id`) | ✅ | `cli/src/config.rs:77-80` |
| `RouteProvider::DiscordWebhook` with clap `value_name="discord-webhook"` | ✅ | `cli/src/config.rs:24-26` |
| Default env var `DISCORD_WEBHOOK_URL` | ✅ | `cli/src/config.rs:435-437` |
| Setup flow for webhook URL with direct vs env-var choice | ✅ | `cli/src/setup.rs:174-212` |
| Config round-trip (both, env-only, default, neither) | ✅ | `cli/src/config.rs:561-606` (4 tests) |
| URL resolution priority (direct > env > error) | ✅ | `cli/src/main.rs:582-623` (3 serial-gated tests) |
| Wiremock integration POSTs to webhook endpoint | ✅ | `lib/src/tests/discord_webhook_integration.rs` |
| Existing Discord bot tests unchanged & green | ✅ | `cargo test -p messenger --lib` shows 101 passing |
| Docs updated (user-guide, SKILL.md, providers.md, cli-reference.md, READMEs) | ⚠️ Partial — see Finding 7 | uncommitted working-tree edits |

All spec acceptance criteria trace to concrete tests. Nothing critical is
missing from the design — the gaps below are about hardening and rigor.

## Findings

### Correctness

**1. Thread-ID URL injection (medium severity)** — `lib/src/provider/discord_webhook.rs:249-252`

```rust
if let Some(thread) = thread_id.as_deref() {
    url.push_str("&thread_id=");
    url.push_str(thread);
}
```

`thread_id` is raw-concatenated into the request URL without any
validation or percent-encoding. `DiscordWebhookTarget.thread_id` is
`Option<String>` — nothing in the type or in `normalize_dispatch`
enforces that it is a numeric snowflake. A value containing `&`, `#`,
`?`, or whitespace would produce a malformed request or inject
unintended query parameters (e.g. `thread_id=123&wait=false`).

Fix options (pick one):

- Validate at the `Target` constructors (`discord_webhook_thread`) that
  `thread_id` matches `^[0-9]+$`, rejecting at dispatch-build time.
- Percent-encode `thread` when appending to the URL (e.g. via
  `url::form_urlencoded::byte_serialize`).
- Add a `parse_thread_id_u64` helper on the provider mirroring
  `DiscordProvider::parse_channel_id`, and fail with
  `MessengerError::InvalidMessage` at `send_prepared` time.

Recommend option 3 — it matches the bot-provider pattern (`discord.rs`
already does numeric validation for channel/message ids) and keeps the
parse-once error surface close to the send.

**2. Potential webhook-token exposure via `base_url`** — `lib/src/provider/discord_webhook.rs:31-38`

`DiscordWebhookProvider.base_url` stores the full URL *including the
token path segment*. Today no code logs `self.base_url`, but any
future instrumentation that records the request URL at error time
(for example, adding `%url` to the `tracing::warn!(error = %e, ...)`
on line 289) would leak the token into logs.

Low probability of accident today, but worth structuring defensively.
Two options:

- Split into `base_url_prefix: String` + `token: SecretString` and
  reassemble the URL only at send time.
- Keep the current layout but mark `base_url` `pub(self)` and add a
  doc-comment flagging the leak risk.

Related: `ParsedWebhook.token` is captured but marked
`#[allow(dead_code)]` at line 46 and never read again. If we adopt
option 1 the field becomes meaningful; otherwise it should be
deleted.

### Test coverage gaps

**3. No assertion on the JSON POST body** —
`lib/src/tests/discord_webhook_integration.rs:137-172`

`sends_text_message_with_expected_payload` only matches on method,
path, query param, and `Content-Type: application/json`. The spec
explicitly calls for "the expected payload shape". Replace the
`header("Content-Type", ...)` matcher with `body_partial_json` or
`body_json`:

```rust
.and(body_partial_json(serde_json::json!({ "content": "hello" })))
```

This protects against regressions in
`WebhookJsonBody` serialization (field rename, extra fields, etc.).

**4. No integration test for the multipart attachments path** —
`lib/src/provider/discord_webhook.rs:261-285`

The provider has two distinct code paths (`if attachments.is_empty()`
vs. multipart with `payload_json` + `files[i]` parts), but every
existing integration test covers only the JSON path. Add a test with
a `Bytes`-backed attachment:

- Assert the request is `multipart/form-data`.
- Assert a `payload_json` part exists and contains the expected
  `content` + `attachments` metadata (id=0, filename, description).
- Assert a `files[0]` part carries the attachment bytes.

Wiremock supports multipart matching via `body_string` plus
substring checks, or by matching the content-type prefix. This is the
largest blind spot in the test suite right now.

**5. No `MessengerError::Provider` (generic 4xx) test** —
`lib/src/provider/discord_webhook.rs:312-324`

The `if status.is_client_error() && status != TOO_MANY_REQUESTS`
branch maps 400/403/404/etc. into `MessengerError::Provider`. Tests
cover only 200, 401, 429, 500 — the generic 4xx branch is unexercised.
Add a test that returns 400 with a Discord-style error body
(`{"message": "Invalid Form Body", "code": 50006}`) and verifies
`MessengerError::Provider { code: Some("400"), .. }`.

**6. No test for the `webhook_id` fallback** —
`lib/src/provider/discord_webhook.rs:331`

`msg.webhook_id.unwrap_or_else(|| self.webhook_id.clone())` is dead
code against every current test (all responses include
`webhook_id`). If Discord ever omits it, we silently fall back to
the constructor-parsed value. Add a test that returns a response
without the `webhook_id` field and asserts the receipt still contains
the provider's own `webhook_id`.

**7. No test that Markdown body actually reaches the wire as
Markdown** — integration tests always use `Message::text("hello")`.
A small additional test using `Message::markdown("**bold**")` would
exercise `render_body_with_location(ProviderKind::DiscordWebhook)` in
an end-to-end-shaped path and confirm the renderer is wired into the
webhook provider (not just the bot provider).

### Code ergonomics / performance

**8. Attachment builder duplication** —
`discord_webhook.rs::build_part` vs `discord.rs::build_attachment`

Both functions do the same 4-source match (Path/Bytes/Url/ProviderFileId)
with nearly identical error messages. Plan.md explicitly allowed
copy-paste to avoid coupling, so this is by design, but a tiny
private helper returning `(filename, bytes)` from `&AttachmentSource`
(parameterized by a provider-name string for error wording) would
eliminate the duplication without creating trait coupling. Put it in
the existing `provider/http_helpers.rs` or a new
`provider/attachment_helpers.rs`.

**9. Two-pass multipart build** —
`discord_webhook.rs:262-280`

```rust
let mut metas: Vec<AttachmentMeta<'_>> = Vec::with_capacity(...);
let mut owned: Vec<(String, Option<String>)> = Vec::with_capacity(...);
for (index, attachment) in attachments.iter().enumerate() {
    let (filename, part) = Self::build_part(attachment)?;
    let description = attachment.alt_text.clone().or_else(|| attachment.caption.clone());
    owned.push((filename, description));
    form = form.part(format!("files[{index}]"), part);
}
for (index, (filename, description)) in owned.iter().enumerate() {
    metas.push(AttachmentMeta { id: index as u64, ... });
}
```

The two-pass shape exists to satisfy the borrow checker
(`AttachmentMeta` holds `&str`, needs an owning buffer). This works
but is awkward and allocates an extra `Vec`. Two cleaner options:

- Change `AttachmentMeta` to own its strings (`String` instead of
  `&'a str`), pay one `String` clone per attachment, drop the `owned`
  buffer entirely.
- Build `metas` from `owned` in the same loop using `metas.push(...)`
  after `owned.push(...)` and use indexed access — same shape but
  a single pass.

Low priority; correctness is fine.

**10. `build_part` is an inherent `fn` rather than a free function**

It takes no `&self`. Moving it to module scope (or the shared helper
suggested in #8) reduces `impl` noise.

**11. Hand-rolled 401/403 + 4xx branches before `handle_http_response`** —
`discord_webhook.rs:296-324`

The provider short-circuits 401/403 and non-429 4xx *before*
delegating to `handle_http_response`, which itself handles 429 + 5xx.
Three overlapping code paths is harder to follow than one. Consider
extending `http_helpers` with a single helper that returns
`Result<Response, MessengerError>` after classifying auth/rate/5xx
errors, letting the caller then do `.json::<T>()`. This would unify
patterns across Slack, Telegram, WhatsApp, Signal, and the webhook.
Out of scope for this feature, but worth tracking as follow-up.

**12. `DiscordWebhookOverrides` is empty + public** —
`lib/src/dispatch.rs:61-62`

Added per spec for symmetry. It has zero consumers today. Consider
`#[non_exhaustive]` so future fields aren't a breaking change. Low
priority.

**13. `DiscordWebhookProvider::new` panic vs `DiscordProvider::new`
infallibility** — `lib/src/provider/discord_webhook.rs:76-78`

The doc says `new` "Mirrors `DiscordProvider::new` for ergonomic
parity." But `DiscordProvider::new` is *infallible* (it wraps
`Client::new(String)`), while the webhook `new` panics on malformed
URLs. These aren't actually parallel. Either:

- Make the only public constructor fallible (remove `new`, keep
  `try_new`), OR
- Update the doc to say "panicking convenience constructor; prefer
  `try_new` in any code path that accepts user-supplied URLs".

Given that CLI config already goes through `new` at
`cli/src/main.rs:361-366` with a user-supplied URL, a better-formed
webhook URL validation error in the CLI error path would be friendlier
than a panic. Switching the CLI to `try_new` is a one-line change
and would surface a clean `MessengerError::InvalidMessage` in the
`register_provider` `Result` chain.

### Documentation & repo hygiene

**14. Uncommitted documentation edits**

`git status` at the start of this review shows these files modified but
not committed:

- `.claude/skills/messenger/SKILL.md`
- `.claude/skills/messenger/providers.md`
- `.claude/skills/messenger/cli-reference.md`

These contain the Phase 6 doc updates (provider table row, capability
narrative, CLI `--provider discord-webhook` examples). Until committed,
the skill context shipped to future Claude sessions won't reflect the
new provider. These should land in a commit before the feature is
closed.

**15. Unrelated drive-by edit in `biscuit-terminal`**

`biscuit-terminal/lib/src/discovery/detection.rs` has a cosmetic
match-guard rewrite that is unrelated to this feature:

```rust
- | "rxvt-unicode-256color" => {
-     // These may or may not support extended underlines...
-     if has_basic_underline {
-         return basic_only();
-     }
- }
+ | "rxvt-unicode-256color"
+     if has_basic_underline =>
+ {
+     return basic_only();
+ }
```

Functionally equivalent, but it doesn't belong in this feature. Revert
here and land it separately if wanted.

**16. `markdown-rendering.md` skill doc not updated**

Not called out in the plan, but since the webhook provider reuses the
Discord markdown renderer, a short note in
`.claude/skills/messenger/markdown-rendering.md` confirming that both
Discord adapters share the renderer pipeline would close a minor
documentation hole. Low priority.

## Minor observations

- **`parse_webhook_url` is version-agnostic** — it accepts any path
  that contains `/webhooks/{id}/{token}`, so `/api/v999/webhooks/...`
  parses fine. This is intentional future-proofing; no change needed.
- **Attachment MIME type not forwarded to multipart** — `Part::bytes`
  is used without `.mime_str()`. Discord may infer from filename; if
  it doesn't, callers passing `AttachmentSource::Bytes { mime_type, .. }`
  lose that information. Compare with how `DiscordProvider` uses
  `twilight-model`'s `Attachment::from_bytes` (also doesn't set a
  mime). Probably fine, but worth confirming.
- **`webhook_url_env` applies even when `webhook_url` is set** — the
  setup flow always stores `webhook_url_env = "DISCORD_WEBHOOK_URL"`
  if the user provided a direct URL (line 205). On round-trip this is
  preserved. Harmless, but slightly noisy in serialized configs.
  Matches the other providers, so consistent.

## Recommendation

**Not ready for production (`ready: false`)**.

The feature is functionally complete and the happy path is well
tested, but:

1. **Thread-ID injection** (Finding 1) is a real correctness bug,
   even if Discord snowflakes are numeric in practice. Type-level or
   parse-time validation is a one-commit fix.
2. **No multipart integration test** (Finding 4) leaves the
   attachments-over-webhook code path unexercised end-to-end, and
   attachments are an advertised capability.
3. **Uncommitted skill documentation** (Finding 14) should land before
   the branch merges; it's half the spec's Documentation Updates
   Required section.

Fold in Findings 1, 4, and 14 — and ideally 3, 5, and 6 — for a clean
production cut. Findings 8, 9, 10, 11 are polish that can wait.
