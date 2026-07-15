//! Canonical semantic-token model and encoder (spec `DMLS Semantic Tokens`).
//!
//! This is the single owner of the sorted, non-overlapping, legend-encoded
//! token stream. It is deliberately a **standalone** module rather than a
//! registry capability (the Phase-10 precedent for single-correct-answer
//! providers): per-provider union merging would push the overlap invariant
//! into the merge policy, but semantic tokens have exactly one right answer.
//!
//! The module is split into three concerns that this file builds bottom-up
//! (the **frozen V1 legend** itself lives in the [`crate::semantic_legend`]
//! leaf so [`crate::capabilities`] can advertise it without importing a
//! provider):
//!
//! - the canonical [`RawToken`] a family emitter produces, carrying a byte span
//!   plus the priority a family/structural role gives it,
//! - the three V1 family emitters ([`family_tokens`]) — F1 body interpolations,
//!   F2 directive lines, and F4 wiki links — each a pure
//!   `(text, body_base) → Vec<RawToken>` read over the existing passive scanner
//!   surfaces, and
//! - the deterministic pipeline ([`resolve_and_encode`]): family precedence and
//!   clipping, half-open range intersection, per-line splitting through the
//!   [`SourceMap`], byte→position conversion, sort/dedup, an overlap assertion,
//!   and LSP relative-delta encoding.
//!
//! The request handlers land in a later phase; this file owns the model,
//! emitters, and encoder so precedence, clipping, ordering, and encoding are
//! testable in isolation of LSP routing.

use darkmatter::markdown::compose::directives_api::{DirectiveKind, scan_darkmatter_directives};
use darkmatter::markdown::render_tree::disclosure_scan::scan_disclosures;
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{Range, SemanticToken, SemanticTokens};

use super::DocumentContext;
use crate::overlay::expressions;
use crate::semantic_legend::{TokenType, modifier};
use crate::source_map::SourceMap;
use crate::wiki::scan_wiki_links;

/// Family a raw token belongs to, ordered by precedence.
///
/// A higher-priority family **owns** every byte it covers; a lower-priority
/// family's tokens are clipped around that ownership. The `Ord` derive follows
/// the declaration order, so `Interpolation > Wiki > Directive` — the exact
/// F1 > F4 > F2 rule from the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    /// F2 directive lines (lowest priority).
    Directive,
    /// F4 wiki links.
    Wiki,
    /// F1 body interpolations (highest priority).
    Interpolation,
}

/// One classified byte span before precedence resolution and encoding.
///
/// Family emitters (Phase 3) each return a `Vec<RawToken>` and never touch the
/// LSP relative encoding — that stays the encoder's sole responsibility. The
/// `structural` rank breaks ties *within* a family so a structural subtoken (a
/// wiki separator, say) wins over the broader text token it sits inside;
/// `provenance` is a static label that makes a failed overlap/order assertion
/// point at the emitter that produced the offending span.
#[derive(Debug, Clone)]
pub struct RawToken {
    /// Byte span into the document text.
    pub span: SourceSpan,
    /// Standard token type.
    pub token_type: TokenType,
    /// Modifier bitset (see [`modifier`]).
    pub modifiers: u32,
    /// Family precedence bucket.
    pub family: Family,
    /// Structural rank within the family; higher wins over broader tokens.
    pub structural: u8,
    /// Emitter label, surfaced only in assertion messages.
    pub provenance: &'static str,
}

impl RawToken {
    /// Combined precedence: family first, then structural rank. Higher owns
    /// bytes and clips everything below it.
    fn priority(&self) -> (Family, u8) {
        (self.family, self.structural)
    }
}

/// One token after conversion to absolute LSP coordinates (single line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AbsToken {
    line: u32,
    character: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

impl AbsToken {
    /// Sort key mandated by the spec: `(line, character, length, type, modifiers)`.
    fn sort_key(&self) -> (u32, u32, u32, u32, u32) {
        (
            self.line,
            self.character,
            self.length,
            self.token_type,
            self.modifiers,
        )
    }
}

/// Resolves family precedence, clips to `range`, splits per line, and encodes
/// the canonical LSP relative-delta stream for `raw`.
///
/// `range` is an optional half-open byte range (a `textDocument/…/range`
/// request); `None` encodes the whole document. The result is always sorted by
/// `(line, character, …)`, free of exact duplicates, and non-overlapping — a
/// `debug_assert` guards the invariant and any residual overlap is dropped
/// defensively in release builds so the wire stream is never invalid.
pub fn resolve_and_encode(
    raw: Vec<RawToken>,
    source_map: &SourceMap,
    range: Option<SourceSpan>,
) -> SemanticTokens {
    let resolved = resolve_precedence(raw);
    let clipped = match range {
        Some(range) => intersect_range(resolved, &range),
        None => resolved,
    };
    let absolute = to_absolute(clipped, source_map);
    encode(absolute)
}

/// Applies family/structural precedence: process highest-priority tokens first
/// and clip every lower token around the bytes already owned.
///
/// Equal-priority tokens are assumed disjoint (distinct constructs never share
/// bytes at the same rank); the final overlap assertion in [`encode`] is the
/// safety net if an emitter ever violates that.
fn resolve_precedence(mut raw: Vec<RawToken>) -> Vec<ResolvedToken> {
    // Highest priority first; break ties by span start for determinism.
    raw.sort_by(|a, b| {
        b.priority()
            .cmp(&a.priority())
            .then_with(|| a.span.start.cmp(&b.span.start))
            .then_with(|| a.span.end.cmp(&b.span.end))
    });

    let mut occupied: Vec<SourceSpan> = Vec::new();
    let mut out: Vec<ResolvedToken> = Vec::new();
    for token in raw {
        for fragment in subtract(token.span.clone(), &occupied) {
            occupied.push(fragment.clone());
            out.push(ResolvedToken {
                span: fragment,
                token_type: token.token_type,
                modifiers: token.modifiers,
            });
        }
    }
    out
}

/// A token whose ownership is settled but that still carries a byte span.
#[derive(Debug, Clone)]
struct ResolvedToken {
    span: SourceSpan,
    token_type: TokenType,
    modifiers: u32,
}

/// Subtracts every interval in `occupied` from `span`, returning the surviving
/// non-empty fragments in ascending order.
fn subtract(span: SourceSpan, occupied: &[SourceSpan]) -> Vec<SourceSpan> {
    let mut fragments = vec![span];
    for occ in occupied {
        let mut next = Vec::with_capacity(fragments.len());
        for frag in fragments {
            if occ.end <= frag.start || occ.start >= frag.end {
                next.push(frag); // disjoint
                continue;
            }
            if frag.start < occ.start {
                next.push(frag.start..occ.start);
            }
            if occ.end < frag.end {
                next.push(occ.end..frag.end);
            }
            // The overlapping middle is removed.
        }
        fragments = next;
    }
    fragments
}

/// Intersects each token span with the half-open byte range `[range.start,
/// range.end)`, discarding tokens (and fragments) that fall entirely outside.
fn intersect_range(tokens: Vec<ResolvedToken>, range: &SourceSpan) -> Vec<ResolvedToken> {
    tokens
        .into_iter()
        .filter_map(|token| {
            let start = token.span.start.max(range.start);
            let end = token.span.end.min(range.end);
            (start < end).then_some(ResolvedToken {
                span: start..end,
                token_type: token.token_type,
                modifiers: token.modifiers,
            })
        })
        .collect()
}

/// Splits every span per line through the [`SourceMap`] and converts each
/// resulting single-line fragment to absolute LSP coordinates. A fragment whose
/// endpoints are not on codepoint boundaries is dropped rather than panicking.
fn to_absolute(tokens: Vec<ResolvedToken>, source_map: &SourceMap) -> Vec<AbsToken> {
    let mut out = Vec::new();
    for token in tokens {
        for line_span in source_map.split_span_by_line(token.span.clone()) {
            let (Some(start), Some(end)) = (
                source_map.byte_to_lsp(line_span.start),
                source_map.byte_to_lsp(line_span.end),
            ) else {
                continue;
            };
            // split_span_by_line keeps a fragment on one line, so both
            // positions share a line and the encoding-unit length is the
            // character delta.
            if end.line != start.line || end.character < start.character {
                continue;
            }
            out.push(AbsToken {
                line: start.line,
                character: start.character,
                length: end.character - start.character,
                token_type: token.token_type.index(),
                modifiers: token.modifiers,
            });
        }
    }
    out
}

/// Sorts, deduplicates, asserts non-overlap, and encodes the LSP relative
/// deltas.
fn encode(mut tokens: Vec<AbsToken>) -> SemanticTokens {
    tokens.sort_by_key(AbsToken::sort_key);
    tokens.dedup();

    let mut data = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;
    // Last accepted token's exclusive end column, per line, for the overlap
    // guard.
    let mut last_line = u32::MAX;
    let mut last_end = 0u32;
    for token in tokens {
        if token.line == last_line && token.character < last_end {
            debug_assert!(
                false,
                "overlapping semantic tokens at line {} char {} (prev end {last_end})",
                token.line, token.character
            );
            continue;
        }
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.character - prev_char
        } else {
            token.character
        };
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: token.modifiers,
        });
        prev_line = token.line;
        prev_char = token.character;
        last_line = token.line;
        last_end = token.character + token.length;
    }

    SemanticTokens {
        result_id: None,
        data,
    }
}

// ----------------------------------------------------------------------------
// Family emitters (Phase 3)
//
// Each family is a pure read over an existing passive scanner surface and
// returns raw byte-span tokens; nothing here encodes, clips, or orders — that
// is the pipeline's job. Fenced-code exclusion and malformed-construct
// suppression already live in the scanners, so no emitter re-filters them.
// ----------------------------------------------------------------------------

/// The three V1 families combined into one raw stream, ready for
/// [`resolve_and_encode`].
///
/// `body_base` is the document byte offset where the body begins (after any
/// frontmatter block); F1 and F4 are body-scoped, and F2 directives are scanned
/// over the same body slice so a `::`-looking frontmatter line is never
/// tokenized.
pub fn family_tokens(text: &str, body_base: usize) -> Vec<RawToken> {
    let mut out = interpolation_tokens(text, body_base);
    out.extend(directive_tokens(text, body_base));
    out.extend(wiki_tokens(text, body_base));
    out
}

/// F1: one whole-outer-span token per body `{{ … }}` interpolation
/// (`macro.interpolation`) and per `{{{ … }}}` literal
/// (`macro.interpolation.inert`).
///
/// Interpolation and literal regions are mutually exclusive in the scanner, so
/// the two passes never overlap.
fn interpolation_tokens(text: &str, body_base: usize) -> Vec<RawToken> {
    let mut out = Vec::new();
    for interpolation in expressions::interpolations(text, body_base) {
        out.push(RawToken {
            span: interpolation.outer,
            token_type: TokenType::Macro,
            modifiers: modifier::INTERPOLATION,
            family: Family::Interpolation,
            structural: 0,
            provenance: "f1.interpolation",
        });
    }
    for literal in expressions::literals(text, body_base) {
        out.push(RawToken {
            span: literal.outer,
            token_type: TokenType::Macro,
            modifiers: modifier::INTERPOLATION | modifier::INERT,
            family: Family::Interpolation,
            structural: 0,
            provenance: "f1.literal",
        });
    }
    out
}

/// F2: directive keywords (`macro.directive`, plus `closer` on the three
/// structural closers), structured targets (`string.directive`), and option
/// keys/values (`property.directive` / `string.directive`).
///
/// Free-form payloads are excluded: `::shell` command text is deferred to F6
/// and a `::disclosure` summary is prose. Disclosure openers instead have their
/// recognized inline style tokens split at the source `=` into a
/// `property.directive` key and a `string.directive` value.
fn directive_tokens(text: &str, body_base: usize) -> Vec<RawToken> {
    let body = &text[body_base..];
    let mut out = Vec::new();

    for directive in scan_darkmatter_directives(body) {
        let mut keyword_modifiers = modifier::DIRECTIVE;
        if is_closer(directive.kind) {
            keyword_modifiers |= modifier::CLOSER;
        }
        out.push(directive_raw(
            shift(directive.keyword_span, body_base),
            TokenType::Macro,
            keyword_modifiers,
            "f2.keyword",
        ));

        if has_structured_target(directive.kind)
            && let Some(target) = directive.target
        {
            out.push(directive_raw(
                shift(target.span, body_base),
                TokenType::Str,
                modifier::DIRECTIVE,
                "f2.target",
            ));
        }

        for option in directive.options {
            out.push(directive_raw(
                shift(option.key.span, body_base),
                TokenType::Property,
                modifier::DIRECTIVE,
                "f2.option-key",
            ));
            if let Some(value) = option.value {
                out.push(directive_raw(
                    shift(value.span, body_base),
                    TokenType::Str,
                    modifier::DIRECTIVE,
                    "f2.option-value",
                ));
            }
        }
    }

    // Disclosure openers carry inline `key=value` style tokens the directive
    // scanner folds into the free-form summary; split each at its source `=`.
    // A malformed disclosure yields no style tokens (the keyword is still
    // emitted above), so the scan error is intentionally ignored.
    if let Ok(disclosures) = scan_disclosures(body) {
        for disclosure in disclosures {
            for token_span in disclosure.style_token_spans {
                push_style_token(body, token_span, body_base, &mut out);
            }
        }
    }

    out
}

/// Splits one disclosure style token (`max-width=60ch`) at its source `=` into
/// a `property.directive` key and a `string.directive` value.
///
/// `span` is a body-relative byte range; both halves are shifted into document
/// coordinates. A token with no `=` (defensive — the scanner only recognizes
/// `key=value` tokens) is emitted whole as the key.
fn push_style_token(body: &str, span: SourceSpan, body_base: usize, out: &mut Vec<RawToken>) {
    match body[span.clone()].find('=') {
        Some(offset) => {
            let key = span.start..span.start + offset;
            out.push(directive_raw(
                shift(key, body_base),
                TokenType::Property,
                modifier::DIRECTIVE,
                "f2.disclosure-style-key",
            ));
            let value = span.start + offset + 1..span.end;
            if !value.is_empty() {
                out.push(directive_raw(
                    shift(value, body_base),
                    TokenType::Str,
                    modifier::DIRECTIVE,
                    "f2.disclosure-style-value",
                ));
            }
        }
        None => out.push(directive_raw(
            shift(span, body_base),
            TokenType::Property,
            modifier::DIRECTIVE,
            "f2.disclosure-style-key",
        )),
    }
}

/// A directive-family raw token. All F2 tokens are structurally disjoint, so
/// they share the same structural rank.
fn directive_raw(
    span: SourceSpan,
    token_type: TokenType,
    modifiers: u32,
    provenance: &'static str,
) -> RawToken {
    RawToken {
        span,
        token_type,
        modifiers,
        family: Family::Directive,
        structural: 0,
        provenance,
    }
}

/// Whether a directive family carries a structured (tokenizable) target. The
/// free-form `::shell` command (F6) and `::disclosure` summary (prose) are
/// excluded; the remaining families with no target never reach this branch.
fn has_structured_target(kind: DirectiveKind) -> bool {
    matches!(
        kind,
        DirectiveKind::File
            | DirectiveKind::Code
            | DirectiveKind::Url
            | DirectiveKind::FileLinks
            | DirectiveKind::TocLinking
    )
}

/// Whether a directive is one of the three structural closers that additionally
/// carry the `closer` modifier.
fn is_closer(kind: DirectiveKind) -> bool {
    matches!(
        kind,
        DirectiveKind::EndBlock | DirectiveKind::Details | DirectiveKind::EndDisclosure
    )
}

/// F4: one broad `macro.wiki` frame per `[[wiki]]` link with the non-empty
/// path/heading/alias segments as higher-ranked `string.wiki` subtokens.
///
/// Precedence resolution clips the frame around the segments, so the brackets
/// and the `#` / `|` separators survive as `macro.wiki` and the inner text as
/// `string.wiki` — no span is reconstructed from an unescaped value, and empty
/// segments contribute no zero-length token. Resolved, unresolved, and
/// scanner-recognized unsupported forms all produce the same token shapes.
fn wiki_tokens(text: &str, body_base: usize) -> Vec<RawToken> {
    let body = &text[body_base..];
    let mut out = Vec::new();
    for link in scan_wiki_links(body) {
        out.push(RawToken {
            span: shift(link.span, body_base),
            token_type: TokenType::Macro,
            modifiers: modifier::WIKI,
            family: Family::Wiki,
            structural: 0,
            provenance: "f4.frame",
        });
        for segment in [link.target_span, link.heading_span.unwrap_or(0..0), link.alias_span.unwrap_or(0..0)] {
            if segment.is_empty() {
                continue;
            }
            out.push(RawToken {
                span: shift(segment, body_base),
                token_type: TokenType::Str,
                modifiers: modifier::WIKI,
                family: Family::Wiki,
                structural: 1,
                provenance: "f4.segment",
            });
        }
    }
    out
}

/// Shifts a body-relative span into document coordinates.
fn shift(span: SourceSpan, body_base: usize) -> SourceSpan {
    body_base + span.start..body_base + span.end
}

// ----------------------------------------------------------------------------
// Request handlers (Phase 4)
//
// The standalone provider entry points. Semantic tokens have exactly one right
// answer, so they are not registry-merged; the router calls these directly with
// a `DocumentContext` built from the open-buffer snapshot. Both handlers honor
// the `semantic_tokens.enable` master switch and suppress F4 when `wiki.enable`
// is false.
// ----------------------------------------------------------------------------

/// `textDocument/semanticTokens/full`: the canonical whole-document stream.
pub fn semantic_tokens_full(ctx: &DocumentContext) -> SemanticTokens {
    encode_document(ctx, None)
}

/// `textDocument/semanticTokens/range`: the canonical stream intersected with
/// `range`.
///
/// The LSP range is mapped to a half-open byte span through the document's
/// [`SourceMap`]; an endpoint that cannot be mapped clamps to the document
/// bounds (start → 0, end → EOF), so a malformed range degrades to the widest
/// safe intersection rather than an error.
pub fn semantic_tokens_range(ctx: &DocumentContext, range: Range) -> SemanticTokens {
    let start = ctx.source_map.lsp_to_byte(range.start).unwrap_or(0);
    let end = ctx
        .source_map
        .lsp_to_byte(range.end)
        .unwrap_or(ctx.text.len());
    encode_document(ctx, Some(start..end))
}

/// Builds the raw family stream (gated by config) and encodes it, optionally
/// clipped to `range`.
fn encode_document(ctx: &DocumentContext, range: Option<SourceSpan>) -> SemanticTokens {
    if !ctx.config.semantic_tokens.enable {
        // Master switch off: an empty stream clears the client's tokens.
        return SemanticTokens {
            result_id: None,
            data: Vec::new(),
        };
    }
    let body_base = crate::providers::dsl::body_base(ctx.text);
    let mut raw = interpolation_tokens(ctx.text, body_base);
    raw.extend(directive_tokens(ctx.text, body_base));
    // F4 is the only family gated by `wiki.enable`; F1/F2 are unaffected.
    if ctx.config.wiki.enable {
        raw.extend(wiki_tokens(ctx.text, body_base));
    }
    resolve_and_encode(raw, ctx.source_map, range)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::sync::Arc;

    use lsp_types::Uri;

    use super::*;
    use crate::capabilities::{ClientProfile, HoverMediaProfile};
    use crate::config::DmlsConfig;
    use crate::graph::WorkspaceGraph;
    use crate::semantic_legend::legend;
    use crate::source_map::PositionEncoding;

    fn source_map(text: &str, encoding: PositionEncoding) -> SourceMap {
        let uri: Uri = "file:///t.md".parse().unwrap();
        SourceMap::new(uri, 1, encoding, Arc::from(text))
    }

    /// A minimal capability-empty client profile for handler tests.
    fn test_profile() -> ClientProfile {
        ClientProfile {
            client_name: None,
            client_version: None,
            position_encoding: PositionEncoding::Utf16,
            supports_resource_operations: false,
            supports_change_annotations: false,
            supports_code_action_resolve: false,
            supports_completion_resolve: false,
            resolve_provides_text_edit: false,
            supports_snippets: false,
            client_watches_files: false,
            needs_watch_fallback: false,
            supports_file_operations: false,
            supports_workspace_configuration: false,
            supports_semantic_tokens: true,
            supports_semantic_tokens_refresh: false,
            supports_folding: false,
            folding_line_only: false,
            supports_selection_range: false,
            supports_linked_editing: false,
            supports_work_done_progress: false,
            hover_media: HoverMediaProfile::default(),
            helix_one_char_selection_is_empty: false,
        }
    }

    /// Runs `f` against a [`DocumentContext`] for `text` under `config`.
    fn with_ctx(text: &str, config: &DmlsConfig, f: impl FnOnce(&DocumentContext)) {
        let uri: Uri = "file:///t.md".parse().unwrap();
        let path = Path::new("/t.md");
        let sm = source_map(text, PositionEncoding::Utf16);
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1);
        let profile = test_profile();
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &sm,
            graph: &graph,
            doc_id: None,
            config,
            profile: &profile,
            overlay: None,
        };
        f(&ctx);
    }

    /// One decoded, absolute token — the readable shape assertions compare.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Decoded {
        line: u32,
        character: u32,
        length: u32,
        token_type: u32,
        modifiers: u32,
    }

    /// Reverses the LSP relative-delta encoding back to absolute positions so
    /// tests read as `(line, char, len, type, mods)` tuples.
    fn decode(tokens: &SemanticTokens) -> Vec<Decoded> {
        let mut out = Vec::new();
        let mut line = 0u32;
        let mut character = 0u32;
        for token in &tokens.data {
            if token.delta_line == 0 {
                character += token.delta_start;
            } else {
                line += token.delta_line;
                character = token.delta_start;
            }
            out.push(Decoded {
                line,
                character,
                length: token.length,
                token_type: token.token_type,
                modifiers: token.token_modifiers_bitset,
            });
        }
        out
    }

    fn raw(
        span: SourceSpan,
        token_type: TokenType,
        modifiers: u32,
        family: Family,
        structural: u8,
    ) -> RawToken {
        RawToken {
            span,
            token_type,
            modifiers,
            family,
            structural,
            provenance: "test",
        }
    }

    /// A whole-span interpolation token, the F1 shape.
    fn interp(span: SourceSpan) -> RawToken {
        raw(
            span,
            TokenType::Macro,
            modifier::INTERPOLATION,
            Family::Interpolation,
            0,
        )
    }

    fn dec(line: u32, character: u32, length: u32, token_type: TokenType, modifiers: u32) -> Decoded {
        Decoded {
            line,
            character,
            length,
            token_type: token_type.index(),
            modifiers,
        }
    }

    #[test]
    fn legend_is_frozen_in_protocol_order() {
        let legend = legend();
        let types: Vec<&str> = legend.token_types.iter().map(|t| t.as_str()).collect();
        assert_eq!(
            types,
            ["macro", "function", "variable", "property", "string", "number", "operator"]
        );
        let mods: Vec<&str> = legend.token_modifiers.iter().map(|m| m.as_str()).collect();
        assert_eq!(
            mods,
            [
                "interpolation",
                "inert",
                "directive",
                "closer",
                "wiki",
                "injected",
                "defaultLibrary",
                "readonly",
            ]
        );
    }

    #[test]
    fn modifier_bits_match_legend_indices() {
        // Bit position n must equal legend modifier index n.
        assert_eq!(modifier::INTERPOLATION, 1 << 0);
        assert_eq!(modifier::READONLY, 1 << 7);
        // A combined bitset stays addressable per-bit.
        let combined = modifier::INTERPOLATION | modifier::INERT;
        assert_eq!(combined, 0b11);
    }

    #[test]
    fn single_ascii_token_encodes_absolute_first_delta() {
        let sm = source_map("a {{ x }} b", PositionEncoding::Utf16);
        let tokens = resolve_and_encode(vec![interp(2..9)], &sm, None);
        assert_eq!(decode(&tokens), vec![dec(0, 2, 7, TokenType::Macro, modifier::INTERPOLATION)]);
    }

    #[test]
    fn utf8_and_utf16_agree_on_logical_stream_but_differ_in_units() {
        // "é" before the interpolation: 2 bytes, 1 UTF-16 unit. The span still
        // covers the same 7-byte "{{ x }}".
        let text = "é {{ x }}";
        let start = text.find("{{").unwrap();
        let end = start + "{{ x }}".len();

        let sm16 = source_map(text, PositionEncoding::Utf16);
        let t16 = decode(&resolve_and_encode(vec![interp(start..end)], &sm16, None));
        assert_eq!(t16, vec![dec(0, 2, 7, TokenType::Macro, modifier::INTERPOLATION)]);

        let sm8 = source_map(text, PositionEncoding::Utf8);
        let t8 = decode(&resolve_and_encode(vec![interp(start..end)], &sm8, None));
        // Same length in bytes here (ASCII interior) but the start column is a
        // byte column: "é " is 3 bytes.
        assert_eq!(t8, vec![dec(0, 3, 7, TokenType::Macro, modifier::INTERPOLATION)]);
    }

    #[test]
    fn astral_scalar_length_is_encoding_specific() {
        // "{{💡}}": 💡 = 4 bytes / 2 UTF-16 units. Span covers the whole thing.
        let text = "{{💡}}";
        let sm16 = source_map(text, PositionEncoding::Utf16);
        let t16 = decode(&resolve_and_encode(vec![interp(0..text.len())], &sm16, None));
        // 2 (`{{`) + 2 (💡 in UTF-16) + 2 (`}}`) = 6 units.
        assert_eq!(t16, vec![dec(0, 0, 6, TokenType::Macro, modifier::INTERPOLATION)]);

        let sm8 = source_map(text, PositionEncoding::Utf8);
        let t8 = decode(&resolve_and_encode(vec![interp(0..text.len())], &sm8, None));
        // 2 + 4 + 2 = 8 bytes.
        assert_eq!(t8, vec![dec(0, 0, 8, TokenType::Macro, modifier::INTERPOLATION)]);
    }

    #[test]
    fn multiline_span_is_split_per_line_lf() {
        // A single interpolation spanning two lines becomes two tokens.
        let text = "{{ a\nb }}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(vec![interp(0..text.len())], &sm, None));
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 4, TokenType::Macro, modifier::INTERPOLATION),
                dec(1, 0, 4, TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn multiline_span_excludes_crlf_terminator() {
        let text = "{{ a\r\nb }}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(vec![interp(0..text.len())], &sm, None));
        // Line 0 content is "{{ a" (4), line 1 is "b }}" (4); the CRLF is not
        // part of either token.
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 4, TokenType::Macro, modifier::INTERPOLATION),
                dec(1, 0, 4, TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn multiline_span_excludes_lone_cr_terminator() {
        let text = "{{ a\rb }}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(vec![interp(0..text.len())], &sm, None));
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 4, TokenType::Macro, modifier::INTERPOLATION),
                dec(1, 0, 4, TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn exact_duplicates_are_removed() {
        let sm = source_map("{{ x }}", PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(vec![interp(0..7), interp(0..7)], &sm, None));
        assert_eq!(tokens, vec![dec(0, 0, 7, TokenType::Macro, modifier::INTERPOLATION)]);
    }

    #[test]
    fn interpolation_owns_bytes_and_clips_lower_directive() {
        // Directive string spanning the whole option value, with an
        // interpolation inside it: `when="{{ e }}"`. Interpolation (F1) owns
        // the `{{ e }}` bytes; the directive string (F2) is clipped to the two
        // quote-flanked fragments.
        let text = "when=\"{{ e }}\"";
        let interp_span = text.find("{{").unwrap()..text.find("}}").unwrap() + 2;
        let value_span = 5..text.len(); // "\"{{ e }}\""
        let raw_tokens = vec![
            interp(interp_span.clone()),
            raw(
                value_span,
                TokenType::Str,
                modifier::DIRECTIVE,
                Family::Directive,
                0,
            ),
        ];
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(raw_tokens, &sm, None));
        // Fragment 1: the opening quote (col 5, len 1). Interpolation: cols
        // 6..12 (len 7 — wait `{{ e }}` is 7). Fragment 2: closing quote.
        assert_eq!(
            tokens,
            vec![
                dec(0, 5, 1, TokenType::Str, modifier::DIRECTIVE),
                dec(0, 6, 7, TokenType::Macro, modifier::INTERPOLATION),
                dec(0, 13, 1, TokenType::Str, modifier::DIRECTIVE),
            ]
        );
    }

    #[test]
    fn wiki_beats_directive_but_loses_to_interpolation() {
        // Contrived overlap of all three families on one line to prove the
        // full F1 > F4 > F2 ordering. Directive covers [0,20); wiki covers
        // [4,16); interpolation covers [8,12).
        let text = "0123456789abcdefghij"; // 20 ASCII chars
        let sm = source_map(text, PositionEncoding::Utf16);
        let raw_tokens = vec![
            raw(0..20, TokenType::Str, modifier::DIRECTIVE, Family::Directive, 0),
            raw(4..16, TokenType::Str, modifier::WIKI, Family::Wiki, 0),
            interp(8..12),
        ];
        let tokens = decode(&resolve_and_encode(raw_tokens, &sm, None));
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 4, TokenType::Str, modifier::DIRECTIVE),
                dec(0, 4, 4, TokenType::Str, modifier::WIKI),
                dec(0, 8, 4, TokenType::Macro, modifier::INTERPOLATION),
                dec(0, 12, 4, TokenType::Str, modifier::WIKI),
                dec(0, 16, 4, TokenType::Str, modifier::DIRECTIVE),
            ]
        );
    }

    #[test]
    fn structural_subtoken_wins_within_family() {
        // A broad wiki text token [0,10) with a structural separator at [4,5).
        // The separator (higher structural rank) owns its byte; the text is
        // clipped around it.
        let text = "abcd|fghij";
        let sm = source_map(text, PositionEncoding::Utf16);
        let raw_tokens = vec![
            raw(0..10, TokenType::Str, modifier::WIKI, Family::Wiki, 0),
            raw(4..5, TokenType::Macro, modifier::WIKI, Family::Wiki, 1),
        ];
        let tokens = decode(&resolve_and_encode(raw_tokens, &sm, None));
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 4, TokenType::Str, modifier::WIKI),
                dec(0, 4, 1, TokenType::Macro, modifier::WIKI),
                dec(0, 5, 5, TokenType::Str, modifier::WIKI),
            ]
        );
    }

    #[test]
    fn containment_higher_priority_fully_absorbs_lower() {
        // Interpolation fully contains a wiki-looking span: the wiki token is
        // discarded (no fragment survives).
        let text = "{{ [[x]] }}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let raw_tokens = vec![
            interp(0..text.len()),
            raw(3..8, TokenType::Str, modifier::WIKI, Family::Wiki, 0),
        ];
        let tokens = decode(&resolve_and_encode(raw_tokens, &sm, None));
        assert_eq!(tokens, vec![dec(0, 0, 11, TokenType::Macro, modifier::INTERPOLATION)]);
    }

    #[test]
    fn adjacent_tokens_do_not_trigger_overlap() {
        // Two touching-but-not-overlapping interpolations.
        let text = "{{a}}{{b}}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(vec![interp(0..5), interp(5..10)], &sm, None));
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 5, TokenType::Macro, modifier::INTERPOLATION),
                dec(0, 5, 5, TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn zero_length_fragment_produces_no_token() {
        // A directive string exactly equal to the interpolation span leaves no
        // surviving fragment.
        let text = "{{ x }}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let raw_tokens = vec![
            interp(0..7),
            raw(0..7, TokenType::Str, modifier::DIRECTIVE, Family::Directive, 0),
        ];
        let tokens = decode(&resolve_and_encode(raw_tokens, &sm, None));
        assert_eq!(tokens, vec![dec(0, 0, 7, TokenType::Macro, modifier::INTERPOLATION)]);
    }

    #[test]
    fn deterministic_tie_break_by_start() {
        // Same-priority disjoint tokens in reverse order still sort by (line,
        // char).
        let text = "{{a}} {{b}}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(vec![interp(6..11), interp(0..5)], &sm, None));
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 5, TokenType::Macro, modifier::INTERPOLATION),
                dec(0, 6, 5, TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn range_clips_token_crossing_start_edge() {
        // Two interpolations; the range starts inside the first token.
        let text = "{{aaa}} {{bbb}}";
        let sm = source_map(text, PositionEncoding::Utf16);
        // Range covers byte 3..text.len(): first token clipped to [3,7).
        let tokens = decode(&resolve_and_encode(
            vec![interp(0..7), interp(8..15)],
            &sm,
            Some(3..text.len()),
        ));
        assert_eq!(
            tokens,
            vec![
                dec(0, 3, 4, TokenType::Macro, modifier::INTERPOLATION),
                dec(0, 8, 7, TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn range_clips_token_crossing_end_edge_and_drops_outside() {
        let text = "{{aaa}} {{bbb}}";
        let sm = source_map(text, PositionEncoding::Utf16);
        // Range 0..11: second token clipped to [8,11), i.e. "{{b".
        let tokens = decode(&resolve_and_encode(
            vec![interp(0..7), interp(8..15)],
            &sm,
            Some(0..11),
        ));
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 7, TokenType::Macro, modifier::INTERPOLATION),
                dec(0, 8, 3, TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn range_boundary_inside_non_ascii_text() {
        // "{{é}}" — é is bytes 2..4. A range beginning at byte 2 (a char
        // boundary) keeps the whole token from that byte.
        let text = "{{é}}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(
            vec![interp(0..text.len())],
            &sm,
            Some(2..text.len()),
        ));
        // From col 2 (after `{{`) to end: é(1) + }}(2) = 3 UTF-16 units.
        assert_eq!(tokens, vec![dec(0, 2, 3, TokenType::Macro, modifier::INTERPOLATION)]);
    }

    #[test]
    fn range_is_exactly_full_intersected() {
        // Property-style: the range response equals the full response clipped
        // to the range, token for token.
        let text = "{{aaa}} {{bbb}} {{ccc}}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let raw_tokens = || vec![interp(0..7), interp(8..15), interp(16..23)];
        let full = decode(&resolve_and_encode(raw_tokens(), &sm, None));
        let range = 5..18;
        let ranged = decode(&resolve_and_encode(raw_tokens(), &sm, Some(range.clone())));
        // Manually intersect the full absolute tokens with the range in byte
        // space, then compare to the range response.
        let expected: Vec<Decoded> = full
            .into_iter()
            .filter_map(|tok| {
                let start_byte = sm.lsp_to_byte(lsp_types::Position::new(tok.line, tok.character))?;
                let end_byte = start_byte + tok.length as usize; // ASCII: 1 byte per unit
                let s = start_byte.max(range.start);
                let e = end_byte.min(range.end);
                (s < e).then(|| Decoded {
                    line: tok.line,
                    character: tok.character + (s - start_byte) as u32,
                    length: (e - s) as u32,
                    token_type: tok.token_type,
                    modifiers: tok.modifiers,
                })
            })
            .collect();
        assert_eq!(ranged, expected);
    }

    #[test]
    fn inert_literal_carries_both_modifiers() {
        // A `{{{ }}}` literal is macro + interpolation + inert.
        let text = "{{{ x }}}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let mods = modifier::INTERPOLATION | modifier::INERT;
        let raw_tokens = vec![raw(
            0..text.len(),
            TokenType::Macro,
            mods,
            Family::Interpolation,
            0,
        )];
        let tokens = decode(&resolve_and_encode(raw_tokens, &sm, None));
        assert_eq!(tokens, vec![dec(0, 0, 9, TokenType::Macro, mods)]);
    }

    // ------------------------------------------------------------------------
    // Family emitters (Phase 3)
    // ------------------------------------------------------------------------

    /// A readable projection of one raw token: its source slice, type, and
    /// modifier bitset.
    fn shapes<'a>(tokens: &[RawToken], text: &'a str) -> Vec<(&'a str, TokenType, u32)> {
        tokens
            .iter()
            .map(|token| (&text[token.span.clone()], token.token_type, token.modifiers))
            .collect()
    }

    #[test]
    fn f1_interpolation_is_whole_span_macro() {
        let text = "Body: {{ title }} and {{ ctx.today }}.";
        let tokens = interpolation_tokens(text, 0);
        assert_eq!(
            shapes(&tokens, text),
            vec![
                ("{{ title }}", TokenType::Macro, modifier::INTERPOLATION),
                ("{{ ctx.today }}", TokenType::Macro, modifier::INTERPOLATION),
            ]
        );
    }

    #[test]
    fn f1_literal_carries_inert_modifier() {
        let text = "See {{{ verbatim }}} here.";
        let tokens = interpolation_tokens(text, 0);
        let inert = modifier::INTERPOLATION | modifier::INERT;
        assert_eq!(
            shapes(&tokens, text),
            vec![("{{{ verbatim }}}", TokenType::Macro, inert)]
        );
    }

    #[test]
    fn f1_malformed_and_fenced_constructs_emit_nothing() {
        // Scanner-level suppression, not re-filtered here: an unclosed brace
        // and a fenced `{{ … }}` both yield no F1 token.
        assert!(interpolation_tokens("Hello {{ title", 0).is_empty());
        let fenced = "```\n{{ hidden }}\n```\n";
        assert!(interpolation_tokens(fenced, 0).is_empty());
    }

    #[test]
    fn f1_body_base_excludes_frontmatter_interpolation() {
        let text = "---\ntitle: {{ seed }}\n---\n\n{{ shown }}\n";
        let body_base = text.find("\n\n").unwrap() + 2;
        assert_eq!(
            shapes(&interpolation_tokens(text, body_base), text),
            vec![("{{ shown }}", TokenType::Macro, modifier::INTERPOLATION)]
        );
    }

    #[test]
    fn f2_emits_all_twelve_directive_keywords() {
        // Every keyword is `macro.directive`; the three structural closers add
        // `closer`. The scan is lenient, so unpaired block/disclosure lines are
        // still individually reported.
        let text = concat!(
            "::file ./f.md\n",
            "::code ./c.rs\n",
            "::url https://x/y.md\n",
            "::file-links ./notes\n",
            "::toc-linking ./toc.md\n",
            "::shell echo hi\n",
            "::block when=\"x\"\n",
            "::shell-block\n",
            "::end-block\n",
            "::disclosure Summary\n",
            "::details\n",
            "::end-disclosure\n",
        );
        let keywords: Vec<(&str, u32)> = directive_tokens(text, 0)
            .into_iter()
            .filter(|token| token.token_type == TokenType::Macro)
            .map(|token| {
                assert!(
                    token.modifiers & modifier::DIRECTIVE != 0,
                    "keyword lacks directive modifier"
                );
                (&text[token.span.clone()], token.modifiers)
            })
            .collect();
        let names: Vec<&str> = keywords.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            vec![
                "::file",
                "::code",
                "::url",
                "::file-links",
                "::toc-linking",
                "::shell",
                "::block",
                "::shell-block",
                "::end-block",
                "::disclosure",
                "::details",
                "::end-disclosure",
            ]
        );
        // Exactly the three structural closers carry `closer`.
        let closers: Vec<&str> = keywords
            .iter()
            .filter(|(_, mods)| mods & modifier::CLOSER != 0)
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(closers, vec!["::end-block", "::details", "::end-disclosure"]);
    }

    #[test]
    fn f2_structured_target_and_options() {
        let text = "::file ./doc.md when=env.DEBUG quotation=true\n";
        assert_eq!(
            shapes(&directive_tokens(text, 0), text),
            vec![
                ("::file", TokenType::Macro, modifier::DIRECTIVE),
                ("./doc.md", TokenType::Str, modifier::DIRECTIVE),
                ("when", TokenType::Property, modifier::DIRECTIVE),
                ("env.DEBUG", TokenType::Str, modifier::DIRECTIVE),
                ("quotation", TokenType::Property, modifier::DIRECTIVE),
                ("true", TokenType::Str, modifier::DIRECTIVE),
            ]
        );
    }

    #[test]
    fn f2_unknown_directive_yields_no_token() {
        // An unrecognized `::frobnicate` is never surfaced by the scanner, so it
        // stays plain prose (the `dm.directive.unknown` diagnostic owns it).
        assert!(directive_tokens("::frobnicate ./x.md\n", 0).is_empty());
    }

    #[test]
    fn f2_shell_payload_is_not_tokenized() {
        // The `::shell` keyword tokens; the command payload (F6) does not.
        let text = "::shell echo hello world\n";
        assert_eq!(
            shapes(&directive_tokens(text, 0), text),
            vec![("::shell", TokenType::Macro, modifier::DIRECTIVE)]
        );
    }

    #[test]
    fn f2_disclosure_summary_prose_is_not_tokenized() {
        // Only the three disclosure keywords token; the summary text is prose.
        let text = "::disclosure License Agreement\n::details\nbody\n::end-disclosure\n";
        assert_eq!(
            shapes(&directive_tokens(text, 0), text),
            vec![
                ("::disclosure", TokenType::Macro, modifier::DIRECTIVE),
                ("::details", TokenType::Macro, modifier::DIRECTIVE | modifier::CLOSER),
                ("::end-disclosure", TokenType::Macro, modifier::DIRECTIVE | modifier::CLOSER),
            ]
        );
    }

    #[test]
    fn f2_disclosure_style_tokens_split_at_equals() {
        // Recognized inline style tokens split at their source `=` into a
        // property key and a string value; the trailing summary stays prose.
        let text = "::disclosure max-width=60ch color=red-500 License\n::end-disclosure\n";
        assert_eq!(
            shapes(&directive_tokens(text, 0), text),
            vec![
                ("::disclosure", TokenType::Macro, modifier::DIRECTIVE),
                ("::end-disclosure", TokenType::Macro, modifier::DIRECTIVE | modifier::CLOSER),
                ("max-width", TokenType::Property, modifier::DIRECTIVE),
                ("60ch", TokenType::Str, modifier::DIRECTIVE),
                ("color", TokenType::Property, modifier::DIRECTIVE),
                ("red-500", TokenType::Str, modifier::DIRECTIVE),
            ]
        );
    }

    #[test]
    fn f2_body_base_shifts_spans_and_excludes_frontmatter() {
        let text = "---\ntitle: x\n---\n\n::file ./doc.md\n";
        let body_base = text.find("\n\n").unwrap() + 2;
        let tokens = directive_tokens(text, body_base);
        assert_eq!(
            shapes(&tokens, text),
            vec![
                ("::file", TokenType::Macro, modifier::DIRECTIVE),
                ("./doc.md", TokenType::Str, modifier::DIRECTIVE),
            ]
        );
    }

    #[test]
    fn f4_wiki_brackets_and_segments() {
        // Decoded through the pipeline: brackets and separators are macro.wiki,
        // segments string.wiki. `[[doc#H|Alias]]` at column 0.
        let text = "[[doc#H|Alias]]";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(wiki_tokens(text, 0), &sm, None));
        let m = modifier::WIKI;
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 2, TokenType::Macro, m),  // [[
                dec(0, 2, 3, TokenType::Str, m),    // doc
                dec(0, 5, 1, TokenType::Macro, m),  // #
                dec(0, 6, 1, TokenType::Str, m),    // H
                dec(0, 7, 1, TokenType::Macro, m),  // |
                dec(0, 8, 5, TokenType::Str, m),    // Alias
                dec(0, 13, 2, TokenType::Macro, m), // ]]
            ]
        );
    }

    #[test]
    fn f4_resolved_and_unresolved_have_identical_shapes() {
        // Token structure is span-only; resolution status is never encoded.
        let a = "[[exists]]";
        let b = "[[missing]]";
        let shape = |text: &str| {
            let tokens = wiki_tokens(text, 0);
            tokens
                .iter()
                .map(|t| (t.token_type, t.modifiers, t.structural))
                .collect::<Vec<_>>()
        };
        assert_eq!(shape(a), shape(b));
    }

    #[test]
    fn f4_unsupported_form_shares_the_shape() {
        // An embed `![[…]]` is recognized-but-unsupported; it still tokens like
        // any wiki link (the diagnostic owns validity).
        let text = "![[embed|A]]";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(wiki_tokens(text, 0), &sm, None));
        let m = modifier::WIKI;
        // Link span starts at `[[` (byte 1); the `!` is not part of it.
        assert_eq!(
            tokens,
            vec![
                dec(0, 1, 2, TokenType::Macro, m),  // [[
                dec(0, 3, 5, TokenType::Str, m),    // embed
                dec(0, 8, 1, TokenType::Macro, m),  // |
                dec(0, 9, 1, TokenType::Str, m),    // A
                dec(0, 10, 2, TokenType::Macro, m), // ]]
            ]
        );
    }

    #[test]
    fn f4_empty_segments_produce_no_token() {
        // `[[#Local]]`: zero-width target is dropped; `[[a|]]`: empty alias is
        // dropped. Only the non-empty segments survive as string.wiki.
        let heading_only = "[[#Local]]";
        let segments: Vec<&str> = wiki_tokens(heading_only, 0)
            .into_iter()
            .filter(|t| t.token_type == TokenType::Str)
            .map(|t| &heading_only[t.span.clone()])
            .collect();
        assert_eq!(segments, vec!["Local"]);

        let empty_alias = "[[a|]]";
        let segments: Vec<&str> = wiki_tokens(empty_alias, 0)
            .into_iter()
            .filter(|t| t.token_type == TokenType::Str)
            .map(|t| &empty_alias[t.span.clone()])
            .collect();
        assert_eq!(segments, vec!["a"]);
    }

    #[test]
    fn f4_fenced_wiki_link_is_not_surfaced() {
        let text = "```\n[[hidden]]\n```\n[[real]]\n";
        let segments: Vec<&str> = wiki_tokens(text, 0)
            .into_iter()
            .filter(|t| t.token_type == TokenType::Str)
            .map(|t| &text[t.span.clone()])
            .collect();
        assert_eq!(segments, vec!["real"]);
    }

    #[test]
    fn combined_interpolation_inside_directive_option_value() {
        // F1 owns the `{{ … }}` bytes; the directive option value (F2) is
        // clipped to the quotes around it.
        let text = "::file ./x.md when=\"{{ e }}\"\n";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(family_tokens(text, 0), &sm, None));
        // Recover the pieces by column for readability.
        let quote_open = text.find('"').unwrap() as u32;
        let interp = text.find("{{").unwrap() as u32;
        let quote_close = text.rfind('"').unwrap() as u32;
        assert!(tokens.contains(&dec(0, quote_open, 1, TokenType::Str, modifier::DIRECTIVE)));
        assert!(tokens.contains(&dec(0, interp, 7, TokenType::Macro, modifier::INTERPOLATION)));
        assert!(tokens.contains(&dec(0, quote_close, 1, TokenType::Str, modifier::DIRECTIVE)));
    }

    #[test]
    fn combined_wiki_looking_text_inside_interpolation() {
        // `{{ [[x]] }}`: the interpolation (F1) fully contains the wiki-looking
        // span (F4), so only the interpolation token survives.
        let text = "{{ [[x]] }}";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(family_tokens(text, 0), &sm, None));
        assert_eq!(
            tokens,
            vec![dec(0, 0, 11, TokenType::Macro, modifier::INTERPOLATION)]
        );
    }

    #[test]
    fn combined_wiki_structural_subtoken_precedence() {
        // Real F4 output exercises structural precedence: the `#`/`|` separators
        // (macro.wiki) win over the broad frame around the string.wiki segments.
        let text = "[[a#b]]";
        let sm = source_map(text, PositionEncoding::Utf16);
        let tokens = decode(&resolve_and_encode(wiki_tokens(text, 0), &sm, None));
        let m = modifier::WIKI;
        assert_eq!(
            tokens,
            vec![
                dec(0, 0, 2, TokenType::Macro, m), // [[
                dec(0, 2, 1, TokenType::Str, m),   // a
                dec(0, 3, 1, TokenType::Macro, m), // #
                dec(0, 4, 1, TokenType::Str, m),   // b
                dec(0, 5, 2, TokenType::Macro, m), // ]]
            ]
        );
    }

    #[test]
    fn combined_duplicate_family_output_is_deduplicated() {
        // Concatenating the family output twice must not double any token.
        let text = "{{ x }} ::file ./y.md\n";
        let sm = source_map(text, PositionEncoding::Utf16);
        let mut doubled = family_tokens(text, 0);
        doubled.extend(family_tokens(text, 0));
        let once = decode(&resolve_and_encode(family_tokens(text, 0), &sm, None));
        let twice = decode(&resolve_and_encode(doubled, &sm, None));
        assert_eq!(once, twice);
    }

    #[test]
    fn combined_full_and_range_stream_stays_ordered_and_nonoverlapping() {
        // A document mixing all three families: the full stream is strictly
        // ordered and non-overlapping, and the range response equals the full
        // response clipped to the half-open range.
        let text = "{{ a }} [[doc#H]] ::file ./z.md when=b\n";
        let sm = source_map(text, PositionEncoding::Utf16);
        let full = decode(&resolve_and_encode(family_tokens(text, 0), &sm, None));
        // Strictly ordered, non-overlapping.
        for pair in full.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                (a.line, a.character) < (b.line, b.character),
                "tokens not strictly ordered: {a:?} then {b:?}"
            );
            if a.line == b.line {
                assert!(a.character + a.length <= b.character, "tokens overlap: {a:?} then {b:?}");
            }
        }
        // Range clip equals the full stream intersected in byte space (ASCII).
        let range = 5..30usize;
        let ranged = decode(&resolve_and_encode(family_tokens(text, 0), &sm, Some(range.clone())));
        let expected: Vec<Decoded> = full
            .into_iter()
            .filter_map(|tok| {
                let start = sm.lsp_to_byte(lsp_types::Position::new(tok.line, tok.character))?;
                let end = start + tok.length as usize;
                let s = start.max(range.start);
                let e = end.min(range.end);
                (s < e).then(|| Decoded {
                    line: tok.line,
                    character: tok.character + (s - start) as u32,
                    length: (e - s) as u32,
                    token_type: tok.token_type,
                    modifiers: tok.modifiers,
                })
            })
            .collect();
        assert_eq!(ranged, expected);
    }

    // ------------------------------------------------------------------------
    // Request handlers (Phase 4)
    // ------------------------------------------------------------------------

    #[test]
    fn handler_master_switch_toggles_emission() {
        // A runtime true -> false -> true transition: tokens, then an empty
        // stream, then the identical tokens again.
        let text = "Body {{ title }} here.\n";
        let mut config = DmlsConfig::default();

        let mut enabled = Vec::new();
        with_ctx(text, &config, |ctx| {
            enabled = decode(&semantic_tokens_full(ctx));
        });
        assert_eq!(
            enabled,
            vec![dec(0, 5, 11, TokenType::Macro, modifier::INTERPOLATION)]
        );

        config.semantic_tokens.enable = false;
        with_ctx(text, &config, |ctx| {
            assert!(
                semantic_tokens_full(ctx).data.is_empty(),
                "master switch off must emit no tokens"
            );
        });

        config.semantic_tokens.enable = true;
        with_ctx(text, &config, |ctx| {
            assert_eq!(decode(&semantic_tokens_full(ctx)), enabled);
        });
    }

    #[test]
    fn handler_wiki_enable_suppresses_only_f4() {
        // `[[doc]]` (F4) is dropped when `wiki.enable` is false, but the F1
        // interpolation survives untouched.
        let text = "{{ a }} [[doc]]\n";
        let mut config = DmlsConfig::default();

        with_ctx(text, &config, |ctx| {
            let tokens = decode(&semantic_tokens_full(ctx));
            assert!(
                tokens
                    .iter()
                    .any(|t| t.modifiers & modifier::WIKI != 0),
                "wiki enabled must emit wiki tokens"
            );
        });

        config.wiki.enable = false;
        with_ctx(text, &config, |ctx| {
            let tokens = decode(&semantic_tokens_full(ctx));
            assert!(
                tokens.iter().all(|t| t.modifiers & modifier::WIKI == 0),
                "wiki disabled must suppress every F4 token"
            );
            // F1 is unaffected by the wiki gate.
            assert!(
                tokens
                    .iter()
                    .any(|t| t.modifiers & modifier::INTERPOLATION != 0),
                "wiki disabled must keep F1 interpolation tokens"
            );
        });
    }

    #[test]
    fn handler_range_is_full_intersected() {
        // The range response equals the full response clipped to the requested
        // LSP range (ASCII, so bytes == UTF-16 units).
        let text = "{{aaa}} {{bbb}} {{ccc}}\n";
        let config = DmlsConfig::default();
        with_ctx(text, &config, |ctx| {
            let full = decode(&semantic_tokens_full(ctx));
            let range = Range::new(
                lsp_types::Position::new(0, 5),
                lsp_types::Position::new(0, 18),
            );
            let ranged = decode(&semantic_tokens_range(ctx, range));
            let expected: Vec<Decoded> = full
                .into_iter()
                .filter_map(|tok| {
                    let s = tok.character.max(5);
                    let e = (tok.character + tok.length).min(18);
                    (s < e).then_some(Decoded {
                        line: tok.line,
                        character: s,
                        length: e - s,
                        token_type: tok.token_type,
                        modifiers: tok.modifiers,
                    })
                })
                .collect();
            assert_eq!(ranged, expected);
        });
    }
}
