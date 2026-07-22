//! Context-aware lexical source map for YAML text.
//!
//! [`SourceMap`] is a single-pass, line-oriented lexical scan. It records
//! line classifications, block mapping/sequence entries, flow-collection
//! regions, quoted scalar styles, comments, block scalars, anchor/alias
//! occurrences, and document markers, each with exact UTF-8 byte spans. It
//! never reparses or reserializes the document, and it stays useful on input
//! that fails to parse — which is exactly where repair analysis needs it.
//!
//! The map is deliberately lexical, not grammatical: entries describe what
//! the source *looks like*, and every repair decision is ultimately proven
//! against the real parser (`serde_yaml_ng`) by the analysis engine.

use crate::span::SourceSpan;

/// Lexical classification of a source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineKind {
    /// No content bytes (possibly whitespace only).
    Blank,
    /// First non-whitespace byte is `#`.
    Comment,
    /// `---` document-start marker line.
    DocumentStart,
    /// `...` document-end marker line.
    DocumentEnd,
    /// Any other line.
    Content,
    /// A line belonging to a block scalar's content (reclassified during
    /// block-scalar detection, including lines that would otherwise look
    /// like comments or blanks).
    BlockContent,
}

/// A source line with byte spans and lexical classification.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    /// Whole line including any line terminator.
    pub span: SourceSpan,
    /// Line content excluding the terminator (`\n`, `\r\n`, or `\r`).
    pub content: SourceSpan,
    /// Bytes of leading whitespace before the first content byte.
    pub indent: usize,
    /// Lexical classification.
    pub kind: LineKind,
}

/// Kind of a flow collection region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlowKind {
    /// `[ ... ]`
    Sequence,
    /// `{ ... }`
    Mapping,
}

/// A flow-collection region, from the opening bracket to the closing
/// bracket (inclusive). Unclosed regions extend to the end of the source.
#[derive(Debug, Clone)]
pub(crate) struct FlowRegion {
    /// Byte span covering both brackets (or to end of source when unclosed).
    pub span: SourceSpan,
    /// Sequence or mapping.
    pub kind: FlowKind,
    /// Whether a closing bracket was found.
    pub closed: bool,
}

/// Quoting style of a quoted scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuoteStyle {
    /// `'...'` (no escapes; `''` is a literal quote).
    Single,
    /// `"..."` (backslash escapes).
    Double,
}

/// A quoted scalar occurrence, including the quote characters.
#[derive(Debug, Clone)]
pub(crate) struct QuotedScalar {
    /// Byte span including the surrounding quotes.
    pub span: SourceSpan,
    /// Single or double quoted.
    pub style: QuoteStyle,
    /// Whether the closing quote was found.
    pub closed: bool,
}

/// A block scalar (`|` or `>` header plus its content lines).
#[derive(Debug, Clone)]
pub(crate) struct BlockScalar {
    /// Index of the header line (the line carrying `|` or `>`).
    pub header_line: usize,
    /// Byte offset of the `|` or `>` indicator.
    pub indicator: usize,
    /// Byte span covering all content lines (empty when there are none).
    pub content: SourceSpan,
}

/// Anchor definition versus alias reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnchorKind {
    /// `&name` — declares an anchor.
    Anchor,
    /// `*name` — references an anchor.
    Alias,
}

/// An anchor or alias occurrence.
#[derive(Debug, Clone)]
pub(crate) struct AnchorRef {
    /// Anchor or alias.
    pub kind: AnchorKind,
    /// Byte span of the name text (after the indicator).
    pub name: SourceSpan,
    /// Byte span including the `&` or `*` indicator.
    pub full: SourceSpan,
}

/// Document marker kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerKind {
    /// `---`
    Start,
    /// `...`
    End,
}

/// A document marker line.
#[derive(Debug, Clone)]
pub(crate) struct DocumentMarker {
    /// Byte span of the marker line's content.
    pub span: SourceSpan,
    /// Start or end marker.
    pub kind: MarkerKind,
}

/// A block-mapping entry located on a content line (`key: value`).
#[derive(Debug, Clone)]
pub(crate) struct MappingEntry {
    /// Byte span of the key text, excluding trailing whitespace before the
    /// colon.
    pub key: SourceSpan,
    /// Byte offset of the mapping colon.
    pub colon: usize,
    /// Value region after the colon (trimmed, comment-excluded), when the
    /// entry has an inline value.
    pub value: Option<SourceSpan>,
}

/// Lexical entries discovered on a content line.
#[derive(Debug, Clone, Default)]
pub(crate) struct LineEntry {
    /// Byte offset of the sequence dash when the line is a block-sequence
    /// entry (`- `).
    pub dash: Option<usize>,
    /// Value region after the dash (trimmed, comment-excluded).
    pub dash_value: Option<SourceSpan>,
    /// Block-mapping entry on the line; for sequence lines this is the
    /// inline mapping within the entry (`- key: value`).
    pub mapping: Option<MappingEntry>,
}

/// One segment of a lexical context path into the parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PathSegment {
    /// A mapping key.
    Key(String),
    /// A sequence index.
    Index(usize),
}

/// The lexical context of a block mapping value or sequence entry: where the
/// value lives in the parsed document and which source bytes hold it.
#[derive(Debug, Clone)]
pub(crate) struct BlockValueContext {
    /// Path from the document root to the value node.
    pub path: Vec<PathSegment>,
    /// Byte span of the raw value text (single line, trailing whitespace
    /// excluded, comments included).
    pub lexeme: SourceSpan,
}

/// A block-mapping key occurrence with its lexical scope path.
///
/// The scope path identifies the mapping the key belongs to: duplicate keys
/// share the full path (scope plus key), and similar-key analysis compares
/// keys within one scope and across sibling scopes. Paths are lexical —
/// they are available even when the document fails to parse.
#[derive(Debug, Clone)]
pub(crate) struct KeyOccurrence {
    /// Path from the document root to this key, inclusive.
    pub path: Vec<PathSegment>,
    /// Byte span of the authored key text.
    pub key_span: SourceSpan,
    /// The key text (simply-quoted keys are unquoted).
    pub key_text: String,
}

/// The full lexical source map for one YAML source text.
#[derive(Debug)]
pub(crate) struct SourceMap {
    lines: Vec<Line>,
    entries: Vec<LineEntry>,
    flow_regions: Vec<FlowRegion>,
    block_scalars: Vec<BlockScalar>,
    comments: Vec<SourceSpan>,
    quoted: Vec<QuotedScalar>,
    anchors: Vec<AnchorRef>,
    markers: Vec<DocumentMarker>,
}

impl SourceMap {
    /// Scans `source` into a lexical source map.
    pub(crate) fn new(source: &str) -> Self {
        let mut lines = split_lines(source);
        let mut markers = Vec::new();
        classify_lines(source, &mut lines, &mut markers);
        let block_scalars = detect_block_scalars(source, &mut lines);
        let scan = scan_bytes(source, &lines);
        let entries = extract_entries(source, &lines, &scan.flow_regions, &scan.quoted, &scan.comments);
        Self {
            lines,
            entries,
            flow_regions: scan.flow_regions,
            block_scalars,
            comments: scan.comments,
            quoted: scan.quoted,
            anchors: scan.anchors,
            markers,
        }
    }

    /// All lines in source order.
    pub(crate) fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Lexical entries for `line`.
    pub(crate) fn entry(&self, line: usize) -> &LineEntry {
        &self.entries[line]
    }

    /// Flow-collection regions in source order.
    pub(crate) fn flow_regions(&self) -> &[FlowRegion] {
        &self.flow_regions
    }

    /// Block scalars in source order.
    pub(crate) fn block_scalars(&self) -> &[BlockScalar] {
        &self.block_scalars
    }

    /// Comment spans in source order.
    pub(crate) fn comments(&self) -> &[SourceSpan] {
        &self.comments
    }

    /// Quoted scalar occurrences in source order.
    pub(crate) fn quoted_scalars(&self) -> &[QuotedScalar] {
        &self.quoted
    }

    /// Anchor and alias occurrences in source order.
    pub(crate) fn anchors(&self) -> &[AnchorRef] {
        &self.anchors
    }

    /// Document markers in source order.
    pub(crate) fn markers(&self) -> &[DocumentMarker] {
        &self.markers
    }

    /// Returns the index of the line containing `byte`. A byte offset at the
    /// very end of the source maps to the last line.
    pub(crate) fn line_at_byte(&self, byte: usize) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        for (index, line) in self.lines.iter().enumerate() {
            if byte < line.span.end {
                return Some(index);
            }
        }
        Some(self.lines.len() - 1)
    }

    /// Returns `true` when `byte` falls strictly inside a flow-collection
    /// region.
    pub(crate) fn in_flow(&self, byte: usize) -> bool {
        self.flow_regions
            .iter()
            .any(|region| region.span.start < byte && byte < region.span.end)
    }

    /// Returns `true` when `span` intersects any flow-collection region.
    pub(crate) fn flow_intersects(&self, span: &SourceSpan) -> bool {
        self.flow_regions
            .iter()
            .any(|region| span.start < region.span.end && region.span.start < span.end)
    }

    /// Returns `true` when `span` intersects any quoted scalar.
    pub(crate) fn quoted_intersects(&self, span: &SourceSpan) -> bool {
        self.quoted
            .iter()
            .any(|quoted| span.start < quoted.span.end && quoted.span.start < span.end)
    }

    /// Computes the block value context for `line`: the lexical path to the
    /// value and the raw lexeme span holding it.
    ///
    /// The lexeme runs from the first non-space byte after the mapping colon
    /// or sequence dash to the last non-trailing-whitespace byte of the line,
    /// per the ratified bounded grammar; comments are *not* trimmed from it.
    /// Returns `None` when the line is not a block mapping value or sequence
    /// entry, when there is no inline value, or when the context path cannot
    /// be represented (e.g. a non-plain ancestor key).
    pub(crate) fn block_value_context(&self, source: &str, line: usize) -> Option<BlockValueContext> {
        if self.lines[line].kind != LineKind::Content {
            return None;
        }
        let entry = &self.entries[line];
        let indent = self.lines[line].indent;
        let content_end = trim_end(source, self.lines[line].content.clone());

        let (mut path, value_start) = if let Some(mapping) = &entry.mapping {
            let key = plain_key_text(source, &mapping.key)?;
            let mut path = self.parent_path(source, line, indent)?;
            if let Some(dash) = entry.dash {
                let _ = dash;
                path.push(PathSegment::Index(self.sequence_index(line)));
            }
            path.push(PathSegment::Key(key));
            (path, mapping.value.as_ref()?.start)
        } else if entry.dash.is_some() {
            let mut path = self.parent_path(source, line, indent)?;
            path.push(PathSegment::Index(self.sequence_index(line)));
            (path, entry.dash_value.as_ref()?.start)
        } else {
            return None;
        };

        if value_start >= content_end {
            return None;
        }
        path.shrink_to_fit();
        Some(BlockValueContext {
            path,
            lexeme: value_start..content_end,
        })
    }

    /// Parent path for a line at `indent`: walks upward through strictly
    /// less-indented content lines, recording mapping keys and sequence
    /// indices. Returns `None` when an ancestor key is not a plain scalar or
    /// the nesting is not representable.
    fn parent_path(&self, source: &str, line: usize, indent: usize) -> Option<Vec<PathSegment>> {
        if indent == 0 {
            return Some(Vec::new());
        }
        let mut parent = None;
        for candidate in (0..line).rev() {
            let candidate_line = &self.lines[candidate];
            if candidate_line.kind != LineKind::Content {
                continue;
            }
            if candidate_line.indent < indent {
                parent = Some(candidate);
                break;
            }
        }
        let parent = parent?;
        let parent_line = &self.lines[parent];
        let parent_entry = &self.entries[parent];
        let mut path = self.parent_path(source, parent, parent_line.indent)?;
        if parent_entry.dash.is_some() {
            path.push(PathSegment::Index(self.sequence_index(parent)));
        }
        if let Some(mapping) = &parent_entry.mapping {
            if mapping.value.is_none() {
                // An empty value opens the block this line's children live in.
                path.push(PathSegment::Key(plain_key_text(source, &mapping.key)?));
            }
            // A mapping with an inline value cannot parent deeper lines; the
            // sequence index above (when present) is the only ancestor.
        } else if parent_entry.dash.is_none() {
            // A content line with neither dash nor mapping (e.g. a plain
            // scalar continuation) cannot anchor a context path.
            return None;
        }
        Some(path)
    }

    /// Every block-mapping key occurrence with its lexical scope path, in
    /// source order. Keys whose text cannot be represented (e.g. quoted keys
    /// with escapes) are skipped — detection precision on such documents is
    /// left to the parser diagnostic.
    pub(crate) fn key_occurrences(&self, source: &str) -> Vec<KeyOccurrence> {
        let mut occurrences = Vec::new();
        for (index, line) in self.lines.iter().enumerate() {
            if line.kind != LineKind::Content {
                continue;
            }
            let Some(mapping) = &self.entries[index].mapping else {
                continue;
            };
            let Some(key_text) = plain_key_text(source, &mapping.key) else {
                continue;
            };
            let mut path = self.parent_path(source, index, line.indent);
            if let Some(path) = &mut path {
                if self.entries[index].dash.is_some() {
                    path.push(PathSegment::Index(self.sequence_index(index)));
                }
                path.push(PathSegment::Key(key_text.clone()));
            }
            if let Some(path) = path {
                occurrences.push(KeyOccurrence {
                    path,
                    key_span: mapping.key.clone(),
                    key_text,
                });
            }
        }
        occurrences
    }

    /// Number of preceding sibling sequence entries at the same indentation,
    /// giving `line`'s index within its parent sequence.
    fn sequence_index(&self, line: usize) -> usize {
        let indent = self.lines[line].indent;
        let mut index = 0;
        for candidate in (0..line).rev() {
            let candidate_line = &self.lines[candidate];
            if candidate_line.kind != LineKind::Content {
                continue;
            }
            if candidate_line.indent > indent {
                continue;
            }
            if candidate_line.indent < indent {
                break;
            }
            if self.entries[candidate].dash.is_some() {
                index += 1;
            } else {
                break;
            }
        }
        index
    }
}

/// Byte walk products shared with the entry extractor.
struct ByteScan {
    flow_regions: Vec<FlowRegion>,
    comments: Vec<SourceSpan>,
    quoted: Vec<QuotedScalar>,
    anchors: Vec<AnchorRef>,
}

/// Splits `source` into lines, handling `\n`, `\r\n`, and lone `\r`
/// terminators.
fn split_lines(source: &str) -> Vec<Line> {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                lines.push(build_line(source, start, index, index + 1));
                index += 1;
                start = index;
            }
            b'\r' => {
                if bytes.get(index + 1) == Some(&b'\n') {
                    lines.push(build_line(source, start, index, index + 2));
                    index += 2;
                } else {
                    lines.push(build_line(source, start, index, index + 1));
                    index += 1;
                }
                start = index;
            }
            _ => index += 1,
        }
    }
    if start < bytes.len() {
        lines.push(build_line(source, start, bytes.len(), bytes.len()));
    }
    lines
}

fn build_line(source: &str, span_start: usize, content_end: usize, span_end: usize) -> Line {
    let bytes = source.as_bytes();
    let mut indent = 0;
    while span_start + indent < content_end
        && matches!(bytes[span_start + indent], b' ' | b'\t')
    {
        indent += 1;
    }
    Line {
        span: span_start..span_end,
        content: span_start..content_end,
        indent,
        kind: LineKind::Content,
    }
}

/// Classifies blank, comment, and document-marker lines; records markers.
///
/// Document markers are only markers at zero indentation; an indented
/// `---`/`...` is scalar content (e.g. inside a block scalar).
fn classify_lines(source: &str, lines: &mut [Line], markers: &mut Vec<DocumentMarker>) {
    let bytes = source.as_bytes();
    for line in lines.iter_mut() {
        if line.content.start + line.indent >= line.content.end {
            line.kind = LineKind::Blank;
            continue;
        }
        let content = &bytes[line.content.clone()];
        let rest = &content[line.indent..];
        if rest[0] == b'#' {
            line.kind = LineKind::Comment;
        } else if line.indent == 0 && marker_matches(rest, b"---") {
            line.kind = LineKind::DocumentStart;
            markers.push(DocumentMarker {
                span: line.content.clone(),
                kind: MarkerKind::Start,
            });
        } else if line.indent == 0 && marker_matches(rest, b"...") {
            line.kind = LineKind::DocumentEnd;
            markers.push(DocumentMarker {
                span: line.content.clone(),
                kind: MarkerKind::End,
            });
        } else {
            line.kind = LineKind::Content;
        }
    }
}

/// A document marker is exactly `---`/`...` followed by end of content or a
/// whitespace byte.
fn marker_matches(rest: &[u8], marker: &[u8; 3]) -> bool {
    rest.len() >= 3 && rest[..3] == *marker && (rest.len() == 3 || matches!(rest[3], b' ' | b'\t'))
}

/// End offset of `content` with trailing spaces and tabs removed.
fn trim_end(source: &str, content: SourceSpan) -> usize {
    let bytes = source.as_bytes();
    let mut end = content.end;
    while end > content.start && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    end
}

/// Detects block scalar headers and reclassifies their content lines.
///
/// A header is a content line whose value position holds `|` or `>`
/// followed only by chomping/indentation modifiers, whitespace, and an
/// optional comment. Root-level headers (a line that is only the indicator)
/// are recognized the same way. Content extends over following lines while
/// they are blank, comment-looking, or indented deeper than the header; the
/// first less-indented content line ends the scalar.
fn detect_block_scalars(source: &str, lines: &mut [Line]) -> Vec<BlockScalar> {
    let mut scalars = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].kind != LineKind::Content {
            index += 1;
            continue;
        }
        if let Some(indicator) = block_scalar_indicator(source, &lines[index]) {
            let header_indent = lines[index].indent;
            let content_start = lines.get(index + 1).map(|line| line.span.start);
            let mut last = index;
            let mut probe = index + 1;
            while probe < lines.len() {
                let line = &lines[probe];
                let is_content = match line.kind {
                    LineKind::Blank => true,
                    LineKind::Comment | LineKind::Content => line.indent > header_indent,
                    LineKind::DocumentStart | LineKind::DocumentEnd | LineKind::BlockContent => false,
                };
                if !is_content {
                    break;
                }
                lines[probe].kind = LineKind::BlockContent;
                last = probe;
                probe += 1;
            }
            let content = match (content_start, probe > index + 1) {
                (Some(start), true) => start..lines[last].span.end,
                _ => {
                    let end = lines[index].span.end;
                    end..end
                }
            };
            scalars.push(BlockScalar {
                header_line: index,
                indicator,
                content,
            });
            index = probe;
        } else {
            index += 1;
        }
    }
    scalars
}

/// Returns the byte offset of a block-scalar indicator when `line` is a
/// block scalar header.
fn block_scalar_indicator(source: &str, line: &Line) -> Option<usize> {
    let bytes = source.as_bytes();
    let content = &bytes[line.content.clone()];
    let mut value_start = None;

    if content.get(line.indent) == Some(&b'-')
        && matches!(content.get(line.indent + 1), Some(b' ') | Some(b'\t'))
    {
        // `- |` form.
        let mut after = line.indent + 1;
        while matches!(content.get(after), Some(b' ') | Some(b'\t')) {
            after += 1;
        }
        value_start = Some(after);
    } else {
        // `key: |` form — the colon must be followed by whitespace or end of
        // content to count as a mapping colon.
        let mut probe = line.indent;
        while probe < content.len() {
            if content[probe] == b':'
                && matches!(content.get(probe + 1), Some(b' ') | Some(b'\t') | None)
            {
                let mut after = probe + 1;
                while matches!(content.get(after), Some(b' ') | Some(b'\t')) {
                    after += 1;
                }
                value_start = Some(after);
                break;
            }
            probe += 1;
        }
        // Root-level header: the whole line is the indicator plus modifiers.
        if value_start.is_none() {
            value_start = Some(line.indent);
        }
    }
    let start = value_start?;
    if !matches!(content.get(start), Some(b'|') | Some(b'>')) {
        return None;
    }
    // After the indicator only chomping (`+`/`-`) and indentation (digit)
    // modifiers, whitespace, and an optional comment may follow.
    let mut rest = start + 1;
    while matches!(content.get(rest), Some(b'+') | Some(b'-') | Some(b'0'..=b'9')) {
        rest += 1;
    }
    while matches!(content.get(rest), Some(b' ') | Some(b'\t')) {
        rest += 1;
    }
    if rest < content.len() && content[rest] != b'#' {
        return None;
    }
    Some(line.content.start + start)
}

/// Single byte walk over the source collecting quoted scalars, comments,
/// flow regions, and anchor/alias occurrences. Block-content bytes are
/// skipped: inside a block scalar, quotes, brackets, and `#` are literal
/// text.
fn scan_bytes(source: &str, lines: &[Line]) -> ByteScan {
    let bytes = source.as_bytes();
    let mut flow_regions: Vec<FlowRegion> = Vec::new();
    let mut comments = Vec::new();
    let mut quoted = Vec::new();
    let mut anchors = Vec::new();
    let mut stack: Vec<(usize, FlowKind, usize)> = Vec::new();
    let mut block_line = 0usize;

    let mut index = 0;
    while index < bytes.len() {
        // Skip block-scalar content: advance the block-line cursor and jump.
        while block_line < lines.len() && index >= lines[block_line].span.end {
            block_line += 1;
        }
        if block_line < lines.len()
            && lines[block_line].kind == LineKind::BlockContent
            && index >= lines[block_line].span.start
        {
            index = lines[block_line].span.end;
            continue;
        }

        match bytes[index] {
            b'\'' | b'"' if at_token_boundary(bytes, index) => {
                let style = if bytes[index] == b'\'' {
                    QuoteStyle::Single
                } else {
                    QuoteStyle::Double
                };
                let (end, closed) = scan_quoted(bytes, index, style);
                quoted.push(QuotedScalar {
                    span: index..end,
                    style,
                    closed,
                });
                index = end;
            }
            b'#' if at_word_boundary(bytes, index) => {
                let end = line_end_of(bytes, index);
                comments.push(index..end);
                index = end;
            }
            b'[' | b'{' => {
                let kind = if bytes[index] == b'[' {
                    FlowKind::Sequence
                } else {
                    FlowKind::Mapping
                };
                stack.push((index, kind, stack.len()));
                index += 1;
            }
            b']' | b'}' => {
                let kind = if bytes[index] == b']' {
                    FlowKind::Sequence
                } else {
                    FlowKind::Mapping
                };
                if let Some(position) = stack.iter().rposition(|(_, open, _)| *open == kind) {
                    let (open, open_kind, _) = stack.remove(position);
                    flow_regions.push(FlowRegion {
                        span: open..index + 1,
                        kind: open_kind,
                        closed: true,
                    });
                }
                index += 1;
            }
            b'&' | b'*' if at_anchor_position(bytes, index) => {
                let name_start = index + 1;
                let mut name_end = name_start;
                while name_end < bytes.len() && is_anchor_char(bytes[name_end]) {
                    name_end += 1;
                }
                if name_end > name_start {
                    anchors.push(AnchorRef {
                        kind: if bytes[index] == b'&' {
                            AnchorKind::Anchor
                        } else {
                            AnchorKind::Alias
                        },
                        name: name_start..name_end,
                        full: index..name_end,
                    });
                }
                index = name_end;
            }
            _ => index += 1,
        }
    }

    for (open, kind, _) in stack {
        flow_regions.push(FlowRegion {
            span: open..bytes.len(),
            kind,
            closed: false,
        });
    }
    flow_regions.sort_by_key(|region| (region.span.start, region.span.end));

    ByteScan {
        flow_regions,
        comments,
        quoted,
        anchors,
    }
}

/// Scans a quoted scalar starting at `start` (the opening quote). Returns
/// the end offset (one past the closing quote, or end of source) and whether
/// the scalar was closed. Quoted scalars may span lines.
fn scan_quoted(bytes: &[u8], start: usize, style: QuoteStyle) -> (usize, bool) {
    let mut index = start + 1;
    match style {
        QuoteStyle::Single => {
            while index < bytes.len() {
                if bytes[index] == b'\'' {
                    if bytes.get(index + 1) == Some(&b'\'') {
                        index += 2;
                    } else {
                        return (index + 1, true);
                    }
                } else {
                    index += 1;
                }
            }
        }
        QuoteStyle::Double => {
            while index < bytes.len() {
                match bytes[index] {
                    b'\\' => index += 2,
                    b'"' => return (index + 1, true),
                    _ => index += 1,
                }
            }
        }
    }
    (bytes.len(), false)
}

/// A quote opens a scalar only where a node can start: at source start or
/// after whitespace, a line break, a flow opener, a comma, or a colon.
fn at_token_boundary(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    matches!(
        bytes[index - 1],
        b' ' | b'\t' | b'\n' | b'\r' | b'[' | b'{' | b',' | b':'
    )
}

/// A `#` starts a comment at source start or after whitespace/line breaks.
fn at_word_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0 || matches!(bytes[index - 1], b' ' | b'\t' | b'\n' | b'\r')
}

/// An anchor/alias is only meaningful where a node can start: after a
/// mapping colon, sequence dash, comma, flow opener, or at a line start.
/// This keeps mid-scalar `*`/`&` (e.g. `2 * 3`) from being misrecorded.
fn at_anchor_position(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    if !matches!(bytes[index - 1], b' ' | b'\t' | b'\n' | b'\r' | b'[' | b'{' | b',') {
        return false;
    }
    // Look back to the last non-whitespace byte.
    let mut probe = index;
    while probe > 0 && matches!(bytes[probe - 1], b' ' | b'\t') {
        probe -= 1;
    }
    if probe == 0 {
        return true;
    }
    let previous = bytes[probe - 1];
    matches!(previous, b':' | b'-' | b',' | b'[' | b'{' | b'\n' | b'\r')
}

fn is_anchor_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
}

/// End offset of the line containing `index` (start of the line terminator
/// or end of source).
fn line_end_of(bytes: &[u8], index: usize) -> usize {
    let mut end = index;
    while end < bytes.len() && !matches!(bytes[end], b'\n' | b'\r') {
        end += 1;
    }
    end
}

/// Extracts block mapping/sequence entries for every content line.
fn extract_entries(
    source: &str,
    lines: &[Line],
    flow_regions: &[FlowRegion],
    quoted: &[QuotedScalar],
    comments: &[SourceSpan],
) -> Vec<LineEntry> {
    let bytes = source.as_bytes();
    let mut entries = Vec::with_capacity(lines.len());
    for line in lines {
        let mut entry = LineEntry::default();
        if line.kind == LineKind::Content && !inside_any(flow_regions, line.content.start) {
            extract_line_entry(source, bytes, line, flow_regions, quoted, comments, &mut entry);
        }
        entries.push(entry);
    }
    entries
}

fn inside_any(regions: &[FlowRegion], byte: usize) -> bool {
    regions
        .iter()
        .any(|region| region.span.start < byte && byte < region.span.end)
}

fn extract_line_entry(
    source: &str,
    bytes: &[u8],
    line: &Line,
    flow_regions: &[FlowRegion],
    quoted: &[QuotedScalar],
    comments: &[SourceSpan],
    entry: &mut LineEntry,
) {
    let content = line.content.clone();
    let mut search_start = content.start + line.indent;

    // Sequence entry: `-` followed by whitespace or end of content.
    let dash = bytes.get(search_start) == Some(&b'-')
        && (search_start + 1 >= content.end
            || matches!(bytes.get(search_start + 1), Some(b' ') | Some(b'\t')));
    if dash {
        entry.dash = Some(search_start);
        let value_start = skip_whitespace(bytes, search_start + 1, content.end);
        entry.dash_value = value_region(source, value_start, content.end, comments);
        search_start = value_start;
    }

    // Mapping colon: first `:` outside quoted scalars and flow regions that
    // is followed by whitespace or end of content.
    let mut probe = search_start;
    while probe < content.end {
        let is_mapping_colon = bytes[probe] == b':'
            && !inside_any_quoted(quoted, probe)
            && !inside_any(flow_regions, probe)
            && (probe + 1 >= content.end
                || matches!(bytes.get(probe + 1), Some(b' ') | Some(b'\t')));
        if is_mapping_colon {
            let key_start = first_content_byte(bytes, search_start, content.end);
            let key_end = trim_end(source, key_start..probe);
            if key_end > key_start {
                let value_start = skip_whitespace(bytes, probe + 1, content.end);
                entry.mapping = Some(MappingEntry {
                    key: key_start..key_end,
                    colon: probe,
                    value: value_region(source, value_start, content.end, comments),
                });
            }
            break;
        }
        probe += 1;
    }
}

fn inside_any_quoted(quoted: &[QuotedScalar], byte: usize) -> bool {
    quoted
        .iter()
        .any(|scalar| scalar.span.start < byte && byte < scalar.span.end)
}

fn first_content_byte(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut index = start;
    while index < end && matches!(bytes[index], b' ' | b'\t') {
        index += 1;
    }
    index
}

fn skip_whitespace(bytes: &[u8], start: usize, end: usize) -> usize {
    first_content_byte(bytes, start, end)
}

/// Value region from `start` to `content_end`, trimmed of trailing
/// whitespace and of any trailing comment.
fn value_region(
    source: &str,
    start: usize,
    content_end: usize,
    comments: &[SourceSpan],
) -> Option<SourceSpan> {
    if start >= content_end {
        return None;
    }
    let mut end = content_end;
    for comment in comments {
        if comment.start >= start && comment.start < end {
            // The byte before `#` is whitespace by construction; trim back
            // to before that whitespace.
            end = comment.start;
        }
    }
    let end = trim_end(source, start..end);
    if end <= start { None } else { Some(start..end) }
}

/// Key text for context paths: plain keys verbatim; simply-quoted keys
/// unquoted when they contain no escapes. Anything else cannot participate
/// in a proven context path.
fn plain_key_text(source: &str, span: &SourceSpan) -> Option<String> {
    let text = &source[span.clone()];
    if text.is_empty() {
        return None;
    }
    if text.starts_with('\'') && text.ends_with('\'') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        if !inner.contains('\'') {
            return Some(inner.to_string());
        }
        return None;
    }
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        if !inner.contains('\\') && !inner.contains('"') {
            return Some(inner.to_string());
        }
        return None;
    }
    if text.starts_with(['\'', '"']) {
        return None;
    }
    Some(text.to_string())
}
