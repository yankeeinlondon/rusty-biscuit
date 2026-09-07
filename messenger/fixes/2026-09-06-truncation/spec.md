---
status: draft
created: 2026-09-06
updated: 2026-09-06
clarified: claude/claude-opus-5[1m]
reviewed: true
reviewed_by: claude/default
reviewed_on: 2026-09-06
area: messenger
packages:
    - messenger
---

# Discord Content Truncation

> **Status:** fully decided. Every requirement below is settled, including the
> three questions the 2026-09-06 review left open: attachment `description` is
> **in scope** (R4.2), the truncation flag **does** surface in CLI stdout (R6),
> and `provider/limits.rs` compiles **unconditionally** (R7). There are no open
> questions; this is ready for a plan.

## Summary

Discord rejects any message whose `content` field exceeds 2000 characters, any
embed whose `description` exceeds 4096, and any attachment whose `description`
exceeds 1024. The messenger library never bounds any of the three. This fix
teaches both Discord adapters all three limits — expressed as
`DISCORD_MAX_CONTENT_LENGTH`, `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH`, and
`DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH` — and truncates over-length text
instead of failing the send, flagging the loss with a `truncated` key on the
returned `SendReceipt` and a new `truncated` field on the CLI's stdout JSON. The
truncation trims trailing whitespace at the cut (R2) and reserves budget so that
a message's location line always survives (R4.1) — the location line being itself
bounded by a fourth, messenger-level constant so that the reservation can never
starve the body. The reserve-truncate-append sequence lives in one shared
function in `prepared.rs` that all four location-carrying sites call, and every
one of the six body sites reads its truncation flag off the `BoundedField`
produced where the cut was made, rather than re-deriving it from lengths at the
adapter.

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
  `render_body_with_location(kind)` (plain branch), and sets the embed
  `description` from `render_rich(kind)`, with no length check on any of them.
  (`render_rich` and `render_body_with_location` both take a `ProviderKind`; the
  spec writes them bare below for brevity, and each adapter passes its own kind.)
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

Introduce three public constants in `discord.rs`:

```rust
/// Maximum number of characters Discord accepts in a message `content` field.
///
/// Discord rejects longer payloads with HTTP 400 and the body
/// `{"content": ["Must be 2000 or fewer in length."]}`; `twilight-http`
/// rejects them client-side before the request is issued.
pub const DISCORD_MAX_CONTENT_LENGTH: usize = 2000;

/// Maximum number of characters Discord accepts in an embed `description`.
pub const DISCORD_MAX_EMBED_DESCRIPTION_LENGTH: usize = 4096;

/// Maximum number of characters Discord accepts in an attachment's
/// accessibility `description` (alt text).
///
/// `twilight-validate` enforces it client-side as
/// `ATTACHMENT_DESCIPTION_LENGTH_MAX`
/// (`twilight-validate-0.17.0/src/message.rs:21`, spelled with that crate's own
/// typo); the webhook path has no validator and takes the HTTP 400.
pub const DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH: usize = 1024;
```

The content constant is deliberately *not* named `DISCORD_MAX_LENGTH`: once
there is more than one limit the bare name is ambiguous about which surface it
bounds.

All three are documented by the platform error they prevent, which is the
provenance test this section applies. The attachment constant passes it: `1024`
is `twilight-validate`'s `ATTACHMENT_DESCIPTION_LENGTH_MAX`
(`twilight-validate-0.17.0/src/message.rs:21`), read by `attachment_description`
(`:224`) via `attachment` (`:210`) — a Discord API limit in exactly the sense
the other two are. See R4.2 for what it bounds.

The magic numbers `2000`, `4096`, and `1024` must not appear anywhere else in the
Discord adapters or their tests; every reference goes through a constant.

#### The fourth limit is *not* defined here

R4.1 introduces a fourth named limit, `MAX_LOCATION_LINE_CHARS`, which bounds the
rendered location line. It deliberately does **not** live in `discord.rs`. The
three constants above are each documented by the platform error they prevent; the
location cap prevents no platform error, because Discord has no location concept
at all — the line is a messenger-level rendering invented by
`Location::format_text_line` (`message.rs:49-57`). Filing it beside three Discord
API limits would assert a provenance it does not have. It lives in `limits.rs` —
see R4.1 for the value and R7 for the placement argument.

`discord.rs` does, however, carry the compile-time assertions that tie the
location cap to the two *body* limits it must fit inside, because it is the only
module that sees both sides. (The attachment limit needs no such assertion: no
location line is ever appended to an attachment description.)

```rust
const _: () = assert!(
    DISCORD_MAX_CONTENT_LENGTH > MAX_LOCATION_LINE_CHARS + 1 + ELLIPSIS_CHARS,
    "the location reservation must always leave room for body text plus its marker",
);
const _: () = assert!(
    DISCORD_MAX_EMBED_DESCRIPTION_LENGTH > MAX_LOCATION_LINE_CHARS + 1 + ELLIPSIS_CHARS,
    "the location reservation must always leave room for body text plus its marker",
);
```

These two lines are what replace R4.1's former run-time fallback: they are the
mechanism that keeps the removal of that branch sound if any of the three
constants is ever edited. See "Why there is no run-time fallback any more" in
R4.1.

### R2 — Truncation helper

A single shared truncation core performs the cut so the bot and webhook adapters
cannot drift, and so the content, embed, and location cases cannot drift from
each other. The core is parameterized by the marker it appends; two named
wrappers pin the two markers this fix uses:

```rust
/// Truncate to at most `max_chars` characters, appending `marker` (of
/// `marker_chars` characters) when a cut is made.
fn truncate_with_marker(
    text: String,
    max_chars: usize,
    marker: &str,
    marker_chars: usize,
) -> String;

/// Body/field truncation: block marker, `"\n\n..."`.
pub(crate) fn truncate_to_chars(text: String, max_chars: usize) -> String {
    truncate_with_marker(text, max_chars, ELLIPSIS, ELLIPSIS_CHARS)
}

/// Location-line truncation: inline marker, fixed cap (R4.1).
pub(crate) fn truncate_location_line(line: String) -> String {
    truncate_with_marker(
        line,
        MAX_LOCATION_LINE_CHARS,
        LOCATION_ELLIPSIS,
        LOCATION_ELLIPSIS_CHARS,
    )
}

/// `truncate_to_chars` plus the bookkeeping R5 and R6 need: the pre-cut
/// character count and whether a cut happened. Used by the two location-free
/// sites (1 and 4); the four location-carrying sites get the same
/// [`BoundedField`] out of `PreparedMessage::bound_with_location` (R4.1).
pub(crate) fn bound_to_chars(text: String, max_chars: usize) -> BoundedField;
```

> **Reader's note (review 2026-09-06).** `bound_to_chars` did not exist in the
> reviewed draft: sites 1 and 4 were told to "set `content` from
> `truncate_to_chars`", which returns a bare `String` and so leaves the adapter
> to re-derive the truncation flag by comparing character counts. That is exactly
> the re-derivation R4.1 rejects one section later ("the signal originates at the
> same place the cut is made rather than being re-derived by comparing lengths at
> the adapter"), and it would have given the two location-free sites a different
> provenance for the same flag than the four location-carrying ones. Adding a
> thin wrapper that returns [`BoundedField`] costs three lines and makes **all
> six** sites produce the flag at the cut. `truncate_to_chars` stays as the pure
> `String -> String` primitive that `bound_to_chars` and `bound_with_location`
> both call — R2's "no knowledge of messages, locations, or providers" property
> is unchanged.

Behavior of the core:

1. If the text is at most `max_chars` characters, return it **unchanged** — no
   allocation churn, no marker appended, and **no trim** (see "Why the trim is
   conditional" below).
2. Otherwise, keep the first `max_chars - marker_chars` characters, apply
   `trim_end()` to that kept slice, and append the marker.

`ELLIPSIS` is `"\n\n..."` and `ELLIPSIS_CHARS` is `5`; `LOCATION_ELLIPSIS` is
`"…"` (U+2026) and `LOCATION_ELLIPSIS_CHARS` is `1`. All four constants, plus
`MAX_LOCATION_LINE_CHARS`, live in `limits.rs` beside the helper (R7). Why the
location gets its own marker is argued in R4.1.

`truncate_location_line` takes no `max_chars`: the location cap is a fixed
presentation choice, not a per-provider budget, so making it an argument would
only create a way for a call site to pass the wrong number. R4.2 adds a third
wrapper, `truncate_inline(text, max_chars)`, which is the same inline-marker cut
with the budget as a parameter — attachment descriptions are bounded by a
*platform* limit, so that one does take an argument — and reduces
`truncate_location_line` to `truncate_inline(line, MAX_LOCATION_LINE_CHARS)`.
`bound_inline_to_chars` is its `BoundedField`-returning counterpart. Neither
changes the core.

**Postcondition:** a truncated result is **at most** `max_chars` characters —
not exactly `max_chars`. The trim in step 2 is what weakens this from an
equality to an inequality.

**Totality.** The postcondition holds for *every* `max_chars`, including values
below the marker's own length. When `max_chars < marker_chars` the marker cannot
fit, so the core returns the first `max_chars` characters with **no** marker
rather than emitting a 5-character marker against a 3-character budget. This
clause is load-bearing now that R4.1 has no run-time guard shielding the helper
from small budgets: a primitive whose postcondition depends on a caller-side
check is exactly the drift this fix exists to stop. Through the Discord adapters
the clause is unreachable (R4.1's cap guarantees a body budget of at least 1743),
but it is the helper's contract, not the caller's.

#### D2 — trailing-whitespace trim: decided **yes**

If the cut lands inside a run of newlines or spaces, appending the marker
directly would stack blank lines. Because the marker itself leads with `"\n\n"`,
trimming first and then appending always yields **exactly one blank line** before
the dots, whatever was at the cut.

Worked example with small numbers — `max_chars = 12`, so the keep window is
`12 - 5 = 7` characters. Input `"alpha\n\nbravo!"` (13 chars, over the limit);
the first 7 characters are `"alpha\n\n"`:

| | Result | Length | Renders as |
|---|---|---|---|
| Without trim | `"alpha\n\n" + "\n\n..."` = `"alpha\n\n\n\n..."` | 12 | `alpha`, three blank lines, `...` |
| With trim | `"alpha" + "\n\n..."` = `"alpha\n\n..."` | 10 | `alpha`, one blank line, `...` |

Whitespace at the cut is the common case, not a corner case. The Discord
renderer joins top-level paragraphs with a literal `"\n\n"`
(`markdown/discord.rs:70-75`), list items push a trailing `'\n'`
(`markdown/discord.rs:67`), and headings push a trailing `'\n'`
(`markdown/discord.rs:82`). An arbitrary 2000-character cut through a real
lifecycle message lands on one of those boundaries often.

**The exact-`max_chars` property was verified to have no consumer.** There is no
truncation anywhere in the crate today — zero hits for `truncate`,
`chars().take`, or `char_indices` across `messenger/lib/src` and
`messenger/cli/src` — so nothing can be relying on an exact length. The only real
gate is `twilight_validate::message::content()`
(`twilight-validate-0.17.0/src/message.rs:324-326`), which is
`value.as_ref().chars().count() <= MESSAGE_CONTENT_LENGTH_MAX` — a `<=`, not an
`==`. Coming in under the limit is as valid as landing on it.

#### Why the trim is conditional (step 1 must not trim)

Trimming an under-limit input would be wrong three times over:

- **It would violate Success Criterion 7**, which requires under-limit output to
  be byte-identical to its pre-fix value.
- **It would be a no-op for `Markdown` and `Summarized` bodies anyway.** All four
  Markdown renderers already `trim_end()` their own output before returning it:
  `markdown/discord.rs:7`, `markdown/plain_text.rs:7`,
  `markdown/slack_mrkdwn.rs:7`, `markdown/telegram_html.rs:7`.
- **The one case where it would *not* be a no-op is the case where it does
  damage.** `MessageBody::Plain` is returned verbatim at `prepared.rs:72`
  (`text.clone()`), never through a renderer. Trimming there would silently edit
  caller-authored text that the caller chose to end with whitespace.

**Edge case:** if the entire keep window is whitespace, the trim yields an empty
slice and the result is the bare marker `"\n\n..."` (5 characters). That is still
non-empty, so the `content.is_empty()` → `None` check in R4 is unaffected. This
requires a body whose first `max_chars - 5` characters are all whitespace and
which is nonetheless over-length; it is documented, not defended against.

The helper is provider-neutral: the limit is a parameter, not baked in, and both
wrappers stay pure `String -> String` functions with no knowledge of messages,
locations, or providers. The reserve-truncate-append *sequence* that combines
them is not here — it lives in `prepared.rs` (R4.1), which is where a
`PreparedMessage` and its `Location` are both in scope. See R7 for where the
helper lives.

**Note for R4.1:** step 1's "return unchanged" applies to the location wrapper
too, so a location line at or under `MAX_LOCATION_LINE_CHARS` — every one a
supported entry point can build — is byte-identical to today's
`format_text_line()` output. The location cap is invisible in the common case.

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

There are **six** real application sites for the two *body* limits, not four.
(Attachment `description` is a seventh and eighth site under a third limit; it
has its own requirement, R4.2, because its plumbing and its marker differ.) The
earlier draft's table was
wrong about which string overflows: in the embed branch, `content` comes from
`render_summary()`, which for a `Summarized` body returns `summary.clone()`
verbatim (`prepared.rs:101`) — the short hand-written banner, almost never the
overflowing field. The long text goes to the embed **description**.

| # | Site | Field | Body shape | Source of text | Limit |
|---|------|-------|------------|----------------|-------|
| 1 | `discord.rs:72-76` | `content` | `Summarized` | `render_summary()` | `DISCORD_MAX_CONTENT_LENGTH` |
| 2 | `discord.rs:71` | embed `description` | `Summarized` | `render_rich()` → becomes `bound_with_location(rich_md, …)`, see R4.1 | `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` |
| 3 | `discord.rs:83-88` | `content` | `Plain` / `Markdown` | `render_body_with_location()` → becomes `render_body_with_location_bounded()`, see R4.1 | `DISCORD_MAX_CONTENT_LENGTH` |
| 4 | `discord_webhook.rs:261-265` | `content` | `Summarized` | `render_summary()` | `DISCORD_MAX_CONTENT_LENGTH` |
| 5 | `discord_webhook.rs:268-271` | embed `description` | `Summarized` | `render_rich()` → becomes `bound_with_location(rich_md, …)`, see R4.1 | `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` |
| 6 | `discord_webhook.rs:274-279` | `content` | `Plain` / `Markdown` | `render_body_with_location()` → becomes `render_body_with_location_bounded()`, see R4.1 | `DISCORD_MAX_CONTENT_LENGTH` |

Sites 1 and 4 are the low-traffic cases and are covered for symmetry, not
because they are the reported failure: a `Summarized` summary is caller-authored
and short by construction. Sites 2 and 5 are where the reported class of failure
actually lands for `Summarized` bodies; sites 3 and 6 are where the reported
failure landed for the plain/markdown lifecycle message.

The existing empty-string check (`content.is_empty()` → `None`) is applied to the
truncated value. At the limits actually in play (2000 and 4096) truncation cannot
produce an empty string from a non-empty input — that requires a `max_chars` of 0,
which R2's totality clause covers but no Discord site can reach — so ordering
between the two is not load-bearing, but keep truncation first for clarity.

**Embed-shape invariant.** The embed carries **only** a description today — no
title, footer, author, or fields, in either adapter (`discord.rs:71` builds it
with `EmbedBuilder::new().description(...)` and nothing else;
`discord_webhook.rs:181-184` defines `EmbedBody` as a single `description`
field). Capping the description at 4096 therefore bounds the *whole* embed, which
makes `EMBED_TOTAL_LENGTH` (6000) and `TITLE_LENGTH` (256) unreachable at the one
place either is ever checked — `req.embeds(...)` (`discord.rs:208`) on the bot
path, and nowhere at all on the webhook path — and keeps this fix to a single
embed constant. **If the embed ever gains a title, footer, or fields, this
invariant breaks and the total-length budget must be revisited.**
R4.1 preserves this invariant deliberately — see its rejected footer alternative.

### R4.1 — The location line reserves its own budget

**Decided.** An earlier draft accepted, as a "known consequence", that the
location line is appended *last* and is therefore the *first* thing lost on an
over-length body. That is resolved, not accepted: **the location always survives
the body's truncation.** It is itself bounded — a location line longer than
`MAX_LOCATION_LINE_CHARS` is cut with its own inline marker — but it is never
dropped, never displaced by body text, and never allowed to starve the body.

At each of the four location-carrying sites, the order inverts. Instead of
`truncate(body + '\n' + location)`, one shared function does:

```text
capped       = truncate_location_line(location_line)   // <= MAX_LOCATION_LINE_CHARS
reserved     = capped.chars().count() + 1              // +1 for the '\n' separator
body_budget  = max_chars.saturating_sub(reserved)
result       = truncate_to_chars(body, body_budget) + '\n' + capped
```

Arithmetic is saturating throughout. The `+ 1` is the separator that
`render_body_with_location` already inserts (`prepared.rs:60-62`) and that the
embed branches already insert inline (`discord.rs:66-68`,
`discord_webhook.rs:256-258`); as today, it is only inserted when the body is
non-empty.

**Length proof.** `truncate_to_chars` returns at most `body_budget` (R2), so the
assembled value is at most

```text
body_budget + 1 + capped_chars
  = (max_chars - capped_chars - 1) + 1 + capped_chars
  = max_chars
```

exactly, for every input, with no run-time guard. Two degenerate inputs check
out: with **no location**, `reserved` is 0 and the result is
`truncate_to_chars(body, max_chars)` — bit-for-bit the pre-R4.1 behavior; with an
**empty body**, the separator is not inserted (as today) and the result is the
capped location line alone, so the one-character over-reservation is harmless.
`saturating_sub` never actually saturates here, because R1's compile-time
assertions guarantee `capped_chars + 1 < max_chars`.

#### The four sites, and where the logic lives

| Site | Field | Today | After |
|------|-------|-------|-------|
| 2 (`discord.rs:65-70`) | embed `description` | inline append into `rich_md` | `message.bound_with_location(rich_md, DISCORD_MAX_EMBED_DESCRIPTION_LENGTH)` |
| 5 (`discord_webhook.rs:255-260`) | embed `description` | inline append into `rich_md` | `message.bound_with_location(rich_md, DISCORD_MAX_EMBED_DESCRIPTION_LENGTH)` |
| 3 (`discord.rs:83`) | `content` | `render_body_with_location(Discord)` | `message.render_body_with_location_bounded(Discord, DISCORD_MAX_CONTENT_LENGTH)` |
| 6 (`discord_webhook.rs:274`) | `content` | `render_body_with_location(DiscordWebhook)` | `message.render_body_with_location_bounded(DiscordWebhook, DISCORD_MAX_CONTENT_LENGTH)` |

Sites 1 and 4 (the `Summarized` summary → `content`) carry no location and are
unaffected; they call `bound_to_chars` directly (R2).

#### The reservation sequence lives in `prepared.rs`, not at the call sites

**Decided.** An earlier draft of this section put the reservation arithmetic at
each Discord call site. That is four copies of a five-step sequence in two files,
across two structurally different branches — precisely the drift this fix exists
to prevent. The sequence moves into `messenger/lib/src/prepared.rs`, beside
`render_body_with_location`, as **two** functions:

`BoundedField` itself lives in `limits.rs` (R7), not in `prepared.rs`, because
`bound_to_chars` returns it too and neither adapter should have to import a
`prepared.rs` type to read a flag produced in `limits.rs`:

```rust
/// A field value bounded to a character limit, plus what that cost.
pub(crate) struct BoundedField {
    /// The final value. From `bound_with_location`: body (possibly cut) +
    /// separator + capped location line. From `bound_to_chars`: the text alone.
    pub(crate) text: String,
    /// Character count of the value the caller asked to fit, before any cutting
    /// — for `bound_with_location`, body + separator + the **uncapped** location
    /// line, so the count reflects both possible losses. R5 logs this as the
    /// "original" count.
    pub(crate) original_chars: usize,
    /// Whether producing `text` required cutting the body, the location line,
    /// or both. Feeds R6's `truncated` receipt key.
    pub(crate) truncated: bool,
}

impl PreparedMessage {
    /// Append this message's location line to an already-rendered field value,
    /// bounding the whole to `max_chars`.
    ///
    /// Caps the location line first (`MAX_LOCATION_LINE_CHARS`), reserves its
    /// budget, truncates `text` to what remains, then appends. The result is at
    /// most `max_chars` characters. With no location, this is exactly
    /// `truncate_to_chars(text, max_chars)`.
    pub(crate) fn bound_with_location(&self, text: String, max_chars: usize) -> BoundedField;

    /// Render the body for `provider` and append the location line, bounded to
    /// `max_chars`. The bounded counterpart of [`render_body_with_location`].
    pub(crate) fn render_body_with_location_bounded(
        &self,
        provider: ProviderKind,
        max_chars: usize,
    ) -> BoundedField {
        self.bound_with_location(self.render_body_for_provider(provider), max_chars)
    }
}
```

**On the naming.** The working name was `render_body_reserving(provider,
max_chars)`. Rejected on two counts. First, this file names functions by *what
they produce* — `render_body_for_provider`, `render_body_with_location`,
`render_summary`, `render_rich` — not by the mechanism used to produce it;
"reserving" names the mechanism and does not say what is reserved or that a bound
is being applied. `render_body_with_location_bounded` reads as "the same thing as
`render_body_with_location`, bounded", which is exactly the relationship, and
sorts beside its sibling. Second, a single `render_body_*` function cannot serve
the embed sites at all — see below — so the pair needs the split anyway, and
`bound_with_location` is the half that does the actual work.

**Why the embed sites (2 and 5) use `bound_with_location` rather than a
parameter.** The embed branch already holds its rendered text: `rich_md` comes
from `render_rich()`, not from `render_body_for_provider()`. Two designs were
considered:

- *A "which body rendering" parameter on one function.* Rejected. It would be a
  two-valued enum whose only job is to pick between two `PreparedMessage`
  accessors the caller could just as easily call itself, and it would push a
  branch into the shared function for the benefit of exactly one of its two
  callers.
- *Have the embed sites call `render_body_with_location_bounded` too.* Rejected,
  though it is **verified to be behaviorally identical**: on the embed branch the
  body is necessarily `Summarized` (`render_rich` returns `Some` only for
  `Summarized`, `prepared.rs:114-125`), and for a `Summarized` body on a Discord
  provider `render_body_for_provider` and `render_rich` both evaluate to
  `render_nodes_for_provider(nodes, provider)` (`prepared.rs:79-88` vs
  `:117-119`) — the same string. It is rejected anyway because it re-renders the
  AST a second time for no gain, and because it would make the embed's content
  depend on an incidental equality between two accessors rather than on the value
  the branch already computed.

So: **the embed sites do not keep their inline assembly.** They stop appending
the location themselves and hand `rich_md` to `bound_with_location`. All four
location-carrying sites execute the identical cap-reserve-truncate-append
sequence, in one place, and the only difference between them is which string goes
in and which constant bounds it. There is no second copy of the sequence anywhere
in the fix.

A consequence worth recording: the `location_line` local at `discord.rs:61` and
`discord_webhook.rs:251` becomes **dead and is deleted**. Both branches now get
the location from `PreparedMessage` inside `bound_with_location`, so neither
adapter formats a location line any more, and the previous draft's note about
that local's move semantics is moot.

**`render_body_with_location` is not modified.** It has five callers, not two:
`discord.rs:83`, `discord_webhook.rs:274`, `signal.rs:108`, `slack.rs:97`, and
`slack_webhook.rs:232` — all verified on HEAD. Sites 3 and 6 stop calling it;
Signal, Slack, and Slack-Webhook keep calling it unchanged. Those three have no
limit implemented (their limits are explicitly [out of scope](#out-of-scope)), so
adding a `max_chars` parameter to the existing function would hand three
untouched providers an argument they have no value for. The new function is
additive; the old one keeps its signature, its body, and its four tests
(`messenger/lib/src/tests/builders.rs:123`, `:132`, `:140`, `:148` — an
**in-crate** module reached from `lib.rs:52`, which is why `pub(crate)` items are
testable at all).

**But its doc comment does change — this is drift, and the fix must fix it.**
`prepared.rs:56` currently reads *"Use this for providers without native location
APIs (Discord, Slack, Signal)."* After R4.1, Discord is precisely the provider
that must **not** use it, because doing so re-introduces the unbounded append
this fix removes. Under the repo's authoring-discipline rule ("any edit that
changes a symbol's behavior must include a pass over its docs" — here the
symbol's *contract of use* changes even though its body does not), the doc must
be corrected to name Slack, Slack-Webhook, and Signal and to point Discord — and
any future length-bounded provider — at `render_body_with_location_bounded`. This
is a comment-only edit inside a behavior-changing commit, which the repo's scope
rule permits: the prohibition runs the other way (no behavior changes in
comment-only commits).

**Who calls the new function later.** Only the Discord sites call it today. When
Telegram's 4096-character limit and Slack's 3000-per-block limit get their own
fixes, `telegram.rs:235-243` (via `render_body_for_provider`), `slack.rs:97`, and
`slack_webhook.rs:232` adopt `render_body_with_location_bounded` and inherit the
location reservation for free. That is the concrete content of the "establishes
the pattern and the shared primitives" claim in [Out of Scope](#out-of-scope) —
what those fixes inherit is a shared *sequence*, not merely a shared string
helper.

**Where the truncation outcome is produced.** `bound_with_location` returns it on
`BoundedField`, so the signal originates at the same place the cut is made rather
than being re-derived by comparing lengths at the adapter. Each adapter's
`build_payload` folds the per-field flags into a single value it returns
alongside the payload:

```rust
// provider/limits.rs — provider-neutral, shared by both adapters
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TruncationOutcome {
    pub(crate) content: bool,
    pub(crate) embed_description: bool,
    pub(crate) attachment_description: bool, // R4.2
}

/// R6's receipt value: `None`, a single token, or the set tokens joined by `,`
/// in the fixed order `content,embed_description,attachment_description`.
pub(crate) fn truncation_metadata_value(outcome: TruncationOutcome) -> Option<String>;
```

`DiscordPayload` (`discord.rs:45-49`) and the webhook's extracted equivalent each
gain a `truncated: TruncationOutcome` field. All six **body** sites read
`BoundedField.truncated`: sites 1/4 set `content` from `bound_to_chars`, sites
3/6 set `content` from `render_body_with_location_bounded`, and sites 2/5 set
`embed_description` from `bound_with_location`. The third field is set from
outside `build_payload` — the attachment builders return their own flag and
`send_prepared` merges it in, for the reason R4.2 gives. This keeps the Testing
prerequisite ("the truncation outcome must be observable from `build_payload`'s
return value") satisfied for the two body surfaces, and
`truncation_metadata_value` is the "small pure function that tests can call
directly" that section asks for.

`BoundedField.truncated` is a single boolean, not a body/location pair: R6's
vocabulary names message *surfaces* (`content`, `embed_description`), not their
sub-parts, so a cut location is reported as a cut of the field carrying it. R5's
`warn!` reports the original and final character counts, which is adequate
diagnosis for the only case that can cut a location — a caller constructing a
`Location` by struct literal with more than `MAX_LOCATION_LINE_CHARS` characters
of `name`/`address`, which is the only route to it (R4.1, "Why the cap is worth
having"). Splitting the flag later is a non-breaking addition to a `pub(crate)`
struct.

#### Bounding the location line: `MAX_LOCATION_LINE_CHARS`

**Decided.** An earlier draft guarded only against the *degenerate* case, with a
run-time fallback when `max_chars - reserved` dropped to `ELLIPSIS_CHARS` or
below. That was correctness-only and still permitted near-degenerate output: at
`max_chars = 2000` with a 1990-character location, `body_budget` is 9, which
passes `> 5`, yielding **four characters of body** beside a 1990-character
location line. Technically not location-only; useless in every other sense.

`Location.name` and `Location.address` are unbounded `Option<String>`
(`message.rs:37-38`), and `validate.rs` never length-checks either one — its only
location rule drops the whole location when the provider lacks the capability
(`validate.rs:199-216`). Rather than tolerate whatever a caller supplies and then
catch the worst of it, the rendered line is **capped before the reservation is
computed**, so the reservation is bounded by construction:

```rust
/// Maximum characters of rendered location line that may claim message budget.
pub(crate) const MAX_LOCATION_LINE_CHARS: usize = 256;
```

**The value, and why it is ours.** No Discord or twilight constant governs this
number, because Discord has no location concept — `Location::format_text_line`
(`message.rs:49-57`) is a messenger-level text fallback for providers that lack a
native location API. So the number has to be justified by what a location line
*is*:

- **Fixed chrome is 22–24 characters**: `"📍 "` (2), `" — "` (3), the formatted
  coordinates at `{:.4}` (18–21), plus 3 for `" ("` and `")"` when both name and
  address are present.
- **Two caller-controlled fields**, both meant to render on one line: a venue
  *name* and a one-line *address*. A generous single-line allowance is ~80
  characters for a name and ~140 for a full international postal address on one
  line. 80 + 140 + 24 = **244**, rounded up to **256**.
- **Sanity-checked against the code**: 256 is 3.8× the 67-character fullest
  documented form (`📍 Griffith Observatory (2800 E Observatory Rd) — 34.0500,
  -118.2400`, `message.rs:46`) and 12× the 20–21-character coordinates-only form
  (`message.rs:48`). Every location shape the crate documents passes through
  untouched.

The coincidence with twilight's `TITLE_LENGTH` (256) is exactly that — a
coincidence, and not the justification. This constant is a messenger presentation
choice and would keep the same value if Discord did not exist.

#### Why the cap is worth having, given that nothing reachable needs it

**Decided.** An earlier draft defended the cap as a live budget tradeoff — "at
`max_chars = 2000` the cap can claim at most 12.85% of the message, leaving at
least 1743 characters of body". That framing is wrong, because it presents as a
tradeoff something that no supported entry point can trigger. The defense is
different, and stronger.

**No supported entry point can produce a location line over ~21 characters.**
Verified on HEAD:

- `Message::location(lat, lon)` (`message.rs:109-122`) hardcodes `name: None,
  address: None` (`:117-118`).
- `Message::with_location(lat, lon)` (`message.rs:134-142`) hardcodes the same
  (`:138-139`).
- The CLI's `--location` accepts `"LAT,LON"` and nothing else: `parse_location`
  (`messenger/cli/src/main.rs:1224-1240`) splits on one comma and rejects
  anything that does not parse as two `f64`s.
- Claudine never sets a location at all.

So every location line reachable through the crate's own API is the
coordinates-only arm, `📍 {lat:.4}, {lon:.4}` (`message.rs:55`) — 20–21
characters against a 2000-character budget. The near-degenerate case the earlier
draft argued about is not reachable, and the "cost" of the cap in normal
operation is exactly zero characters.

**The cap is nonetheless correct, because messenger is a library, not an
application.** `Location.name` and `Location.address` are `pub` fields
(`message.rs:37-38`) on a `pub struct Location` (`:34`) that is **not**
`#[non_exhaustive]`, and `Message.location` is likewise a `pub` field. Any
external consumer can write

```rust
let mut msg = Message::markdown(body);
msg.location = Some(Location { latitude, longitude, name: Some(huge), address: None });
```

and construct the pathological case by struct literal, without the crate's
cooperation and without touching a builder. A bound that holds only because the
crate's own constructors happen not to exercise the other three match arms is not
a bound; it is a habit. The ~40 lines buy a guarantee that survives a hostile
caller.

Its real payoff, though, is a **deletion**: the cap is what makes the run-time
fallback branch unreachable, and therefore removable. Trading a run-time branch
no test could reach through the public API for a compile-time-asserted constant
is the whole return on this section — see "Why there is no run-time fallback any
more" below, whose proof is unaffected by any of the above.

**Placement: `limits.rs`, not `discord.rs`** (R1, R7). Three reasons, in
increasing order of force:

1. R1's constants are each documented by the platform error they prevent. This
   one prevents no platform error; filing it beside them would assert a
   provenance it does not have.
2. It is genuinely provider-neutral. The same text-fallback location line is
   emitted for Slack, Slack-Webhook, and Signal today; when those get their own
   limit fixes they need the same cap, and R7 already exists to stop that from
   being re-invented per provider.
3. Structurally decisive: the *only* consumer is `prepared.rs`, which today
   depends on no `provider::*` module. Putting the cap in `discord.rs` would make
   the shared, provider-neutral rendering module import a Discord adapter — an
   inverted layering, for a constant that has nothing to do with Discord.

Moving `limits.rs` out of `provider/` and up to `src/limits.rs` (now that its
consumer sits beside `prepared.rs`) was considered and rejected as churn:
`prepared.rs` is already provider-aware — it imports `ProviderKind` and branches
on it throughout — so `use crate::provider::limits::…` reads correctly.

**What a truncated location renders as.** A location cut with the body's
`ELLIPSIS` marker would be actively bad: `"\n\n..."` is a *block* marker, so it
would turn a one-line `📍` field into three lines with a blank in the middle, and
a field that already carries the body's `\n\n...` would then show two ellipsis
markers with no way to tell which loss each one reported. The location therefore
gets its own **inline** marker, `LOCATION_ELLIPSIS = "…"` (U+2026, one code
point, R2).

Silently cutting with no marker was rejected: a truncated address is not a
shorter address, it is a *wrong* one, and `📍 2800 E Observ` reads as a real
place. The marker is what stops that.

Literal rendering. Take `name = "A" * 400`, `address = None`, coordinates
`34.0500, -118.2400`. `format_text_line()` produces
`"📍 " + 400×"A" + " — 34.0500, -118.2400"` = 423 characters, over the 256 cap.
`truncate_location_line` keeps the first `256 - 1 = 255` characters (all `A`s
after the two-character prefix), finds no trailing whitespace to trim, and
appends the marker. The literal result, written with a repeat count rather than
spelled out:

```text
📍 <253 × 'A'>…
```

— 256 characters, one line, ending in `…`. In full field context, with a body
that was also cut:

```text
…last surviving line of the body

...
📍 AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA…
```

(the `A` run shown short for readability; on the wire it is 253 of them).

Two distinguishable signals: a block `...` alone on a line means the body was
cut; a trailing `…` at the end of the `📍` line means the location was cut. The
location line can never be emptied by the trim, because `format_text_line`
always begins with the non-whitespace `📍`.

#### Why there is no run-time fallback any more

The former rule — honor the reservation only if
`max_chars.saturating_sub(reserved) > ELLIPSIS_CHARS` — is **removed as
unreachable**, not kept as belt-and-braces. Proof: after the cap,
`reserved = capped_chars + 1 <= MAX_LOCATION_LINE_CHARS + 1 = 257`, so

- at `DISCORD_MAX_CONTENT_LENGTH` (2000): `body_budget >= 1743 > 5`
- at `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` (4096): `body_budget >= 3839 > 5`

The condition is true for every possible input on both limits — no `Location`
can be constructed that makes it false, at any of the four sites, on either
adapter. Keeping it would leave a branch that no test can reach through the
public path, and an unreachable branch that tests must contort to exercise is
worse than no branch: it invites a future reader to believe it is doing work.

What replaces it is R1's two `const _: () = assert!(…)` lines, which fail the
**build** rather than a run-time branch if anyone edits the constants such that
`MAX_LOCATION_LINE_CHARS + 1 + ELLIPSIS_CHARS` reaches either limit. That is a
stronger guarantee than the fallback ever gave, and it costs no code path.

Also rejected, as it was before: a policy constant such as "the location may
claim at most half the budget". The cap supersedes it — bounding the line
directly is the same protection expressed in the units of the thing being
bounded, and it does not require an opinion about ratios.

**Implementation coupling:** `ELLIPSIS_CHARS` remains `pub(crate)` (R7) because
R1's compile-time assertions in `discord.rs` read it. It is no longer read by any
run-time guard.

#### What it looks like

The one visible consequence of the reservation, on every location-carrying
message that is also over-length: the `"\n\n..."` marker now sits **above** the
location line
rather than at the very end of the field. The resulting wire value is

```text
<body truncated to body_budget chars>\n\n...\n📍 34.0500, -118.2400
```

which renders as:

```text
…last surviving line of the body

...
📍 34.0500, -118.2400
```

#### Rejected alternative for the embed branch: a footer

Moving the location into an embed footer instead of the description was
considered and rejected:

- It covers only the two embed sites (2 and 5), leaving sites 3 and 6 needing the
  reservation anyway — so it is an *addition* to this design, not a replacement.
- It changes the rendered look of **every** `Summarized`-with-location message,
  truncated or not, which is a visual change well outside a truncation fix.
- It reintroduces the multi-field embed budget that R4's embed-shape invariant
  exists to avoid. `twilight-validate` checks `FOOTER_TEXT_LENGTH = 2048`
  (`embed.rs:31`) per-field but `EMBED_TOTAL_LENGTH = 6000` (`embed.rs:19`) as a
  running sum (`embed.rs:243`, `message.rs:358`), so a 4096-character description
  plus a footer is legal only while the footer is ≤ 1904 characters. **This
  argument is weaker than it was in the earlier draft** and is recorded honestly
  as such: `MAX_LOCATION_LINE_CHARS = 256` would keep any footer well inside 1904,
  so the total-length trap could no longer fire. What survives is the structural
  objection — a footer makes the embed a two-field object whose budget is a
  running sum, which is a second length model to maintain for no benefit the
  description-only design does not already give. Bullets one and two are the
  load-bearing reasons; this one is now supporting, not decisive.

### R4.2 — Attachment `description` is bounded too

**Decided.** The 2026-09-06 review raised this as its one open question and
recommended full coverage; that is the ruling, and the question is closed.
`Attachment.alt_text` and `Attachment.caption` are unbounded caller-supplied
`Option<String>` fields (`attachment.rs:10-11`), and both Discord adapters feed
one of them into a Discord field with a hard 1024-character limit. That is the
same defect class this fix exists to close, in the same two files, one module
away from the fix's own primitive. It is in scope.

#### What the field actually is

Stated as fact, verified on HEAD, because the earlier draft's phrasing left it
ambiguous:

- **The mapping is a fallback chain, not two fields.**
  `attachment.alt_text.clone().or_else(|| attachment.caption.clone())` —
  `discord.rs:108-114` on the bot path, `discord_webhook.rs:310-313` on the
  webhook path. `alt_text` wins whenever it is `Some`; `caption` is consulted
  **only** in its absence. Exactly one string is produced, so exactly one bound
  is needed.
- **It lands in Discord's attachment accessibility description.** On the bot path
  `DiscordAttachment::description(...)` (`discord.rs:113`); on the webhook path
  `AttachmentMeta.description` (`discord_webhook.rs:186-193`), serialized into
  the `attachments[]` array inside the multipart `payload_json` part
  (`discord_webhook.rs:321-328`).
- **Neither adapter appends `caption` to `content` or to the embed.** There is no
  second surface where this text is rendered. Whether that alt-text-only mapping
  is the *right* semantics is a separate question, deliberately not settled here
  — see [Out of Scope](#out-of-scope).

#### How it fails today

- **Bot:** `CreateMessage::attachments()` runs twilight's `attachment` validator,
  which calls `attachment_description`
  (`twilight-validate-0.17.0/src/message.rs:210`, `:224`) —
  `chars().count() <= ATTACHMENT_DESCIPTION_LENGTH_MAX` (= 1024, `:21`). Over
  the limit, the `Err` is stored on the request and surfaces at `.await` as
  `ErrorType::Validation` → `MessengerError::Transport`. Byte-for-byte the same
  failure shape "The two adapters do *not* fail the same way" describes for
  `content`.
- **Webhook:** no validator at all. The over-length description goes onto the
  wire and produces the **400 with the whole message lost** that this fix exists
  to eliminate.

#### The requirement

1. Bound the resolved description to `DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH`
   (R1) on both adapters.
2. Cut with the **inline** `LOCATION_ELLIPSIS` marker (`"…"`, R2's
   `truncate_with_marker` unchanged), **not** the block `ELLIPSIS`. Alt text is a
   one-line field, for exactly the reasons R4.1 already gives for the location
   line: `"\n\n..."` would turn a one-line accessibility string into three lines
   with a blank in the middle, and a trailing `…` is what stops a cut alt text
   from reading as a complete one.
3. Report it on the receipt as a third token (R6).

No location is ever appended to an attachment description, so this site uses a
plain bound rather than `bound_with_location`, and needs no reservation
arithmetic and no compile-time assertion. R2 gains two more named helpers beside
`truncate_to_chars`, `truncate_location_line`, and `bound_to_chars`:

```rust
/// Attachment alt-text truncation: inline marker, caller-supplied budget.
pub(crate) fn truncate_inline(text: String, max_chars: usize) -> String {
    truncate_with_marker(text, max_chars, LOCATION_ELLIPSIS, LOCATION_ELLIPSIS_CHARS)
}

/// `truncate_inline` with the R5/R6 bookkeeping attached. R4.2's two sites.
pub(crate) fn bound_inline_to_chars(text: String, max_chars: usize) -> BoundedField;
```

`truncate_location_line` keeps its name and its no-argument signature — R2's
argument for that is unchanged — and its *body* reduces to
`truncate_inline(line, MAX_LOCATION_LINE_CHARS)`. It stops being the only
inline-marker wrapper; it does not stop existing.

#### The third token

`TruncationOutcome` (R7's `limits.rs`) gains a third field:

```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TruncationOutcome {
    pub(crate) content: bool,
    pub(crate) embed_description: bool,
    pub(crate) attachment_description: bool,
}
```

and `truncation_metadata_value` gains a third token. **The fixed order is
`content,embed_description,attachment_description`** — declaration order, table
order in R6, and emission order, all the same, so the value stays deterministic
and parseable.

R6's contract absorbs this without change: it explicitly designed the `truncated`
key's value to grow *by tokens rather than by keys*, and this is the first
exercise of that property. One attachment cut or five produce the same single
`attachment_description` token; the receipt names surfaces, not instances.

#### Plumbing — the two signatures that change

The attachment flag cannot ride on `DiscordPayload`, because attachments are
built by a separate call: `send_prepared` calls `build_payload` and
`build_attachments` independently (`discord.rs:183-184`). The flag therefore
comes out of the attachment builders and is merged into the payload's
`TruncationOutcome` in `send_prepared`, on both adapters.

**Bot** (`discord.rs:97-117` and `:119-128`). Both signatures change:

```rust
pub(crate) fn build_attachment(
    attachment: &crate::attachment::Attachment,
    id: u64,
) -> Result<(DiscordAttachment, bool), MessengerError>;

pub(crate) fn build_attachments(
    message: &PreparedMessage,
) -> Result<(Vec<DiscordAttachment>, bool), MessengerError>;
```

The `bool` is "this attachment's description was cut" / "any attachment's
description was cut". `build_attachments`' `.map(...).collect()`
(`discord.rs:122-127`) becomes a loop or a `try_fold` that ORs the per-attachment
flags; the `Result` shape is otherwise unchanged. Returning a bare `bool` rather
than a `BoundedField` is deliberate: the caller needs only the flag, and the
`original_chars` R5 wants is logged at the cut inside `build_attachment` rather
than carried out through two layers.

**Webhook** (`discord_webhook.rs:307-320`). The inline `metas` loop currently
resolves the description itself at `:310-313`. Extract the per-attachment half
into a `pub(crate)` associated function mirroring the bot's, so the two adapters
cannot drift and so the tests can reach it:

```rust
pub(crate) fn build_attachment_meta(
    attachment: &Attachment,
    index: usize,
) -> Result<(AttachmentMeta, Part, bool), MessengerError>;
```

It absorbs the existing `Self::build_part` call (`discord_webhook.rs:309`) and
the fallback chain, and returns the multipart `Part` alongside the metadata so
the loop body reduces to pushing both and OR-ing the flag.

**Visibility.** `build_attachment`, `build_attachments`, and
`build_attachment_meta` are raised to `pub(crate)` for the same reason and under
the same rule as `build_payload` — see the Testing section's visibility decision.
`AttachmentMeta` (`discord_webhook.rs:186-193`) is raised with them.

### R5 — Structured logging

The existing `tracing::debug!` in each adapter reports the post-truncation
length; where a length is logged it must be a **character** count, since the
current `str::len` / `String::len` calls at `discord.rs:192` and
`discord_webhook.rs:287` report bytes against a char-denominated limit and are
therefore misleading in exactly the multi-byte case this fix exists for.

When truncation occurs, emit a `tracing::warn!` naming the provider, the field
(`content`, `embed_description`, or `attachment_description`), and both the
original and final character counts. The original count comes from
`BoundedField.original_chars` on all six body sites — from `bound_with_location`
on the four location-carrying ones (where the adapter no longer holds the
pre-truncation string at all) and from `bound_to_chars` on sites 1 and 4 — and
from `bound_inline_to_chars` inside the attachment builders (R4.2). The adapter
never recomputes a length it did not observe being cut.

One `warn!` per cut field, not per send: a message that truncates both `content`
and the embed `description` emits two lines, distinguished by the `field` value.
This matches R6's per-surface vocabulary and keeps the log greppable by field.
Attachment descriptions are the one exception to "per surface": one `warn!` per
cut *attachment*, since the count that matters is per-string and the receipt
token already collapses them.

#### Why `warn!` and not `debug!` — a deliberate departure

**Decided.** The crate's established convention is the other way round, and the
departure has to be argued rather than drifted into. In `validate.rs`,
best-effort degradation logs at `debug!` — `tracing::debug!(feature = …,
"dropping unsupported feature")` at `:210` (location), `:229` (replies), and
`:248` (silent delivery) — and `warn!` is reserved for the strict-mode
rejections that sit immediately above each of them (`:201`, `:220`, `:239`).
Truncation is best-effort degradation, so convention would put it at `debug!`.

It goes to `warn!` anyway, for two reasons:

1. **What is discarded is different in kind.** `validate.rs`'s `debug!` cases
   drop a *feature the provider cannot render at all* — a capability mismatch the
   caller could have predicted from `CapabilitySet` before sending, and which the
   crate also reports through `CompatibilityWarning` on the `SendPlan`.
   Truncation silently discards caller **content** that was valid, that the
   provider would have rendered, and that nothing in the pre-send API could have
   warned about.
2. **The receipt channel is thin.** Per R6's "Audience reality", nothing in the
   workspace reads Discord receipt metadata today, and even after R6's CLI stdout
   field lands, the most likely place a human notices a truncated send is the
   log. `debug!` is off by default; a loss that is off by default is the silent
   partial delivery the "Root Cause" section warns about, with no compensating
   signal.

This is a departure, not an oversight, and it is recorded here so a future
reader does not "correct" it to `debug!` for consistency.

### R6 — Truncation is always applied and flagged on the receipt

**Decided (replaces the former D3).** Truncation is unconditional.
`CompatibilityMode` / the CLI's `--strict` flag is **not** consulted: a strict
caller does not get an error instead of a truncated send.

In addition to R5's logging, a **successful** send whose `content`, embed
`description`, or any attachment `description` was truncated records the fact on
`SendReceipt.metadata`.

**Decided contract** — a **distinct `truncated` key**, not `dropped`:

| Key | Value | Meaning |
|-----|-------|---------|
| `truncated` | `content` | only `content` was cut |
| `truncated` | `embed_description` | only the embed description was cut |
| `truncated` | `attachment_description` | only an attachment description was cut (R4.2) |
| `truncated` | `content,embed_description` | both of those were cut |
| `truncated` | `content,attachment_description` | … and so on for every non-empty subset |
| `truncated` | `content,embed_description,attachment_description` | all three were cut |

The key is **absent** when nothing was truncated. A multi-token value is the
tokens joined by `,` in the **fixed order
`content,embed_description,attachment_description`** — never sorted at emission,
never in the order the cuts happened, so the value stays deterministic and
parseable. Field names are **provider-neutral** — `content`, not
`content_truncated`; the key already says "truncated", and the value's job is to
name the field.

The value is produced by `truncation_metadata_value(TruncationOutcome)` in
`limits.rs`, from flags that originate at the cut itself — `BoundedField.truncated`
on all six body sites, via `bound_with_location` on the four location-carrying ones and
`bound_to_chars` on sites 1 and 4, plus the attachment flag merged in
`send_prepared` (R4.2). See "Where the truncation outcome is produced" in R4.1 for the
plumbing, and the Testing prerequisite for why it has to be a pure function.

#### Why a comma-joined value is the right shape here

It is the **established idiom in this exact map**, not a novel one.
`helper_fallbacks` is a comma-joined list: `summarize()`
(`provider/desktop/linux.rs:314-320`, and its twins in `macos.rs` and
`windows.rs`) does `.map(|attempt| attempt.summary()).collect::<Vec<_>>().join(",")`
over `format!("{}:{}", name, tag)` elements built at
`provider/desktop/helpers/election.rs:28-30`. The delimiter is safe for the same
reason it is safe there: every token is a crate-chosen identifier drawn from a
closed set that can never contain a comma. `content`, `embed_description`, and
`attachment_description` are `&'static str` literals in this crate, not user
data.

#### Why the field names are provider-neutral

`content`, `embed_description`, and `attachment_description` name *message
surfaces*, not Discord internals. When Telegram (4096) and Slack
(3000-per-block) get their own limit fixes, they reuse `truncated=content`
rather than inventing `telegram_text_truncated`. One key, one vocabulary,
growing by tokens instead of by keys — R4.2 is the first exercise of that
property, and it required no change to this contract.

#### Why it is non-foreclosing

Adding a quantified companion later — say `truncated_content_chars=2000` — is
purely additive: the `truncated` key keeps its exact meaning and old readers keep
working. The rejected shape (a per-field key with value `"true"`, e.g.
`content_truncated=true`) does not have that property: adding counts would force
`"true"` to become a number, changing the meaning of an existing key.

#### Why `dropped` is left alone

Two concrete reasons, both verified:

1. **`dropped`'s own documentation is already drifted.** `receipt.rs:6` describes
   the map as carrying "dropped-attachment hints", but only one of its three
   values is attachment-related. The three writers are
   `snoretoast.rs:189` (`image_too_large` — attachment),
   `notify_send.rs:135` (`actions_libnotify_old` — an interaction feature), and
   `terminal_notifier.rs:105` (`sound_low_urgency` — a delivery option). Piling a
   fourth, differently-shaped vocabulary onto a key whose doc already
   misdescribes it makes the drift worse.
2. **All three writers do a bare `insert` with no merge logic.** Each is a plain
   `metadata.insert("dropped".to_string(), <literal>.to_string())`. Today that is
   harmless because no two of them can fire on one send. Adding a fourth
   vocabulary would make silent last-writer-wins clobbering a live correctness
   question the first time two lossy events coincide on a single send — a
   question this fix would be creating, not answering. A separate key has no such
   interaction.

There is no collision risk for the new key either: both Discord adapters
currently construct `metadata: BTreeMap::new()` (`discord.rs:238`,
`discord_webhook.rs:387`), so `truncated` is the only entry they will write.

**Out-of-scope observation, recorded so it is not lost:** the `receipt.rs:6` doc
drift described in point 1 is a real, separate, unconditional documentation fix
under the repo's drift rule (*"assume the code is correct and the comment is
wrong"*). It is **not** fixed here — this spec touches neither the desktop
helpers nor that module doc — but it should be raised as its own change.

#### Audience reality — what actually reads this today

Nothing in the workspace reads Discord receipt metadata:

- **Claudine discards the receipt outright.** `claudine/lib/src/messaging/send.rs:714-717`
  is `messenger.send_planned(plan).await.map_err(...)?;` — the `?` is in statement
  position, so the `Ok(SendReceipt)` value is dropped on the floor.
- **The messenger CLI has no subcommand that shows a receipt's contents.** The
  `Commands` enum (`messenger/cli/src/main.rs:53`) is `Send`, `Replace`,
  `Dismiss`, `Setup`, `Init`, `Info`, `Install`, `Completions`; `Replace` and
  `Dismiss` *consume* a saved receipt rather than displaying it.
- **`emit_send_response` prints four fields only** — `id`, `receipt`, `helper`,
  `os` (`main.rs:466-481`, `:495-513`).

So before this fix the flag's audience would be a human reading
`~/.messenger/receipts/*.json` plus this crate's own tests. That bounds how much
the shape matters — but it is also exactly why picking a clean shape now is
cheap: there are no existing readers to migrate. It is also why the CLI stdout
gap below is closed here rather than deferred.

Rationale for choosing the receipt over a `CompatibilityWarning`. Two reasons,
both needed — the reviewed draft gave only the first, which is half the argument:

1. **Timing.** `SendPlan::warnings` (`provider/mod.rs:81`) is frozen at
   `plan_send()` (`provider/mod.rs:141`), before per-provider rendering, so it
   structurally cannot carry a post-render fact.
2. **Shape.** `CompatibilityWarning` is a **struct**, not an enum
   (`validate.rs:64-67`): two fields, `provider: ProviderKind` and
   `feature: &'static str`. Its `Display` hardcodes the sentence *"⚠️ the {} feature
   is not supported on {} and will be dropped"* (`validate.rs:69-77`). Reusing it
   would therefore need a **shape change**, not merely a new `feature` string —
   truncation is not a feature the provider fails to support, and rendering it
   through that sentence would say something false.

Receipt metadata is the crate's established channel for post-render
lossy-delivery signals. `SendReceipt` is `Serialize`/`Deserialize`
(`receipt.rs:211`), so the flag persists into the CLI receipt store for free.

#### Decided: the flag is surfaced in CLI stdout, here, not in a follow-up

**Decided (reverses the reviewed draft's "explicitly accepted gap").**
`emit_send_response` (`messenger/cli/src/main.rs:495-513`) prints only `id` /
`receipt` / `helper` / `os` from the `SendResponse` struct
(`main.rs:465-481`), so without a new field the flag would reach stdout nowhere.
That is now in scope. Two additions:

**A typed accessor on the library type**, beside `helper_used()`
(`receipt.rs:236-241`) and in the same shape:

```rust
/// Message surfaces this send had to truncate, comma-joined.
///
/// Returns `Some("content")`, `Some("embed_description")`,
/// `Some("attachment_description")`, or a comma-joined combination in that
/// fixed order. Returns `None` when nothing was truncated.
pub fn truncated_fields(&self) -> Option<&str> {
    self.metadata.get("truncated").map(String::as_str)
}
```

No `filter` clause, unlike `helper_used`: there is no sentinel value to suppress,
because R6 makes the key absent rather than empty when nothing was cut.

**A field on the CLI response struct**, mirroring `helper` exactly — same
`Option<String>`, same `skip_serializing_if`, same source pattern:

```rust
// main.rs:465-481
    /// Message surfaces truncated to fit provider limits, comma-joined.
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<String>,

// main.rs:495-513, inside emit_send_response
        truncated: receipt.truncated_fields().map(str::to_string),
```

**Why it is not deferred.** R6's "Audience reality" section is the argument
against deferring it, not for it: it *proves* the receipt channel is read by
nothing today. Shipping a behavior change that turns a loud `Err` into a silent
`Ok` (see "The two adapters do *not* fail the same way") while deferring the one
cheap remedy is not defensible. The cost is roughly ten lines across two files,
with no new mechanism, no library type change, and no change to any existing
field's meaning — `SendResponse` already carries an optional, conditionally
serialized field, so nothing about the response schema is new.

**Named known consequence — this does not fix Claudine.** Recorded plainly
because Claudine is where the reported failure originated, and a reader could
otherwise conclude the loop is closed:

- `claudine/lib/src/messaging/send.rs:714-717` is
  `messenger.send_planned(plan).await.map_err(…)?;` — the `?` sits in statement
  position, so the `Ok(SendReceipt)` is discarded. Claudine never sees the
  receipt, and therefore never sees `truncated_fields()`.
- Twelve lines earlier, at `:702-709`, Claudine *does* surface `plan.warnings`
  through a `warn!`. That channel is unavailable to truncation for the structural
  reason above: `SendPlan::warnings` is frozen at `plan_send()`
  (`provider/mod.rs:141`), so a post-render fact cannot ride it.

After this fix, Claudine's own reporting on a truncated lifecycle message says
"sent successfully" and nothing more: no `Err`, no `plan.warnings` entry, and no
stdout field, since Claudine calls the library rather than the CLI. The one
signal that does escape is R5's `tracing::warn!`, emitted from the messenger
crate into whatever subscriber the host process installed — so the loss is not
literally silent, but it arrives unattributed to a route and outside the path
Claudine's send reporting uses, which is exactly why R5 argues for `warn!` over
`debug!` rather than treating the log as a formality. That is strictly better
than today's total message loss, and still short of what an operator needs. The
remedy is a **Claudine-side** change — bind the receipt and log
`receipt.truncated_fields()` — and it is named as a follow-up in
[Out of Scope](#out-of-scope). It is not a messenger-side change and cannot be
made from this crate.

### R7 — Helper lives in a shared `provider/limits.rs`

**Decided (replaces the former D4, against that draft's own recommendation).**

Create `messenger/lib/src/provider/limits.rs` containing the provider-neutral
truncation primitives and nothing provider-specific:

```rust
/// Marker appended to a truncated field value. Leads with `"\n\n"` so that
/// trimming the cut point (R2) always yields exactly one blank line before the
/// dots.
pub(crate) const ELLIPSIS: &str = "\n\n...";

/// Character length of [`ELLIPSIS`]. Exposed because R1's compile-time
/// assertions in `discord.rs` read it.
pub(crate) const ELLIPSIS_CHARS: usize = 5;

/// Marker appended to a truncated location line. Inline, so the line stays one
/// line, and visually distinct from [`ELLIPSIS`] so the two losses are
/// distinguishable in one field (R4.1).
pub(crate) const LOCATION_ELLIPSIS: &str = "…";

/// Character length of [`LOCATION_ELLIPSIS`].
pub(crate) const LOCATION_ELLIPSIS_CHARS: usize = 1;

/// Maximum characters of rendered location line that may claim message budget.
/// A messenger presentation choice, not a platform limit — see R4.1.
pub(crate) const MAX_LOCATION_LINE_CHARS: usize = 256;

fn truncate_with_marker(
    text: String,
    max_chars: usize,
    marker: &str,
    marker_chars: usize,
) -> String;

pub(crate) fn truncate_to_chars(text: String, max_chars: usize) -> String;
pub(crate) fn truncate_inline(text: String, max_chars: usize) -> String;
pub(crate) fn truncate_location_line(line: String) -> String;

/// A field value bounded to a character limit, plus what that cost (R4.1).
pub(crate) struct BoundedField {
    pub(crate) text: String,
    pub(crate) original_chars: usize,
    pub(crate) truncated: bool,
}

/// `truncate_to_chars` with the R5/R6 bookkeeping attached. Sites 1 and 4.
pub(crate) fn bound_to_chars(text: String, max_chars: usize) -> BoundedField;

/// `truncate_inline` with the same bookkeeping. R4.2's two attachment sites.
pub(crate) fn bound_inline_to_chars(text: String, max_chars: usize) -> BoundedField;

/// Which message surfaces a single send had to cut (R6).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TruncationOutcome {
    pub(crate) content: bool,
    pub(crate) embed_description: bool,
    pub(crate) attachment_description: bool,
}

pub(crate) fn truncation_metadata_value(outcome: TruncationOutcome) -> Option<String>;
```

`ELLIPSIS_CHARS` and `LOCATION_ELLIPSIS_CHARS` are `pub(crate)` rather than
private because R1's compile-time assertions in `discord.rs` read the former.
Keeping each marker adjacent to its character count is what stops them from
drifting; unit tests asserting `ELLIPSIS.chars().count() == ELLIPSIS_CHARS` and
`LOCATION_ELLIPSIS.chars().count() == LOCATION_ELLIPSIS_CHARS` are the cheap way
to pin both.

`TruncationOutcome`, `BoundedField`, and `truncation_metadata_value` live here
rather than in either adapter because both adapters need them and R6's vocabulary
is explicitly provider-neutral — Telegram and Slack will write the same
`truncated=content` value from the same function. `BoundedField` in particular is
the *return shape of a bounding operation*, not a property of a
`PreparedMessage`; `prepared.rs` imports it from here rather than owning it, so
that `bound_to_chars` (no message in scope) and `bound_with_location` (message in
scope) return the same type.

The Discord *limit* constants stay in `discord.rs` and are passed in as the
`max_chars` argument. `MAX_LOCATION_LINE_CHARS` is the one limit that lives here
instead — it is not a platform limit and its only consumer is `prepared.rs`; the
argument is in R4.1 under "Placement".

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

#### Decided: `limits.rs` is **not** feature-gated

**Decided (reverses the reviewed draft's recommendation).** The module is
declared in `provider/mod.rs` with no `cfg` at all, and R4.1's two new
`prepared.rs` methods carry no `cfg` either. The reviewed draft's recommendation
— `#[cfg(feature = "discord")]`, matching `attachment_helpers`
(`provider/mod.rs:3-4`) — is rejected.

The two existing precedents do disagree, and neither settles it:
`attachment_helpers` is single-feature-gated, while `http_helpers` carries an
explicit seven-feature `any(...)` list (`provider/mod.rs:27-36`) that notably
omits `desktop` and has to be edited every time a provider is added. What settles
it is what the module actually contains.

**Nothing in `limits.rs` depends on twilight or on any Discord type.** It holds
two marker strings and their char counts, `MAX_LOCATION_LINE_CHARS`,
`BoundedField`, `TruncationOutcome`, and pure `String`/`usize` functions. There
is no `use` it needs from a gated module. Gating it would be gating a module on
a feature it has no relationship to.

**Gating it would make R4.1's neutrality argument aspirational rather than
true.** R4.1 places `MAX_LOCATION_LINE_CHARS` here on the explicit ground that
the same text-fallback location line is emitted for Slack, Slack-Webhook, and
Signal, all of which still call the untouched `render_body_with_location`
(`slack.rs:97`, `slack_webhook.rs:232`, `signal.rs:108`). Under
`--no-default-features --features slack`, a `discord`-gated cap would simply not
exist while those three providers went on emitting unbounded location lines —
the provider-neutral constant would be, in that build, a Discord constant. An
ungated module is what makes the claim literally true rather than
conditionally true.

**It also removes a step from the fixes that inherit this.** When Telegram's and
Slack's limits get their own fixes, they adopt
`render_body_with_location_bounded` and the shared sequence directly, rather than
first having to un-gate a module and widen an `any(...)` list — which is exactly
the "maintenance cost of the list" the reviewed draft proposed to defer.

**The `dead_code` consequence, addressed rather than left implicit.** Under a
build with no Discord feature — `--no-default-features --features desktop`, say —
nothing calls `truncate_to_chars`, `bound_to_chars`, `truncate_inline`,
`bound_inline_to_chars`, `truncation_metadata_value`, or constructs a
`TruncationOutcome`, and `rustc` will warn. The resolution is a single
module-level `#![allow(dead_code)]` at the top of `provider/limits.rs`, with a
comment naming the reason:

```rust
//! Provider-neutral length limits and truncation primitives.
//!
//! Compiled unconditionally: nothing here depends on a provider feature, and the
//! location cap it defines bounds a line that Slack, Slack-Webhook, and Signal
//! render too. Under a feature set with no length-bounded provider enabled,
//! every item here is legitimately unused.
#![allow(dead_code)]
```

The alternative — leaving `TruncationOutcome` and `truncation_metadata_value`
gated while ungating the rest — was considered and rejected: it splits one small
module across two compilation conditions to silence a warning, reintroduces the
`any(...)` list this decision exists to avoid, and would make `BoundedField`
ungated while the type that aggregates its flags is not. A blanket allow on a
38-to-60-line module of pure primitives is the smaller cost, and it is scoped to
that module rather than to the crate.

**No coupled gate in `prepared.rs`.** R4.1's `bound_with_location` and
`render_body_with_location_bounded` therefore carry **no** `cfg` either, which is
what `prepared.rs` — itself ungated — wants. They stay `pub(crate)` rather than
`pub` for an unrelated reason: they are not public API and nothing outside the
crate has a reason to bound a rendered body. `render_body_with_location` and
`render_body_for_provider` stay `pub` and ungated, unchanged.

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

The sample has **two** independent defects, not one, and the reviewed draft
recorded only the first:

1. **It is byte-slicing at an arbitrary offset** — the exact panic R3 exists to
   prevent — and it measures `s.len()` (bytes) against a character limit at line
   946. Leaving it in place as the repo's reference implementation is how the
   next adapter reintroduces the bug.
2. **It is written against an API this crate does not use at all.** The
   surrounding code builds the embed with `CreateEmbed::new()` (`:937`, `:953`),
   which is **serenity**'s builder. `serenity` is not a dependency of
   `messenger/lib` — nothing in the workspace pulls it — and both Discord
   adapters build embeds through `twilight_util::builder::embed::EmbedBuilder`
   (`discord.rs:9`, `:71`) or, on the webhook path, through the hand-rolled
   `EmbedBody` struct (`discord_webhook.rs:181-184`). A reference implementation
   that cannot compile against the crate it documents is worse than no reference
   implementation: a reader who copies it gets a compile error and then invents
   their own truncation, which is defect 1 again.

**In scope for this fix:** replace that sample with a call to
`truncate_to_chars`, or with a char-correct inline equivalent, expressed against
the builder the crate actually uses, and note the byte-vs-char trap in the
surrounding prose. This is a documentation-only change and belongs in its own
commit per the repo's scope-discipline rule.

Note the file also over-reaches in the same paragraph: it truncates an embed
**title** at 256 (`:954`), a field neither Discord adapter sets (R4's
embed-shape invariant). Trim the sample to the fields messenger actually uses, so
it does not read as a recommendation to add a title.

### R9 — Documentation drift this fix creates

**Added by review 2026-09-06; extended by the 2026-09-06 rulings.** This fix
changes *public, observable* behavior on both Discord adapters — an over-length
bot send stops returning `Err` and starts returning `Ok` with a shortened body,
successful receipts gain a new metadata key, `SendReceipt` gains a new public
accessor, and the CLI's stdout JSON gains a new field. Under the repo's Drift
Maintenance rule that obliges six updates, none of which the reviewed draft
named. R8 covers only the research doc.

| File | Change |
|------|--------|
| `messenger/README.md` | One line under the Discord rows: content over 2000 chars, embed descriptions over 4096, and attachment descriptions over 1024 are truncated rather than rejected, and the receipt carries `metadata["truncated"]`. The README already documents `metadata["delivery_confirmed"]` for Slack-Webhook (`README.md:36`), so this is the established place for a receipt-metadata note. |
| `messenger/docs/user-guide.md` (provider + receipts) | The Discord provider section gains the same note; the receipts section (`:451-457`) gains the `truncated` key alongside the existing `delivery_confirmed` example (`:626-629`), and `SendReceipt::truncated_fields()` is named beside `helper_used()` (`:686`) as the typed way to read it. |
| `messenger/docs/user-guide.md` (CLI stdout) | The new `SendResponse.truncated` field (R6). **Note:** the stdout response schema is currently documented *nowhere* — not in the user guide, not in the README, not in the skill — despite `SendResponse`'s own doc comment referring to "the labels documented in the response schema" (`messenger/cli/src/main.rs:483-484`). Adding a field to an undocumented schema is the moment to document the schema, so this row is "document `id` / `receipt` / `helper` / `os` / `truncated` as a set", not "add one line". |
| `.claude/skills/messenger/cli-reference.md` | The same stdout schema, in the "Receipts and Replies" section (`:88`), which today describes the receipt *file* but not the stdout JSON. |
| `.claude/skills/messenger/providers.md` | A "Receipt semantics" note for Discord and Discord-Webhook mirroring the Slack-Webhook one at `:100`. |
| `docs/dependencies.md` (repo root) | The new `twilight-validate` dev-dependency (see Testing → Prerequisites). The root doc already lists dev-only crates such as `wiremock` (`:1295`), so omitting it would be a real gap, not a judgment call. |

These are documentation-only edits and belong in their own commit, alongside R8's.
Nothing here changes behavior, and nothing here is optional: the repo's rule is
that public-behavior changes carry their doc updates in the same change.

## Out of Scope

- **Splitting long messages across multiple sends.** Discord clients commonly
  chunk over-length content into several messages. That changes the receipt
  contract (one `SendReceipt` per `send`) and is a feature, not a fix.
- **Attachment fallback.** Uploading the full body as a `.md` attachment and
  truncating the inline content is a plausible future behavior; not here.
- **Other providers' limits.** Telegram (4096), Slack (3000 per block), APNs
  payload size, and desktop-helper body limits all have their own caps and their
  own failure modes. Each deserves its own fix with its own constant. This spec
  establishes the pattern and the shared primitives; it does not roll them out.
  Concretely, the reuse those fixes inherit is `truncate_to_chars`,
  `truncation_metadata_value`, and — the payoff of R4.1's Decision — the whole
  reserve-truncate-append sequence via
  `PreparedMessage::render_body_with_location_bounded`, which R7's ungating makes
  reachable from any feature set without further edits. Only the Discord sites
  call it today.

  **Caveat, so the inheritance claim is not read as more than it is:**
  `truncate_to_chars` is **markup-unaware**, and that is sufficient for Discord
  only. Discord receives raw Markdown and renders a broken construct harmlessly
  (see "Markdown integrity across the cut" below). Telegram does not: it sends
  `parse_mode: "HTML"` (`telegram.rs:236`, `:240`), so a cut landing inside a
  `<b>` tag or between an opening and closing tag produces a Telegram **400** —
  the same total-message-loss failure this fix exists to eliminate, reintroduced
  by the very primitive that was supposed to prevent it. The Telegram fix
  inherits the *sequence* and the *char-safety*, but it must add markup-aware
  cutting of its own; the shared primitive is not sufficient for the provider
  named here as its first inheritor. Slack mrkdwn and Signal plain text have no
  equivalent hazard.
- **Fixing the byte-vs-char `len()` logging in the non-Discord adapters.** The
  five sites listed in R7 are named there as motivation for the shared helper;
  correcting them belongs with each provider's own limit fix.
- **Surfacing the truncation flag to Claudine's operator.** *This fix does not
  close the loop on the failure that opened it.* Claudine discards the
  `SendReceipt` — `messenger.send_planned(plan).await.map_err(…)?;` with the `?`
  in statement position (`claudine/lib/src/messaging/send.rs:714-717`) — so
  `SendReceipt::truncated_fields()` (R6) reaches nothing there, and
  `SendPlan::warnings`, which Claudine *does* surface at `:702-709`, structurally
  cannot carry a post-render fact. The remedy is a **Claudine-side** change: bind
  the receipt instead of discarding it and log `receipt.truncated_fields()`
  beside the existing `debug!("Message sent successfully")` at `:719-723`. It is
  a follow-up in the `claudine` package area, not a messenger change, and it
  cannot be made from this crate. See R6's named known consequence.
- **The `dropped` doc drift at `receipt.rs:6`.** Real and unconditional under the
  repo's drift rule, but a separate change — see the out-of-scope observation in
  R6. This fix does not write the `dropped` key and does not touch that module
  doc.
- **Length-validating `Location.name` / `Location.address`.** R4.1 caps the
  *rendered* line at the shared assembly point in `prepared.rs`, which is enough
  to bound the reservation. Bounding the two source fields themselves in
  `validate.rs`, for every provider and every send, is a broader contract change:
  it would turn an over-long location into a `MessengerError` for providers that
  have no length problem at all.
- **A `Location` builder that accepts `name` and `address`.** A real API gap,
  surfaced by R4.1's reachability finding and recorded so it is not lost.
  `Location::format_text_line` has four match arms (`message.rs:52-55`) and only
  one of them — `(None, None)`, the coordinates-only form at `:55` — is reachable
  through any constructor the crate offers, because `Message::location()`
  (`:109-122`) and `Message::with_location()` (`:134-142`) both hardcode
  `name: None, address: None`, and the CLI's `--location` parses `"LAT,LON"` only
  (`messenger/cli/src/main.rs:1224-1240`). Two of the three shapes the doc
  advertises at `:46-48` cannot be produced by a supported caller. Closing that
  gap means adding a builder — which is a feature, not a truncation fix, and
  which would **create** the pathological input R4.1's cap defends against rather
  than merely bounding it. The cap must land first; the builder is a separate
  change that can then be made safely.
- **Rendering of provider 400 bodies.** The raw-JSON-in-the-error-message
  ergonomics issue is real but independent.
- **Markdown integrity across the cut.** Truncation can leave an unterminated
  code fence or bold run. Discord renders that harmlessly. Making the cut
  markdown-aware is not worth the complexity. **This dismissal is scoped to
  Discord** and does not generalize — see the Telegram caveat under "Other
  providers' limits" above.
- **What `Attachment::caption` is supposed to mean.** Recorded as a fact rather
  than fixed: `caption` currently renders **nowhere a Discord reader sees**. It
  is an alt-text fallback only — `alt_text.clone().or_else(|| caption.clone())`
  (`discord.rs:108-114`, `discord_webhook.rs:310-313`) — so an attachment with a
  `caption` and no `alt_text` has its caption silently repurposed as the
  accessibility description, and an attachment with both loses the caption
  entirely. Neither adapter appends it to `content` or to the embed. Whether that
  is the right mapping is a **semantics** question, not a length one, and
  settling it would change what a public builder means:
  `Attachment::caption()` (`attachment.rs:59-62`) is `pub`, documented as "Set a
  caption on this attachment", and sits beside `Attachment::alt_text()`
  (`:65-68`) as if the two were distinct surfaces. R4.2 **bounds the field as it
  exists**; it does not ratify the mapping. A follow-up starts from those two
  line references.
- **`Message.title`.** Recorded so a reader does not go looking for a seventh
  site: neither Discord adapter reads `PreparedMessage::title()` at all
  (`discord.rs:58-95`, `discord_webhook.rs:249-282`), so there is no title
  surface to bound. Discord's `TITLE_LENGTH` (256) stays unreachable for the same
  reason R4's embed-shape invariant makes `EMBED_TOTAL_LENGTH` unreachable.

## Testing

All L1 (`just test` in `messenger/`). No network or L2 coverage is required —
the behavior is pure string handling on the payload-construction path.

**The `discord` feature is on for local L1** — worth stating because
`messenger/lib/Cargo.toml:116` sets `local-features = ["desktop"]` and the
justfile recipes read `-p messenger --features desktop`, which looks like a
desktop-only run. `--features` is *additive*: `default = ["discord", "slack"]`
stays enabled, so every test below compiles and runs under plain `just test`. Do
**not** "fix" this by reaching for `--all-features`; that pulls in five providers
this fix does not touch and is CI's job, not the local recipe's.

All new tests live **in-crate** — helper tests in `provider/limits.rs`'s own
`#[cfg(test)] mod tests`, and the rest under `messenger/lib/src/tests/`
(`lib.rs:51-52`). This is what makes the design's `pub(crate)` surface testable
at all; an external `tests/` target could not reach `bound_with_location`,
`BoundedField`, or `TruncationOutcome`.

**Feature gating of the new test modules.** R7 compiles `limits.rs` and R4.1's
two `prepared.rs` methods unconditionally, so `provider/limits.rs`'s own
`mod tests` needs **no** `cfg` — its subject compiles in every feature set, and
gating the tests would leave the primitives untested in exactly the builds where
they are the only thing present. `src/tests/mod.rs` already has both shapes and
the split is exactly this one: `mod builders;`, `mod receipts;`, and
`mod validation;` are declared ungated, while every `mod *_integration;` carries
its provider's `cfg`. The *application-site* modules added by this fix are the
second kind — `#[cfg(feature = "discord")]`, because their subjects
(`build_payload`, `build_attachment`, the adapters) are Discord-gated. The
Limit-model rows are gated for the same reason plus the dev-dependency (below).

### Decided: `build_payload` and `DiscordPayload` are raised to `pub(crate)`

**Decided.** As written, the instruction above is **impossible** on HEAD:
`fn build_payload` (`discord.rs:58`) and `struct DiscordPayload` (`:46`) are both
fully private — no `pub`, no `pub(crate)` — so nothing under `src/tests/` can
name either one, and the seven application-site rows below could not be written
where this section places them.

Resolution: raise both to `pub(crate)`, together with the webhook's extracted
equivalent (`build_payload` and its payload struct), and with R4.2's
`build_attachment`, `build_attachments`, `build_attachment_meta`, and
`AttachmentMeta`. This is a **deliberate visibility change made for test reach**,
not an incidental one, and it is recorded as such so a future reader does not
narrow it back on the grounds that "nothing outside the module calls it". Nothing
becomes public API: `pub(crate)` is the same crate-internal boundary
`bound_with_location` and `BoundedField` already sit behind, and it is the
boundary `src/tests/` needs.

**Recorded consequence:** the six existing `build_payload_*` tests in
`discord.rs`'s inline `mod tests` (`:243`, tests at `:373`, `:382`, `:391`,
`:403`, `:412`, `:423`) and the four existing `build_attachment` tests (`:303`,
`:324`, `:344`, `:361`) **stay where they are**. Discord adapter tests will
therefore live in two locations — inline for the pre-existing ones, `src/tests/`
for the new ones. Moving the existing ten is out of scope for this fix under
Rule 3; it is a pure test relocation with no behavior change and belongs in its
own commit if anyone wants it.

### Prerequisites

**A new dev-dependency is required.** The three Limit-model tests below call
`twilight_validate::message::content()`, `::embeds()`, and `::attachment()`, but
`twilight-validate`
is **not** a dependency of `messenger/lib` and `twilight-http` does **not**
re-export it (`twilight-http-0.17.1/src/lib.rs:23` re-exports only `Client`,
`Error`, and `Response`). Those tests do not compile as written. Add
`twilight-validate = "0.17"` to `[dev-dependencies]` — unconditionally, since
Cargo does not accept `optional` on dev-dependencies — and gate the tests
themselves with `#[cfg(feature = "discord")]`. Resolution is stable: `twilight-http`
0.17.1 already pins `twilight-validate` 0.17.0, so no second copy enters the graph
for the feature set that runs these tests. Per R9, record the crate in the root
`docs/dependencies.md`.

Writing the assertions against `DISCORD_MAX_CONTENT_LENGTH` and
`chars().count()` instead, and skipping the dependency, was considered and
rejected: that only re-asserts the constant the production code already used, and
the entire point of these rows is to pin messenger's counting model against
the *external* validator that actually gates the bot path.

The webhook `send_prepared` builds its payload inline (`discord_webhook.rs:249-282`)
rather than in a testable `build_payload` function like the bot adapter has
(`discord.rs:58`). This is still accurate on HEAD. Extracting the webhook payload
construction into a `pub(crate)` `build_payload` mirroring `discord.rs` is a
prerequisite for the webhook rows below and is in scope for this fix.

**Note on that extraction — the existing wiremock tests constrain it.** Three of
the webhook integration tests match on **exact body equality**, not on a subset:
`body_json(serde_json::json!({…}))` at
`messenger/lib/src/tests/discord_webhook_integration.rs:165`, `:205`, and `:520`.
`body_json` compares the whole deserialized body, so **any** change to
`WebhookJsonBody`'s serialized shape — a renamed field, a new field that
serializes when it should not, a changed `skip_serializing_if` — turns those
three green tests red for a reason unrelated to truncation. The extraction is
safe precisely while it is a *move*: the same `WebhookJsonBody`, the same fields,
the same `#[serde(skip_serializing_if = "Option::is_none")]` on all three
(`discord_webhook.rs:171-179`). Do not take the opportunity to tidy the struct.
If those three tests go red, the extraction changed the wire shape, not the
matcher.

Additionally, so the R6 receipt flag can be tested without a live twilight client
or a wiremock server, the truncation outcome must be **observable from
`build_payload`'s return value** (both adapters), with the flag→metadata mapping
done by a small pure function that tests can call directly. R4.1 fixes both
halves: `DiscordPayload` and the webhook's extracted equivalent each carry a
`truncated: TruncationOutcome` field, and `truncation_metadata_value` in
`limits.rs` is the pure mapping. Tests therefore reach the outcome through
`build_payload` and the value through a direct call, with no transport involved.
R4.2's attachment flag is reached the same way, through `build_attachments` and
`build_attachment_meta`.

### The bot path cannot be verified end-to-end, and that is deliberate

**Stated plainly, because Success Criterion 3 otherwise has no verification route
on one of its two adapters.** Verified on HEAD: there is no
`discord_integration.rs` under `messenger/lib/src/tests/`, there is no mock
transport for `twilight_http::Client`, and the bot adapter's `SendReceipt` is
constructed only inside `send_prepared`, after a live request has been issued and
decoded (`discord.rs:231-239`). There is no seam between "payload built" and
"receipt returned" on that path.

So SC3 is verified at two different depths:

- **Webhook path: end-to-end.** The existing wiremock harness
  (`src/tests/discord_webhook_integration.rs`) drives `send_prepared` to a real
  `SendReceipt` against a mock server, so `metadata["truncated"]` is asserted on
  the actual returned receipt.
- **Bot path: at two levels instead.** (1) The `TruncationOutcome` on
  `build_payload`'s return value, asserted directly — this is the input to the
  metadata insert. (2) `truncation_metadata_value` called as a pure function —
  this is the mapping from that input to the string. Together they cover every
  line between the cut and the receipt except the `metadata.insert` call itself,
  which is one statement in `send_prepared` beside an already-tested
  `BTreeMap::new()` (`discord.rs:238`).

This is a **stated limitation of the L1-only, network-free testing posture** this
fix commits to at the top of this section — not an oversight, and not something
to be fixed by loosening that posture. Adding a mock transport for twilight so
the bot path could be driven end-to-end is out of scope: it is a test-harness
feature that would serve one assertion in this fix, and it would be the first
such harness in the crate.

### Helper unit tests (`provider/limits.rs`)

| Test | Assertion |
|------|-----------|
| `text_under_limit_is_unchanged` | input of `max-1` chars round-trips byte-identical; no marker appended |
| `text_at_exact_limit_is_unchanged` | input of exactly `max` chars round-trips unchanged |
| `text_over_limit_is_truncated_within_limit` | input of `max+1` chars → result is **`<= max`** chars (R2/D2 weakened this from an equality) |
| `under_limit_input_with_trailing_whitespace_is_byte_identical` | input of `max-1` chars **ending in `"\n\n  "`** round-trips byte-identical — the trim must not reach the under-limit path. This is the `MessageBody::Plain` guard (`prepared.rs:72`) and Success Criterion 7 in one row |
| `truncation_trims_whitespace_before_the_marker` | input whose keep window ends in whitespace (e.g. the `"alpha\n\nbravo!"` / `max=12` example in R2) → result has **no whitespace immediately before the marker**: the character preceding `"\n\n..."` is non-whitespace, and the result is strictly shorter than `max` |
| `truncation_mid_word_is_unaffected_by_the_trim` | input whose keep window ends mid-word (no whitespace at the cut) → result is **exactly** `max` chars, proving the trim is a no-op there |
| `all_whitespace_keep_window_yields_bare_marker` | over-length input whose first `max-5` chars are all whitespace → result is exactly `"\n\n..."` and is non-empty (documented edge in R2) |
| `truncated_text_ends_with_marker` | result ends with `"\n\n..."` |
| `truncated_text_preserves_leading_text` | the input's leading text survives up to the (possibly trimmed) cut point — assert the result's pre-marker prefix is a prefix of the input |
| `truncation_is_char_safe_for_multibyte_input` | input of 3000 multi-byte chars (e.g. `'é'`, emoji, the `󰀨` glyph) truncates without panicking, and `chars().count() <= max` |
| `truncation_does_not_split_a_multibyte_char` | no replacement characters or invalid UTF-8 in the result |
| `helper_honors_the_limit_argument` | the same input truncated at 2000 and at 4096 produces different lengths — proves the limit is a parameter, not baked in |
| `budget_below_the_marker_length_still_respects_the_limit` | `truncate_to_chars(over_length, 3)` → result is exactly 3 chars with **no** marker, not the 5-char marker (R2 "Totality") |
| `bound_to_chars_reports_no_cut_for_under_limit_input` | under-limit input → `truncated == false`, `original_chars == input.chars().count()`, `text` byte-identical to the input. Sites 1 and 4 must not flag an untruncated summary |
| `bound_to_chars_reports_the_cut_and_the_original_length` | over-limit input → `truncated == true`, `original_chars` is the **pre-cut** count (not the post-cut one — this is the value R5 logs), and `text` equals `truncate_to_chars(input, max)` exactly, proving the wrapper adds bookkeeping and nothing else |
| `ellipsis_char_count_matches_the_published_constant` | `ELLIPSIS.chars().count() == ELLIPSIS_CHARS` — pins the constant R1's compile-time assertions read (R7) |
| `location_ellipsis_char_count_matches_the_published_constant` | `LOCATION_ELLIPSIS.chars().count() == LOCATION_ELLIPSIS_CHARS` |
| `location_line_under_the_cap_is_unchanged` | a `format_text_line()` output at or under `MAX_LOCATION_LINE_CHARS` round-trips byte-identical through `truncate_location_line` — the cap is invisible for every location a supported entry point can build |
| `location_line_over_the_cap_gets_the_inline_marker` | a line built from a 400-char `name` → result is `<= MAX_LOCATION_LINE_CHARS` chars, `ends_with("…")`, contains **no** `"\n"`, and does **not** contain `"\n\n..."` |
| `capped_location_line_still_begins_with_the_pin` | the capped result still starts with `"📍"` — the trim can never empty a location line |
| `truncate_location_line_is_truncate_inline_at_the_cap` | `truncate_location_line(x)` == `truncate_inline(x, MAX_LOCATION_LINE_CHARS)` for over-cap, at-cap, and under-cap inputs — pins R4.2's refactor of the location wrapper onto the shared inline one |
| `inline_truncation_uses_the_single_char_marker` | `truncate_inline(over_length, 1024)` → `<= 1024` chars, `ends_with("…")`, contains **no** `"\n\n..."` and no `'\n'` at all — the R4.2 marker choice, asserted at the attachment budget rather than the location cap |
| `bound_inline_to_chars_reports_no_cut_for_under_limit_input` | under-limit input → `truncated == false`, `original_chars == input.chars().count()`, `text` byte-identical. An attachment with a normal alt text must not flag |
| `bound_inline_to_chars_reports_the_cut_and_the_original_length` | over-limit input → `truncated == true`, `original_chars` is the **pre-cut** count (the value R5's per-attachment `warn!` logs), and `text == truncate_inline(input, max)` |
| `inline_truncation_is_char_safe_for_multibyte_input` | a 3000-char multi-byte alt text at 1024 truncates without panicking and `chars().count() <= 1024` — R3 applies to the inline marker path too |

### Application-site tests (six body sites from R4)

| Test | Assertion |
|------|-----------|
| `build_payload_truncates_plain_content` | site 3: `DiscordProvider::build_payload` with a 5000-char `Markdown` body → `content` is `<= DISCORD_MAX_CONTENT_LENGTH` chars |
| `build_payload_truncates_summary_content` | site 1: 5000-char `Summarized` *summary* → `content` bounded |
| `build_payload_truncates_embed_description` | site 2: `Summarized` body whose markdown exceeds 4096 → embed `description` is `<= DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` chars |
| `webhook_build_payload_truncates_plain_content` | site 6, webhook equivalent |
| `webhook_build_payload_truncates_summary_content` | site 4, webhook equivalent |
| `webhook_build_payload_truncates_embed_description` | site 5, webhook equivalent |
| `content_limit_and_embed_limit_are_not_swapped` | a 3000-char `Summarized` markdown is **not** truncated (it is under 4096) while a 3000-char plain body **is** — guards against the two constants being crossed |

### Attachment-description tests (R4.2)

| Test | Assertion |
|------|-----------|
| `build_attachment_truncates_long_alt_text` | bot: `alt_text` of 3000 chars → the returned `DiscordAttachment.description` is `<= DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH` chars and `ends_with("…")`, and the returned flag is `true` |
| `build_attachment_truncates_long_caption_used_as_fallback` | bot: `alt_text = None`, `caption` of 3000 chars → same bound and same flag, proving the cut is applied to the *resolved* string rather than to `alt_text` only |
| `build_attachment_prefers_alt_text_over_caption_when_both_are_long` | bot: both fields over-length → the surviving prefix is the **`alt_text`** prefix, pinning the fallback chain (`discord.rs:108-114`) through the truncation |
| `build_attachment_under_limit_description_is_byte_identical` | bot: a normal-length alt text → `description` byte-identical to its pre-fix value and the flag is `false`. Success Criterion 7 for the attachment surface; this is what the existing inline test at `discord.rs:303` already asserts, preserved |
| `build_attachment_with_no_description_reports_no_cut` | bot: `alt_text = None`, `caption = None` → `description` is `None` and the flag is `false` — the `if let` guard is unaffected |
| `build_attachments_ors_the_flags_across_attachments` | bot: three attachments, only the second over-length → the aggregate flag is `true`; all three over-length → still one `true`. Pins that the receipt names surfaces, not instances |
| `webhook_build_attachment_meta_truncates_long_description` | webhook: `AttachmentMeta.description` is `<= DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH` chars, `ends_with("…")`, flag `true` |
| `webhook_build_attachment_meta_matches_the_bot_result` | the same `Attachment` produces the **same** description string on both adapters — the anti-drift row, mirroring `embed_and_content_sites_share_the_reservation` |

### Location-reservation tests (R4.1)

| Test | Assertion |
|------|-----------|
| `location_survives_truncated_content` | site 3: 5000-char `Markdown` body **with** a location → `content` still `ends_with(location_line)`, and is `<= DISCORD_MAX_CONTENT_LENGTH` chars |
| `location_survives_truncated_embed_description` | site 2: `Summarized` markdown over 4096 **with** a location → embed `description` still `ends_with(location_line)`, and is `<= DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` |
| `webhook_location_survives_truncated_content` | site 6, webhook equivalent |
| `webhook_location_survives_truncated_embed_description` | site 5, webhook equivalent |
| `truncation_marker_sits_above_the_location_line` | the result contains `"\n\n...\n"` immediately before the location line — i.e. the marker is **not** the last thing in the field |
| `pathological_location_is_capped_and_the_body_survives` | a `Location` whose `name` is 2500 chars, on a 5000-char body, at the 2000-char content limit: the result is `<= DISCORD_MAX_CONTENT_LENGTH`, the location segment is `<= MAX_LOCATION_LINE_CHARS` chars and ends with `"…"`, **at least 1743 chars of body text survive**, and nothing panics. This is the row that supersedes the removed fallback test |
| `location_reservation_never_exceeds_the_cap` | across location lines of 0, 67, 256, 257, and 2500 rendered chars, the reserved budget is never more than `MAX_LOCATION_LINE_CHARS + 1` — pins the bound the removed run-time guard used to approximate |
| `message_without_location_is_unaffected_by_the_reservation` | 5000-char body, **no** location → identical result to the pre-R4.1 truncation; `reserved` is 0 |
| `under_limit_body_with_location_is_byte_identical` | short body + location → byte-identical to the pre-fix `render_body_with_location` output, proving sites 3 and 6 lost nothing by switching to `render_body_with_location_bounded` |
| `bounded_render_matches_render_body_with_location_when_under_limit` | for messages whose location is under `MAX_LOCATION_LINE_CHARS` (every one a supported entry point can build), `render_body_with_location_bounded(kind, usize::MAX).text` == `render_body_with_location(kind)` across plain, markdown, empty-body, and with/without-location cases — the direct pin on Success Criterion 7. An over-cap location is deliberately *not* covered here: the cap is the one intended difference |
| `under_limit_embed_with_location_is_byte_identical` | sites 2 and 5: `Summarized` markdown + location, both under limit → embed `description` byte-identical to the adapters' former inline `rich_md` + `'\n'` + `format_text_line()` assembly. The sites-2/5 half of Success Criterion 7 |
| `embed_and_content_sites_share_the_reservation` | the same pathological location produces the same capped location segment on site 2 (embed) and site 3 (content) — pins that both branches go through `bound_with_location` and that no second copy of the sequence exists |
| `render_body_with_location_is_unchanged` | its existing tests (`src/tests/builders.rs:123`, `:132`, `:140`, `:148`) still pass untouched — R4.1 adds a function beside it and must not modify the one Signal, Slack, and Slack-Webhook also call |
| `non_discord_callers_are_untouched` | `signal.rs:108`, `slack.rs:97`, and `slack_webhook.rs:232` still call `render_body_with_location`; their rendered text is byte-identical before and after the fix |
| `original_chars_counts_the_uncapped_location` | pathological location + over-length body → `BoundedField.original_chars` equals `body.chars().count() + 1 + format_text_line().chars().count()` using the **uncapped** line, so R5's `warn!` reports the true size of what was lost rather than the post-cap size |
| `location_only_cut_still_flags_the_field` | short body (well under the limit) + a 2500-char location → `truncated == true`, because the location was cut even though the body fit. Pins R4.1's decision that `BoundedField.truncated` is one flag over both losses |

### Limit-model tests

| Test | Assertion |
|------|-----------|
| `truncated_payload_passes_twilight_content_validation` | `twilight_validate::message::content()` returns `Ok` for a truncated `content` — the bot path's client-side gate is what actually has to be satisfied |
| `truncated_payload_passes_twilight_embed_validation` | `twilight_validate::message::embeds()` returns `Ok` for the built embed slice |
| `truncated_attachment_passes_twilight_attachment_validation` | `twilight_validate::message::attachment()` returns `Ok` for a `DiscordAttachment` built from a 3000-char alt text — the gate at `twilight-validate-0.17.0/src/message.rs:210`, `:224` that R4.2 exists to satisfy |

### Receipt-metadata tests (R6)

| Test | Assertion |
|------|-----------|
| `untruncated_send_has_no_truncated_metadata` | receipt `metadata` has no `truncated` key when nothing was cut |
| `truncated_content_sets_truncated_metadata` | `metadata["truncated"] == "content"` |
| `truncated_embed_description_sets_truncated_metadata` | `metadata["truncated"] == "embed_description"` |
| `truncated_attachment_description_sets_truncated_metadata` | `metadata["truncated"] == "attachment_description"` (R4.2) |
| `all_three_truncated_sets_combined_truncated_metadata` | `metadata["truncated"] == "content,embed_description,attachment_description"` — comma-joined in that fixed order, asserted through a send rather than through the pure function |
| `truncation_does_not_write_the_dropped_key` | `metadata` has **no** `dropped` key on any truncated send — pins the R6 decision to keep the two vocabularies separate |
| `truncated_metadata_survives_receipt_json_roundtrip` | `SendReceipt::to_pretty_json` → `from_json_str` preserves the key and value |
| `webhook_truncated_send_sets_truncated_metadata` | webhook adapter equivalent, **end-to-end** through the existing wiremock harness — the only end-to-end route SC3 has, per "The bot path cannot be verified end-to-end" above |

`truncation_metadata_value` is a pure function, so its eight cases are pinned
directly rather than through a send. All eight rows, one test each or one
table-driven test — the fixed order
`content,embed_description,attachment_description` is what these exist to pin:

| `TruncationOutcome` | Expected value |
|---------------------|----------------|
| all `false` | `None` |
| `content` | `Some("content")` |
| `embed_description` | `Some("embed_description")` |
| `attachment_description` | `Some("attachment_description")` |
| `content` + `embed_description` | `Some("content,embed_description")` |
| `content` + `attachment_description` | `Some("content,attachment_description")` |
| `embed_description` + `attachment_description` | `Some("embed_description,attachment_description")` |
| all `true` | `Some("content,embed_description,attachment_description")` |

### CLI stdout tests (R6)

| Test | Assertion |
|------|-----------|
| `truncated_fields_returns_none_without_the_key` | `SendReceipt::truncated_fields()` on a receipt with an empty `metadata` → `None` |
| `truncated_fields_returns_the_raw_value` | a receipt carrying `truncated = "content,embed_description"` → `Some("content,embed_description")`, returned verbatim with no filtering (unlike `helper_used`, `receipt.rs:236-241`) |
| `send_response_omits_truncated_when_absent` | `SendResponse` serialized from a receipt with no `truncated` key contains **no** `truncated` field at all — the `skip_serializing_if` behaves as `helper`'s does |
| `send_response_carries_the_truncated_value` | `SendResponse` serialized from a truncated receipt contains `"truncated": "content"` |

These live in `messenger/cli/src/main.rs`'s existing `#[cfg(test)] mod tests`
(`:1243`), which is where the CLI's other pure-function tests already are.

Per the monorepo testing guidance, each new test must be proven non-vacuous:
neuter the truncation call (or the metadata insert), confirm the test goes red,
restore.

## Success Criteria

1. **Plain/Markdown body.** For a 5000-character lifecycle message on both
   adapters, `build_payload` yields a `content` of at most
   `DISCORD_MAX_CONTENT_LENGTH` characters ending in `"\n\n..."` (or, when the
   message carries a location, in `"\n\n...\n"` + the location line — see R4.1),
   and that `content` passes `twilight_validate::message::content()`. Passing the
   validator *is* the bot path's delivery gate: it is the check that produced
   today's `MessengerError::Transport`, and no network call happens before it.
   Live delivery through a real `discord_webhook` route with no 400 is a one-off
   manual smoke check — worth doing once, but it is **not** a gating criterion,
   because the Testing section deliberately keeps this fix network-free.
2. **`Summarized` body.** For a message whose rich markdown exceeds 4096
   characters, both adapters' `build_payload` yields an embed `description` of at
   most `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH` characters that passes
   `twilight_validate::message::embeds()`, with the `content` (the summary
   banner) byte-identical to its pre-fix value.
3. A successful send that truncated anything returns a `SendReceipt` carrying the
   `truncated` key with the value specified in R6, and that key survives the
   receipt JSON round-trip. No send writes the `dropped` key. **Verification is
   asymmetric by design, and the criterion is met by both halves together:** on
   the **webhook** path this is verified end-to-end through the existing wiremock
   harness, on a real returned `SendReceipt`; on the **bot** path there is no
   mock transport for twilight and no seam between payload construction and the
   live call (`discord.rs:231-239`), so it is verified at two levels instead —
   the `TruncationOutcome` on `build_payload`'s return value, and
   `truncation_metadata_value` called directly as a pure function. That is a
   stated limitation of the L1-only, network-free posture, not an oversight; see
   "The bot path cannot be verified end-to-end" in Testing. A bot-path mock
   transport is out of scope.
4. **Attachment `description` is bounded (R4.2).** For an attachment whose
   resolved description (`alt_text`, else `caption`) exceeds 1024 characters,
   both adapters produce a description of at most
   `DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH` characters ending in the inline
   `"…"` marker, the built `DiscordAttachment` passes
   `twilight_validate::message::attachment()`, and the receipt carries the
   `attachment_description` token. Under-limit descriptions are byte-identical to
   their pre-fix value and set no token.
5. `DISCORD_MAX_CONTENT_LENGTH`, `DISCORD_MAX_EMBED_DESCRIPTION_LENGTH`, and
   `DISCORD_MAX_ATTACHMENT_DESCRIPTION_LENGTH` are
   the only places `2000`, `4096`, and `1024` appear in the Discord adapters and
   their tests, and `MAX_LOCATION_LINE_CHARS` is the only place `256` appears as
   a location bound in `limits.rs`, `prepared.rs`, and their tests. The two
   compile-time assertions in `discord.rs` (R1) are present and the crate builds.
6. Multi-byte content at any length never panics, on any of the three fields, on
   either adapter.
7. Messages under all three limits are byte-identical to their pre-fix output —
   no regression in the common case, and no `truncated` key on their receipts.
   This holds for the under-limit-with-trailing-whitespace case (the trim is
   conditional, R2), for the under-limit-with-location case on sites 3 and 6
   (`render_body_with_location_bounded` must reproduce
   `render_body_with_location` exactly, R4.1), for the under-limit-with-location
   case on sites 2 and 5 (`bound_with_location` must reproduce the adapters'
   former inline append exactly), and for an under-limit attachment description
   on both adapters (R4.2). A location at or under `MAX_LOCATION_LINE_CHARS` —
   every location any supported entry point can construct — is byte-identical to
   `format_text_line()`.
8. **Location survives, bounded.** On every over-length send that carries a
   location, the location line is present in the final field value, on all four
   location-carrying sites, on both adapters (R4.1). A location longer than
   `MAX_LOCATION_LINE_CHARS` is itself truncated with the inline `"…"` marker, so
   the reservation is bounded by construction: there is **no** fallback path, no
   location-only message is reachable, and at least 1743 characters of body
   survive at the 2000-character limit whatever the caller supplies — including a
   caller who constructs `Location` by struct literal, which is the only way to
   reach the case at all (R4.1, "Why the cap is worth having").
9. `render_body_with_location` (`prepared.rs:57-66`) keeps its signature and body
   unchanged — only its doc comment changes, to stop naming Discord as a caller
   (R4.1) — and it still has exactly three callers: `signal.rs:108`,
   `slack.rs:97`, `slack_webhook.rs:232`, which have no behavior change. The
   reserve-truncate-append sequence exists in exactly **one** place,
   `PreparedMessage::bound_with_location`, and all four location-carrying sites
   reach it; no Discord adapter formats or appends a location line itself.
10. All six body sites obtain their `truncated` flag from `BoundedField.truncated`
    — from `bound_to_chars` on sites 1 and 4, from `bound_with_location` on the
    other four — and R4.2's two attachment sites from `bound_inline_to_chars`. No
    adapter derives a truncation flag by comparing character counts (R2's
    reader's note).
11. **The flag reaches CLI stdout (R6).** `SendReceipt::truncated_fields()`
    exists on the library type beside `helper_used()`, `SendResponse` carries an
    `Option<String> truncated` field with the same `skip_serializing_if` as
    `helper`, and `emit_send_response` populates it from the receipt. A truncated
    `messenger send` prints `"truncated": …` in its stdout JSON; an untruncated
    one prints no such field.
12. **`provider/limits.rs` compiles unconditionally (R7).** The module carries no
    `cfg`, and neither do `bound_with_location` and
    `render_body_with_location_bounded` in `prepared.rs`. Verified by building
    with `--no-default-features --features desktop`: the crate compiles, the
    `limits` unit tests run, and no `dead_code` warning escapes the module-level
    `#![allow(dead_code)]`.
13. **The visibility change is the one specified, and no wider (Testing).**
    `build_payload`, `DiscordPayload`, the webhook's payload equivalent,
    `build_attachment`, `build_attachments`, `build_attachment_meta`, and
    `AttachmentMeta` are `pub(crate)`. Nothing among them is `pub`. The crate's
    public API surface is unchanged except for `SendReceipt::truncated_fields()`
    and R1's three constants.
14. `messenger/docs/research/platforms/discord.md` no longer presents a
    byte-slicing truncation as the reference implementation, and no longer
    presents a serenity `CreateEmbed` sample as this crate's reference (R8).
15. The six documentation updates in R9 have landed, and `twilight-validate`
    appears in both `messenger/lib/Cargo.toml`'s `[dev-dependencies]` and the root
    `docs/dependencies.md`.
16. **Attachment `description` is covered, not deferred.** The 2026-09-06
    review's open question is resolved as implemented (R4.2): the constant exists, both adapters bound the
    field, the `attachment_description` token is emitted, and the Out-of-Scope
    section records only the *semantics* question about `caption` — not the
    length one.
17. `just test` and `just lint` are green in `messenger/`.
