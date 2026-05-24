---
phases: 5
created: 2026-05-09
start_phase: 1
source_files_during_phase_1:
  - messenger/lib/src/message.rs
  - messenger/lib/src/prepared.rs
  - messenger/lib/src/validate.rs
  - messenger/lib/src/tests/builders.rs
  - messenger/lib/src/tests/validation.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - messenger/lib/src/provider/apns.rs
  - messenger/lib/src/provider/fcm.rs
  - messenger/lib/src/provider/telegram.rs
  - messenger/lib/src/tests/builders.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - messenger/lib/src/provider/discord.rs
  - messenger/lib/src/tests/mod.rs
  - messenger/lib/src/tests/discord_integration.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - messenger/lib/src/provider/discord_webhook.rs
  - messenger/lib/src/tests/discord_webhook_integration.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - messenger/cli/src/main.rs
  - messenger/cli/tests/snapshots/snapshots__help_output.snap
  - messenger/docs/user-guide.md
docs_updated_during_phase_5:
  - messenger/docs/user-guide.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .opencode/skill/messenger/SKILL.md
packages:
  - messenger
  - messenger-cli
---

# Execution Plan: Notification-Aware Message Bodies

## Summary

Add a `MessageBody::Summarized { summary, markdown }` variant plus `Message::markdown_stripped` sugar so callers can express three notification-aware shapes (single string, summary+rich, auto-stripped). Wire the new variant through the rendering pipeline (`PreparedMessage`), update the five direct match sites (`prepared`, `validate`, `apns`, `fcm`, `telegram`), and split Discord/Discord-webhook output between `content` (notification surface) and an embed `description` (rich surface). CLI gains `--summary` and `--strip-markdown` flags. No breaking changes — existing `Plain` and `Markdown` variants behave exactly as today.

The work is phased so each phase produces an independently mergeable, fully-tested slice. Phase 1 lands the type and pipeline. Phase 2 covers all non-Discord providers that match on `MessageBody` directly. Phase 3 lights up the Discord bot provider (the user-visible payoff). Phase 4 brings Discord webhook to parity. Phase 5 exposes the feature to the CLI and updates docs.

---

## Phase 1 — Core Type + Rendering Pipeline

**Goal**: Add `MessageBody::Summarized`, supporting constructors, `PreparedMessage::render_summary` / `render_rich` helpers, and update `validate.rs` to recognize the new variant. No provider-visible behavior change yet (every existing provider continues through `render_body_for_provider` which falls back to existing variants).

### Step 1.1 — Add the `Summarized` variant and constructors

- **Files**: `messenger/lib/src/message.rs`
- **Action**: Extend `MessageBody`:

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum MessageBody {
      Plain(String),
      Markdown(String),
      /// Plain summary paired with rich Markdown body.
      ///
      /// Providers with a notification surface distinct from a rich-rendering
      /// surface (Discord) place `summary` in the notification field and render
      /// `markdown` in the rich field. Providers without that split fall back
      /// to whichever half is appropriate for their feature set.
      Summarized { summary: String, markdown: String },
  }
  ```

  Add two constructors on `Message`:

  ```rust
  /// Create a message with a plain notification summary and a rich Markdown body.
  pub fn summarized(
      summary: impl Into<String>,
      markdown: impl Into<String>,
  ) -> Self {
      Self {
          title: None,
          body: Some(MessageBody::Summarized {
              summary: summary.into(),
              markdown: markdown.into(),
          }),
          attachments: Vec::new(),
          location: None,
          metadata: BTreeMap::new(),
      }
  }

  /// Create a Plain-body message by stripping Markdown formatting from `md`.
  ///
  /// Equivalent to `Message::text(plain)` where `plain` is the result of
  /// rendering `md` to plain text.
  pub fn markdown_stripped(md: impl Into<String>) -> Self {
      let md = md.into();
      let nodes = crate::markdown::parse::parse_markdown(&md);
      let plain = crate::markdown::plain_text::render_plain_text(&nodes);
      Self::text(plain)
  }
  ```

- **Validation**: `cargo check -p messenger-lib` compiles.

### Step 1.2 — Write failing tests for the new constructors

- **Files**: `messenger/lib/src/tests/builders.rs`
- **Action**: Append:

  ```rust
  #[test]
  fn summarized_message_builder() {
      let msg = Message::summarized("plain hi", "**bold** hi");
      match msg.body {
          Some(MessageBody::Summarized { ref summary, ref markdown }) => {
              assert_eq!(summary, "plain hi");
              assert_eq!(markdown, "**bold** hi");
          }
          other => panic!("expected Summarized, got {other:?}"),
      }
  }

  #[test]
  fn markdown_stripped_produces_plain_body() {
      let msg = Message::markdown_stripped("**bold** and `code`");
      match msg.body {
          Some(MessageBody::Plain(ref s)) => {
              // plain_text renderer drops markdown syntax
              assert!(!s.contains("**"));
              assert!(!s.contains('`'));
              assert!(s.contains("bold"));
              assert!(s.contains("code"));
          }
          other => panic!("expected Plain, got {other:?}"),
      }
  }
  ```

- **Validation**: `cargo test -p messenger-lib --lib tests::builders::summarized_message_builder` and `tests::builders::markdown_stripped_produces_plain_body` both PASS once Step 1.1 is in.

### Step 1.3 — Update `PreparedMessage` to parse and cache the markdown half

- **Files**: `messenger/lib/src/prepared.rs`
- **Action**: Update `PreparedMessage::new` so that both `MessageBody::Markdown(md)` _and_ `MessageBody::Summarized { markdown, .. }` parse and cache the AST:

  ```rust
  impl PreparedMessage {
      pub fn new(message: &Message) -> Self {
          let markdown_nodes = match &message.body {
              Some(MessageBody::Markdown(markdown))
              | Some(MessageBody::Summarized { markdown, .. }) => {
                  Some(parse_markdown(markdown))
              }
              _ => None,
          };

          Self {
              message: message.clone(),
              markdown_nodes,
          }
      }
      // ... existing methods unchanged
  }
  ```

  Update `render_body_for_provider` to add a `Summarized` arm. The existing arm renders Markdown for the provider's flavor; `Summarized` does the same on its `markdown` half:

  ```rust
  pub fn render_body_for_provider(&self, provider: ProviderKind) -> String {
      use crate::markdown::{plain_text, render_for_provider, render_nodes_for_provider};
      match (&self.message.body, &self.markdown_nodes) {
          (Some(MessageBody::Plain(text)), _) => text.clone(),
          (Some(MessageBody::Markdown(_)), Some(nodes)) => {
              render_nodes_for_provider(nodes, provider)
          }
          (Some(MessageBody::Markdown(markdown)), None) => {
              render_for_provider(markdown, provider)
          }
          (Some(MessageBody::Summarized { markdown, summary }), nodes_opt) => {
              // Notification-only providers want the summary; rich providers want
              // the rendered markdown. The decision is made by the provider via
              // `render_summary` / `render_rich`. For the legacy single-string
              // path we pick the rich form on capable providers and the summary
              // on flat ones.
              match provider {
                  ProviderKind::Apns | ProviderKind::Fcm => summary.clone(),
                  ProviderKind::Signal
                  | ProviderKind::WhatsApp
                  | ProviderKind::Desktop => {
                      // Flat plain-text providers: prefer the summary because
                      // it is already plain and intended for notifications.
                      summary.clone()
                  }
                  _ => match nodes_opt {
                      Some(nodes) => render_nodes_for_provider(nodes, provider),
                      None => render_for_provider(markdown, provider),
                  },
              }
              // `summary` is unused on rich providers; suppress unused-binding.
              // (No actual unused warning because both arms reference it.)
          }
          (None, _) => String::new(),
      }
  }
  ```

  Add two new public methods:

  ```rust
  /// Plain text suitable for notification banners and flat-text providers.
  ///
  /// For `Summarized` bodies, returns the explicit summary. For `Markdown`
  /// bodies, returns a Markdown-stripped plain rendering. For `Plain`,
  /// returns the text as-is.
  pub fn render_summary(&self) -> String {
      use crate::markdown::plain_text;
      match (&self.message.body, &self.markdown_nodes) {
          (Some(MessageBody::Summarized { summary, .. }), _) => summary.clone(),
          (Some(MessageBody::Plain(text)), _) => text.clone(),
          (Some(MessageBody::Markdown(_)), Some(nodes)) => {
              plain_text::render_plain_text(nodes)
          }
          (Some(MessageBody::Markdown(md)), None) => {
              plain_text::render_plain_text(&crate::markdown::parse::parse_markdown(md))
          }
          (None, _) => String::new(),
      }
  }

  /// Rich body for providers with a separate rich-rendering surface (e.g.
  /// Discord embeds). Returns `None` when there is no rich body distinct from
  /// the summary.
  pub fn render_rich(&self, provider: ProviderKind) -> Option<String> {
      use crate::markdown::{render_for_provider, render_nodes_for_provider};
      match (&self.message.body, &self.markdown_nodes) {
          (Some(MessageBody::Summarized { markdown, .. }), Some(nodes)) => {
              Some(render_nodes_for_provider(nodes, provider))
          }
          (Some(MessageBody::Summarized { markdown, .. }), None) => {
              Some(render_for_provider(markdown, provider))
          }
          _ => None,
      }
  }
  ```

- **Validation**: `cargo check -p messenger-lib` compiles. `cargo test -p messenger-lib --lib tests::builders` still passes.

### Step 1.4 — Add unit tests for `render_summary` / `render_rich`

- **Files**: `messenger/lib/src/tests/builders.rs`
- **Action**: Append:

  ```rust
  #[test]
  fn prepared_render_summary_for_summarized() {
      let msg = Message::summarized("plain summary", "**rich** body");
      let prepared = PreparedMessage::new(&msg);
      assert_eq!(prepared.render_summary(), "plain summary");
  }

  #[test]
  fn prepared_render_summary_for_markdown_strips() {
      let msg = Message::markdown("**bold** text");
      let prepared = PreparedMessage::new(&msg);
      let s = prepared.render_summary();
      assert!(!s.contains("**"));
      assert!(s.contains("bold"));
  }

  #[test]
  fn prepared_render_summary_for_plain() {
      let msg = Message::text("hello");
      let prepared = PreparedMessage::new(&msg);
      assert_eq!(prepared.render_summary(), "hello");
  }

  #[test]
  fn prepared_render_summary_for_empty_body() {
      let msg = Message::location(0.0, 0.0);
      let prepared = PreparedMessage::new(&msg);
      assert_eq!(prepared.render_summary(), "");
  }

  #[test]
  fn prepared_render_rich_returns_some_only_for_summarized() {
      let summarized = PreparedMessage::new(&Message::summarized("s", "**m**"));
      assert!(summarized.render_rich(ProviderKind::Discord).is_some());

      let markdown = PreparedMessage::new(&Message::markdown("**m**"));
      assert!(markdown.render_rich(ProviderKind::Discord).is_none());

      let plain = PreparedMessage::new(&Message::text("p"));
      assert!(plain.render_rich(ProviderKind::Discord).is_none());
  }

  #[test]
  fn prepared_render_rich_renders_for_provider_flavor() {
      let msg = Message::summarized("s", "**bold** _italic_");
      let prepared = PreparedMessage::new(&msg);
      // Discord uses *italic* (single asterisk in our renderer)
      let rich = prepared.render_rich(ProviderKind::Discord).unwrap();
      assert!(rich.contains("**bold**"));
      assert!(rich.contains("*italic*"));
  }
  ```

- **Validation**: `cargo test -p messenger-lib --lib tests::builders` passes.

### Step 1.5 — Update `validate.rs` to treat `Summarized` as Markdown for capability checks

- **Files**: `messenger/lib/src/validate.rs`
- **Action**: Replace the existing `has_markdown` `matches!` (around line 139–142):

  ```rust
  let has_markdown = matches!(
      normalized_message.body,
      Some(crate::message::MessageBody::Markdown(_))
          | Some(crate::message::MessageBody::Summarized { .. })
  );
  ```

  Same change at `messenger/lib/src/tests/validation.rs:309` if a duplicate matcher exists there (Read first to confirm).

- **Validation**: `cargo test -p messenger-lib --lib tests::validation` passes. Add a regression test in `tests/validation.rs`:

  ```rust
  #[test]
  fn summarized_body_triggers_markdown_capability_check() {
      // Build a message + dispatch + provider that lacks markdown support
      // (signal/whatsapp/desktop). Confirm the existing Strict-mode path
      // still rejects when capability is missing — i.e. Summarized counts
      // as markdown for capability purposes.
      // (Reuse the existing test scaffold for `Markdown` rejection;
      // replace the body with Message::summarized("s", "**m**").)
      // ...exact body mirrors the existing `markdown_rejected_in_strict_mode`
      // test if present, else build inline using the validation test helpers.
  }
  ```

  The point is: the existing Strict-mode rejection test for Markdown should now have a sibling test for `Summarized`. Mirror the existing test exactly, swapping `Message::markdown(...)` for `Message::summarized("s", "**m**")`.

### Step 1.6 — Commit

- **Action**:

  ```bash
  cd /Users/ken/.claudine/worktrees/rusty-biscuit/messenger
  git add messenger/lib/src/message.rs messenger/lib/src/prepared.rs \
          messenger/lib/src/validate.rs messenger/lib/src/tests/builders.rs \
          messenger/lib/src/tests/validation.rs
  git commit -m "feat(messenger-lib): add Summarized message body and render_summary/render_rich helpers"
  ```

### Checkpoint 1

- `cargo test -p messenger-lib` passes.
- `cargo clippy -p messenger-lib --all-targets -- -D warnings` clean.
- No provider has changed yet. Discord still sends Markdown into `content` exactly as before.

---

## Phase 2 — Provider Match-Arm Coverage (apns, fcm, telegram)

**Goal**: Add `Summarized` arms to the three providers that pattern-match on `MessageBody` directly (besides `prepared` and `validate`, already done in Phase 1). Each picks the contextually correct half: notification-centric providers take `summary`, rich providers take rendered `markdown`.

### Step 2.1 — APNs: route `Summarized` to the `summary` half

- **Files**: `messenger/lib/src/provider/apns.rs`
- **Action**: Update the body match (around line 160–164):

  ```rust
  let body = match message.body() {
      Some(MessageBody::Plain(text)) => Some(text.as_str()),
      Some(MessageBody::Markdown(text)) => Some(text.as_str()),
      Some(MessageBody::Summarized { summary, .. }) => Some(summary.as_str()),
      None => None,
  };
  ```

- **Validation**: `cargo build -p messenger-lib --features apns` (or whatever feature gates APNs). Existing APNs tests pass. Add to `tests/builders.rs` or a new `tests/apns_integration.rs` test if integration tests exercise the body path:

  ```rust
  #[test]
  fn apns_summarized_uses_summary_for_notification_body() {
      // PreparedMessage path: confirm message.body() returns Summarized,
      // and the alert body matches `summary`. (Delegated to APNs
      // payload-building helper if one exists; otherwise rely on the
      // match-arm covering compile-time exhaustiveness.)
  }
  ```

### Step 2.2 — FCM: route `Summarized` to the `summary` half

- **Files**: `messenger/lib/src/provider/fcm.rs`
- **Action**: Mirror Step 2.1 at line 127–129:

  ```rust
  let body = match message.body() {
      Some(MessageBody::Plain(text)) => Some(text.as_str()),
      Some(MessageBody::Markdown(text)) => Some(text.as_str()),
      Some(MessageBody::Summarized { summary, .. }) => Some(summary.as_str()),
      None => None,
  };
  ```

- **Validation**: `cargo build -p messenger-lib --features fcm`. Existing FCM tests pass.

### Step 2.3 — Telegram: route `Summarized` through HTML rendering of the markdown half

- **Files**: `messenger/lib/src/provider/telegram.rs`
- **Action**: Update the format-selection match (around line 233–243). Telegram has no embed concept; the rendered Markdown half is the natural choice:

  ```rust
  let (text, parse_mode) = match message.body() {
      Some(MessageBody::Markdown(_)) => (
          message.render_body_for_provider(ProviderKind::Telegram),
          Some("HTML"),
      ),
      Some(MessageBody::Summarized { .. }) => (
          message.render_body_for_provider(ProviderKind::Telegram),
          Some("HTML"),
      ),
      Some(MessageBody::Plain(_)) => (
          message.render_body_for_provider(ProviderKind::Telegram),
          None,
      ),
      None => (String::new(), None),
  };
  ```

  (`render_body_for_provider` already handles the `Summarized` arm correctly per Phase 1, returning the rendered markdown half on Telegram.)

- **Validation**: `cargo build -p messenger-lib --features telegram`. Add a smoke test in `tests/telegram_integration.rs` if the existing test pattern allows it (look for a `wiremock`-style test that asserts on `parse_mode = HTML` and message text — duplicate it with `Message::summarized(...)` and confirm the rendered Markdown surfaces in the request body).

### Step 2.4 — Sanity check: no other provider-side `MessageBody` matches remain

- **Action**: Run `grep -rn "MessageBody::" messenger/lib/src/ | grep -v test`. Expected output: only the four sites already touched (`message.rs`, `prepared.rs`, `validate.rs`, the three provider files in this phase). If any other site appears, add a `Summarized` arm there too following the same routing convention (notification-centric → summary; rich → render_body_for_provider).

- **Validation**: Grep returns only the expected sites.

### Step 2.5 — Commit

- **Action**:

  ```bash
  git add messenger/lib/src/provider/apns.rs messenger/lib/src/provider/fcm.rs \
          messenger/lib/src/provider/telegram.rs
  git commit -m "feat(messenger-lib): handle Summarized body in apns, fcm, telegram providers"
  ```

### Checkpoint 2

- `cargo test -p messenger-lib` passes (all features that have providers).
- `cargo clippy -p messenger-lib --all-targets -- -D warnings` clean (no missing-arm warnings).
- All providers compile; pre-existing behavior on `Plain`/`Markdown` is byte-identical.

---

## Phase 3 — Discord Provider Embed Split

**Goal**: When the Discord provider receives a `Summarized` body, send `summary` as the message `content` (notification-safe) and the rendered Markdown as a single embed `description` (rich rendering). When the body is `Plain` or `Markdown`, behavior is unchanged.

### Step 3.1 — Add the Embed import and helper for building a description-only embed

- **Files**: `messenger/lib/src/provider/discord.rs`
- **Action**: Add import alongside the existing twilight imports:

  ```rust
  use twilight_model::channel::message::Embed;
  use twilight_util::builder::embed::EmbedBuilder;
  ```

  If `twilight-util` isn't already a dependency, add to `messenger/lib/Cargo.toml` under the `discord` feature:

  ```toml
  twilight-util = { version = "0.17", optional = true, features = ["builder"] }
  ```

  And add `"dep:twilight-util"` to the existing `discord = [...]` feature list.

- **Validation**: `cargo build -p messenger-lib --features discord` compiles.

### Step 3.2 — Write a failing integration test for the `Summarized` payload split

- **Files**: New `messenger/lib/src/tests/discord_integration.rs` (if absent — check `tests/mod.rs` for a `discord_integration` entry; if there isn't one, add `pub mod discord_integration;` after the other provider integration mods).

  Note: the Discord bot provider tests in `provider/discord.rs` use `tempfile`/`bytes`-style unit tests (rejecting invalid IDs, building attachments). There is no wiremock harness for the bot client because `twilight-http`'s `Client` does not accept a custom base URL. For this phase we will instead extend the **unit test module** in `provider/discord.rs` with a test that exercises the request-building branch via a small refactor.

- **Action**: Refactor `send_prepared` so that the request-shaping logic is a pure function returning a typed payload, then unit-test that function. Concretely, extract a helper:

  ```rust
  /// Result of preparing the Discord request body shape for a message.
  ///
  /// `embed` is `Some` only for `Summarized` bodies; otherwise the rendered
  /// content is sent in the top-level `content` field as today.
  #[derive(Debug)]
  struct DiscordPayload {
      content: Option<String>,
      embed: Option<Embed>,
  }

  impl DiscordProvider {
      fn build_payload(message: &PreparedMessage) -> DiscordPayload {
          let summary = message.render_summary();
          let rich = message.render_rich(ProviderKind::Discord);
          let location_line = message.location().map(|l| l.format_text_line());

          match rich {
              Some(mut rich_md) => {
                  // Append location to the embed body (rich surface).
                  if let Some(loc) = location_line {
                      if !rich_md.is_empty() {
                          rich_md.push('\n');
                      }
                      rich_md.push_str(&loc);
                  }
                  let embed = EmbedBuilder::new()
                      .description(rich_md)
                      .build();
                  let content = if summary.is_empty() { None } else { Some(summary) };
                  DiscordPayload { content, embed: Some(embed) }
              }
              None => {
                  // Legacy single-surface path: render body for Discord and
                  // append location text inline as before.
                  let mut content = message.render_body_with_location(ProviderKind::Discord);
                  let content = if content.is_empty() { None } else { Some(content) };
                  DiscordPayload { content, embed: None }
              }
          }
      }
  }
  ```

  Then add the failing tests in the existing `#[cfg(test)] mod tests` in `provider/discord.rs`:

  ```rust
  #[test]
  fn build_payload_plain_body_uses_content_only() {
      let msg = crate::Message::text("hello");
      let prepared = PreparedMessage::new(&msg);
      let p = DiscordProvider::build_payload(&prepared);
      assert_eq!(p.content.as_deref(), Some("hello"));
      assert!(p.embed.is_none());
  }

  #[test]
  fn build_payload_markdown_body_uses_content_only() {
      let msg = crate::Message::markdown("**bold**");
      let prepared = PreparedMessage::new(&msg);
      let p = DiscordProvider::build_payload(&prepared);
      assert_eq!(p.content.as_deref(), Some("**bold**"));
      assert!(p.embed.is_none());
  }

  #[test]
  fn build_payload_summarized_body_splits_into_content_and_embed() {
      let msg = crate::Message::summarized("plain banner", "**rich** body");
      let prepared = PreparedMessage::new(&msg);
      let p = DiscordProvider::build_payload(&prepared);
      assert_eq!(p.content.as_deref(), Some("plain banner"));
      let embed = p.embed.expect("expected embed for Summarized body");
      let desc = embed.description.expect("embed should have description");
      assert!(desc.contains("**rich**"));
      assert!(desc.contains("body"));
  }

  #[test]
  fn build_payload_summarized_with_empty_summary_omits_content() {
      let msg = crate::Message::summarized("", "**rich**");
      let prepared = PreparedMessage::new(&msg);
      let p = DiscordProvider::build_payload(&prepared);
      assert!(p.content.is_none());
      assert!(p.embed.is_some());
  }

  #[test]
  fn build_payload_summarized_appends_location_to_embed_description() {
      let msg = crate::Message::summarized("s", "body").with_location(34.05, -118.24);
      let prepared = PreparedMessage::new(&msg);
      let p = DiscordProvider::build_payload(&prepared);
      let desc = p.embed.unwrap().description.unwrap();
      assert!(desc.contains("body"));
      assert!(desc.contains("📍"));
      assert!(desc.contains("34.0500"));
  }

  #[test]
  fn build_payload_legacy_appends_location_to_content() {
      let msg = crate::Message::markdown("**body**").with_location(34.05, -118.24);
      let prepared = PreparedMessage::new(&msg);
      let p = DiscordProvider::build_payload(&prepared);
      let content = p.content.unwrap();
      assert!(content.contains("**body**"));
      assert!(content.contains("📍"));
  }
  ```

- **Validation**: Run `cargo test -p messenger-lib --features discord --lib provider::discord::tests::build_payload`. Tests FAIL because `build_payload` does not yet exist (or returns the legacy shape on Summarized).

### Step 3.3 — Implement `build_payload` and rewire `send_prepared`

- **Files**: `messenger/lib/src/provider/discord.rs`
- **Action**: Add the `build_payload` helper exactly as written in Step 3.2's snippet. Replace the body-rendering portion of `send_prepared` (lines 131–155) with:

  ```rust
  // Build the message body shape (content + optional embed)
  let payload = Self::build_payload(message);
  let attachments = Self::build_attachments(message)?;
  let attachment_kinds: Vec<_> = message
      .attachments()
      .iter()
      .map(|attachment| &attachment.kind)
      .collect();
  tracing::debug!(
      has_reply = dispatch.reply_to.is_some(),
      content_len = payload.content.as_deref().map(str::len).unwrap_or(0),
      has_embed = payload.embed.is_some(),
      attachment_count = attachments.len(),
      "sending Discord message"
  );
  tracing::trace!(attachment_kinds = ?attachment_kinds, "built Discord attachments");

  // Build the message request
  let mut req = self.client.create_message(channel_id);

  if let Some(ref content) = payload.content {
      req = req.content(content);
  }
  let embeds_owned;
  if let Some(embed) = payload.embed {
      embeds_owned = [embed];
      req = req.embeds(&embeds_owned);
  }
  if !attachments.is_empty() {
      req = req.attachments(&attachments);
  }
  ```

  Note: `req.embeds(&[Embed])` requires the slice to outlive the request, which is why we bind `embeds_owned` outside the `if let` and pass `&embeds_owned`. If twilight-http's API requires a different lifetime shape (verify by reading the trait docs at build time — `cargo doc -p twilight-http --no-deps --features rustls-ring` if needed), restructure to clone or move accordingly.

- **Validation**: `cargo test -p messenger-lib --features discord --lib provider::discord::tests::build_payload` PASSES (all six new tests). `cargo test -p messenger-lib --features discord` overall passes.

### Step 3.4 — Manual smoke test against a real Discord channel (optional, gated)

- **Files**: None.
- **Action**: With a configured Discord bot route (`messenger config show` to confirm), run:

  ```bash
  cargo run -p messenger-cli --features discord -- send \
      "**this is rich** with `code` and *emphasis*" \
      --route discord:smoketest
  cargo run -p messenger-cli --features discord -- send \
      "**this is rich** with `code` and *emphasis*" \
      --route discord:smoketest \
      --summary "Plain notification banner"
  ```

  Inspect: the second send should produce a clean desktop notification reading `Plain notification banner` while the in-channel embed renders the rich Markdown. (CLI `--summary` flag lands in Phase 5; for now, write a one-off binary or `cargo test --ignored` test to call `Messenger` directly with `Message::summarized(...)` if Phase 5 hasn't shipped yet.)

  Skip this step if a live Discord environment is not available.

### Step 3.5 — Commit

- **Action**:

  ```bash
  git add messenger/lib/src/provider/discord.rs messenger/lib/Cargo.toml
  git commit -m "feat(messenger-lib): split Discord output into content+embed for Summarized bodies"
  ```

### Checkpoint 3

- `cargo test -p messenger-lib --features discord` passes.
- `cargo clippy -p messenger-lib --features discord --all-targets -- -D warnings` clean.
- Manual smoke test (if performed) confirms clean notification on `Summarized` payloads.
- Discord webhook still uses single-surface (legacy) shape — that's Phase 4.

---

## Phase 4 — Discord Webhook Embed Split

**Goal**: Bring `DiscordWebhookProvider` to parity. The wire format is JSON (not twilight-http), so the embed payload goes into the `embeds` array of `WebhookJsonBody`. Wiremock-style tests already exist for the webhook provider, so we get full request-body assertions.

### Step 4.1 — Extend `WebhookJsonBody` and add an `EmbedBody` struct

- **Files**: `messenger/lib/src/provider/discord_webhook.rs`
- **Action**: Around line 170–177, replace:

  ```rust
  #[derive(Serialize)]
  struct WebhookJsonBody<'a> {
      #[serde(skip_serializing_if = "Option::is_none")]
      content: Option<&'a str>,
      #[serde(skip_serializing_if = "Option::is_none")]
      attachments: Option<Vec<AttachmentMeta>>,
      #[serde(skip_serializing_if = "Option::is_none")]
      embeds: Option<Vec<EmbedBody>>,
  }

  #[derive(Serialize)]
  struct EmbedBody {
      description: String,
  }
  ```

- **Validation**: `cargo check -p messenger-lib --features discord-webhook` compiles. (If feature flag has a different name, follow Cargo.toml.)

### Step 4.2 — Write failing wiremock test for `Summarized` payload

- **Files**: `messenger/lib/src/tests/discord_webhook_integration.rs`
- **Action**: After existing tests, append:

  ```rust
  #[tokio::test]
  async fn webhook_summarized_body_splits_into_content_and_embed() {
      let server = MockServer::start().await;
      Mock::given(method("POST"))
          .and(path_regex(r"/api/webhooks/.*"))
          .and(body_json(serde_json::json!({
              "content": "plain banner",
              "embeds": [{ "description": "**rich** body" }]
          })))
          .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
              "id": "123",
              "channel_id": "456",
              "webhook_id": "789"
          })))
          .expect(1)
          .mount(&server)
          .await;

      // Build provider against the mock server URL.
      // (Mirror the URL-construction pattern from existing tests in this file —
      // typically `format!("{}/api/webhooks/789/abc", server.uri())`.)
      let webhook_url = format!("{}/api/webhooks/789/abc", server.uri());
      let provider = DiscordWebhookProvider::new(DiscordWebhookConfig {
          webhook_url: SecretString::from(webhook_url),
      }).unwrap();

      let msg = Message::summarized("plain banner", "**rich** body");
      let dispatch = Dispatch::to(Target::DiscordWebhook(DiscordWebhookTarget {
          thread_id: None,
      }));
      let prepared = PreparedMessage::new(&msg);

      provider.send_prepared(&dispatch, &prepared).await.unwrap();
  }
  ```

  Match the imports and helper patterns from existing tests in the same file (look at `webhook_sends_plain_text` or similar around line 120–170 for the canonical setup). Validate that the `body_json` matcher matches exactly.

- **Validation**: `cargo test -p messenger-lib --features discord-webhook --lib tests::discord_webhook_integration::webhook_summarized_body_splits_into_content_and_embed` FAILS because the embed branch isn't wired up yet.

### Step 4.3 — Wire the embed branch in `send_prepared`

- **Files**: `messenger/lib/src/provider/discord_webhook.rs`
- **Action**: Replace the body-rendering portion (lines 242–266 plus the parallel multipart branch):

  ```rust
  let summary = message.render_summary();
  let rich = message.render_rich(ProviderKind::DiscordWebhook);
  let location_line = message.location().map(|l| l.format_text_line());

  let (content_owned, embeds_owned): (Option<String>, Option<Vec<EmbedBody>>) = match rich {
      Some(mut rich_md) => {
          if let Some(loc) = location_line {
              if !rich_md.is_empty() {
                  rich_md.push('\n');
              }
              rich_md.push_str(&loc);
          }
          let content = if summary.is_empty() { None } else { Some(summary) };
          (content, Some(vec![EmbedBody { description: rich_md }]))
      }
      None => {
          let content = message.render_body_with_location(ProviderKind::DiscordWebhook);
          let content = if content.is_empty() { None } else { Some(content) };
          (content, None)
      }
  };
  let content_opt = content_owned.as_deref();

  // ... rest of send_prepared uses content_opt and includes embeds_owned in the body
  ```

  In both branches (`attachments.is_empty()` and the multipart path), include `embeds: embeds_owned.clone()` (or move) in `WebhookJsonBody`.

- **Validation**: `cargo test -p messenger-lib --features discord-webhook --lib tests::discord_webhook_integration` — the new test PASSES, and existing tests still pass (because the legacy path uses `embeds: None` which serde skips).

### Step 4.4 — Add a fall-through test confirming legacy payloads are unchanged

- **Files**: `messenger/lib/src/tests/discord_webhook_integration.rs`
- **Action**: Append:

  ```rust
  #[tokio::test]
  async fn webhook_markdown_body_does_not_include_embeds() {
      let server = MockServer::start().await;
      Mock::given(method("POST"))
          .and(path_regex(r"/api/webhooks/.*"))
          // Match any body, but assert below by capturing.
          .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
              "id": "1", "channel_id": "2", "webhook_id": "3"
          })))
          .expect(1)
          .mount(&server)
          .await;

      let url = format!("{}/api/webhooks/789/abc", server.uri());
      let provider = DiscordWebhookProvider::new(DiscordWebhookConfig {
          webhook_url: SecretString::from(url),
      }).unwrap();
      let msg = Message::markdown("**bold**");
      let dispatch = Dispatch::to(Target::DiscordWebhook(DiscordWebhookTarget {
          thread_id: None,
      }));
      let prepared = PreparedMessage::new(&msg);
      provider.send_prepared(&dispatch, &prepared).await.unwrap();

      // Verify recorded request has no `embeds` key.
      let received = server.received_requests().await.unwrap();
      let body: serde_json::Value =
          serde_json::from_slice(&received[0].body).unwrap();
      assert_eq!(body.get("content").and_then(|v| v.as_str()), Some("**bold**"));
      assert!(body.get("embeds").is_none(),
          "legacy Markdown body must not produce an embeds field");
  }
  ```

- **Validation**: Test passes.

### Step 4.5 — Commit

- **Action**:

  ```bash
  git add messenger/lib/src/provider/discord_webhook.rs \
          messenger/lib/src/tests/discord_webhook_integration.rs
  git commit -m "feat(messenger-lib): split Discord webhook output into content+embed for Summarized bodies"
  ```

### Checkpoint 4

- `cargo test -p messenger-lib --features discord-webhook` passes (new `Summarized` test + the legacy non-regression test + all pre-existing tests).
- `cargo clippy --all-features --all-targets -- -D warnings` clean.
- Library is feature-complete. Only the CLI surface remains.

---

## Phase 5 — CLI Flags + Documentation

**Goal**: Expose the two new shapes from the CLI (`--summary`, `--strip-markdown`), update the help-output snapshot, and document the feature in the user guide and SKILL file.

### Step 5.1 — Add the two new flags to the Send subcommand

- **Files**: `messenger/cli/src/main.rs`
- **Action**: After the existing `--plain` flag (around line 93–95), insert:

  ```rust
  /// Plain notification summary paired with a Markdown body.
  ///
  /// On providers with a notification surface distinct from the rendered
  /// surface (Discord), the summary becomes the notification banner text and
  /// the message argument becomes a rich embed. Implies Markdown body —
  /// cannot combine with --plain.
  #[arg(long)]
  summary: Option<String>,

  /// Strip Markdown formatting from the message body before sending.
  ///
  /// Mutually exclusive with --plain (redundant) and --summary (incoherent).
  #[arg(long, conflicts_with_all = ["plain", "summary"])]
  strip_markdown: bool,
  ```

  Note: `--summary` and `--plain` are also mutually exclusive. Add `conflicts_with = "plain"` to the `summary` arg, or add `summary` to `--plain`'s conflicts. Easiest: extend `--plain` arg attribute to `#[arg(long, conflicts_with = "summary")]` (verify the existing `--plain` definition before editing).

- **Validation**: `cargo build -p messenger-cli` compiles.

### Step 5.2 — Thread the flags into message construction

- **Files**: `messenger/cli/src/main.rs`
- **Action**: At the existing message-construction site (around line 547–562):

  ```rust
  let mut message = match message_text.as_deref() {
      Some(text) if !text.is_empty() => {
          if strip_markdown {
              messenger::Message::markdown_stripped(text)
          } else if let Some(summary) = summary.as_deref() {
              messenger::Message::summarized(summary, text)
          } else if plain {
              messenger::Message::text(text)
          } else {
              messenger::Message::markdown(text)
          }
      }
      _ => messenger::Message {
          title: None,
          body: None,
          attachments: Vec::new(),
          location: None,
          metadata: std::collections::BTreeMap::new(),
      },
  };
  ```

  Confirm that `summary` and `strip_markdown` are destructured from the `Send` variant near where `plain`, `message_text`, `title`, etc. are pulled out (around line 530–545; pattern-match on the `Commands::Send { .. }` enum and add the two new fields to the destructuring).

- **Validation**: `cargo build -p messenger-cli` compiles.

### Step 5.3 — Update the help-output snapshot

- **Files**: `messenger/cli/tests/snapshots/snapshots__help_output.snap`
- **Action**: Run `cargo insta test -p messenger-cli` (or `cargo test -p messenger-cli` then `cargo insta accept`) to regenerate the snapshot. Inspect the diff: only the new `--summary` and `--strip-markdown` lines should appear. Accept.

  If `cargo-insta` is not installed: `cargo install cargo-insta` first.

- **Validation**: `cargo test -p messenger-cli` passes. The snapshot file diff shows only the new flags.

### Step 5.4 — Add a CLI integration test for `--summary`

- **Files**: `messenger/cli/tests/` (use the existing test harness — look for an existing test that runs `messenger send ... --route ...` against a stub provider; if one exists, mirror it).
- **Action**: If the existing CLI test harness can dispatch to a Discord-webhook stub via wiremock, add:

  ```rust
  #[tokio::test]
  async fn send_with_summary_uses_summarized_body() {
      // Set up a wiremock Discord webhook server expecting:
      //   { "content": "banner", "embeds": [{"description": "**rich**"}] }
      // Run: messenger send "**rich**" --summary "banner" --route discord-webhook:test
      // Assert exit 0 and the mock saw exactly one matching request.
  }
  ```

  If no such CLI integration harness exists today, skip this step — the library-level test in Phase 4 already covers the wire payload, and the CLI plumbing is a thin pass-through. (Don't invent a new harness for one test.)

- **Validation**: New test passes (if added).

### Step 5.5 — Update the user guide

- **Files**: `messenger/docs/user-guide.md`
- **Action**: Add a new section (place it near the existing Markdown discussion). Example content:

  ```markdown
  ## Notification-Aware Messages

  Some providers — most notably Discord — render Markdown nicely in the chat
  channel but generate desktop notifications from the raw, unrendered text. A
  message body of `phase **1** of *6*` shows up cleanly in chat but appears as
  literal asterisks in the notification banner.

  Messenger gives you three ways to control this trade-off:

  ### 1. Single Markdown string (default)

  ```bash
  messenger send "phase **1** of *6* complete" --route discord:bot-updates
  ```

  Markdown is rendered in chat; the notification shows the raw characters.
  Use this when you don't care about the notification's appearance.

  ### 2. Plain summary + rich body

  ```bash
  messenger send "phase **1** of *6* complete" \
      --summary "Phase 1 of 6 complete" \
      --route discord:bot-updates
  ```

  The notification banner reads `Phase 1 of 6 complete`; the in-channel message
  is a rich embed rendering the Markdown. Library callers use:

  ```rust
  Message::summarized("Phase 1 of 6 complete", "phase **1** of *6* complete")
  ```

  ### 3. Strip formatting

  ```bash
  messenger send "phase **1** of *6* complete" --strip-markdown \
      --route discord:bot-updates
  ```

  Markdown is removed before sending; both the chat and the notification show
  plain text. Library callers use `Message::markdown_stripped(md)`.

  Providers without a notification/rich split (Telegram, Signal, Slack)
  receive a single rendered string per their native flavor; the summary half
  of `Summarized` is used by push providers (APNs, FCM) and flat-text
  providers (Signal, WhatsApp, Desktop).
  ```

- **Validation**: Render the markdown locally (or in a Markdown-aware editor) and confirm the section reads correctly.

### Step 5.6 — Update the messenger SKILL file

- **Files**: `.opencode/skill/messenger/SKILL.md` (path verified during Phase 1 scan; if path differs in this repo, find via `find . -name SKILL.md -path '*messenger*'`).
- **Action**: Add a brief mention of the three calling shapes under the existing "Sending Messages" section (or equivalent). One paragraph plus a code-block example matching the user-guide content.

- **Validation**: Skill file renders correctly; `markdown` lint clean.

### Step 5.7 — Commit

- **Action**:

  ```bash
  git add messenger/cli/src/main.rs \
          messenger/cli/tests/snapshots/snapshots__help_output.snap \
          messenger/docs/user-guide.md \
          .opencode/skill/messenger/SKILL.md
  git commit -m "feat(messenger-cli): add --summary and --strip-markdown flags; document notification-aware bodies"
  ```

### Checkpoint 5

- `cargo test --workspace --all-features` passes.
- `cargo clippy --workspace --all-features --all-targets -- -D warnings` clean.
- `messenger send --help` shows the two new flags in alphabetical order with the existing flags.
- Manual smoke (if Discord credentials available): `messenger send "**rich**" --summary "plain" --route discord:test` produces a clean notification + rich embed.
- Feature is shipped end-to-end. Spec acceptance criteria 1–9 satisfied.

---

## Acceptance Criteria Mapping

| Spec criterion | Phase | Step(s) |
|----------------|-------|---------|
| 1. Constructors compile and produce expected shapes | 1 | 1.1, 1.2 |
| 2. `cargo test -p messenger-lib` passes with new builder tests | 1 | 1.2, 1.4, 1.6 |
| 3. Discord wiremock-style test for content+embed split | 3 | 3.2, 3.3 |
| 4. Discord webhook wiremock test for the same split | 4 | 4.2, 4.3 |
| 5. Telegram/APNs/FCM/validation tests cover Summarized | 1, 2 | 1.5, 2.1, 2.2, 2.3 |
| 6. CLI `--summary` produces the expected runtime behavior | 5 | 5.1, 5.2, 5.4 |
| 7. CLI `--strip-markdown` produces single plain content | 5 | 5.1, 5.2 |
| 8. No existing callers break (`cargo test --workspace` clean) | All | Checkpoints 1–5 |
| 9. User guide section explaining the three shapes | 5 | 5.5 |

## Risk + Mitigations

- **`twilight-util` dependency addition**: confirmed at version 0.17 to match `twilight-http`/`twilight-model`. Feature-gated under existing `discord` feature so it does not bloat default builds. Mitigation: if `twilight-util` is undesirable, hand-build the `Embed` struct by setting only the `description` field directly on `twilight_model::channel::message::Embed` (it implements `Default`).
- **Embed lifetime in `req.embeds(&[..])`**: twilight-http expects a borrowed slice. The plan stores the single embed in a stack binding (`embeds_owned`) before passing the borrow. If a different lifetime shape is required by the API, the helper can return `(Option<String>, Option<Vec<Embed>>)` and the caller stores it locally — same pattern as `attachments`.
- **Snapshot-test churn**: only the help-output snapshot changes. If CI uses `--check` mode for snapshots, the snapshot must be committed in the same commit as the CLI change (Step 5.3 + 5.7).
- **Notification routing convention for non-Discord providers**: documented in Phase 1 (`render_body_for_provider` Summarized arm) and the user-guide section. If a future provider needs different routing, update the central match in `prepared.rs` rather than each provider individually.
