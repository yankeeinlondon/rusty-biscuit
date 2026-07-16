//! The wiki-link scanner (R-8 "Target Parsing").
//!
//! Darkmatter's Markdown parser does not model `[[wiki]]` links, so DMLS scans
//! them itself. The scan is deliberately lexical: it walks the document body
//! line by line, skips fenced code blocks and inline code spans (so `[[…]]`
//! written inside a code example is not treated as a link), and recognizes the
//! six v1 forms plus the backslash escapes `\|`, `\#`, `\]`, `\\`.
//!
//! Spans are body-relative byte ranges; the substrate indexer shifts them into
//! document coordinates alongside heading and Markdown-link spans.

use darkmatter::markdown::span::SourceSpan;

/// One `[[…]]` occurrence found in the source.
///
/// Every `*_span` is a byte range into the scanned source, derived from the
/// scan offsets — never reconstructed from the unescaped `target`/`heading`/
/// `alias` strings (whose lengths differ from the source when escapes are
/// present). The separator spans (`hash_span`, `pipe_span`) each cover the
/// single `#` / `|` byte, so a consumer can token the structural separators
/// distinctly from the surrounding segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedWikiLink {
    /// Byte span of the whole link including the `[[` and `]]` delimiters.
    pub span: SourceSpan,
    /// Byte span of the file-target text (before any `#`/`|`). Zero-width and
    /// anchored just after `[[` for a headings-only `[[#heading]]` link.
    pub target_span: SourceSpan,
    /// Byte span of the present `#` heading separator (one byte), when the link
    /// had a `#`.
    pub hash_span: Option<SourceSpan>,
    /// Byte span of the heading text (after `#`), when present.
    pub heading_span: Option<SourceSpan>,
    /// Byte span of the present `|` alias separator (one byte), when the link
    /// had a `|`.
    pub pipe_span: Option<SourceSpan>,
    /// Byte span of the alias text (after `|`), when present. May be zero-width
    /// for an empty alias (`[[a|]]`).
    pub alias_span: Option<SourceSpan>,
    /// Unescaped file-target text (may be empty).
    pub target: String,
    /// Unescaped heading text, when the link had a `#`.
    pub heading: Option<String>,
    /// Unescaped alias text, when the link had a `|`.
    pub alias: Option<String>,
    /// A v1-unsupported form (embed `![[…]]`, block ref `#^…`).
    pub unsupported: bool,
    /// 1-indexed body line the link starts on.
    pub line: usize,
}

/// Scans `body` for wiki links, returning them in source order.
pub fn scan_wiki_links(body: &str) -> Vec<ScannedWikiLink> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut offset = 0;
    let mut line_number = 0;

    for line in body.split_inclusive('\n') {
        line_number += 1;
        let line_start = offset;
        offset += line.len();

        let trimmed = line.trim_start();
        if is_code_fence(trimmed) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        scan_line(line, line_start, line_number, &mut out);
    }
    out
}

/// Whether a line (already left-trimmed) opens or closes a fenced code block.
fn is_code_fence(trimmed: &str) -> bool {
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Scans a single line for wiki links outside inline code spans.
fn scan_line(line: &str, base: usize, line_number: usize, out: &mut Vec<ScannedWikiLink>) {
    let bytes = line.as_bytes();
    let length = bytes.len();
    let mut index = 0;
    let mut inline_code: Option<usize> = None;

    while index < length {
        match bytes[index] {
            b'`' => {
                let mut end = index;
                while end < length && bytes[end] == b'`' {
                    end += 1;
                }
                let run = end - index;
                inline_code = match inline_code {
                    None => Some(run),
                    Some(open) if open == run => None,
                    other => other,
                };
                index = end;
            }
            b'[' if inline_code.is_none()
                && index + 1 < length
                && bytes[index + 1] == b'[' =>
            {
                let embed = index > 0 && bytes[index - 1] == b'!';
                match parse_link(line, index, embed) {
                    Some((link_end, mut link)) => {
                        shift_spans(&mut link, base);
                        link.line = line_number;
                        out.push(link);
                        index = link_end;
                    }
                    None => index += 2,
                }
            }
            _ => index += 1,
        }
    }
}

/// Parses a `[[…]]` starting at `open` (the first `[`), returning the byte
/// index just past the closing `]]` and the link with line-relative spans.
fn parse_link(line: &str, open: usize, embed: bool) -> Option<(usize, ScannedWikiLink)> {
    let bytes = line.as_bytes();
    let length = bytes.len();
    let inner_start = open + 2;

    // Find the closing `]]`, honoring escapes and never crossing a newline.
    let mut cursor = inner_start;
    let mut escaped = false;
    let mut close = None;
    while cursor < length {
        let byte = bytes[cursor];
        if byte == b'\n' {
            break;
        }
        if escaped {
            escaped = false;
            cursor += 1;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b']' if cursor + 1 < length && bytes[cursor + 1] == b']' => {
                close = Some(cursor);
                break;
            }
            _ => {}
        }
        cursor += 1;
    }
    let close = close?;
    let inner = &line[inner_start..close];
    let after = close + 2;

    // Split alias, then heading, at the first unescaped `|` / `#`. Positions are
    // byte offsets into `inner`; the separators occupy one byte each.
    let (before_alias, alias, pipe_pos) = match find_unescaped(inner, b'|') {
        Some(position) => (&inner[..position], Some(&inner[position + 1..]), Some(position)),
        None => (inner, None, None),
    };
    let (target_part, heading_part, hash_pos) = match find_unescaped(before_alias, b'#') {
        Some(position) => (
            &before_alias[..position],
            Some(&before_alias[position + 1..]),
            Some(position),
        ),
        None => (before_alias, None, None),
    };

    let target_span = inner_start..inner_start + target_part.len();
    let hash_span = hash_pos.map(|position| {
        let at = inner_start + position;
        at..at + 1
    });
    let heading_span = heading_part.map(|heading| {
        // The heading text starts one byte after the `#`.
        let start = inner_start + target_part.len() + 1;
        start..start + heading.len()
    });
    let pipe_span = pipe_pos.map(|position| {
        let at = inner_start + position;
        at..at + 1
    });
    let alias_span = alias.map(|alias| {
        // The alias text starts one byte after the `|`; `pipe_pos` is `Some`
        // exactly when `alias` is.
        let start = inner_start + pipe_pos.expect("pipe present when alias present") + 1;
        start..start + alias.len()
    });
    // A block reference `#^…` is a v1-unsupported form.
    let block_ref = heading_part.is_some_and(|heading| heading.starts_with('^'));

    let link = ScannedWikiLink {
        span: open..after,
        target_span,
        hash_span,
        heading_span,
        pipe_span,
        alias_span,
        target: unescape(target_part),
        heading: heading_part.map(unescape),
        alias: alias.map(unescape),
        unsupported: embed || block_ref,
        line: 0,
    };
    Some((after, link))
}

/// Shifts a link's line-relative spans into body coordinates.
fn shift_spans(link: &mut ScannedWikiLink, base: usize) {
    let shift = |span: SourceSpan| base + span.start..base + span.end;
    link.span = shift(link.span.clone());
    link.target_span = shift(link.target_span.clone());
    link.hash_span = link.hash_span.take().map(shift);
    link.heading_span = link.heading_span.take().map(shift);
    link.pipe_span = link.pipe_span.take().map(shift);
    link.alias_span = link.alias_span.take().map(shift);
}

/// The byte index of the first `delimiter` in `text` that is not backslash-
/// escaped, honoring the recognized escape set (`\|`, `\#`, `\]`, `\\`).
fn find_unescaped(text: &str, delimiter: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if index + 1 < bytes.len() && is_escapable(bytes[index + 1]) => {
                index += 2;
            }
            byte if byte == delimiter => return Some(index),
            _ => index += 1,
        }
    }
    None
}

/// Resolves the recognized escapes; an unrecognized `\x` keeps its backslash.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\\'
            && chars.peek().is_some_and(|next| is_escapable(*next as u8))
        {
            out.push(chars.next().expect("peeked escape char"));
        } else {
            out.push(character);
        }
    }
    out
}

/// Whether `byte` is one of the wiki-link-escapable ASCII characters.
fn is_escapable(byte: u8) -> bool {
    matches!(byte, b'|' | b'#' | b']' | b'\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(body: &str) -> Vec<ScannedWikiLink> {
        scan_wiki_links(body)
    }

    #[test]
    fn test_all_six_forms() {
        let links = scan("[[a]] [[b|Alias]] [[c#H]] [[d#H|Alias]] [[#Local]] [[#Local|Alias]]\n");
        assert_eq!(links.len(), 6);
        assert_eq!(links[0].target, "a");
        assert_eq!(links[1].alias.as_deref(), Some("Alias"));
        assert_eq!(links[2].heading.as_deref(), Some("H"));
        assert_eq!(links[3].heading.as_deref(), Some("H"));
        assert_eq!(links[3].alias.as_deref(), Some("Alias"));
        assert_eq!(links[4].target, "");
        assert_eq!(links[4].heading.as_deref(), Some("Local"));
        assert_eq!(links[5].alias.as_deref(), Some("Alias"));
    }

    #[test]
    fn test_span_covers_delimiters() {
        let body = "see [[Target]] here\n";
        let link = &scan(body)[0];
        assert_eq!(&body[link.span.clone()], "[[Target]]");
        assert_eq!(&body[link.target_span.clone()], "Target");
    }

    #[test]
    fn test_heading_span_excludes_hash() {
        let body = "[[Doc#My Heading]]\n";
        let link = &scan(body)[0];
        let heading_span = link.heading_span.clone().unwrap();
        assert_eq!(&body[heading_span], "My Heading");
    }

    #[test]
    fn test_escapes() {
        let links = scan(r"[[a\|b]] [[c\#d]] [[e\]f]]" .to_string().as_str());
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].target, "a|b");
        assert_eq!(links[0].alias, None);
        assert_eq!(links[1].target, "c#d");
        assert_eq!(links[1].heading, None);
        assert_eq!(links[2].target, "e]f");
    }

    #[test]
    fn test_embed_and_block_ref_flagged_unsupported() {
        let links = scan("![[embed]] [[doc#^block]]\n");
        assert_eq!(links.len(), 2);
        assert!(links[0].unsupported);
        assert!(links[1].unsupported);
    }

    #[test]
    fn test_fenced_and_inline_code_skipped() {
        let body = "```\n[[in-fence]]\n```\n`[[in-code]]` [[real]]\n";
        let links = scan(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "real");
    }

    #[test]
    fn test_unterminated_is_ignored() {
        assert!(scan("[[no close here\nnext line]]\n").is_empty());
    }

    #[test]
    fn test_line_numbers() {
        let links = scan("first\n[[here]]\n");
        assert_eq!(links[0].line, 2);
    }

    #[test]
    fn test_separator_and_alias_spans() {
        let body = "[[doc#H|Alias]]\n";
        let link = &scan(body)[0];
        assert_eq!(&body[link.target_span.clone()], "doc");
        assert_eq!(&body[link.hash_span.clone().unwrap()], "#");
        assert_eq!(&body[link.heading_span.clone().unwrap()], "H");
        assert_eq!(&body[link.pipe_span.clone().unwrap()], "|");
        assert_eq!(&body[link.alias_span.clone().unwrap()], "Alias");
    }

    #[test]
    fn test_plain_target_has_no_separator_or_alias_spans() {
        let link = &scan("[[a]]\n")[0];
        assert!(link.hash_span.is_none());
        assert!(link.heading_span.is_none());
        assert!(link.pipe_span.is_none());
        assert!(link.alias_span.is_none());
    }

    #[test]
    fn test_spans_are_source_derived_not_reconstructed_from_unescaped() {
        // The escaped source slices are longer than the unescaped values, so a
        // span reconstructed from `target`/`alias` lengths would be wrong.
        let body = r"[[a\#b#H|Al\|ias]]" .to_string() + "\n";
        let link = &scan(&body)[0];
        assert_eq!(link.target, "a#b");
        assert_eq!(&body[link.target_span.clone()], r"a\#b");
        assert_eq!(link.heading.as_deref(), Some("H"));
        assert_eq!(&body[link.heading_span.clone().unwrap()], "H");
        assert_eq!(link.alias.as_deref(), Some("Al|ias"));
        assert_eq!(&body[link.alias_span.clone().unwrap()], r"Al\|ias");
    }

    #[test]
    fn test_empty_alias_span_is_zero_width() {
        let body = "[[a|]]\n";
        let link = &scan(body)[0];
        assert_eq!(link.alias.as_deref(), Some(""));
        let alias_span = link.alias_span.clone().unwrap();
        assert!(alias_span.is_empty(), "empty alias is a zero-width span");
        assert_eq!(&body[link.pipe_span.clone().unwrap()], "|");
    }

    #[test]
    fn test_headings_only_link_spans() {
        let body = "[[#Local]]\n";
        let link = &scan(body)[0];
        assert!(link.target_span.is_empty());
        assert_eq!(&body[link.hash_span.clone().unwrap()], "#");
        assert_eq!(&body[link.heading_span.clone().unwrap()], "Local");
        assert!(link.pipe_span.is_none());
        assert!(link.alias_span.is_none());
    }

    #[test]
    fn test_recognized_but_unsupported_form_keeps_spans() {
        // A block-ref `#^…` is recognized-but-invalid: spans are still emitted so
        // it tokens like any other wiki link; the diagnostic owns validity.
        let body = "[[doc#^block]]\n";
        let link = &scan(body)[0];
        assert!(link.unsupported);
        assert_eq!(&body[link.hash_span.clone().unwrap()], "#");
        assert_eq!(&body[link.heading_span.clone().unwrap()], "^block");
    }

    #[test]
    fn test_embed_spans_are_body_relative() {
        // The embed marker `!` precedes `[[`; the link span still starts at `[[`.
        let body = "text ![[embed|A]]\n";
        let link = &scan(body)[0];
        assert!(link.unsupported);
        assert_eq!(&body[link.span.clone()], "[[embed|A]]");
        assert_eq!(&body[link.pipe_span.clone().unwrap()], "|");
        assert_eq!(&body[link.alias_span.clone().unwrap()], "A");
    }
}
