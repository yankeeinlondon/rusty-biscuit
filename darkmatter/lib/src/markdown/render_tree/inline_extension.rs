//! Source-layer rewriter for darkmatter inline extensions.
//!
//! This module rewrites darkmatter's inline syntax (`==mark==`, `⌄dim⌄`) into a
//! canonical GFM-strikethrough envelope **before** the source reaches
//! `pulldown-cmark`. The parser then recognises each envelope as an ordinary
//! [`Tag::Strikethrough`](pulldown_cmark::Tag::Strikethrough) container, and a
//! fold-side dispatcher (Phase 5) peeks the envelope marker to emit
//! [`renderable::tree::NodeKind::Extended`] nodes. This replaces the per-event
//! span-aware inline transport that previously split `==mark==` / `⌄dim⌄` while
//! preserving byte ranges; see
//! `renderable/features/2026-05-26-inline-span/spec.md`.
//!
//! The rewriter produces a rewritten source plus a provenance table that
//! `fold_markdown_spanned_with_frontmatter` consumes to resolve every span back
//! to the original source.
//!
//! ## Canonical envelope
//!
//! Each recognised pair is rewritten to (sentinel shown as `␦`):
//!
//! ```text
//! ~~{{!TOKEN!}}␦payload{{!TOKEN!}}␦~~
//! ```
//!
//! `{{!TOKEN!}}` is the literal marker and `␦` is the U+FDD0 non-character
//! sentinel that immediately follows it at both opener and closer. Phase 1
//! proved the spec's primary `<<TOKEN>>` form is parsed as inline HTML; the
//! `{{!TOKEN!}}` form survives `pulldown-cmark` verbatim (curly braces and the
//! `!` are inert in CommonMark inline parsing — `!` is only special as the
//! image lead-in `![`, which the marker never forms). See
//! `renderable/features/2026-05-26-inline-span/phase-1-prototype-notes.md` and
//! the prototype `darkmatter/lib/tests/inline_envelope_prototype.rs`.
//!
//! The marker is deliberately **pipe-free**. The earlier `{{|TOKEN|}}` form
//! injected raw `|` bytes into the rewritten source; inside a GFM table cell
//! those extra pipes were counted as column separators and split the row, so
//! `| ==hi== |` lost its cell boundaries. Curly-brace + `!` carries no
//! table meaning, so an envelope is safe in any cell.
//!
//! ## Verbatim regions are never rewritten
//!
//! The scan runs over raw source bytes, but a rewrite must only ever touch
//! eligible prose. [`protected_ranges`] pre-parses the original source once and
//! records every region the rewrite must leave alone — inline code spans,
//! fenced and indented code blocks, raw HTML (block and inline), image
//! constructs (alt + destination + title), and link destinations/titles. Any
//! delimiter intersecting a protected region stays literal, so `` `==code==` ``,
//! fenced blocks containing `==`/`⌄`, and `[x](a==b==c)` URLs survive untouched.
//! The pre-parse only runs when [`scan_delimiters`] already found a candidate
//! delimiter, so a document with no darkmatter inline syntax never pays for it.
//!
//! ## One shared scan (hard rule)
//!
//! > All inline token patterns are matched in **one shared scan** of the source
//! > text (a single multi-pattern matcher — `memchr`-based or `aho-corasick`),
//! > never one pass per token. Adding a new inline token adds a pattern to the
//! > shared scan; it must not add another full pass over the source. This is the
//! > direct equivalent of the prior architecture's rule that "all inline features
//! > share one processor walk," and it is the reason this rewrite exists.
//!
//! [`scan_delimiters`] is that single walk. New tokens extend [`INLINE_TOKENS`]
//! — they must not add another pass over the source.
//!
//! ## Provenance
//!
//! A no-extension document returns its source borrowed unchanged with an empty
//! [`ProvenanceTable`] (every byte maps 1:1). When a rewrite occurs, the table
//! maps rewritten byte offsets back to **original** byte offsets so the fold can
//! construct [`renderable::tree::SourceSpan`]s and diagnostics that point at the
//! bytes the user actually typed, not at synthetic envelope bytes.

use pulldown_cmark::{Event, LinkType, Parser, Tag, TagEnd};
use std::borrow::Cow;
use std::ops::Range;

use crate::markdown::block::matches_horizontal_rule_pattern;

/// The non-character sentinel that immediately follows each `{{!TOKEN!}}`
/// marker at both opener and closer. U+FDD0 is a permanent Unicode
/// non-character — it never appears in well-formed user text, which is what
/// makes the marker collision-proof. The fold requires marker **and** sentinel
/// together before treating an envelope as an extension.
pub(crate) const ENVELOPE_SENTINEL: char = '\u{FDD0}';

/// Flanking policy applied to a token's source delimiter while pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flanking {
    /// The delimiter always opens or closes when unescaped (used by `mark`'s
    /// `==`, which historically pairs unconditionally).
    Unconstrained,
    /// Emphasis-style flanking based on adjacent characters: a delimiter with
    /// whitespace after cannot open, whitespace before cannot close, and one
    /// flanked by alphanumerics on both sides can do neither (used by `dim`'s
    /// `⌄`, mirroring the legacy span processor's `classify_dim`).
    Emphasis,
}

/// One inline extension token in the central registry.
///
/// For the built-in wrap-style tokens the source delimiter doubles as the
/// Markdown roundtrip delimiter and every payload is nested inline Markdown, so
/// a single [`delimiter`](InlineTokenSpec::delimiter) is all the fold and the
/// Markdown renderer need. Atomic tokens (emoji, var) would extend this shape
/// with a payload-kind discriminator; they are out of scope for the current
/// `mark`/`dim` rollout.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InlineTokenSpec {
    /// Registered identifier emitted into the envelope marker and carried on
    /// [`renderable::tree::NodeKind::Extended`]'s `token` field.
    pub name: &'static str,
    /// The paired source delimiter the rewriter recognises and the Markdown
    /// renderer re-emits around the node's children (mark `==`, dim `⌄`).
    pub delimiter: &'static str,
    /// How the delimiter is classified while pairing.
    pub flanking: Flanking,
}

/// The central inline token registry — the single source of truth shared by the
/// rewriter (here) and the Phase 5 fold dispatcher.
pub(crate) const INLINE_TOKENS: &[InlineTokenSpec] = &[
    InlineTokenSpec {
        name: "mark",
        delimiter: "==",
        flanking: Flanking::Unconstrained,
    },
    InlineTokenSpec {
        name: "dim",
        delimiter: "\u{2304}",
        flanking: Flanking::Emphasis,
    },
];

/// Looks up a registered token by name (used by the fold dispatcher).
pub(crate) fn token_by_name(name: &str) -> Option<&'static InlineTokenSpec> {
    INLINE_TOKENS.iter().find(|t| t.name == name)
}

/// Maps rewritten byte offsets back to original source byte offsets.
///
/// Entries are `(rewritten_offset, original_offset)` sorted ascending by
/// `rewritten_offset`. One entry is recorded at the start of each verbatim
/// region whose alignment diverges from the original stream; between entries
/// the two streams advance in lockstep, so resolution is a binary search plus a
/// linear offset. An empty table means the source was not rewritten and every
/// byte maps 1:1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProvenanceTable {
    entries: Vec<(usize, usize)>,
}

impl ProvenanceTable {
    /// Returns `true` when no rewrite occurred (identity mapping).
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Resolves a rewritten byte offset to its original source byte offset.
    ///
    /// Offsets before the first divergence resolve to themselves. Offsets that
    /// land inside synthetic envelope marker bytes resolve into the original
    /// delimiter region they replaced; offsets inside a copied payload resolve
    /// exactly.
    pub(crate) fn resolve(&self, rewritten_offset: usize) -> usize {
        match self
            .entries
            .binary_search_by(|entry| entry.0.cmp(&rewritten_offset))
        {
            Ok(idx) => self.entries[idx].1,
            Err(0) => rewritten_offset,
            Err(idx) => {
                let (rewritten_base, original_base) = self.entries[idx - 1];
                original_base + (rewritten_offset - rewritten_base)
            }
        }
    }

    /// Resolves both ends of a rewritten byte range to the original source.
    pub(crate) fn resolve_range(&self, range: Range<usize>) -> Range<usize> {
        self.resolve(range.start)..self.resolve(range.end)
    }

    /// The raw entries, exposed for tests.
    #[cfg(test)]
    pub(crate) fn entries(&self) -> &[(usize, usize)] {
        &self.entries
    }
}

/// Result of [`rewrite_inline_extensions`].
///
/// `source` is borrowed unchanged when no extension was found and owned when at
/// least one envelope was emitted. `provenance` is empty in the borrowed case.
#[derive(Debug, Clone)]
pub(crate) struct InlineRewrite<'a> {
    /// The rewritten (or unchanged) source text.
    pub source: Cow<'a, str>,
    /// Translation table from rewritten offsets back to original offsets.
    pub provenance: ProvenanceTable,
}

impl InlineRewrite<'_> {
    /// Whether any envelope was emitted (the source is owned).
    pub(crate) fn was_rewritten(&self) -> bool {
        matches!(self.source, Cow::Owned(_))
    }
}

/// The role a scanned delimiter plays after pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Left verbatim in the output (escaped, unmatched, or an empty pair).
    Literal,
    /// Opening delimiter of a matched pair — replaced by the envelope opener.
    Open,
    /// Closing delimiter of a matched pair — replaced by the envelope closer.
    Close,
}

/// A delimiter occurrence found by the shared scan.
#[derive(Debug, Clone, Copy)]
struct Delim {
    /// Index into [`INLINE_TOKENS`].
    spec_index: usize,
    /// Byte offset of the delimiter in the original source.
    start: usize,
    /// Byte offset (exclusive) of the delimiter in the original source.
    end: usize,
    /// `true` when a backslash immediately precedes the delimiter.
    escaped: bool,
    /// Whether the delimiter may open a pair (after escape + flanking).
    can_open: bool,
    /// Whether the delimiter may close a pair (after escape + flanking).
    can_close: bool,
}

/// Rewrites darkmatter inline extensions in `source` to canonical envelopes.
///
/// Returns the source borrowed unchanged with an empty provenance table when no
/// recognised pair is present, otherwise an owned rewritten string plus the
/// rewritten-to-original offset table.
pub(crate) fn rewrite_inline_extensions(source: &str) -> InlineRewrite<'_> {
    let delimiters = scan_delimiters(source);

    // No candidate delimiter anywhere — the common case for ordinary prose.
    // Return the borrowed source without paying for the protected-region
    // pre-parse below.
    if delimiters.is_empty() {
        return InlineRewrite {
            source: Cow::Borrowed(source),
            provenance: ProvenanceTable::default(),
        };
    }

    // A rewrite can only happen when some token kind has an opener followed by
    // a later closer of the same kind. When no such ordered pair exists here it
    // cannot exist in any subset `protected_ranges` filtering later produces
    // (openness/closedness is a per-delimiter property of the source, so
    // removing delimiters never creates a pair), so the expensive second
    // pulldown-cmark parse below would only rediscover an all-literal result.
    // This is the common case for a lone `==` in `a == b` — ubiquitous in code
    // and technical prose (finding 19).
    if !has_plausible_pair(&delimiters) {
        return InlineRewrite {
            source: Cow::Borrowed(source),
            provenance: ProvenanceTable::default(),
        };
    }

    // Drop any delimiter that falls inside a verbatim region (code, raw HTML,
    // link destination, image) so the rewrite only rephrases eligible prose.
    let protected = protected_ranges(source);
    let delimiters: Vec<Delim> = delimiters
        .into_iter()
        .filter(|delim| !intersects_protected(delim.start..delim.end, &protected))
        .collect();

    let roles = pair_delimiters(&delimiters);

    if roles.iter().all(|role| *role == Role::Literal) {
        return InlineRewrite {
            source: Cow::Borrowed(source),
            provenance: ProvenanceTable::default(),
        };
    }

    let mut out = String::with_capacity(source.len() + 16);
    let mut entries: Vec<(usize, usize)> = Vec::new();
    let mut cursor = 0usize;

    for (delim, role) in delimiters.iter().zip(roles.iter()) {
        match role {
            Role::Literal => continue,
            Role::Open | Role::Close => {
                copy_verbatim(&mut out, &mut entries, source, &mut cursor, delim.start);
                let spec = &INLINE_TOKENS[delim.spec_index];
                match role {
                    Role::Open => push_envelope_opener(&mut out, spec.name),
                    Role::Close => push_envelope_closer(&mut out, spec.name),
                    Role::Literal => unreachable!(),
                }
                cursor = delim.end;
            }
        }
    }
    copy_verbatim(&mut out, &mut entries, source, &mut cursor, source.len());

    InlineRewrite {
        source: Cow::Owned(out),
        provenance: ProvenanceTable { entries },
    }
}

/// Byte ranges in the **original** source that the rewriter must never touch.
///
/// A single `pulldown-cmark` pre-parse (with the same options the fold uses)
/// classifies every region whose bytes are verbatim or structural rather than
/// eligible prose:
///
/// - inline code spans ([`Event::Code`]);
/// - fenced and indented code blocks (whole `CodeBlock` start…end);
/// - raw HTML, block and inline ([`Event::Html`] / [`Event::InlineHtml`] and
///   whole `HtmlBlock` start…end);
/// - image constructs — alt text, destination, and title (whole `Image`
///   start…end), plus autolink / email-autolink URLs whose visible text *is*
///   the URL;
/// - link destinations and titles (`](dest "title")`) — only the tail after the
///   link text, so `[==hi==](url)` still marks its visible link text;
/// - HR-attribute paragraph bodies — a simple single-`Text` paragraph matching
///   `---|***|___ { ... }`. The whole paragraph is a block-extension construct
///   the block processor lifts to a generated rule, so its body must reach that
///   processor unrewritten; otherwise a delimiter-like scalar like
///   `--- { kind: "==waves==" }` would be split into a strikethrough envelope
///   and never recognized as an HR. This mirrors the block processor's
///   simple-paragraph gate on the original source.
///
/// Returned ranges may overlap or nest; [`intersects_protected`] tolerates that.
fn protected_ranges(source: &str) -> Vec<Range<usize>> {
    /// Tracks the currently open paragraph so its body can be protected when it
    /// matches the HR-attribute pattern. Mirrors the block processor's gate:
    /// exactly one buffered event, which is a `Text` matching the pattern.
    struct ParaScan {
        /// Count of events buffered inside the paragraph so far.
        events: usize,
        /// Byte range of the first (and, when matched, only) `Text` event.
        text: Option<Range<usize>>,
    }

    /// A protected construct currently open on the scan stack.
    enum Open {
        /// Protect the whole `[start..end]` construct (code/HTML block, image,
        /// autolink).
        Whole(usize),
        /// A normal link: protect only the `](dest "title")` tail. `inner_end`
        /// tracks the running end of the link-text content.
        Link { inner_end: usize },
    }

    /// Advances the innermost open link's inner-content end past `end`.
    fn bump_link(stack: &mut [Open], end: usize) {
        if let Some(Open::Link { inner_end }) = stack.last_mut() {
            *inner_end = (*inner_end).max(end);
        }
    }

    let options = super::fold::render_tree_parser_options();
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut stack: Vec<Open> = Vec::new();
    let mut para: Option<ParaScan> = None;

    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        // HR-attribute paragraph body detection. Runs alongside the verbatim
        // stack scan in the same pre-parse so no extra pass over the source is
        // added. A bare `---` arrives as `Event::Rule` (never a paragraph), so
        // only attribute paragraphs are caught here.
        match &event {
            Event::Start(Tag::Paragraph) => {
                para = Some(ParaScan {
                    events: 0,
                    text: None,
                });
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(scan) = para.take()
                    && scan.events == 1
                    && let Some(text_range) = scan.text
                    && source
                        .get(text_range.clone())
                        .is_some_and(|t| matches_horizontal_rule_pattern(t).is_some())
                {
                    ranges.push(text_range);
                }
            }
            other => {
                if let Some(scan) = para.as_mut() {
                    if scan.events == 0 && matches!(other, Event::Text(_)) {
                        scan.text = Some(range.clone());
                    }
                    scan.events += 1;
                }
            }
        }

        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock | Tag::Image { .. }) => {
                stack.push(Open::Whole(range.start));
            }
            // An autolink / email autolink renders its URL as the visible text,
            // so the whole construct is verbatim; a normal link only protects
            // its destination tail.
            Event::Start(Tag::Link { link_type, .. }) => {
                if matches!(link_type, LinkType::Autolink | LinkType::Email) {
                    stack.push(Open::Whole(range.start));
                } else {
                    stack.push(Open::Link {
                        inner_end: range.start,
                    });
                }
            }
            Event::Start(_) => bump_link(&mut stack, range.end),
            Event::End(TagEnd::CodeBlock | TagEnd::HtmlBlock | TagEnd::Image | TagEnd::Link) => {
                match stack.pop() {
                    Some(Open::Whole(start)) => ranges.push(start..range.end),
                    Some(Open::Link { inner_end }) => ranges.push(inner_end..range.end),
                    None => {}
                }
                bump_link(&mut stack, range.end);
            }
            Event::End(_) => bump_link(&mut stack, range.end),
            Event::Code(_) | Event::Html(_) | Event::InlineHtml(_) => {
                ranges.push(range.clone());
                bump_link(&mut stack, range.end);
            }
            _ => bump_link(&mut stack, range.end),
        }
    }

    ranges
}

/// Returns `true` when `range` overlaps any protected region (half-open
/// intersection).
fn intersects_protected(range: Range<usize>, protected: &[Range<usize>]) -> bool {
    protected
        .iter()
        .any(|p| range.start < p.end && p.start < range.end)
}

/// Copies `source[*cursor..upto]` into `out`, recording a provenance entry when
/// the rewritten and original streams have diverged.
fn copy_verbatim(
    out: &mut String,
    entries: &mut Vec<(usize, usize)>,
    source: &str,
    cursor: &mut usize,
    upto: usize,
) {
    if upto <= *cursor {
        return;
    }
    let rewritten_offset = out.len();
    if rewritten_offset != *cursor {
        entries.push((rewritten_offset, *cursor));
    }
    out.push_str(&source[*cursor..upto]);
    *cursor = upto;
}

/// Pushes `~~{{!name!}}␦` — the canonical envelope opener.
fn push_envelope_opener(out: &mut String, name: &str) {
    out.push_str("~~{{!");
    out.push_str(name);
    out.push_str("!}}");
    out.push(ENVELOPE_SENTINEL);
}

/// Pushes `{{!name!}}␦~~` — the canonical envelope closer.
fn push_envelope_closer(out: &mut String, name: &str) {
    out.push_str("{{!");
    out.push_str(name);
    out.push_str("!}}");
    out.push(ENVELOPE_SENTINEL);
    out.push_str("~~");
}

/// The single shared scan over `source`.
///
/// Walks the source once, recognising every registered delimiter at each char
/// boundary. Adding a token extends [`INLINE_TOKENS`]; it must never add a
/// second pass (see the module's *One shared scan* rule).
fn scan_delimiters(source: &str) -> Vec<Delim> {
    let bytes = source.as_bytes();
    let mut delimiters = Vec::new();
    let mut index = 0usize;

    while index < source.len() {
        let mut matched = false;
        for (spec_index, spec) in INLINE_TOKENS.iter().enumerate() {
            if source[index..].starts_with(spec.delimiter) {
                let start = index;
                let end = index + spec.delimiter.len();
                let escaped = start > 0 && bytes[start - 1] == b'\\';
                let (can_open, can_close) = if escaped {
                    (false, false)
                } else {
                    match spec.flanking {
                        Flanking::Unconstrained => (true, true),
                        Flanking::Emphasis => classify_flanking(source, start, spec.delimiter.len()),
                    }
                };
                delimiters.push(Delim {
                    spec_index,
                    start,
                    end,
                    escaped,
                    can_open,
                    can_close,
                });
                index = end;
                matched = true;
                break;
            }
        }
        if !matched {
            index += next_char_len(source, index);
        }
    }

    delimiters
}

/// Byte length of the UTF-8 char starting at `index` in `source`.
fn next_char_len(source: &str, index: usize) -> usize {
    source[index..].chars().next().map_or(1, char::len_utf8)
}

/// Emphasis-style flanking classification, mirroring the legacy span
/// processor's `classify_dim`: a text-event boundary (`None`) is treated as
/// non-whitespace, whitespace after blocks opening, whitespace before blocks
/// closing, and an alphanumeric on both sides blocks both.
fn classify_flanking(text: &str, byte_pos: usize, len: usize) -> (bool, bool) {
    let prev_char = text[..byte_pos].chars().next_back();
    let next_char = text[byte_pos + len..].chars().next();

    let can_open = next_char.is_none_or(|c| !c.is_whitespace());
    let can_close = prev_char.is_none_or(|c| !c.is_whitespace());

    let prev_is_alnum = prev_char.is_some_and(char::is_alphanumeric);
    let next_is_alnum = next_char.is_some_and(char::is_alphanumeric);
    if prev_is_alnum && next_is_alnum {
        return (false, false);
    }
    (can_open, can_close)
}

/// Whether the scanned delimiters could possibly form at least one pair.
///
/// A pair requires an opener delimiter followed by a later closer delimiter of
/// the *same* token kind. Because each delimiter's open/close capability is a
/// property of the surrounding source (never of the other delimiters), a `false`
/// result here holds for every subset of these delimiters too — so the caller
/// can skip the [`protected_ranges`] pre-parse and return the source borrowed
/// unchanged. Escaped delimiters carry `can_open == can_close == false`, so they
/// neither open nor satisfy this check.
fn has_plausible_pair(delimiters: &[Delim]) -> bool {
    let mut seen_open = vec![false; INLINE_TOKENS.len()];
    for delim in delimiters {
        if delim.can_close && seen_open[delim.spec_index] {
            return true;
        }
        if delim.can_open {
            seen_open[delim.spec_index] = true;
        }
    }
    false
}

/// Assigns each scanned delimiter a [`Role`] by pairing greedily, left to
/// right, with an independent stack per token kind.
///
/// Escaped delimiters and unmatched openers stay [`Role::Literal`]. An empty
/// pair (closer immediately following its opener) collapses both ends to
/// literal text rather than emitting an empty envelope, matching the legacy
/// span processor.
fn pair_delimiters(delimiters: &[Delim]) -> Vec<Role> {
    let mut roles = vec![Role::Literal; delimiters.len()];
    let mut open_stacks: Vec<Vec<usize>> = vec![Vec::new(); INLINE_TOKENS.len()];

    for (i, delim) in delimiters.iter().enumerate() {
        if delim.escaped {
            continue;
        }
        let stack = &mut open_stacks[delim.spec_index];

        if delim.can_close && let Some(&opener) = stack.last() {
            // Empty pair: revert both to literal instead of an empty envelope.
            if delimiters[opener].end == delim.start {
                stack.pop();
                roles[opener] = Role::Literal;
                roles[i] = Role::Literal;
                continue;
            }
            stack.pop();
            roles[opener] = Role::Open;
            roles[i] = Role::Close;
            continue;
        }

        if delim.can_open {
            stack.push(i);
        }
        // Unmatched openers remain `Literal` (their tentative role) once the
        // scan ends — nothing to do here.
    }

    roles
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders a token's literal marker, e.g. `{{!mark!}}`.
    fn marker(token: &str) -> String {
        format!("{{{{!{token}!}}}}")
    }

    #[test]
    fn no_extension_document_is_borrowed_unchanged() {
        let source = "plain text with no darkmatter inline syntax";
        let result = rewrite_inline_extensions(source);
        assert!(!result.was_rewritten(), "no-extension input must not rewrite");
        assert_eq!(result.source, source);
        assert!(result.provenance.is_empty());
    }

    #[test]
    fn lone_unpaired_delimiter_is_borrowed_unchanged() {
        // `a == b` is a single `==` — no closer follows, so no pair is
        // possible. The result must be the borrowed source (finding 19: this
        // path must skip the second pulldown-cmark parse).
        for source in ["a == b", "value ⌄ 3", "trailing =="] {
            let result = rewrite_inline_extensions(source);
            assert!(
                !result.was_rewritten(),
                "unpaired/odd delimiters must not rewrite: {source:?}"
            );
            assert_eq!(result.source, source);
            assert!(result.provenance.is_empty());
        }
    }

    #[test]
    fn plausible_pair_detection_matches_pairing_outcome() {
        // The pre-parse skip must never suppress a real rewrite: whenever a
        // document does rewrite, `has_plausible_pair` must have returned true.
        for source in ["==hi==", "a ==b== c", "\u{2304}d\u{2304}"] {
            let delimiters = scan_delimiters(source);
            assert!(
                has_plausible_pair(&delimiters),
                "a rewriting document must report a plausible pair: {source:?}"
            );
            assert!(rewrite_inline_extensions(source).was_rewritten());
        }
    }

    #[test]
    fn mark_pair_emits_canonical_envelope() {
        let result = rewrite_inline_extensions("==highlighted==");
        assert!(result.was_rewritten());
        let expected = format!(
            "~~{m}{s}highlighted{m}{s}~~",
            m = marker("mark"),
            s = ENVELOPE_SENTINEL
        );
        assert_eq!(result.source, expected);
    }

    #[test]
    fn dim_pair_emits_canonical_envelope() {
        let result = rewrite_inline_extensions("\u{2304}dim text\u{2304}");
        assert!(result.was_rewritten());
        let expected = format!(
            "~~{m}{s}dim text{m}{s}~~",
            m = marker("dim"),
            s = ENVELOPE_SENTINEL
        );
        assert_eq!(result.source, expected);
    }

    #[test]
    fn every_envelope_includes_both_markers_and_sentinels() {
        for (source, token) in [("==x==", "mark"), ("\u{2304}y\u{2304}", "dim")] {
            let result = rewrite_inline_extensions(source);
            let rewritten = result.source.as_ref();
            assert_eq!(
                rewritten.matches(&marker(token)).count(),
                2,
                "both markers must appear: {rewritten:?}"
            );
            assert_eq!(
                rewritten.matches(ENVELOPE_SENTINEL).count(),
                2,
                "both U+FDD0 sentinels must appear: {rewritten:?}"
            );
        }
    }

    #[test]
    fn nested_markdown_payload_is_preserved_verbatim() {
        // The rewriter copies the payload byte-for-byte; pulldown-cmark parses
        // the inner emphasis after the envelope is recognised.
        let result = rewrite_inline_extensions("==*emphasized*==");
        let rewritten = result.source.as_ref();
        assert!(
            rewritten.contains("*emphasized*"),
            "inline Markdown payload must survive verbatim: {rewritten:?}"
        );
    }

    #[test]
    fn escaped_mark_delimiter_is_not_rewritten() {
        let source = "foo \\== bar";
        let result = rewrite_inline_extensions(source);
        assert!(!result.was_rewritten(), "escaped `\\==` must stay literal");
        assert_eq!(result.source, source);
    }

    #[test]
    fn escaped_dim_delimiter_is_not_rewritten() {
        let source = "foo \\\u{2304} bar";
        let result = rewrite_inline_extensions(source);
        assert!(!result.was_rewritten(), "escaped `\\⌄` must stay literal");
        assert_eq!(result.source, source);
    }

    #[test]
    fn unclosed_delimiter_stays_literal() {
        let result = rewrite_inline_extensions("this ==is not closed");
        assert!(!result.was_rewritten(), "an unclosed `==` must not rewrite");
    }

    #[test]
    fn empty_pair_collapses_to_literal() {
        let result = rewrite_inline_extensions("====");
        assert!(!result.was_rewritten(), "an empty `====` pair must not rewrite");
    }

    #[test]
    fn three_openers_pair_greedily_leaving_orphan_literal() {
        // `==a==b==` pairs the first two `==`; the third is an orphan literal.
        let result = rewrite_inline_extensions("==a==b==");
        let rewritten = result.source.as_ref();
        assert_eq!(
            rewritten.matches(&marker("mark")).count(),
            2,
            "exactly one envelope (two markers): {rewritten:?}"
        );
        assert!(
            rewritten.ends_with("b=="),
            "the orphan `==` must remain literal: {rewritten:?}"
        );
    }

    #[test]
    fn adjacent_spans_emit_two_envelopes() {
        let result = rewrite_inline_extensions("==a== ==b==");
        let rewritten = result.source.as_ref();
        assert_eq!(
            rewritten.matches(&marker("mark")).count(),
            4,
            "two adjacent envelopes carry four markers: {rewritten:?}"
        );
        assert_eq!(rewritten.matches(ENVELOPE_SENTINEL).count(), 4);
    }

    #[test]
    fn utf8_payload_offsets_resolve_to_original_bytes() {
        // `é` and `☃` are multi-byte; the payload must round-trip and its
        // resolved offsets must point back at the original payload bytes.
        let source = "==café ☃==";
        let result = rewrite_inline_extensions(source);
        let rewritten = result.source.as_ref();
        let payload = "café ☃";

        let payload_start = rewritten.find(payload).expect("payload must survive");
        let resolved = result
            .provenance
            .resolve_range(payload_start..payload_start + payload.len());
        assert_eq!(
            &source[resolved],
            payload,
            "resolved range must point at the original payload bytes"
        );
    }

    #[test]
    fn provenance_resolves_payload_to_original_offsets() {
        // `==highlighted==`: payload `highlighted` sits at original bytes 2..13.
        let source = "==highlighted==";
        let result = rewrite_inline_extensions(source);
        let rewritten = result.source.as_ref();

        let start = rewritten.find("highlighted").expect("payload present");
        let end = start + "highlighted".len();
        assert_eq!(result.provenance.resolve(start), 2);
        assert_eq!(result.provenance.resolve(end), 13);
        assert_eq!(&source[result.provenance.resolve(start)..result.provenance.resolve(end)], "highlighted");
    }

    #[test]
    fn provenance_is_sorted_by_rewritten_offset() {
        let result = rewrite_inline_extensions("a ==one== b ==two== c");
        let entries = result.provenance.entries();
        assert!(
            entries.windows(2).all(|w| w[0].0 < w[1].0),
            "entries must be strictly ascending by rewritten offset: {entries:?}"
        );
    }

    #[test]
    fn token_lookup_finds_built_ins_and_rejects_unknown() {
        assert_eq!(token_by_name("mark").map(|t| t.delimiter), Some("=="));
        assert_eq!(token_by_name("dim").map(|t| t.delimiter), Some("\u{2304}"));
        assert!(token_by_name("emoji").is_none());
    }

    // -----------------------------------------------------------------------
    // Table safety (review-1 finding 2): the envelope marker must carry no `|`
    // bytes, so an envelope inside a GFM table cell cannot be miscounted as an
    // extra column separator.
    // -----------------------------------------------------------------------

    #[test]
    fn envelope_marker_contains_no_pipe_bytes() {
        let result = rewrite_inline_extensions("==hi==");
        assert!(
            !result.source.contains('|'),
            "envelope must be pipe-free for GFM table safety: {:?}",
            result.source
        );
    }

    #[test]
    fn mark_inside_table_cell_rewrites_without_injecting_pipes() {
        // A mark span living in a table cell must rewrite to an envelope whose
        // pipe count matches the original row's, so the table parser still sees
        // the same cell boundaries.
        let source = "| ==hi== | ok |\n";
        let result = rewrite_inline_extensions(source);
        assert!(result.was_rewritten(), "mark in a cell must rewrite");
        assert_eq!(
            result.source.matches('|').count(),
            source.matches('|').count(),
            "rewrite must not change the cell-separator pipe count: {:?}",
            result.source
        );
    }

    // -----------------------------------------------------------------------
    // Verbatim-region protection (review-1 finding 1): delimiters inside code,
    // raw HTML, link destinations, and image constructs must stay literal.
    // -----------------------------------------------------------------------

    #[test]
    fn delimiters_inside_inline_code_are_not_rewritten() {
        let result = rewrite_inline_extensions("a `==code==` b");
        assert!(
            !result.was_rewritten(),
            "`==` inside an inline code span must stay literal: {:?}",
            result.source
        );
    }

    #[test]
    fn delimiters_inside_fenced_code_block_are_not_rewritten() {
        let result = rewrite_inline_extensions("```\n==code==\n\u{2304}dim\u{2304}\n```\n");
        assert!(
            !result.was_rewritten(),
            "delimiters inside a fenced code block must stay literal: {:?}",
            result.source
        );
    }

    #[test]
    fn delimiters_inside_indented_code_block_are_not_rewritten() {
        let result = rewrite_inline_extensions("    ==code==\n");
        assert!(
            !result.was_rewritten(),
            "delimiters inside an indented code block must stay literal: {:?}",
            result.source
        );
    }

    #[test]
    fn delimiters_inside_raw_html_are_not_rewritten() {
        let result = rewrite_inline_extensions("<div>==x==</div>\n");
        assert!(
            !result.was_rewritten(),
            "delimiters inside raw HTML must stay literal: {:?}",
            result.source
        );
    }

    #[test]
    fn delimiters_inside_link_destination_are_not_rewritten() {
        let result = rewrite_inline_extensions("[text](http://a==b==c)");
        assert!(
            !result.was_rewritten(),
            "delimiters inside a link destination must stay literal: {:?}",
            result.source
        );
    }

    #[test]
    fn mark_inside_link_text_still_rewrites() {
        // The destination is protected, but visible link text is eligible prose.
        let result = rewrite_inline_extensions("[==hi==](http://example.com)");
        assert!(
            result.was_rewritten(),
            "a mark span in link text must still rewrite: {:?}",
            result.source
        );
        assert!(
            result.source.contains("http://example.com"),
            "the destination must survive verbatim: {:?}",
            result.source
        );
    }

    #[test]
    fn delimiters_inside_image_are_not_rewritten() {
        let result = rewrite_inline_extensions("![==alt==](img.png)");
        assert!(
            !result.was_rewritten(),
            "delimiters in image alt/destination must stay literal: {:?}",
            result.source
        );
    }

    #[test]
    fn opener_in_prose_closer_in_code_does_not_pair_across_code() {
        // The opening `==` is eligible prose; the closing `==` hides inside an
        // inline code span. They must not pair into an envelope spanning the
        // code — the opener stays a literal `==`.
        let result = rewrite_inline_extensions("==open `code ==` rest");
        assert!(
            !result.was_rewritten(),
            "a delimiter must not pair with one inside a code span: {:?}",
            result.source
        );
    }

    #[test]
    fn prose_mark_still_rewrites_when_a_code_span_is_present() {
        // Protection must be surgical: a real mark in prose still rewrites even
        // when an unrelated code span sits in the same document.
        let result = rewrite_inline_extensions("see `code` then ==hi== done");
        assert!(result.was_rewritten());
        assert!(result.source.contains("`code`"), "code span must survive");
        assert_eq!(result.source.matches(&marker("mark")).count(), 2);
    }

    // -----------------------------------------------------------------------
    // HR-attribute paragraph protection (review-4): a delimiter-like scalar
    // inside an HR-attribute paragraph belongs to the block-extension construct
    // and must not be rewritten, so the block processor still sees one Text
    // event and lifts the paragraph to a generated rule.
    // -----------------------------------------------------------------------

    #[test]
    fn delimiters_inside_hr_attribute_body_are_not_rewritten() {
        for source in [
            "--- { kind: \"==waves==\" }\n",
            "*** { kind: \"\u{2304}dots\u{2304}\" }\n",
        ] {
            let result = rewrite_inline_extensions(source);
            assert!(
                !result.was_rewritten(),
                "delimiters in an HR-attribute body must stay literal: {:?}",
                result.source
            );
        }
    }

    #[test]
    fn prose_mark_after_hr_attribute_paragraph_still_rewrites() {
        // Protection is scoped to the HR-attribute paragraph body only — a real
        // mark in a following prose paragraph must still rewrite.
        let result = rewrite_inline_extensions("--- { kind: \"==x==\" }\n\nthen ==hi== done\n");
        assert!(result.was_rewritten(), "prose mark must still rewrite");
        assert_eq!(
            result.source.matches(&marker("mark")).count(),
            2,
            "exactly the prose mark rewrites, not the HR body: {:?}",
            result.source
        );
    }
}
