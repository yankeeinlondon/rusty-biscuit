//! Unified, span-carrying directive scanner (read-only).
//!
//! A single passive entry point over every Darkmatter block/inline directive
//! family — transclusion (`::file`, `::code`, `::url`), file links
//! (`::file-links`), TOC linking (`::toc-linking`), shell (`::shell`), page and
//! shell blocks (`::block` / `::shell-block` / `::end-block`), and disclosures
//! (`::disclosure` / `::details` / `::end-disclosure`). It reports each
//! directive's byte spans (keyword, target, and per-option key/value) without
//! executing anything, so a language server can offer navigation, hover, and
//! completion over directive source.
//!
//! This surface **shares** the existing per-family scanning primitives (the
//! [`Cursor`](super::parse_utils::Cursor), code-region detection, and the
//! crate-private [`scan_block_pairs`](super::block_pairs::scan_block_pairs)); it
//! does not re-implement any family's semantics and never mutates content. The
//! scan is lenient: a malformed directive line still yields a best-effort
//! [`ParsedDirective`] with whatever spans could be recovered, rather than an
//! error, because the consumer is a passive analyzer.

use super::block_pairs::{BlockOpenKind, BlockPairError, scan_block_pairs};
use super::parse_utils::{Cursor, find_code_regions, is_in_code_region};
use super::shell_blocks::body::group_logical_commands;
use crate::markdown::span::{SourceSpan, Spanned};

/// The directive family a scanned line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveKind {
    /// `::file` block transclusion.
    File,
    /// `::code` code transclusion.
    Code,
    /// `::url` remote transclusion.
    Url,
    /// `::file-links` directory/glob link listing.
    FileLinks,
    /// `::toc-linking` cross-document TOC linking.
    TocLinking,
    /// `::shell` inline shell directive.
    Shell,
    /// `::block` page-block opener.
    Block,
    /// `::shell-block` shell-block opener.
    ShellBlock,
    /// `::end-block` page/shell-block closer.
    EndBlock,
    /// `::disclosure` disclosure opener.
    Disclosure,
    /// `::details` disclosure summary/body divider.
    Details,
    /// `::end-disclosure` disclosure closer.
    EndDisclosure,
}

impl DirectiveKind {
    /// The canonical directive keyword, including the leading `::`.
    pub fn keyword(self) -> &'static str {
        match self {
            DirectiveKind::File => "::file",
            DirectiveKind::Code => "::code",
            DirectiveKind::Url => "::url",
            DirectiveKind::FileLinks => "::file-links",
            DirectiveKind::TocLinking => "::toc-linking",
            DirectiveKind::Shell => "::shell",
            DirectiveKind::Block => "::block",
            DirectiveKind::ShellBlock => "::shell-block",
            DirectiveKind::EndBlock => "::end-block",
            DirectiveKind::Disclosure => "::disclosure",
            DirectiveKind::Details => "::details",
            DirectiveKind::EndDisclosure => "::end-disclosure",
        }
    }
}

/// A single `key=value` (or bare flag) option on a directive line.
///
/// `value` is `None` for a bare flag such as `--dir` in
/// `::file-links --dir ./notes`. When present, `value.value` is the logical
/// value (quotes stripped, escapes resolved) and `value.span` covers the raw
/// source of the value including any quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveOption {
    /// The option key with its byte span.
    pub key: Spanned<String>,
    /// The option value with its byte span, or `None` for a bare flag.
    pub value: Option<Spanned<String>>,
}

/// A directive line located by [`scan_darkmatter_directives`].
///
/// Every span is a byte-offset range into the scanned source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDirective {
    /// Which directive family this line belongs to.
    pub kind: DirectiveKind,
    /// Full byte span of the directive line (excluding the trailing newline).
    pub span: SourceSpan,
    /// 1-based line number of the directive.
    pub line: usize,
    /// Byte span of the keyword token itself, including the leading `::`.
    pub keyword_span: SourceSpan,
    /// The directive's primary target (a path, glob, or command), when the
    /// family carries one. Absent for closers (`::end-block`,
    /// `::end-disclosure`) and the `::details` divider.
    pub target: Option<Spanned<String>>,
    /// Parsed `key=value` / flag options in source order.
    pub options: Vec<DirectiveOption>,
}

/// Keyword table, longest keyword first so `::file-links` is matched before
/// `::file` and `::end-block` before `::block`.
const KEYWORDS: &[(&str, DirectiveKind)] = &[
    ("::end-disclosure", DirectiveKind::EndDisclosure),
    ("::end-block", DirectiveKind::EndBlock),
    ("::file-links", DirectiveKind::FileLinks),
    ("::toc-linking", DirectiveKind::TocLinking),
    ("::shell-block", DirectiveKind::ShellBlock),
    ("::disclosure", DirectiveKind::Disclosure),
    ("::details", DirectiveKind::Details),
    ("::shell", DirectiveKind::Shell),
    ("::block", DirectiveKind::Block),
    ("::code", DirectiveKind::Code),
    ("::file", DirectiveKind::File),
    ("::url", DirectiveKind::Url),
];

/// Matches the leading directive keyword of a trimmed line at a word boundary.
///
/// Returns the matched [`DirectiveKind`] and the keyword's byte length. A word
/// boundary (whitespace or end-of-line) is required after the keyword so
/// `::filet` or `::block-quote`-style near-misses are rejected.
fn match_keyword(trimmed: &str) -> Option<(DirectiveKind, usize)> {
    for (literal, kind) in KEYWORDS {
        if let Some(rest) = trimmed.strip_prefix(literal)
            && (rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            return Some((*kind, literal.len()));
        }
    }
    None
}

/// Scans `source` for every Darkmatter directive line and returns them in
/// document order with byte-accurate spans.
///
/// Directives inside fenced/inline code regions are skipped. The scan never
/// executes a directive and never errors — a syntactically malformed line still
/// contributes a best-effort [`ParsedDirective`] carrying at least its keyword
/// span. Block openers and closers are reported as individual directives here;
/// use [`scan_darkmatter_blocks`] to pair them with body spans.
pub fn scan_darkmatter_directives(source: &str) -> Vec<ParsedDirective> {
    let code_regions = find_code_regions(source);
    let bytes = source.as_bytes();
    let mut directives = Vec::new();

    let mut line_start = 0usize;
    let mut line_number = 1usize;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let line_end = i;
        let text_line_end = if line_end > line_start && bytes[line_end - 1] == b'\r' {
            line_end - 1
        } else {
            line_end
        };
        let line = &source[line_start..text_line_end];
        let trimmed_start = line_start + (line.len() - line.trim_start().len());
        let trimmed = line.trim_start();
        let trimmed = trimmed.trim_end();

        if let Some((kind, kw_len)) = match_keyword(trimmed)
            && !is_in_code_region(trimmed_start, &code_regions)
        {
            directives.push(parse_directive_line(
                kind,
                kw_len,
                trimmed,
                trimmed_start,
                text_line_end,
                line_number,
            ));
        }

        line_start = i.saturating_add(1);
        line_number += 1;
    }

    directives
}

/// Parses one already-classified directive line into a [`ParsedDirective`].
///
/// `trimmed` is the line with surrounding whitespace removed; `base` is the
/// document byte offset of `trimmed[0]`, so `base + cursor.pos()` is a document
/// offset. `line_span_end` is the document offset of the line's logical end
/// (before any trailing `\r`/`\n`).
fn parse_directive_line(
    kind: DirectiveKind,
    kw_len: usize,
    trimmed: &str,
    base: usize,
    line_span_end: usize,
    line_number: usize,
) -> ParsedDirective {
    let keyword_span = base..base + kw_len;
    let span = base..line_span_end;

    let mut cursor = Cursor::new(trimmed);
    // Advance the cursor past the keyword so span math and option parsing start
    // from the directive body.
    for _ in 0..kw_len {
        cursor.advance();
    }
    cursor.skip_ws();

    let mut target = None;
    let mut options = Vec::new();

    match kind {
        DirectiveKind::File
        | DirectiveKind::Code
        | DirectiveKind::Url
        | DirectiveKind::TocLinking => {
            target = read_target(&mut cursor, base, line_number);
            options = read_options(&mut cursor, trimmed, base, line_number);
        }
        DirectiveKind::FileLinks => {
            // Flag form (`--dir ...`) carries no positional target; glob form
            // does. Everything else is captured as flag/option tokens.
            if !cursor.is_eof() && !trimmed[cursor.pos()..].starts_with("--") {
                target = read_target(&mut cursor, base, line_number);
            }
            options = read_options(&mut cursor, trimmed, base, line_number);
        }
        DirectiveKind::Shell | DirectiveKind::Disclosure => {
            // The remainder is a free-form payload (a shell command, or a
            // disclosure summary that may carry inline style tokens). A
            // structured product for disclosures is exposed separately; here the
            // whole remainder is the target.
            target = read_rest_as_target(&cursor, trimmed, base);
        }
        DirectiveKind::Block => {
            options = read_options(&mut cursor, trimmed, base, line_number);
        }
        DirectiveKind::ShellBlock
        | DirectiveKind::EndBlock
        | DirectiveKind::Details
        | DirectiveKind::EndDisclosure => {
            // Keyword-only forms carry no target or options.
        }
    }

    ParsedDirective {
        kind,
        span,
        line: line_number,
        keyword_span,
        target,
        options,
    }
}

/// Reads a single value token as the directive target, returning `None` when
/// the cursor is at end-of-line or the value is empty.
fn read_target(cursor: &mut Cursor<'_>, base: usize, line: usize) -> Option<Spanned<String>> {
    cursor.skip_ws();
    if cursor.is_eof() {
        return None;
    }
    let start = cursor.pos();
    let value = cursor.read_value(line).ok()?;
    let end = cursor.pos();
    if value.is_empty() {
        return None;
    }
    Some(Spanned::new(value, base + start..base + end))
}

/// Reads the entire remainder of the line as a single target span.
///
/// The cursor is positioned past any leading whitespace and `trimmed` has no
/// trailing whitespace, so `trimmed[start..]` is the exact payload.
fn read_rest_as_target(cursor: &Cursor<'_>, trimmed: &str, base: usize) -> Option<Spanned<String>> {
    let start = cursor.pos();
    let rest = &trimmed[start..];
    if rest.is_empty() {
        return None;
    }
    Some(Spanned::new(rest.to_string(), base + start..base + trimmed.len()))
}

/// Reads the trailing `key=value` / flag options of a directive line.
///
/// Keys are read permissively (up to `=` or whitespace) so dotted keys
/// (`set.name`) and flag keys (`--dir`) round-trip. Values use the shared
/// [`Cursor::read_value`] grammar (quoted, JSON-balanced, or bare), so an option
/// value's span covers its raw source including quotes. `trimmed` is the cursor's
/// backing slice, used to recover raw key text by offset.
fn read_options(
    cursor: &mut Cursor<'_>,
    trimmed: &str,
    base: usize,
    line: usize,
) -> Vec<DirectiveOption> {
    let mut options = Vec::new();

    loop {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        let key_start = cursor.pos();
        read_key_token(cursor);
        let key_end = cursor.pos();
        if key_end == key_start {
            // No progress (e.g. a stray `=`); consume one char to stay live.
            cursor.advance();
            continue;
        }
        let key_text = trimmed[key_start..key_end].to_string();
        let key = Spanned::new(key_text, base + key_start..base + key_end);

        let value = if cursor.current() == Some('=') {
            cursor.advance(); // consume '='
            let val_start = cursor.pos();
            match cursor.read_value(line) {
                Ok(value) => {
                    let val_end = cursor.pos();
                    Some(Spanned::new(value, base + val_start..base + val_end))
                }
                Err(_) => None,
            }
        } else {
            None
        };

        options.push(DirectiveOption { key, value });
    }

    options
}

/// Reads a bare key token — every character up to `=` or ASCII whitespace.
fn read_key_token(cursor: &mut Cursor<'_>) {
    while let Some(ch) = cursor.current() {
        if ch == '=' || ch.is_whitespace() {
            break;
        }
        cursor.advance();
    }
}

/// Which block family a [`DarkmatterBlock`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// A `::block` … `::end-block` page block.
    Page,
    /// A `::shell-block` … `::end-block` shell block.
    Shell,
}

/// A fully-paired `::block` / `::shell-block` region with opener, closer, and
/// body spans.
///
/// The read-only public view of the crate-private block-pair scanner. Spans are
/// byte offsets into the scanned source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DarkmatterBlock {
    /// Which block family opened this pair.
    pub kind: BlockKind,
    /// Full byte span including both the opener and closer directive lines.
    pub span: SourceSpan,
    /// Byte span of the body between the opener and closer, excluding both
    /// directive lines.
    pub body_span: SourceSpan,
    /// The raw text of the opening directive line.
    pub opening_text: String,
    /// 1-based line number of the opening directive.
    pub start_line: usize,
    /// 1-based line number of the closing directive.
    pub end_line: usize,
}

/// An error encountered while pairing block directives.
///
/// The read-only public projection of the crate-private block-pair scanner's
/// error, so a language server can report an unbalanced block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockScanError {
    /// `::end-block` appeared without a matching opener.
    UnmatchedEnd {
        /// 1-based line number of the stray closer.
        line: usize,
    },
    /// End-of-file reached with an unclosed block.
    UnterminatedBlock {
        /// 1-based line number of the unclosed opener.
        line: usize,
        /// The raw opener directive text.
        opening_text: String,
        /// 1-based line number of the last line scanned.
        file_ends_at_line: usize,
    },
    /// `::end-block` carried trailing non-whitespace content.
    TrailingContent {
        /// 1-based line number of the offending closer.
        line: usize,
        /// The trailing content after `::end-block`.
        content: String,
    },
}

impl std::fmt::Display for BlockScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockScanError::UnmatchedEnd { line } => {
                write!(f, "Unmatched ::end-block at line {line}")
            }
            BlockScanError::UnterminatedBlock {
                line,
                file_ends_at_line,
                ..
            } => write!(
                f,
                "Unterminated block starting at line {line} (file ends at line {file_ends_at_line})"
            ),
            BlockScanError::TrailingContent { line, content } => {
                write!(f, "Unexpected content after ::end-block at line {line}: '{content}'")
            }
        }
    }
}

impl std::error::Error for BlockScanError {}

impl From<BlockPairError> for BlockScanError {
    fn from(error: BlockPairError) -> Self {
        match error {
            BlockPairError::UnmatchedEnd { line } => BlockScanError::UnmatchedEnd { line },
            BlockPairError::UnterminatedBlock {
                line,
                opening_text,
                file_ends_at_line,
            } => BlockScanError::UnterminatedBlock {
                line,
                opening_text,
                file_ends_at_line,
            },
            BlockPairError::TrailingContent { line, content } => {
                BlockScanError::TrailingContent { line, content }
            }
        }
    }
}

/// Scans `source` for paired `::block` / `::shell-block` regions and returns
/// them in document order with opener/closer/body byte spans.
///
/// Shares the crate-private block-pair scanner — page and shell blocks live on
/// one stack, so interleaved nesting is paired unambiguously. Directives inside
/// code regions are skipped.
///
/// ## Errors
///
/// Returns [`BlockScanError`] for an unmatched closer, an unterminated
/// opener, or trailing content after `::end-block`.
pub fn scan_darkmatter_blocks(source: &str) -> Result<Vec<DarkmatterBlock>, BlockScanError> {
    let pairs = scan_block_pairs(source)?;
    Ok(pairs
        .into_iter()
        .map(|pair| DarkmatterBlock {
            kind: match pair.kind {
                BlockOpenKind::Page => BlockKind::Page,
                BlockOpenKind::Shell => BlockKind::Shell,
            },
            span: pair.span,
            body_span: pair.body_span,
            opening_text: pair.opening_text,
            start_line: pair.start_line,
            end_line: pair.end_line,
        })
        .collect())
}

/// A single executable command located inside a `::shell-block` body.
///
/// The read-only, span-carrying companion to the compose shell-block executor.
/// `command` mirrors how compose folds the body into logical commands
/// (blank-line skipping, backslash line-continuation folding, block-quote prefix
/// stripping) — its `value` is the folded command text (trimmed) and its `span`
/// is the command's physical byte extent in the scanned document. So a language
/// server classifies exactly the commands `md compose` would run, without
/// executing any of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellBlockBodyCommand {
    /// The folded command text with its document byte span.
    pub command: Spanned<String>,
    /// 1-based line number where the command starts.
    pub line: usize,
}

/// Scans `source` for every executable command inside a `::shell-block` body.
///
/// Locates shell blocks via [`scan_darkmatter_blocks`], then folds each body
/// through the same logical-command splitting the compose executor uses, so the
/// reported commands match what `md compose` would run — minus the execution.
/// Spans are document byte offsets. A block-pairing error (surfaced by
/// [`scan_darkmatter_blocks`]) yields no commands here.
///
/// Unlike the executor this scan never tokenizes or errors: an unterminated
/// backslash continuation still contributes its best-effort folded command, and
/// a command the shell tokenizer would reject (a pipe, a `;` chain) is returned
/// verbatim for policy classification rather than dropped.
pub fn scan_shell_block_commands(source: &str) -> Vec<ShellBlockBodyCommand> {
    let Ok(blocks) = scan_darkmatter_blocks(source) else {
        return Vec::new();
    };
    let mut commands = Vec::new();
    for block in blocks.into_iter().filter(|block| block.kind == BlockKind::Shell) {
        let body = &source[block.body_span.clone()];
        let base = block.body_span.start;
        // Body lines begin on the line after the `::shell-block` opener.
        let (groups, dangling) = group_logical_commands(body, block.start_line + 1);
        for group in groups.into_iter().chain(dangling) {
            let text = group.raw_command.trim().to_string();
            if text.is_empty() {
                continue;
            }
            // Tighten the physical span to the command's non-whitespace extent so
            // a hover/diagnostic range excludes leading indentation and trailing
            // spaces. For a single-line command the range then equals `text`; a
            // continuation-folded command still spans its full physical extent.
            let raw = base + group.physical_span.start..base + group.physical_span.end;
            let slice = &source[raw.clone()];
            let lead = slice.len() - slice.trim_start().len();
            let trail = slice.len() - slice.trim_end().len();
            let span = raw.start + lead..raw.end - trail;
            commands.push(ShellBlockBodyCommand {
                command: Spanned::new(text, span),
                line: group.start_line,
            });
        }
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<DirectiveKind> {
        scan_darkmatter_directives(source)
            .into_iter()
            .map(|d| d.kind)
            .collect()
    }

    #[test]
    fn scans_file_directive_target_and_options() {
        let source = "::file ./doc.md when=env.DEBUG quotation=true\n";
        let directives = scan_darkmatter_directives(source);
        assert_eq!(directives.len(), 1);
        let d = &directives[0];
        assert_eq!(d.kind, DirectiveKind::File);
        assert_eq!(&source[d.keyword_span.clone()], "::file");
        let target = d.target.as_ref().unwrap();
        assert_eq!(target.value, "./doc.md");
        assert_eq!(&source[target.span.clone()], "./doc.md");
        assert_eq!(d.options.len(), 2);
        assert_eq!(d.options[0].key.value, "when");
        assert_eq!(&source[d.options[0].key.span.clone()], "when");
        assert_eq!(d.options[0].value.as_ref().unwrap().value, "env.DEBUG");
        assert_eq!(d.options[1].key.value, "quotation");
        assert_eq!(d.options[1].value.as_ref().unwrap().value, "true");
    }

    #[test]
    fn quoted_option_value_span_includes_quotes() {
        let source = r#"::file ./doc.md when="a == b""#;
        let directives = scan_darkmatter_directives(source);
        let opt = &directives[0].options[0];
        // Logical value strips the quotes; the span covers the raw quoted source.
        assert_eq!(opt.value.as_ref().unwrap().value, "a == b");
        assert_eq!(
            &source[opt.value.as_ref().unwrap().span.clone()],
            r#""a == b""#
        );
    }

    #[test]
    fn indented_directive_spans_are_document_relative() {
        let source = "text\n    ::code ./mod.rs\n";
        let directives = scan_darkmatter_directives(source);
        assert_eq!(directives.len(), 1);
        let d = &directives[0];
        assert_eq!(d.kind, DirectiveKind::Code);
        assert_eq!(d.line, 2);
        assert_eq!(&source[d.keyword_span.clone()], "::code");
        assert_eq!(d.target.as_ref().unwrap().value, "./mod.rs");
    }

    #[test]
    fn shell_directive_target_is_the_whole_command() {
        let source = "::shell echo hello world\n";
        let directives = scan_darkmatter_directives(source);
        assert_eq!(directives[0].kind, DirectiveKind::Shell);
        assert_eq!(
            directives[0].target.as_ref().unwrap().value,
            "echo hello world"
        );
    }

    #[test]
    fn keyword_boundary_rejects_near_misses() {
        // `::filet` and `::block-quote` share a prefix but are not directives.
        let source = "::filet ./x.md\n::block-quote\n";
        assert!(scan_darkmatter_directives(source).is_empty());
    }

    #[test]
    fn unknown_directive_is_not_surfaced_but_recognized_one_is() {
        // A `::`-prefixed line that is not one of the twelve keywords never
        // produces a `ParsedDirective`, so it stays distinguishable from a
        // recognized directive (semantic tokens style only the recognized set).
        let source = "::frobnicate ./x.md\n::file ./y.md\n";
        let directives = scan_darkmatter_directives(source);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, DirectiveKind::File);
    }

    #[test]
    fn file_links_prefix_wins_over_file() {
        let source = "::file-links ./notes\n";
        let directives = scan_darkmatter_directives(source);
        assert_eq!(directives[0].kind, DirectiveKind::FileLinks);
        assert_eq!(directives[0].target.as_ref().unwrap().value, "./notes");
    }

    #[test]
    fn file_links_flag_form_has_no_positional_target() {
        let source = "::file-links --dir ./notes --depth 2\n";
        let directives = scan_darkmatter_directives(source);
        let d = &directives[0];
        assert!(d.target.is_none());
        assert_eq!(d.options[0].key.value, "--dir");
    }

    #[test]
    fn directives_inside_fenced_code_are_skipped() {
        let source = "```\n::file ./ignored.md\n```\n::file ./included.md\n";
        let directives = scan_darkmatter_directives(source);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].target.as_ref().unwrap().value, "./included.md");
    }

    #[test]
    fn scans_end_and_divider_keywords_without_target() {
        let source = "::disclosure Summary\n::details\ntext\n::end-disclosure\n";
        let ks = kinds(source);
        assert_eq!(
            ks,
            vec![
                DirectiveKind::Disclosure,
                DirectiveKind::Details,
                DirectiveKind::EndDisclosure,
            ]
        );
        let directives = scan_darkmatter_directives(source);
        assert_eq!(directives[0].target.as_ref().unwrap().value, "Summary");
        assert!(directives[1].target.is_none());
        assert!(directives[2].target.is_none());
    }

    #[test]
    fn block_scan_pairs_opener_and_closer_spans() {
        let source = "::block when=\"x\"\nbody line\n::end-block\n";
        let blocks = scan_darkmatter_blocks(source).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, BlockKind::Page);
        assert_eq!(&source[blocks[0].body_span.clone()], "body line\n");
        assert_eq!(blocks[0].start_line, 1);
        assert_eq!(blocks[0].end_line, 3);
    }

    #[test]
    fn block_scan_reports_unmatched_end() {
        let source = "::end-block\n";
        let err = scan_darkmatter_blocks(source).unwrap_err();
        assert!(matches!(err, BlockScanError::UnmatchedEnd { line: 1 }));
    }

    #[test]
    fn block_scan_mixes_page_and_shell() {
        let source = "::block\n::shell-block\nbody\n::end-block\n::end-block\n";
        let blocks = scan_darkmatter_blocks(source).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].kind, BlockKind::Shell);
        assert_eq!(blocks[1].kind, BlockKind::Page);
    }

    #[test]
    fn shell_block_commands_fold_body_lines() {
        let source = "# Doc\n\n::shell-block\nrm -rf /tmp/x\necho hi\n::end-block\n";
        let commands = scan_shell_block_commands(source);
        let texts: Vec<&str> = commands.iter().map(|c| c.command.value.as_str()).collect();
        assert_eq!(texts, vec!["rm -rf /tmp/x", "echo hi"]);
        for command in &commands {
            assert_eq!(&source[command.command.span.clone()], command.command.value);
        }
        assert_eq!(commands[0].line, 4);
        assert_eq!(commands[1].line, 5);
    }

    #[test]
    fn shell_block_commands_fold_line_continuations() {
        // A trailing `\` folds the next line into one logical command (compose
        // parity), so policy sees the whole command, not two fragments.
        let source = "::shell-block\necho \\\n  hello\n::end-block\n";
        let commands = scan_shell_block_commands(source);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command.value, "echo   hello");
        assert_eq!(commands[0].line, 2);
    }

    #[test]
    fn shell_block_commands_strip_blockquote_markers() {
        // Security parity: a `>`-quoted dangerous command exposes its real
        // executable (`rm`), not the `>` marker.
        let source = "> ::shell-block\n> rm -rf /\n> ::end-block\n";
        let commands = scan_shell_block_commands(source);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command.value, "rm -rf /");
        assert_eq!(&source[commands[0].command.span.clone()], "rm -rf /");
    }

    #[test]
    fn shell_block_commands_ignore_page_blocks() {
        let source = "::block when=\"true\"\nrm -rf /\n::end-block\n";
        assert!(scan_shell_block_commands(source).is_empty());
    }

    #[test]
    fn shell_block_commands_surface_unterminated_continuation() {
        // The executor rejects a dangling `\`; the passive scan still surfaces
        // the best-effort folded command so policy can classify it.
        let source = "::shell-block\nrm -rf / \\\n::end-block\n";
        let commands = scan_shell_block_commands(source);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command.value, "rm -rf /");
    }

    #[test]
    fn shell_block_commands_match_compose_splitter() {
        // Compose-parity golden: the passive scan yields the same folded
        // commands (order + text) the compose executor's own splitter produces.
        use crate::markdown::compose::shell_blocks::body::split_logical_commands;
        let source = "::shell-block\necho one\nmake \\\n  install\necho three\n::end-block\n";
        let block = &scan_darkmatter_blocks(source).unwrap()[0];
        let body = &source[block.body_span.clone()];
        let composed = split_logical_commands(body, block.start_line + 1).unwrap();
        let scanned = scan_shell_block_commands(source);
        assert_eq!(scanned.len(), composed.len());
        for (scanned, composed) in scanned.iter().zip(composed.iter()) {
            assert_eq!(scanned.command.value, composed.raw_command.trim());
            assert_eq!(scanned.line, composed.start_line);
        }
    }

    /// Compose-parity goldens: the passive span scanner must agree with compose's
    /// own semantic directive parser on which directives exist and their
    /// targets/options, so the language server never disagrees with `md compose`
    /// about what a directive line says.
    mod parity {
        use super::*;
        use crate::markdown::compose::transclusion::{
            self, DirectiveKind as TxDirectiveKind,
        };
        use biscuit_terminal::errors::SourceContext;
        use std::path::PathBuf;
        use std::sync::Arc;

        fn ctx(content: &str) -> SourceContext {
            SourceContext::new(
                PathBuf::from("/t.md"),
                PathBuf::from("t.md"),
                Arc::from(content),
            )
        }

        fn kinds_agree(scan: DirectiveKind, tx: TxDirectiveKind) -> bool {
            matches!(
                (scan, tx),
                (DirectiveKind::File, TxDirectiveKind::File)
                    | (DirectiveKind::Code, TxDirectiveKind::Code)
                    | (DirectiveKind::Url, TxDirectiveKind::Url)
            )
        }

        const TRANSCLUSION_FIXTURES: &[&str] = &[
            "::file ./doc.md\n",
            "::code ./mod.rs quotation=true\n",
            "::file ./doc.md when=env.DEBUG\n",
            r#"::file ./doc.md when="stage == 'plan'""#,
            "::url https://example.com/x.md\n",
            "text before\n::file ./a.md\ntext\n::code ./b.rs\n",
        ];

        #[test]
        fn scan_agrees_with_transclusion_parser_on_kind_and_target() {
            for fixture in TRANSCLUSION_FIXTURES {
                let scanned: Vec<_> = scan_darkmatter_directives(fixture)
                    .into_iter()
                    .filter(|d| {
                        matches!(
                            d.kind,
                            DirectiveKind::File | DirectiveKind::Code | DirectiveKind::Url
                        )
                    })
                    .collect();
                let semantic = transclusion::parse_directives(fixture, ctx(fixture)).unwrap();
                assert_eq!(scanned.len(), semantic.len(), "count mismatch for {fixture:?}");
                for (s, t) in scanned.iter().zip(semantic.iter()) {
                    assert!(kinds_agree(s.kind, t.kind), "kind mismatch for {fixture:?}");
                    assert_eq!(
                        s.target.as_ref().map(|sp| sp.value.as_str()),
                        Some(t.raw_target.as_str()),
                        "target mismatch for {fixture:?}"
                    );
                }
            }
        }

        #[test]
        fn scan_agrees_on_when_option_value() {
            for fixture in [
                "::file ./doc.md when=env.DEBUG\n",
                r#"::file ./doc.md when="stage == 'plan'""#,
            ] {
                let scanned = &scan_darkmatter_directives(fixture)[0];
                let semantic = &transclusion::parse_directives(fixture, ctx(fixture)).unwrap()[0];
                let scanned_when = scanned
                    .options
                    .iter()
                    .find(|o| o.key.value == "when")
                    .and_then(|o| o.value.as_ref().map(|v| v.value.clone()));
                assert_eq!(
                    scanned_when, semantic.options.when_expr,
                    "when mismatch for {fixture:?}"
                );
            }
        }
    }
}
