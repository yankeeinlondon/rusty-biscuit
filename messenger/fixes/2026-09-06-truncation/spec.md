# Discord Content Truncation

> **Status:** draft — the requirements below are decided; one open question
> (D2, trailing-whitespace trim) still needs Ken's call.

## Summary

Discord rejects any message whose `content` field exceeds 2000 characters, and
any embed whose `description` exceeds 4096. The messenger library never bounds
either string. This fix teaches both Discord adapters both limits — expressed as
`DISCORD_MAX_CONTENT_LENGTH` and `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` — and
truncates over-length text instead of failing the send, flagging the loss on the
returned `SendReceipt`.

## Observed Failure

```
󰀨 Failed to send lifecycle message via discord_webhook route coding: Send failed: Discord-Webhook error (400): {"content": ["Must be 2000 or fewer in length."]}
```

The caller is a Claudine lifecycle message; the failure surfaces from
`MessengerError::Provider { provider: DiscordWebhook, code: Some("400"), message: <raw body> }`.
The whole message is lost — Discord does not partially deliver.

## Root Cause

Neither Discord adapter bounds the strings it builds:

- `messenger/lib/src/provider/discord.rs:58` — `DiscordProvider::build_payload`
  sets `content` from `render_summary()` (embed branch) or
  `render_body_with_location()` (plain branch), and sets the embed
  `description` from `render_rich()`, with no length check on any of them.
- `messenger/lib/src/provider/discord_webhook.rs:253` — the same three
  assignments, inline in `DiscordWebhookProvider::send_prepared`.

Both paths already compute `content_len` for a `tracing::debug!` line
(`discord.rs:192`, `discord_webhook.rs:287`), so the value is observable but
unused as a guard — and both compute it as a **byte** count against a
**character** limit, so even the logged number is the wrong quantity.

### The two adapters do *not* fail the same way

An earlier draft of this spec claimed "the bot path is affected identically to
the webhook path." That is false, and the difference shapes the fix.

`messenger/lib/Cargo.toml:19` pins `twilight-http = "0.17"`.
`CreateMessage::content()` calls `twilight_validate::message::content()`, which
is exactly `value.chars().count() <= MESSAGE_CONTENT_LENGTH_MAX` (= 2000).
`CreateMessage` stores the resulting `Err` in its `fields` and surfaces it at
`.await` through `try_into_request()` → `Error::validation` → `ErrorType::Validation`.
`discord.rs:220-223` maps that through `ProviderKind::Discord.transport_error(e)`
(`receipt.rs:64`) into `MessengerError::Transport` — **not**
`MessengerError::Provider { code: Some("400") }`. No network call is made.

`.embeds()` behaves the same way, validating against `DESCRIPTION_LENGTH` (4096)
and the running `EMBED_TOTAL_LENGTH` (6000) sum. Note that `EmbedBuilder::build()`
does **not** validate — it is a bare `self.0` return; the check happens at
`req.embeds(...)` in `discord.rs:208`.

`DiscordWebhookProvider` hand-rolls `reqwest` against the webhook URL and skips
twilight's validators entirely, which is why only it produced the reported 400.

**Consequence, stated plainly:** `DiscordProvider` today fails *closed*,
client-side, char-counted — already matching R3's counting model. So this fix
means something different on each adapter:

- **Webhook:** turns *nothing delivered* into *partial delivery*. A clear
  improvement.
- **Bot:** turns a *loud client-side failure* into *silent partial delivery*.

That asymmetry is deliberate and accepted (see R6, which is how the loss stops
being silent), but it is a real behavior change on the bot path and must not be
glossed: a caller that today gets an `Err` for an over-length body will, after
this fix, get an `Ok(SendReceipt)` for a shortened message.

Secondary symptom: the webhook's 400 body is passed through verbatim into the
error message, so the operator sees raw Discord JSON rather than a
messenger-level explanation. Improving that rendering is out of scope here (see
[Out of Scope](#out-of-scope)) — once truncation lands, this particular 400
should no longer occur.

## In Scope

### R1 — Named limit constants

Introduce two public constants in `discord.rs`:

```rust
/// Maximum number of characters Discord accepts in a message `content` field.
///
/// Discord rejects longer payloads with HTTP 400 and the body
/// `{"content": ["Must be 2000 or fewer in length."]}`; `twilight-http`
/// rejects them client-side before the request is issued.
pub const DISCORD_MAX_CONTENT_LENGTH: usize = 2000;

/// Maximum number of characters Discord accepts in an embed `description`.
pub const DISCORD_MAX_EMBED_DESCRIPTION_LENGTH: usize = 4096;
```

The content constant is deliberately *not* named `DISCORD_MAX_LENGTH`: once
there are two limits the bare name is ambiguous about which surface it bounds.

The magic numbers `2000` and `4096` must not appear anywhere else in the Discord
adapters or their tests; every reference goes through a constant.

### R2 — Truncation helper

A single shared helper performs the truncation so the bot and webhook adapters
cannot drift, and so the content and embed cases cannot drift from each other:

```rust
pub(crate) fn truncate_to_chars(text: String, max_chars: usize) -> String;
```

Behavior:

1. If the text is at most `max_chars` characters, return it unchanged (no
   allocation churn, no marker appended).
2. Otherwise, keep the first `max_chars - ELLIPSIS.chars().count()` characters
   and append the marker `"\n\n..."`.

`ELLIPSIS` is `"\n\n..."`, five characters, so a truncated result is exactly
`max_chars` characters — matching the "5 characters less than the max, then
append" requirement.

The helper is provider-neutral: the limit is a parameter, not baked in. See R7
for where it lives.

### R3 — Character counting, not byte counting

Discord's limit is expressed in characters, and `String::len()` in Rust is a
**byte** count. The helper must count and slice by `char`, not by byte:

- Use `text.chars().count()` for the length test.
- Use `char_indices()` (or `chars().take(n).collect()`) to find the cut point.

Slicing a `String` at a byte offset that is not a character boundary **panics**.
A naïve `&text[..1995]` implementation would turn a recoverable over-length
message into a process abort for any body containing multi-byte characters —
which lifecycle messages routinely do (the failing message above begins with a
Nerd Font glyph).

Grapheme clusters are deliberately *not* used: Discord counts code points — and
so does `twilight_validate`, via `chars().count()` — and matching that counting
model is what keeps the 400 (and the client-side `Validation` error) from
recurring. A truncation that splits a multi-code-point emoji sequence is
acceptable.

### R4 — Applied at every over-length site

There are **six** real application sites, not four. The earlier draft's table was
wrong about which string overflows: in the embed branch, `content` comes from
`render_summary()`, which for a `Summarized` body returns `summary.clone()`
verbatim (`prepared.rs:101`) — the short hand-written banner, almost never the
overflowing field. The long text goes to the embed **description**.

| # | Site | Field | Body shape | Source of text | Limit |
|---|------|-------|------------|----------------|-------|
| 1 | `discord.rs:72-76` | `content` | `Summarized` | `render_summary()` | `DISCORD_MAX_CONTENT_LENGTH` |
| 2 | `discord.rs:71` | embed `description` | `Summarized` | `render_rich()` + location line | `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` |
| 3 | `discord.rs:83-88` | `content` | `Plain` / `Markdown` | `render_body_with_location()` | `DISCORD_MAX_CONTENT_LENGTH` |
| 4 | `discord_webhook.rs:261-265` | `content` | `Summarized` | `render_summary()` | `DISCORD_MAX_CONTENT_LENGTH` |
| 5 | `discord_webhook.rs:268-271` | embed `description` | `Summarized` | `render_rich()` + location line | `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` |
| 6 | `discord_webhook.rs:274-279` | `content` | `Plain` / `Markdown` | `render_body_with_location()` | `DISCORD_MAX_CONTENT_LENGTH` |

Sites 1 and 4 are the low-traffic cases and are covered for symmetry, not
because they are the reported failure: a `Summarized` summary is caller-authored
and short by construction. Sites 2 and 5 are where the reported class of failure
actually lands for `Summarized` bodies; sites 3 and 6 are where the reported
failure landed for the plain/markdown lifecycle message.

Truncation happens **after** the location line is appended, so the final wire
value is what is measured (`discord.rs:65-70`, `discord_webhook.rs:255-260`).
The existing empty-string check (`content.is_empty()` → `None`) is applied to the
truncated value; truncation can never produce an empty string from a non-empty
input, so ordering between the two is not load-bearing, but keep truncation
first for clarity.

**Known consequence of that ordering:** the location line is the *last* thing
appended to the embed description, so on an over-length `Summarized` body the
location is the *first* thing lost. This is the honest reading of "measure the
wire value", and it is not being changed here, but it is worth Ken's awareness —
preserving the location line across a cut would require truncating the rich body
before the append and is a distinct behavior.

**Embed-shape invariant.** The embed carries **only** a description today — no
title, footer, author, or fields, in either adapter (`discord.rs:71` builds it
with `EmbedBuilder::new().description(...)` and nothing else;
`discord_webhook.rs:181-184` defines `EmbedBody` as a single `description`
field). Capping the description at 4096 therefore bounds the *whole* embed, which
makes `EMBED_TOTAL_LENGTH` (6000) and `TITLE_LENGTH` (256) unreachable and keeps
this fix to a single embed constant. **If the embed ever gains a title, footer,
or fields, this invariant breaks and the total-length budget must be revisited.**

### R5 — Structured logging

The existing `tracing::debug!` in each adapter reports the post-truncation
length; where a length is logged it must be a **character** count, since the
current `str::len` / `String::len` calls at `discord.rs:192` and
`discord_webhook.rs:287` report bytes against a char-denominated limit and are
therefore misleading in exactly the multi-byte case this fix exists for.

When truncation occurs, emit a `tracing::warn!` naming the provider, the field
(`content` or `embed_description`), and both the original and final character
counts.

### R6 — Truncation is always applied and flagged on the receipt

**Decided (replaces the former D3).** Truncation is unconditional.
`CompatibilityMode` / the CLI's `--strict` flag is **not** consulted: a strict
caller does not get an error instead of a truncated send.

In addition to R5's logging, a **successful** send whose `content` or embed
`description` was truncated records the fact on `SendReceipt.metadata`.

**Decided contract** — following the established precedent at
`snoretoast.rs:188-190` (`metadata.insert("dropped".to_string(), "image_too_large".to_string())`):

| Key | Value | Meaning |
|-----|-------|---------|
| `dropped` | `content_truncated` | only `content` was cut |
| `dropped` | `embed_description_truncated` | only the embed description was cut |
| `dropped` | `content_truncated,embed_description_truncated` | both were cut |

The key is absent when nothing was truncated. The both-case value is the two
tokens joined by `,` in that fixed order, so the value stays deterministic and
parseable.

Rationale for reusing `dropped` rather than inventing a key: `dropped` is
already this crate's word for *"delivery succeeded but something was lost"*, and
keeping one key across providers means a future typed accessor on `SendReceipt`
can cover both cases. There is no collision — `dropped=image_too_large` is
written only by the Windows desktop helper, and both Discord adapters currently
write `metadata: BTreeMap::new()` (`discord.rs:238`, `discord_webhook.rs:387`).

Rationale for choosing the receipt over a `CompatibilityWarning`:
`SendPlan::warnings` (`provider/mod.rs:81`) is frozen at `plan_send()`
(`provider/mod.rs:141`), before per-provider rendering, so it structurally
cannot carry a post-render fact. Receipt metadata is the crate's established
channel for post-render lossy-delivery signals. `SendReceipt` is
`Serialize`/`Deserialize` (`receipt.rs:211`), so the flag persists into the CLI
receipt store for free.

**Explicitly accepted gap (not done here):** `emit_send_response`
(`messenger/cli/src/main.rs:495-513`) prints only `id` / `receipt` / `helper` /
`os` from the `SendResponse` struct (`main.rs:466`), so a truncation flag is
**not** visible in CLI stdout without a new `SendResponse` field. It is visible
in the persisted receipt JSON and in the `tracing::warn!`. Surfacing it in
stdout is a follow-up, not part of this fix.

### R7 — Helper lives in a shared `provider/limits.rs`

**Decided (replaces the former D4, against that draft's own recommendation).**

Create `messenger/lib/src/provider/limits.rs` containing **only** the
provider-neutral primitive from R2:

```rust
pub(crate) fn truncate_to_chars(text: String, max_chars: usize) -> String;
```

The Discord constants stay in `discord.rs` and are passed in as the `max_chars`
argument.

Rationale:

- Once the function takes a `usize`, it is genuinely provider-neutral; nothing
  about it is Discord-specific.
- `attachment_helpers.rs` establishes ~20 lines as an accepted module unit in
  this crate — it is 38 lines for one function.
- This repo has already demonstrated that it gets char-safe truncation wrong
  when the logic is written locally: see R8.
- Six sites across five providers already log a **byte** `len()` against
  **char**-counted platform limits — `telegram.rs:253`, `slack.rs:114`,
  `slack_webhook.rs:250`, `signal.rs:165`, `discord.rs:192`,
  `discord_webhook.rs:287`. A named, shared, char-correct primitive is the thing
  those sites will need when their own limits get fixed.

**`cfg` gating — implementation detail to settle during implementation, not a
decision needing Ken.** An ungated module trips `dead_code` in a build without
the `discord` feature. The two existing precedents disagree:

- `attachment_helpers` is single-feature-gated: `#[cfg(feature = "discord")]`
  (`provider/mod.rs:3-4`).
- `http_helpers` uses an explicit seven-feature `any(...)` list
  (`provider/mod.rs:28-37`) — which notably does **not** include `desktop`, and
  which has to be edited every time a provider is added.

Recommendation: gate `limits` with `#[cfg(feature = "discord")]`, matching
`attachment_helpers`, since Discord is its only consumer today. Widen it to an
`any(...)` list when the second provider adopts it — at which point the
maintenance cost of the list is being paid for a reason.

**Rejected alternatives** (recorded so they are not re-litigated):

- **A `ProviderKind`-keyed limit table.** Encodes a uniformity that does not
  exist. Slack's 3000 is per *block*, not per message; APNs is a payload **byte**
  budget, not a character count; desktop-helper limits are per-CLI and per-OS.
  A single `fn limit_for(kind) -> usize` would be wrong for most of its rows.
- **A `max_content_chars` field on `CapabilitySet`.** `CapabilitySet` is consumed
  pre-render by `normalize_dispatch` (`provider/mod.rs:68`, `:134`), so a limit
  declared there would guard the *unrendered* `Message` body — the wrong string,
  since per-provider Markdown rendering changes the length. It is also a breaking
  change: `CapabilitySet` is not `#[non_exhaustive]`
  (`messenger/lib/src/capabilities.rs:7`), so every external constructor of the
  struct would fail to compile.

### R8 — Replace the panicking reference implementation in the research doc

`messenger/docs/research/platforms/discord.md:945-951` presents this as "The
Solution":

```rust
format!("{}...", &s[..max.saturating_sub(3)])
```

That is byte-slicing at an arbitrary offset — the exact panic R3 exists to
prevent — and it is measuring `s.len()` (bytes) against a character limit at
line 946. Leaving it in place as the repo's reference implementation is how the
next adapter reintroduces the bug.

**In scope for this fix:** replace that sample with a call to
`truncate_to_chars`, or with a char-correct inline equivalent, and note the
byte-vs-char trap in the surrounding prose. This is a documentation-only change
and belongs in its own commit per the repo's scope-discipline rule.

## Open Question

One item from the original draft is still undecided.

### D2 — Trailing-whitespace trim (recommend: yes)

If the 1995-character cut lands inside a run of newlines or spaces, the result
reads as `"…text\n\n\n\n..."`. Trimming trailing whitespace from the truncated
slice before appending the marker yields a cleaner result at the cost of the
output being *at most* `DISCORD_MAX_CONTENT_LENGTH` rather than exactly it.

Recommendation: apply `trim_end()` to the truncated slice. The literal reading of
the request ("truncate to 5 less than the max, then append") is preserved for
every case except trailing whitespace at the boundary.

This affects two test rows below
(`content_over_limit_is_truncated_to_exact_limit` and
`embed_description_over_limit_is_truncated_to_exact_limit`), which assert an
exact length; a `yes` turns those into `<=` assertions plus a
"no trailing whitespace before the marker" assertion.

## Out of Scope

- **Splitting long messages across multiple sends.** Discord clients commonly
  chunk over-length content into several messages. That changes the receipt
  contract (one `SendReceipt` per `send`) and is a feature, not a fix.
- **Attachment fallback.** Uploading the full body as a `.md` attachment and
  truncating the inline content is a plausible future behavior; not here.
- **Other providers' limits.** Telegram (4096), Slack (3000 per block), APNs
  payload size, and desktop-helper body limits all have their own caps and their
  own failure modes. Each deserves its own fix with its own constant. This spec
  establishes the pattern and the shared primitive; it does not roll them out.
- **Fixing the byte-vs-char `len()` logging in the non-Discord adapters.** The
  five sites listed in R7 are named there as motivation for the shared helper;
  correcting them belongs with each provider's own limit fix.
- **Surfacing the truncation flag in CLI stdout.** See the accepted gap in R6.
- **Rendering of provider 400 bodies.** The raw-JSON-in-the-error-message
  ergonomics issue is real but independent.
- **Markdown integrity across the cut.** Truncation can leave an unterminated
  code fence or bold run. Discord renders that harmlessly. Making the cut
  markdown-aware is not worth the complexity.

## Testing

All L1 (`just test` in `messenger/`). No network or L2 coverage is required —
the behavior is pure string handling on the payload-construction path.

### Prerequisite

The webhook `send_prepared` builds its payload inline (`discord_webhook.rs:249-282`)
rather than in a testable `build_payload` function like the bot adapter has
(`discord.rs:58`). This is still accurate on HEAD. Extracting the webhook payload
construction into a private `build_payload` mirroring `discord.rs` is a
prerequisite for the webhook rows below and is in scope for this fix.

Additionally, so the R6 receipt flag can be tested without a live twilight client
or a wiremock server, the truncation outcome must be **observable from
`build_payload`'s return value** (both adapters), with the flag→metadata mapping
done by a small pure function that tests can call directly. The exact shape of
that return value is an implementation detail.

### Helper unit tests (`provider/limits.rs`)

| Test | Assertion |
|------|-----------|
| `text_under_limit_is_unchanged` | input of `max-1` chars round-trips byte-identical; no marker appended |
| `text_at_exact_limit_is_unchanged` | input of exactly `max` chars round-trips unchanged |
| `text_over_limit_is_truncated_to_exact_limit` | input of `max+1` chars → result is exactly `max` chars *(becomes `<= max` if D2 is `yes`)* |
| `truncated_text_ends_with_marker` | result ends with `"\n\n..."` |
| `truncated_text_preserves_leading_text` | the first `max-5` chars of the result match the first `max-5` of the input |
| `truncation_is_char_safe_for_multibyte_input` | input of 3000 multi-byte chars (e.g. `'é'`, emoji, the `󰀨` glyph) truncates without panicking, and `chars().count() == max` |
| `truncation_does_not_split_a_multibyte_char` | no replacement characters or invalid UTF-8 in the result |
| `helper_honors_the_limit_argument` | the same input truncated at 2000 and at 4096 produces different lengths — proves the limit is a parameter, not baked in |

### Application-site tests (six sites from R4)

| Test | Assertion |
|------|-----------|
| `build_payload_truncates_plain_content` | site 3: `DiscordProvider::build_payload` with a 5000-char `Markdown` body → `content` is `DISCORD_MAX_CONTENT_LENGTH` chars |
| `build_payload_truncates_summary_content` | site 1: 5000-char `Summarized` *summary* → `content` bounded |
| `build_payload_truncates_embed_description` | site 2: `Summarized` body whose markdown exceeds 4096 → embed `description` is `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` chars |
| `webhook_build_payload_truncates_plain_content` | site 6, webhook equivalent |
| `webhook_build_payload_truncates_summary_content` | site 4, webhook equivalent |
| `webhook_build_payload_truncates_embed_description` | site 5, webhook equivalent |
| `content_limit_and_embed_limit_are_not_swapped` | a 3000-char `Summarized` markdown is **not** truncated (it is under 4096) while a 3000-char plain body **is** — guards against the two constants being crossed |

### Limit-model tests

| Test | Assertion |
|------|-----------|
| `truncated_payload_passes_twilight_content_validation` | `twilight_validate::message::content()` returns `Ok` for a truncated `content` — the bot path's client-side gate is what actually has to be satisfied |
| `truncated_payload_passes_twilight_embed_validation` | `twilight_validate::message::embeds()` returns `Ok` for the built embed slice |

### Receipt-metadata tests (R6)

| Test | Assertion |
|------|-----------|
| `untruncated_send_has_no_dropped_metadata` | receipt `metadata` has no `dropped` key when nothing was cut |
| `truncated_content_sets_dropped_metadata` | `metadata["dropped"] == "content_truncated"` |
| `truncated_embed_description_sets_dropped_metadata` | `metadata["dropped"] == "embed_description_truncated"` |
| `both_truncated_sets_combined_dropped_metadata` | `metadata["dropped"] == "content_truncated,embed_description_truncated"` |
| `dropped_metadata_survives_receipt_json_roundtrip` | `SendReceipt::to_pretty_json` → `from_json_str` preserves the key |
| `webhook_truncated_send_sets_dropped_metadata` | webhook adapter equivalent, through the existing wiremock harness |

Per the monorepo testing guidance, each new test must be proven non-vacuous:
neuter the truncation call (or the metadata insert), confirm the test goes red,
restore.

## Success Criteria

1. **Plain/Markdown body.** A 5000-character lifecycle message sent through a
   `discord_webhook` route is delivered, its `content` ending in `"\n\n..."`,
   with no 400 from Discord. The same message through a `discord` (bot) route is
   delivered rather than failing with `MessengerError::Transport`.
2. **`Summarized` body.** A message whose rich markdown exceeds 4096 characters
   is delivered on both adapters, with the embed `description` truncated and the
   `content` (the summary banner) untouched.
3. A successful send that truncated anything returns a `SendReceipt` carrying the
   `dropped` key with the value specified in R6, and that key survives the
   receipt JSON round-trip.
4. `DISCORD_MAX_CONTENT_LENGTH` and `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` are
   the only places `2000` and `4096` appear in the Discord adapters and their
   tests.
5. Multi-byte content at any length never panics, on either field, on either
   adapter.
6. Messages under both limits are byte-identical to their pre-fix output — no
   regression in the common case, and no `dropped` key on their receipts.
7. `messenger/docs/research/platforms/discord.md` no longer presents a
   byte-slicing truncation as the reference implementation (R8).
8. `just test` and `just lint` are green in `messenger/`.
