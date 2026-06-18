//! Phase-1 prototype for the inline-span source-rewrite architecture.
//!
//! Spec: `renderable/features/2026-05-26-inline-span/spec.md`
//! Plan: `renderable/features/2026-05-26-inline-span/plan.md` (Phase 1)
//! Notes: `renderable/features/2026-05-26-inline-span/phase-1-prototype-notes.md`
//!
//! This is a **decision-locking prototype**, not production behavior. It feeds
//! candidate strikethrough envelopes through a plain `pulldown-cmark` parser
//! with `ENABLE_STRIKETHROUGH` and proves which marker bracket the parser
//! leaves untouched.
//!
//! ## Key finding
//!
//! The spec's *primary* marker `<<TOKEN>>` is **broken**: pulldown-cmark
//! tokenizes the inner `<token>` as inline HTML (`<mark>`, `<dim>`, etc. all
//! satisfy the CommonMark inline-HTML open-tag production), so `<<mark>>`
//! splits into `Text("<")` + `InlineHtml("<mark>")` + `Text(">")`. The
//! envelope marker is therefore the **pipe-free** `{{!TOKEN!}}` form,
//! immediately followed by the U+FDD0 sentinel. Curly braces and `!` are inert
//! in CommonMark inline parsing (`!` is only special as the image lead-in
//! `![`, which the marker never forms), so the marker survives verbatim as
//! literal `Event::Text`.
//!
//! The marker is deliberately pipe-free: the earlier `{{|TOKEN|}}` form
//! injected `|` bytes that a GFM table parser counted as extra column
//! separators, splitting any cell that held a mark/dim span (review-1
//! finding 2). `{{!TOKEN!}}` carries no table meaning, so an envelope is safe
//! in any cell.
//!
//! The canonical envelope is:
//!
//! ```text
//! ~~{{!TOKEN!}}\u{FDD0}payload{{!TOKEN!}}\u{FDD0}~~
//! ```
//!
//! These facts are the load-bearing assumptions of the whole rewrite
//! architecture. The prototype is kept as a regression guard against a future
//! pulldown-cmark upgrade silently changing the behavior.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// The non-character sentinel that immediately follows each `{{!TOKEN!}}`
/// marker. U+FDD0 is a permanent Unicode non-character — it never appears in
/// well-formed user text, which is what makes the marker collision-proof.
const SENTINEL: char = '\u{FDD0}';

/// Renders the locked marker for `token`: `{{!TOKEN!}}`.
fn marker(token: &str) -> String {
    format!("{{{{!{token}!}}}}")
}

/// Builds the canonical paired envelope for `token` wrapping `payload`.
fn envelope(token: &str, payload: &str) -> String {
    let m = marker(token);
    format!("~~{m}{SENTINEL}{payload}{m}{SENTINEL}~~")
}

/// Parses `markdown` with strikethrough enabled and collects the event stream.
fn events(markdown: &str) -> Vec<Event<'_>> {
    Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH).collect()
}

/// Concatenates every `Event::Text` payload in order.
fn text_concat(evs: &[Event<'_>]) -> String {
    evs.iter()
        .filter_map(|e| match e {
            Event::Text(t) => Some(t.as_ref()),
            _ => None,
        })
        .collect()
}

/// Returns `true` when no event in `evs` is inline/block HTML or a link
/// (autolink) tag — i.e. the parser left the marker as literal text.
fn is_literal_only(evs: &[Event<'_>]) -> bool {
    !evs.iter().any(|e| {
        matches!(
            e,
            Event::Html(_) | Event::InlineHtml(_) | Event::Start(Tag::Link { .. })
        )
    })
}

#[test]
fn angle_bracket_marker_is_parsed_as_inline_html() {
    // Documents WHY the envelope marker is not the spec's primary `<<TOKEN>>`.
    // `<mark>` is a real HTML element name and `<dim>` still matches the
    // inline-HTML open-tag production, so pulldown-cmark consumes the inner
    // `<token>` as InlineHtml. This is the evidence that drove the switch to
    // `{{|TOKEN|}}`.
    let src = format!("<<mark>>{SENTINEL}");
    let evs = events(&src);
    assert!(
        evs.iter().any(|e| matches!(e, Event::InlineHtml(_))),
        "expected `<<mark>>` to leak an InlineHtml event: {evs:?}",
    );
    assert!(
        !is_literal_only(&evs),
        "`<<mark>>` is known-broken — it must NOT stay literal: {evs:?}",
    );
}

#[test]
fn locked_marker_survives_for_token_names() {
    // The chosen `{{!TOKEN!}}` marker stays literal text for every built-in
    // token name, including ones that are valid HTML element names (`mark`).
    for token in ["mark", "dim", "emoji", "var", "tooltip"] {
        let src = format!("{}{SENTINEL}", marker(token));
        let evs = events(&src);
        assert!(
            is_literal_only(&evs),
            "`{{{{!{token}!}}}}` must stay literal text, never HTML/autolink: {evs:?}",
        );
        assert!(
            text_concat(&evs).contains(&marker(token)),
            "the full `{{{{!{token}!}}}}` marker must survive verbatim: {evs:?}",
        );
    }
}

#[test]
fn mark_envelope_parses_as_single_strikethrough_container() {
    let src = envelope("mark", "highlighted");
    let evs = events(&src);

    let starts = evs
        .iter()
        .filter(|e| matches!(e, Event::Start(Tag::Strikethrough)))
        .count();
    let ends = evs
        .iter()
        .filter(|e| matches!(e, Event::End(TagEnd::Strikethrough)))
        .count();

    assert_eq!(starts, 1, "envelope must open exactly one Strikethrough: {evs:?}");
    assert_eq!(ends, 1, "envelope must close exactly one Strikethrough: {evs:?}");
}

#[test]
fn mark_envelope_preserves_markers_and_sentinel_as_literal_text() {
    let src = envelope("mark", "highlighted");
    let evs = events(&src);
    let joined = text_concat(&evs);

    // Both opener and closer markers survive verbatim.
    assert_eq!(
        joined.matches(&marker("mark")).count(),
        2,
        "both `{{!mark!}}` markers must survive as literal text: {joined:?}",
    );
    // Both U+FDD0 sentinels survive verbatim.
    assert_eq!(
        joined.matches(SENTINEL).count(),
        2,
        "both U+FDD0 sentinels must survive as literal text: {joined:?}",
    );
    // The payload survives between the markers.
    assert!(
        joined.contains("highlighted"),
        "payload must survive as literal text: {joined:?}",
    );
}

#[test]
fn marker_is_never_inline_html_or_autolink() {
    // The marker alone, the full envelope, the marker amid prose, and a bare
    // `{{!mark!}}` with no sentinel must all stay literal text — never
    // InlineHtml, Html, or a Link tag (autolink).
    for fixture in [
        format!("{}{SENTINEL}", marker("mark")),
        envelope("mark", "x"),
        format!("prefix {}{SENTINEL} suffix", marker("tooltip")),
        format!("a bare {} with no sentinel", marker("mark")),
    ] {
        let evs = events(&fixture);
        assert!(
            is_literal_only(&evs),
            "marker must not parse as HTML or autolink in {fixture:?}: {evs:?}",
        );
        assert!(
            text_concat(&evs).contains("{{!"),
            "`{{!` marker bytes must reach the text stream in {fixture:?}: {evs:?}",
        );
    }
}

/// The locked marker must be **pipe-free** so an envelope can live inside a GFM
/// table cell without injecting a stray column separator. This is the prototype
/// guard for review-1 finding 2: feeding `| ==hi== |`-shaped content (already
/// rewritten to an envelope) through the table parser must keep the cell count.
#[test]
fn locked_marker_is_pipe_free_and_table_safe() {
    // 1. The marker carries no pipe bytes.
    for token in ["mark", "dim", "emoji", "var", "tooltip"] {
        assert!(
            !marker(token).contains('|'),
            "marker for `{token}` must be pipe-free for table safety: {:?}",
            marker(token),
        );
    }

    // 2. A table row whose first cell holds an envelope keeps two cells. Parse
    //    with tables enabled and count the `TableCell` start events in the head.
    let cell = envelope("mark", "hi");
    let table = format!("| {cell} | ok |\n|----|----|\n| a | b |\n");
    let evs: Vec<Event<'_>> = Parser::new_ext(
        &table,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES,
    )
    .collect();
    // Locate the head's cells: count TableCell starts before TableHead closes.
    let mut in_head = false;
    let mut head_cells = 0usize;
    for ev in &evs {
        match ev {
            Event::Start(Tag::TableHead) => in_head = true,
            Event::End(TagEnd::TableHead) => break,
            Event::Start(Tag::TableCell) if in_head => head_cells += 1,
            _ => {}
        }
    }
    assert_eq!(
        head_cells, 2,
        "an envelope in the first cell must not split the row into extra cells: {evs:?}",
    );
}

#[test]
fn dim_envelope_round_trips_like_mark() {
    // The dim token uses the identical envelope shape; only the token name
    // differs. Confirms the format is token-agnostic.
    let src = envelope("dim", "dim text");
    let evs = events(&src);
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, Event::Start(Tag::Strikethrough)))
            .count(),
        1,
    );
    let joined = text_concat(&evs);
    assert_eq!(joined.matches(&marker("dim")).count(), 2, "{joined:?}");
    assert_eq!(joined.matches(SENTINEL).count(), 2, "{joined:?}");
}

#[test]
fn multi_field_sentinel_survives_in_payload() {
    // Tooltips encode two fields with the U+FDD1 field separator inside the
    // envelope payload. Confirm pulldown-cmark passes U+FDD1 through as text
    // exactly like U+FDD0, so the fold can split on it after stripping the
    // markers. Locks the field-separator decision (U+FDD1).
    const FIELD_SEP: char = '\u{FDD1}';
    let payload = format!("term{FIELD_SEP}definition");
    let src = envelope("tooltip", &payload);
    let evs = events(&src);
    let joined = text_concat(&evs);
    assert_eq!(
        joined.matches(FIELD_SEP).count(),
        1,
        "U+FDD1 field separator must survive as literal text: {joined:?}",
    );
    assert!(joined.contains("term") && joined.contains("definition"), "{joined:?}");
}

#[test]
fn nested_markdown_payload_is_parsed_inside_envelope() {
    // A mark payload containing emphasis must keep that emphasis structural —
    // the fold re-parses the payload as inline Markdown, but even the raw
    // envelope parse should surface the emphasis container, proving the
    // markers do not block normal inline parsing of the payload.
    let src = envelope("mark", "*emphasized*");
    let evs = events(&src);
    assert!(
        evs.iter().any(|e| matches!(e, Event::Start(Tag::Emphasis))),
        "emphasis inside the payload must still parse: {evs:?}",
    );
}
