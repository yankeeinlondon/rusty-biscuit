---
total_phases: 8
created: 2026-09-07
phase: 1
agent: claude/default
yolo: "true"
area: messenger
packages:
    - messenger
spec: ./spec.md
---

# Execution Plan — Discord Content Truncation

Converts [`spec.md`](./spec.md) (R1–R9, SC1–SC17) into ordered, observable work.

## Reading This Plan

- Each task leads with `- [ ]`; check it off when the task is **observably** done.
- **Validation checkpoints** (`✅`) end every phase. Do not start the next phase
  until the checkpoint is green.
- Tasks marked **∥** may run in parallel with their siblings in the same group.
- Every `just` command runs from `messenger/` unless the task says otherwise.
- **Commits:** the spec requires the R8/R9 documentation work to land in its own
  commit, separate from behavior changes (repo scope-discipline rule). Commit
  boundaries are noted per phase, but **do not commit unless explicitly asked** —
  committing is a separate operation.

## Dependency Overview

```
Phase 1 (limits.rs primitives + dev-dep)
   │
   ├──► Phase 2 (prepared.rs reservation)
   │        │
   │        ├──► Phase 3 (bot adapter)      ─┐
   │        └──► Phase 4 (webhook adapter)  ─┤  ∥ after Phase 2
   │                                          │
   │                                    Phase 5 (receipt + CLI surfacing)
   │                                          │
   │                                    Phase 6 (test completion)
   │                                          │
   │                                    Phase 7 (cross-cutting validation)
   │
   └──────────────────────────────────► Phase 8 (docs — ∥ from Phase 5 onward)
```

Phases 3 and 4 touch disjoint files (`discord.rs` vs `discord_webhook.rs`) and
are safely parallel once Phase 2 lands. Phase 8 touches only Markdown and is
parallel with Phases 5–7.

## Verified Preconditions (checked on HEAD, 2026-09-07)

These were confirmed before planning; if any is false when you start, stop and
re-read the spec section named:

- `messenger/lib/src/provider/discord.rs` — `DiscordPayload` (private struct,
  `content` + `embed`), `build_payload`, `build_attachment(attachment, id)`,
  `build_attachments` are all **private**; `content_len` logged via `str::len`.
- `messenger/lib/src/provider/discord_webhook.rs` — payload built **inline** in
  `send_prepared`; `WebhookJsonBody<'a>` borrows `content: Option<&'a str>`;
  `EmbedBody` and `AttachmentMeta` are private; attachment description resolved
  inline in the `metas` loop; `content_len` logged via `String::len`.
- `messenger/lib/src/prepared.rs` — `render_body_with_location` present, doc
  comment names Discord; no `provider::*` imports today.
- `messenger/lib/src/tests/mod.rs` — `builders`, `receipts`, `validation`
  ungated; every `*_integration` gated by its provider feature.
- `messenger/lib/Cargo.toml` — `[dev-dependencies]` has `tokio`, `tempfile`,
  `wiremock`, `serial_test`; **no** `twilight-validate`.
- `messenger/cli/src/main.rs` — `SendResponse { id, receipt, helper, os }`;
  `emit_send_response` populates exactly those four.
- `messenger/lib/src/receipt.rs` — `helper_used()` present with its `filter`.

### Two spec line-references drifted (use the anchors, not the numbers)

- R9 cites the user-guide receipts section as `:451-457`. On HEAD the receipts
  material is around `:620-700` (`delivery_confirmed` at `:628-629`,
  `helper_used()` at `:686`). Anchor Phase 8 work on those **symbols**, not the
  line numbers.
- R9's row for `docs/dependencies.md` refers to the **repo root** file, which
  exists. There is **no** `messenger/docs/dependencies.md`; do not create one.

---

## Phase 1 — Shared Primitives and Prerequisites

**Goal:** `provider/limits.rs` exists, compiles unconditionally, and is fully
unit-tested with zero production consumers. Nothing else in the crate changes.

**Covers:** R2, R3, R7, and the Testing → Prerequisites dev-dependency.

### 1.1 Module scaffolding

- [ ] Create `messenger/lib/src/provider/limits.rs` with the module doc comment
      and `#![allow(dead_code)]` exactly as written in R7 (the doc comment must
      name *why* the allow exists: no length-bounded provider may be enabled).
- [ ] Declare it in `messenger/lib/src/provider/mod.rs` as
      `pub(crate) mod limits;` with **no `cfg` attribute**, placed in the
      existing alphabetical block (between `discord_webhook` and `fcm`).
      Do **not** add it to the `http_helpers` `any(...)` list.

### 1.2 Constants

- [ ] Add `ELLIPSIS: &str = "\n\n..."` and `ELLIPSIS_CHARS: usize = 5`
      (`pub(crate)` — R1's compile-time assertions in `discord.rs` read the
      latter).
- [ ] Add `LOCATION_ELLIPSIS: &str = "…"` (U+2026) and
      `LOCATION_ELLIPSIS_CHARS: usize = 1`, both `pub(crate)`.
- [ ] Add `MAX_LOCATION_LINE_CHARS: usize = 256`, `pub(crate)`, documented as a
      **messenger presentation choice, not a platform limit** (R4.1).

### 1.3 Truncation core and wrappers

- [ ] Implement the private `truncate_with_marker(text, max_chars, marker,
      marker_chars) -> String`:
      1. `text.chars().count() <= max_chars` → return `text` **unchanged**
         (no allocation, no trim, no marker).
      2. `max_chars < marker_chars` → return the first `max_chars` chars with
         **no** marker (R2 "Totality").
      3. Otherwise keep the first `max_chars - marker_chars` chars, `trim_end()`
         the kept slice, then append `marker`.
- [ ] Slice by `char_indices()` / `chars().take(n)` only — **never** a byte
      range (R3; a byte slice panics on multi-byte input).
- [ ] Add `pub(crate) fn truncate_to_chars(text, max_chars) -> String`
      (`ELLIPSIS` marker).
- [ ] Add `pub(crate) fn truncate_inline(text, max_chars) -> String`
      (`LOCATION_ELLIPSIS` marker).
- [ ] Add `pub(crate) fn truncate_location_line(line) -> String`, whose body is
      exactly `truncate_inline(line, MAX_LOCATION_LINE_CHARS)` — no `max_chars`
      parameter (R2's argument: a fixed presentation cap must not be passable).

### 1.4 Bookkeeping types

- [ ] Add `pub(crate) struct BoundedField { text: String, original_chars: usize,
      truncated: bool }` with the field docs from R4.1 — in particular that
      `original_chars` is the **pre-cut** count of what the caller asked to fit.
- [ ] Add `pub(crate) fn bound_to_chars(text, max_chars) -> BoundedField`
      (wraps `truncate_to_chars`; adds nothing but bookkeeping).
- [ ] Add `pub(crate) fn bound_inline_to_chars(text, max_chars) -> BoundedField`
      (wraps `truncate_inline`).
- [ ] Add `#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)] pub(crate)
      struct TruncationOutcome { content, embed_description,
      attachment_description }` — **declaration order is the emission order**.
- [ ] Add `pub(crate) fn truncation_metadata_value(outcome: TruncationOutcome)
      -> Option<String>`: `None` when all three are false; otherwise the set
      tokens joined by `,` in the fixed order
      `content,embed_description,attachment_description` — **never sorted at
      emission, never in cut order** (R6).

### 1.5 Dev-dependency

- [ ] Add `twilight-validate = "0.17"` to `messenger/lib/Cargo.toml`
      `[dev-dependencies]`, **unconditionally** (Cargo rejects `optional` on
      dev-dependencies). The Limit-model tests that use it are gated at the
      test module instead (Phase 6).
- [ ] Confirm no duplicate enters the graph:
      `cargo tree -p messenger --features discord -i twilight-validate` shows a
      single 0.17.x, pulled by `twilight-http` 0.17.1.

### 1.6 Helper unit tests (`provider/limits.rs` → `#[cfg(test)] mod tests`)

The module has **no `cfg`** on its test module — the subject compiles in every
feature set, and gating would leave the primitives untested in exactly the builds
where they are the only thing present.

- [ ] `text_under_limit_is_unchanged`
- [ ] `text_at_exact_limit_is_unchanged`
- [ ] `text_over_limit_is_truncated_within_limit` (assert `<= max`, **not** `==`)
- [ ] `under_limit_input_with_trailing_whitespace_is_byte_identical` — input of
      `max-1` chars ending in `"\n\n  "` round-trips byte-identical (SC7 + the
      `MessageBody::Plain` guard, in one row)
- [ ] `truncation_trims_whitespace_before_the_marker` — use R2's
      `"alpha\n\nbravo!"` / `max=12` example; assert the char before the marker
      is non-whitespace **and** the result is strictly shorter than `max`
- [ ] `truncation_mid_word_is_unaffected_by_the_trim` — result is **exactly**
      `max` chars
- [ ] `all_whitespace_keep_window_yields_bare_marker` — result is exactly
      `"\n\n..."` and is non-empty
- [ ] `truncated_text_ends_with_marker`
- [ ] `truncated_text_preserves_leading_text` — pre-marker prefix is a prefix of
      the input
- [ ] `truncation_is_char_safe_for_multibyte_input` — 3000 multi-byte chars
      (include `'é'`, an emoji, and the `󰀨` Nerd Font glyph from the observed
      failure); no panic, `chars().count() <= max`
- [ ] `truncation_does_not_split_a_multibyte_char` — no U+FFFD in the result
- [ ] `helper_honors_the_limit_argument` — same input at 2000 vs 4096 differs
- [ ] `budget_below_the_marker_length_still_respects_the_limit` —
      `truncate_to_chars(over_length, 3)` is exactly 3 chars, **no** marker
- [ ] `bound_to_chars_reports_no_cut_for_under_limit_input`
- [ ] `bound_to_chars_reports_the_cut_and_the_original_length` — `original_chars`
      is pre-cut; `text == truncate_to_chars(input, max)` exactly
- [ ] `ellipsis_char_count_matches_the_published_constant`
- [ ] `location_ellipsis_char_count_matches_the_published_constant`
- [ ] `location_line_under_the_cap_is_unchanged` — a real `format_text_line()`
      output round-trips byte-identical
- [ ] `location_line_over_the_cap_gets_the_inline_marker` — from a 400-char
      `name`: `<= 256` chars, `ends_with("…")`, contains **no** `'\n'`, and does
      **not** contain `"\n\n..."`
- [ ] `capped_location_line_still_begins_with_the_pin` — still starts with `"📍"`
- [ ] `truncate_location_line_is_truncate_inline_at_the_cap` — equal for
      over-cap, at-cap, and under-cap inputs
- [ ] `inline_truncation_uses_the_single_char_marker` — at the 1024 attachment
      budget: `<= 1024`, `ends_with("…")`, no `'\n'` at all
- [ ] `bound_inline_to_chars_reports_no_cut_for_under_limit_input`
- [ ] `bound_inline_to_chars_reports_the_cut_and_the_original_length`
- [ ] `inline_truncation_is_char_safe_for_multibyte_input` — 3000 multi-byte
      chars at 1024
- [ ] `truncation_metadata_value` — all **eight** `TruncationOutcome`
      combinations from R6's table, one table-driven test or eight rows. The
      fixed order is what these exist to pin.

**∥ Parallel:** 1.2/1.3/1.4 are one file and should be written together; 1.5 is
independent and can be done first or last.

### ✅ Checkpoint 1

- [ ] `just test` green in `messenger/`
- [ ] `just lint` green in `messenger/`
- [ ] `cargo build -p messenger --no-default-features --features desktop`
      succeeds with **no `dead_code` warning escaping the module** (early SC12
      signal — the module has consumers only from Phase 2 onward)
- [ ] `grep -rn "truncate\|char_indices" messenger/lib/src --include=*.rs` shows
      hits **only** in `provider/limits.rs`

---

## Phase 2 — Location Reservation in `prepared.rs`

**Goal:** the reserve-truncate-append sequence exists in exactly one place and
is proven byte-identical to today's output for every location a supported entry
point can build.

**Depends on:** Phase 1 (`BoundedField`, `truncate_to_chars`,
`truncate_location_line`, `MAX_LOCATION_LINE_CHARS`).

**Covers:** R4.1 (except the adapter call sites), SC9, the SC7 half that belongs
to sites 3 and 6.

### 2.1 The shared sequence

- [ ] Add `use crate::provider::limits::{...}` to `prepared.rs`. This is the
      first `provider::*` import in the file and is intentional — R4.1's
      "Placement" argument rests on `prepared.rs` already being provider-aware.
- [ ] Implement `pub(crate) fn bound_with_location(&self, text: String,
      max_chars: usize) -> BoundedField` on `PreparedMessage`, performing exactly:
      ```text
      capped      = truncate_location_line(location_line)
      reserved    = capped.chars().count() + 1        // the '\n' separator
      body_budget = max_chars.saturating_sub(reserved)
      result      = truncate_to_chars(text, body_budget) + '\n' + capped
      ```
- [ ] Insert the separator **only when the body is non-empty**, matching
      `render_body_with_location`'s existing behavior.
- [ ] With **no location**: `reserved` is 0 and the result must be exactly
      `truncate_to_chars(text, max_chars)` — bit-for-bit the pre-R4.1 behavior.
- [ ] Set `original_chars` from the **uncapped** location line:
      `body.chars().count() + 1 + format_text_line().chars().count()`, so R5's
      `warn!` reports the true size of what was lost.
- [ ] Set `truncated` to `true` if **either** the body or the location was cut
      (one flag over both losses — R4.1's explicit decision).
- [ ] Add **no** run-time fallback branch. R1's compile-time assertions
      (Phase 3) are what keep this sound.
- [ ] Implement `pub(crate) fn render_body_with_location_bounded(&self, provider:
      ProviderKind, max_chars: usize) -> BoundedField` as the one-line
      delegation in R4.1.
- [ ] Neither new method carries a `cfg` (R7 / SC12).

### 2.2 Doc drift on the untouched function

- [ ] Leave `render_body_with_location`'s **signature and body** untouched
      (SC9). Change **only** its doc comment: it currently reads *"Use this for
      providers without native location APIs (Discord, Slack, Signal)"* —
      Discord is now precisely the provider that must **not** use it. Rewrite to
      name Slack, Slack-Webhook, and Signal, and to point Discord and any future
      length-bounded provider at `render_body_with_location_bounded`.

### 2.3 Reservation tests (`messenger/lib/src/tests/builders.rs`, ungated)

Place the provider-neutral rows here beside the four existing
`render_body_with_location` tests. Adapter-level rows live in Phase 6.

- [ ] `location_reservation_never_exceeds_the_cap` — location lines of 0, 67,
      256, 257, and 2500 rendered chars; reserved budget never exceeds
      `MAX_LOCATION_LINE_CHARS + 1`
- [ ] `message_without_location_is_unaffected_by_the_reservation` — 5000-char
      body, no location → identical to plain `truncate_to_chars`
- [ ] `bounded_render_matches_render_body_with_location_when_under_limit` —
      `render_body_with_location_bounded(kind, usize::MAX).text ==
      render_body_with_location(kind)` across plain / markdown / empty-body /
      with- and without-location. An over-cap location is **deliberately not**
      covered: the cap is the one intended difference.
- [ ] `original_chars_counts_the_uncapped_location`
- [ ] `location_only_cut_still_flags_the_field` — short body + 2500-char
      location → `truncated == true`
- [ ] `render_body_with_location_is_unchanged` — the four existing tests at
      `builders.rs:123`, `:132`, `:140`, `:148` still pass **untouched**
- [ ] Construct pathological locations by **struct literal**
      (`Location { latitude, longitude, name: Some(huge), address: None }`) —
      no supported constructor can produce one (R4.1's reachability finding).

### ✅ Checkpoint 2

- [ ] `just test` and `just lint` green
- [ ] `grep -n "render_body_with_location" messenger/lib/src/provider/*.rs`
      still shows five callers (`discord.rs`, `discord_webhook.rs`, `signal.rs`,
      `slack.rs`, `slack_webhook.rs`) — Phase 2 changes no call site
- [ ] `git diff messenger/lib/src/prepared.rs` shows the only change to
      `render_body_with_location` is its doc comment

---

## Phase 3 — Discord Bot Adapter (∥ with Phase 4)

**Goal:** `discord.rs` bounds all three surfaces, produces a
`TruncationOutcome`, and logs a char-counted `warn!` at each cut.

**Depends on:** Phase 2.

**Covers:** R1, R4 sites 1–3, R4.1 sites 2–3, R4.2 (bot half), R5 (bot half),
SC5, SC13 (bot half).

### 3.1 Constants and compile-time assertions

- [ ] Add `pub const DISCORD_MAX_CONTENT_LENGTH: usize = 2000`,
      `pub const DISCORD_MAX_EMBED_DESCRIPTION_LENGTH: usize = 4096`, and
      `pub const DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH: usize = 1024` to
      `discord.rs`, each with the doc comment from R1 **documenting the platform
      error it prevents** (including `twilight-validate`'s own
      `ATTACHMENT_DESCIPTION_LENGTH_MAX` typo, cited as such).
- [ ] Do **not** name the first one `DISCORD_MAX_LENGTH`.
- [ ] Add the two `const _: () = assert!(...)` lines from R1 tying
      `MAX_LOCATION_LINE_CHARS + 1 + ELLIPSIS_CHARS` below each **body** limit.
      No assertion for the attachment limit — no location is ever appended there.

### 3.2 Visibility (deliberate, for test reach)

- [ ] Raise `DiscordPayload`, `build_payload`, `build_attachment`, and
      `build_attachments` to `pub(crate)`. Add a short comment recording that
      this is **for test reach**, so a future reader does not narrow it back.
- [ ] Nothing becomes `pub` (SC13).

### 3.3 Payload construction

- [ ] Add `truncated: TruncationOutcome` to `DiscordPayload`.
- [ ] **Delete** the `location_line` local — both branches now get the location
      from `PreparedMessage` inside `bound_with_location`.
- [ ] **Embed branch (site 2):** stop appending the location inline. Replace the
      whole `if let Some(loc) = location_line { ... }` block with
      `message.bound_with_location(rich_md, DISCORD_MAX_EMBED_DESCRIPTION_LENGTH)`
      and build the embed from `.text`; set
      `truncated.embed_description` from `.truncated`.
- [ ] **Embed branch (site 1):** set `content` from
      `bound_to_chars(summary, DISCORD_MAX_CONTENT_LENGTH)`; set
      `truncated.content` from `.truncated`.
- [ ] **Plain branch (site 3):** replace `render_body_with_location(Discord)`
      with `render_body_with_location_bounded(Discord,
      DISCORD_MAX_CONTENT_LENGTH)`; set `truncated.content` from `.truncated`.
- [ ] Keep the existing `content.is_empty()` → `None` check, applied to the
      **truncated** value, and keep it **after** truncation for clarity.
- [ ] Do **not** add a title, footer, author, or fields to the embed — R4's
      embed-shape invariant is what keeps `EMBED_TOTAL_LENGTH` unreachable.
- [ ] No adapter code may re-derive a truncation flag by comparing lengths
      (SC10) — every flag comes off a `BoundedField`.

### 3.4 Attachment description (R4.2)

- [ ] Change `build_attachment` to
      `-> Result<(DiscordAttachment, bool), MessengerError>`.
- [ ] Bound the **resolved** description (`alt_text`, else `caption` — do not
      change the fallback chain) with
      `bound_inline_to_chars(description, DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH)`;
      the inline `"…"` marker, **not** the block `ELLIPSIS`.
- [ ] Keep the `if let Some(description)` guard: no description → `None` and
      flag `false`.
- [ ] Change `build_attachments` to
      `-> Result<(Vec<DiscordAttachment>, bool), MessengerError>`; replace the
      `.map(...).collect()` with a loop or `try_fold` that **ORs** the
      per-attachment flags. The `Result` shape is otherwise unchanged.

### 3.5 Logging (R5)

- [ ] Fix the existing `tracing::debug!`: `content_len` must be
      `chars().count()`, not `str::len` — a byte count against a
      char-denominated limit is misleading in exactly the multi-byte case this
      fix exists for.
- [ ] Emit one `tracing::warn!` **per cut field**, naming `provider`, `field`
      (`content` / `embed_description`), and both `original` and `final`
      character counts, sourced from `BoundedField.original_chars`.
- [ ] Emit one `warn!` per cut **attachment** (the documented exception to
      "per surface"), from inside `build_attachment`.
- [ ] Use `warn!`, not `debug!`. Add the one-line comment recording this as a
      **deliberate departure** from `validate.rs`'s `debug!` convention, so a
      future reader does not "correct" it.

### ✅ Checkpoint 3

- [ ] `just test` and `just lint` green (existing ten inline `discord.rs` tests
      still pass **in place** — do not move them)
- [ ] `grep -n "2000\|4096\|1024" messenger/lib/src/provider/discord.rs` shows
      hits only in the three constant definitions and their docs
- [ ] `grep -n "format_text_line" messenger/lib/src/provider/discord.rs` returns
      **nothing** — the adapter no longer formats a location line

---

## Phase 4 — Discord Webhook Adapter (∥ with Phase 3)

**Goal:** the webhook adapter reaches parity with the bot adapter, via an
extraction that provably does not change the wire shape.

**Depends on:** Phase 2. Independent of Phase 3.

**Covers:** R4 sites 4–6, R4.1 sites 5–6, R4.2 (webhook half), R5 (webhook
half), the Testing → Prerequisites extraction, SC13 (webhook half).

### 4.1 Extract `build_payload` — a pure move first

> ⚠️ **Three wiremock tests match on exact body equality**:
> `body_json(json!({…}))` at `tests/discord_webhook_integration.rs:165`, `:205`,
> and `:520`. `body_json` compares the **whole** deserialized body, so any
> change to `WebhookJsonBody`'s serialized shape turns them red for a reason
> unrelated to truncation. **If those three go red, the extraction changed the
> wire shape, not the matcher.**

- [ ] Add `pub(crate) struct DiscordWebhookPayload { content: Option<String>,
      embeds: Option<Vec<EmbedBody>>, truncated: TruncationOutcome }` mirroring
      the bot's `DiscordPayload`.
- [ ] Raise `EmbedBody` to `pub(crate)` — it appears as the field type of a
      `pub(crate)` struct, so leaving it private trips `private_interfaces`.
- [ ] Move the `let (content_owned, embeds_owned) = match rich { … }` block out
      of `send_prepared` into `pub(crate) fn build_payload(message:
      &PreparedMessage) -> DiscordWebhookPayload`, **as a pure move**: same
      logic, same values.
- [ ] Leave `WebhookJsonBody` exactly as it is — same name, same three fields,
      same `#[serde(skip_serializing_if = "Option::is_none")]` on all three, same
      borrowed `content: Option<&'a str>`. **Do not tidy the struct.**
- [ ] **Run the three exact-body tests now, before adding truncation**, so any
      wire-shape regression is attributed to the move rather than to the cut.

### 4.2 Apply the bounds

- [ ] **Delete** the `location_line` local at the top of `send_prepared`.
- [ ] **Site 5 (embed description):** replace the inline location append with
      `message.bound_with_location(rich_md,
      DISCORD_MAX_EMBED_DESCRIPTION_LENGTH)`; set `truncated.embed_description`.
- [ ] **Site 4 (summary → content):**
      `bound_to_chars(summary, DISCORD_MAX_CONTENT_LENGTH)`; set
      `truncated.content`.
- [ ] **Site 6 (plain/markdown → content):**
      `render_body_with_location_bounded(DiscordWebhook,
      DISCORD_MAX_CONTENT_LENGTH)`; set `truncated.content`.
- [ ] Import the three limits from `provider::discord` — the constants stay in
      `discord.rs` (R7) and are passed in as `max_chars`.
- [ ] Keep the `content.is_empty()` → `None` check, applied to the truncated
      value.

### 4.3 Extract `build_attachment_meta` (R4.2)

- [ ] Add `pub(crate) fn build_attachment_meta(attachment: &Attachment, index:
      usize) -> Result<(AttachmentMeta, Part, bool), MessengerError>`, absorbing
      the existing `Self::build_part` call and the `alt_text.or_else(caption)`
      fallback chain.
- [ ] Bound the resolved description with `bound_inline_to_chars(...,
      DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH)` — the same call the bot path
      makes, so the two adapters cannot drift.
- [ ] Raise `AttachmentMeta` to `pub(crate)`.
- [ ] Reduce the `metas` loop body to: call the new function, push the meta and
      the part, OR the flag.

### 4.4 Merge and log

- [ ] Merge the attachment flag into the payload's `TruncationOutcome` inside
      `send_prepared` (the flag cannot ride on the payload struct — attachments
      are built separately).
- [ ] Fix `content_len` in the existing `tracing::debug!` to `chars().count()`.
- [ ] Emit the same per-field and per-attachment `warn!` lines as Phase 3.5.

### ✅ Checkpoint 4

- [ ] `just test` and `just lint` green — **specifically** the three exact-body
      wiremock tests at `:165`, `:205`, `:520`
- [ ] `grep -n "2000\|4096\|1024" messenger/lib/src/provider/discord_webhook.rs`
      returns nothing (all three limits arrive as imported constants)
- [ ] `grep -n "format_text_line" messenger/lib/src/provider/discord_webhook.rs`
      returns nothing

---

## Phase 5 — Receipt Metadata and CLI Stdout

**Goal:** the truncation outcome escapes the crate — onto the receipt, through a
typed accessor, and into the CLI's stdout JSON.

**Depends on:** Phases 3 and 4 (both `TruncationOutcome` producers must exist).

**Covers:** R6, SC3, SC11.

### 5.1 Receipt metadata (both adapters)

- [ ] In `DiscordProvider::send_prepared`, after a successful send, insert
      `truncated` → `truncation_metadata_value(payload.truncated | attachment_flag)`
      into the receipt `metadata` **only when the value is `Some`** — the key is
      **absent** when nothing was cut.
- [ ] Do the same in `DiscordWebhookProvider::send_prepared`.
- [ ] Write **no** `dropped` key from either adapter (SC3). The two vocabularies
      stay separate; do not touch the desktop helpers that write `dropped`.
- [ ] Both adapters currently build `metadata: BTreeMap::new()`, so `truncated`
      is the only entry — no merge logic is needed and none should be added.

### 5.2 Typed accessor on `SendReceipt`

- [ ] Add `pub fn truncated_fields(&self) -> Option<&str>` to `receipt.rs`,
      immediately beside `helper_used()`, returning
      `self.metadata.get("truncated").map(String::as_str)`.
- [ ] **No `filter` clause** — unlike `helper_used`, there is no sentinel value
      to suppress, because R6 makes the key absent rather than empty.
- [ ] Document it with the exact return shapes from R6 (single token, or
      comma-joined in the fixed order, or `None`).
- [ ] This is the **only** new item on the crate's public API surface besides
      R1's three constants (SC13).

### 5.3 CLI stdout field

- [ ] Add `#[serde(skip_serializing_if = "Option::is_none")] truncated:
      Option<String>` to `SendResponse` in `messenger/cli/src/main.rs`,
      mirroring `helper` exactly.
- [ ] Populate it in `emit_send_response` with
      `receipt.truncated_fields().map(str::to_string)`.
- [ ] Change nothing about `id`, `receipt`, `helper`, or `os`.

### ✅ Checkpoint 5

- [ ] `just test` and `just lint` green
- [ ] `cargo build -p messenger-cli` succeeds
- [ ] Manual smoke on the JSON shape: an untruncated desktop send via
      `messenger send` prints **no** `truncated` field

---

## Phase 6 — Test Completion

**Goal:** every remaining test row from the spec's Testing section exists, is
placed where the spec says, and is proven non-vacuous.

**Depends on:** Phase 5.

**Covers:** the Testing section's remaining five tables; SC1, SC2, SC3, SC4,
SC6, SC7, SC8, SC10.

### 6.1 Test module placement

- [ ] Create the new **application-site** modules under
      `messenger/lib/src/tests/`, declared in `tests/mod.rs` with
      `#[cfg(feature = "discord")]` — their subjects (`build_payload`,
      `build_attachment`, the adapters) are Discord-gated. Follow the file's
      existing convention: ungated `mod builders/receipts/validation`, gated
      `mod *_integration`.
- [ ] Leave the ten existing inline `discord.rs` tests **where they are**.
      Relocating them is out of scope under Rule 3 and belongs in its own commit
      if anyone wants it.

### 6.2 Application-site tests (six body sites) — ∥ with 6.3–6.6

- [ ] `build_payload_truncates_plain_content` (site 3)
- [ ] `build_payload_truncates_summary_content` (site 1)
- [ ] `build_payload_truncates_embed_description` (site 2)
- [ ] `webhook_build_payload_truncates_plain_content` (site 6)
- [ ] `webhook_build_payload_truncates_summary_content` (site 4)
- [ ] `webhook_build_payload_truncates_embed_description` (site 5)
- [ ] `content_limit_and_embed_limit_are_not_swapped` — a 3000-char `Summarized`
      markdown is **not** cut while a 3000-char plain body **is**

### 6.3 Attachment-description tests (R4.2) — ∥

- [ ] `build_attachment_truncates_long_alt_text`
- [ ] `build_attachment_truncates_long_caption_used_as_fallback`
- [ ] `build_attachment_prefers_alt_text_over_caption_when_both_are_long`
- [ ] `build_attachment_under_limit_description_is_byte_identical`
- [ ] `build_attachment_with_no_description_reports_no_cut`
- [ ] `build_attachments_ors_the_flags_across_attachments` — three attachments,
      only the second over-length → aggregate `true`; all three → still one
      `true`
- [ ] `webhook_build_attachment_meta_truncates_long_description`
- [ ] `webhook_build_attachment_meta_matches_the_bot_result` — the anti-drift row

### 6.4 Location-reservation tests at the adapters (R4.1) — ∥

- [ ] `location_survives_truncated_content` (site 3)
- [ ] `location_survives_truncated_embed_description` (site 2)
- [ ] `webhook_location_survives_truncated_content` (site 6)
- [ ] `webhook_location_survives_truncated_embed_description` (site 5)
- [ ] `truncation_marker_sits_above_the_location_line` — result contains
      `"\n\n...\n"` immediately before the location line
- [ ] `pathological_location_is_capped_and_the_body_survives` — 2500-char
      `name`, 5000-char body, at 2000: result `<= 2000`, location segment
      `<= 256` ending in `"…"`, **at least 1743 chars of body survive**, no panic
- [ ] `under_limit_body_with_location_is_byte_identical` (sites 3/6, SC7)
- [ ] `under_limit_embed_with_location_is_byte_identical` (sites 2/5, SC7) —
      byte-identical to the former inline `rich_md + '\n' + format_text_line()`
- [ ] `embed_and_content_sites_share_the_reservation` — same capped location
      segment on site 2 and site 3
- [ ] `non_discord_callers_are_untouched` — Signal, Slack, and Slack-Webhook
      still call `render_body_with_location`; rendered text byte-identical

### 6.5 Limit-model tests (`twilight-validate`) — ∥

Gate the module `#[cfg(feature = "discord")]`.

- [ ] `truncated_payload_passes_twilight_content_validation`
- [ ] `truncated_payload_passes_twilight_embed_validation`
- [ ] `truncated_attachment_passes_twilight_attachment_validation`
- [ ] Do **not** replace these with assertions against
      `DISCORD_MAX_CONTENT_LENGTH` and `chars().count()` — the entire point is
      to pin the counting model against the **external** validator that gates
      the bot path.

### 6.6 Receipt-metadata tests (R6) — ∥

- [ ] `untruncated_send_has_no_truncated_metadata`
- [ ] `truncated_content_sets_truncated_metadata`
- [ ] `truncated_embed_description_sets_truncated_metadata`
- [ ] `truncated_attachment_description_sets_truncated_metadata`
- [ ] `all_three_truncated_sets_combined_truncated_metadata` — asserted
      **through a send**, not through the pure function
- [ ] `truncation_does_not_write_the_dropped_key`
- [ ] `truncated_metadata_survives_receipt_json_roundtrip`
- [ ] `webhook_truncated_send_sets_truncated_metadata` — **end-to-end** through
      the existing wiremock harness. This is the **only** end-to-end route SC3
      has; the bot path has no mock transport and no seam between payload
      construction and the live call, which is a stated limitation of the
      L1-only posture, **not** something to fix by adding a twilight mock
      transport (explicitly out of scope).

### 6.7 CLI stdout tests — ∥

Place in `messenger/cli/src/main.rs`'s existing `#[cfg(test)] mod tests`.

- [ ] `truncated_fields_returns_none_without_the_key`
- [ ] `truncated_fields_returns_the_raw_value` — returned verbatim, no filtering
- [ ] `send_response_omits_truncated_when_absent`
- [ ] `send_response_carries_the_truncated_value`

### 6.8 Non-vacuity proof (required by monorepo testing guidance)

- [ ] For each new test: neuter the truncation call (or the metadata insert),
      confirm the test goes **red**, restore. Record the sweep in
      `messenger/fixes/2026-09-06-truncation/log.md` (create it) so the evidence
      survives the session.
- [ ] Cheapest form of the sweep: temporarily make `truncate_with_marker` return
      `text` unchanged → the whole truncation suite must go red; then restore.
      Separately, stub `truncation_metadata_value` to `None` → the receipt and
      CLI rows must go red.

### ✅ Checkpoint 6

- [ ] `just test` green
- [ ] `just lint` green
- [ ] Non-vacuity sweep recorded in `log.md`

---

## Phase 7 — Cross-Cutting Validation

**Goal:** every success criterion has an observed answer, including the ones no
single test covers.

**Depends on:** Phase 6.

**Covers:** SC5, SC6, SC12, SC13, SC17.

### 7.1 Feature-matrix build (SC12)

- [ ] `cargo build -p messenger --no-default-features --features desktop` — the
      crate compiles.
- [ ] `cargo test -p messenger --no-default-features --features desktop
      limits::` — the `limits` unit tests **run** in that feature set.
- [ ] No `dead_code` warning escapes `provider/limits.rs`'s module-level
      `#![allow(dead_code)]`.
- [ ] `cargo build -p messenger --no-default-features --features slack` — the
      location cap exists in a Discord-free build (this is R7's neutrality claim
      made literally true).

### 7.2 Magic-number audit (SC5)

- [ ] `grep -rn "\b2000\b\|\b4096\b\|\b1024\b" messenger/lib/src/provider/discord.rs
      messenger/lib/src/provider/discord_webhook.rs messenger/lib/src/tests/` —
      hits only at the three constant definitions and their doc comments.
- [ ] `grep -rn "\b256\b" messenger/lib/src/provider/limits.rs
      messenger/lib/src/prepared.rs` — `256` appears only as
      `MAX_LOCATION_LINE_CHARS`.
- [ ] The two `const _: () = assert!(...)` lines are present in `discord.rs` and
      the crate builds.

### 7.3 Visibility audit (SC13)

- [ ] `build_payload` (both adapters), `DiscordPayload`,
      `DiscordWebhookPayload`, `EmbedBody`, `build_attachment`,
      `build_attachments`, `build_attachment_meta`, and `AttachmentMeta` are all
      `pub(crate)`.
- [ ] **None** of them is `pub`.
- [ ] The public API delta is exactly: `SendReceipt::truncated_fields()` plus
      R1's three constants. Confirm with
      `cargo public-api -p messenger` if available, else by inspection.

### 7.4 Behavior-change acknowledgement

- [ ] Confirm and record in `log.md`: on the **bot** path, a caller that today
      receives `Err(MessengerError::Transport)` for an over-length body now
      receives `Ok(SendReceipt)` for a shortened one. This is deliberate and
      accepted (Root Cause → "The two adapters do *not* fail the same way"), and
      R6's receipt key plus R5's `warn!` are what stop it being silent.
- [ ] Record the named known consequence: **this does not fix Claudine**.
      `claudine/lib/src/messaging/send.rs:714-717` discards the receipt, so
      `truncated_fields()` reaches nothing there. The remedy is a Claudine-side
      follow-up, out of scope here.

### 7.5 Optional one-off smoke (not gating)

- [ ] If a real `discord_webhook` route is available, send one over-length
      message and confirm no 400. **Not a gating criterion** — the Testing
      posture is deliberately network-free (SC1).

### ✅ Checkpoint 7

- [ ] `just test`, `just test-l2`, and `just lint` green in `messenger/`
- [ ] `just ci-local` green (repo convention before pushing)
- [ ] Every SC1–SC13, SC16, SC17 row has an observed answer recorded in `log.md`

---

## Phase 8 — Documentation (∥ from Phase 5 onward; separate commit)

**Goal:** the six drift updates land and the research doc stops teaching a
panicking implementation.

**Depends on:** Phase 5 for accuracy of the described behavior; may be **drafted**
in parallel from Phase 5 onward.

**Covers:** R8, R9, SC14, SC15.

> **Scope discipline:** these are documentation-only edits and belong in their
> own commit. `git diff` of that commit must show **no** non-comment code change.

### 8.1 R8 — the research doc's reference implementation

- [ ] In `messenger/docs/research/platforms/discord.md` (~`:945-951`), replace
      `format!("{}...", &s[..max.saturating_sub(3)])` with a call to
      `truncate_to_chars`, or a char-correct inline equivalent.
- [ ] Express the sample against the builder the crate **actually uses** —
      `twilight_util::builder::embed::EmbedBuilder`, or the webhook's `EmbedBody`
      — **not** serenity's `CreateEmbed`, which is not a dependency of this
      workspace at all.
- [ ] Note the byte-vs-char trap in the surrounding prose.
- [ ] Trim the sample's embed **title** truncation at `:954` — neither adapter
      sets a title, and leaving it reads as a recommendation to add one.

### 8.2 R9 — the six drift updates (∥ within this group)

- [ ] **`messenger/README.md`** — one line under the Discord rows: content over
      2000, embed descriptions over 4096, and attachment descriptions over 1024
      are **truncated rather than rejected**, and the receipt carries
      `metadata["truncated"]`. The README already documents
      `metadata["delivery_confirmed"]` for Slack-Webhook (`:36`) — follow that
      shape.
- [ ] **`messenger/docs/user-guide.md` (provider + receipts)** — the same note
      in the Discord provider section; the `truncated` key beside the existing
      `delivery_confirmed` example (anchor on the **symbol**, ~`:628-629`, not
      R9's stale `:451-457`); and `SendReceipt::truncated_fields()` named beside
      `helper_used()` (~`:686`).
- [ ] **`messenger/docs/user-guide.md` (CLI stdout)** — the stdout response
      schema is currently documented **nowhere**, despite `SendResponse`'s own
      doc comment referring to "the labels documented in the response schema".
      This row is therefore *"document `id` / `receipt` / `helper` / `os` /
      `truncated` as a set"*, **not** "add one line".
- [ ] **`.claude/skills/messenger/cli-reference.md`** — the same stdout schema,
      in the "Receipts and Replies" section (~`:88`), which today describes the
      receipt *file* but not the stdout JSON.
- [ ] **`.claude/skills/messenger/providers.md`** — a "Receipt semantics" note
      for Discord and Discord-Webhook, mirroring the Slack-Webhook one at `:100`.
- [ ] **`docs/dependencies.md` (repo root)** — record the new
      `twilight-validate` dev-dependency. The root doc already lists dev-only
      crates such as `wiremock`. **There is no `messenger/docs/dependencies.md`
      on HEAD — do not create one.**

### 8.3 Out-of-scope items to record, not fix

- [ ] Confirm the spec's Out of Scope list is intact and that **none** of these
      were touched: message splitting, attachment fallback, other providers'
      limits, the non-Discord byte-vs-char logging, the Claudine-side receipt
      binding, the `dropped` doc drift at `receipt.rs:6`, length-validating
      `Location.name`/`address`, a `Location` builder, 400-body rendering,
      Markdown integrity across the cut, `Attachment::caption` semantics, and
      `Message.title`.
- [ ] Raise as **separate** follow-ups (do not fix here): the `receipt.rs:6`
      `dropped` doc drift, and the Claudine-side change binding the receipt and
      logging `truncated_fields()` beside the existing `debug!("Message sent
      successfully")` at `send.rs:719-723`.

### ✅ Checkpoint 8 — Final

- [ ] `just test` and `just lint` green in `messenger/` (SC17)
- [ ] `md hash` run on any Markdown file carrying a `hash:` frontmatter property
      that this phase edited
- [ ] All seventeen success criteria confirmed, with SC14/SC15 covered by this
      phase and SC1–SC13/SC16 by Checkpoint 7
- [ ] Behavior changes and documentation changes are in **separate** commits

---

## Risk Register

| Risk | Phase | Signal | Mitigation |
|------|-------|--------|------------|
| Webhook payload extraction changes the wire shape | 4 | The three `body_json` tests at `:165`, `:205`, `:520` go red | Extract as a **pure move** and run those three before adding truncation. If they are red, the extraction is wrong — not the matcher. |
| Byte-slicing reintroduced | 1 | Panic on multi-byte input | R3: `char_indices` only; the multi-byte tests use the actual `󰀨` glyph from the observed failure |
| Truncation flag re-derived at an adapter | 3, 4 | SC10 audit finds a `chars().count()` comparison in `discord*.rs` | Every flag comes off a `BoundedField`; the audit is Checkpoint 3/4's grep |
| `dead_code` warnings under Discord-free builds | 1, 7 | `cargo build --no-default-features --features desktop` warns | Module-level `#![allow(dead_code)]` in `limits.rs`, verified at 7.1 |
| `EmbedBody` / `AttachmentMeta` private-in-public | 4 | `private_interfaces` lint | Raise both to `pub(crate)` alongside the payload struct |
| Bot path silently changes `Err` → `Ok` | 3, 7 | No test catches it — it is the intended change | Deliberate and accepted; recorded at 7.4 and mitigated by R5's `warn!` and R6's receipt key |
| Location cap starves the body | 2 | `body_budget` below `ELLIPSIS_CHARS` | R1's two compile-time assertions fail the **build** if a constant is edited badly; there is no run-time fallback by design |
| Scope creep into other providers | all | Telegram/Slack/Signal files appear in `git diff` | Only `signal.rs`/`slack.rs`/`slack_webhook.rs` **call sites** are asserted unchanged; no edits to them |
