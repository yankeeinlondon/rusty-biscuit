//! Parses `::map` / `::filter` regions and their opener headers.
//!
//! Region pairing comes from the shared structural scanner
//! ([`scan_block_pairs_in`]); the header grammar is read once, by
//! [`read_raw_header`], which the public lenient directive scanner
//! ([`scan_darkmatter_directives`](crate::markdown::compose::directives_api::scan_darkmatter_directives))
//! also uses. There are two entry modes over that one grammar:
//!
//! - [`parse_collection_regions`] is strict and returns the first located
//!   [`CollectionError`]. Header validation applies to every enabled authored
//!   region, including ones whose condition is false or whose source is empty.
//! - [`scan_collection_regions_lenient`] never fails; it reports each opener's
//!   best-effort directive and its problems for editors.

use std::ops::Range;
use std::path::Path;

use biscuit_terminal::errors::SourceContext;

use super::error::{CollectionError, CollectionSite};
use super::types::{
    CollectionHeader, CollectionKind, CollectionRegion, QuotedExpression, SortOrder,
};
use crate::markdown::compose::block_pairs::{
    BlockOpenKind, BlockPair, BlockPairError, scan_block_pairs_in,
};
use crate::markdown::compose::body_origin::DataRanges;
use crate::markdown::compose::directives_api::{
    BlockScanError, DirectiveKind, ParsedDirective, read_key_token, scan_darkmatter_directives_in,
};
use crate::markdown::compose::expression::lexer::{ParseMode, Token, lex_spanned};
use crate::markdown::compose::expression::parser::{parse_callback_spanned, parse_condition};
use crate::markdown::compose::parse_utils::{Cursor, strip_blockquote_prefix};
use crate::markdown::compose::pipeline::operations::{ComposeOperation, ComposeOperationSet};
use crate::markdown::span::Spanned;
use crate::markdown::types::{AuthoredSpan, SourceRef};

const OPTION_NAMES: [&str; 6] = ["when", "filter", "sort", "sort-order", "while", "until"];

/// A header defect with the authored span it points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderProblem {
    /// Authored byte span in the scanned text.
    pub span: Range<usize>,
    /// What is wrong.
    pub message: String,
    /// Whether this is a callback grammar defect
    /// ([`CollectionError::InvalidCallback`]) rather than a malformed header
    /// ([`CollectionError::Malformed`]).
    pub callback: bool,
}

impl HeaderProblem {
    fn malformed(span: Range<usize>, message: impl Into<String>) -> Self {
        Self { span, message: message.into(), callback: false }
    }
}

/// An option value as read from the line, before validation.
#[derive(Debug, Clone)]
pub(crate) enum RawValue {
    Quoted(QuotedExpression),
    Bare(Spanned<String>),
    /// `key=` followed by nothing.
    Empty(Range<usize>),
}

#[derive(Debug, Clone)]
pub(crate) struct RawOption {
    pub key: Spanned<String>,
    /// `None` for a token without `=`.
    pub value: Option<RawValue>,
}

/// An opener line's tokens, with the lexical problems met while reading.
#[derive(Debug, Clone)]
pub(crate) struct RawHeader {
    /// The source token, when one was present and not a quoted value.
    pub source: Option<Spanned<String>>,
    /// The positional callback.
    pub callback: Option<QuotedExpression>,
    pub options: Vec<RawOption>,
    /// Header text after the keyword.
    pub header_span: Range<usize>,
    pub problems: Vec<HeaderProblem>,
}

fn is_quote(ch: Option<char>) -> bool {
    matches!(ch, Some('"' | '\''))
}

/// The one reader of a `::map` / `::filter` opener line.
///
/// `text` starts at the keyword and has no trailing whitespace; `base` is the
/// scanned-text offset of `text[0]`.
pub(crate) fn read_raw_header(text: &str, kw_len: usize, base: usize, line: usize) -> RawHeader {
    let mut cursor = Cursor::new(text);
    for _ in 0..kw_len {
        cursor.advance();
    }
    cursor.skip_ws();

    let mut raw = RawHeader {
        source: None,
        callback: None,
        options: Vec::new(),
        header_span: base + cursor.pos()..base + text.len(),
        problems: Vec::new(),
    };

    let source_start = cursor.pos();
    match cursor.current() {
        None => raw.problems.push(HeaderProblem::malformed(
            base + kw_len..base + kw_len,
            "expected a source variable path after the keyword",
        )),
        Some('"' | '\'') => raw.problems.push(HeaderProblem::malformed(
            base + source_start..base + text.len(),
            "the source must be a variable path, not a quoted value",
        )),
        Some(_) => {
            while cursor.current().is_some_and(|ch| !ch.is_whitespace()) {
                cursor.advance();
            }
            let token = &text[source_start..cursor.pos()];
            let span = base + source_start..base + cursor.pos();
            if !is_source_path(token) {
                raw.problems.push(HeaderProblem::malformed(
                    span.clone(),
                    format!(
                        "`{token}` is not a valid source: use a dotted variable path without `{{{{ }}}}`, calls, operators, or brackets"
                    ),
                ));
            }
            raw.source = Some(Spanned::new(token.to_string(), span));
        }
    }

    loop {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        if is_quote(cursor.current()) {
            let start = cursor.pos();
            let Some(quoted) = read_quoted(&mut cursor, text, base, line, &mut raw.problems) else {
                break;
            };
            if raw.callback.is_none() && raw.options.is_empty() {
                raw.callback = Some(quoted);
            } else if raw.options.is_empty() {
                raw.problems.push(HeaderProblem::malformed(
                    base + start..base + cursor.pos(),
                    "unexpected extra positional argument: a region takes one quoted callback",
                ));
            } else {
                raw.problems.push(HeaderProblem::malformed(
                    base + start..base + cursor.pos(),
                    "the positional callback must precede the options",
                ));
            }
            continue;
        }

        let key_start = cursor.pos();
        read_key_token(&mut cursor);
        let key_end = cursor.pos();
        if key_end == key_start {
            raw.problems.push(HeaderProblem::malformed(
                base + key_start..base + key_start + 1,
                "an option needs a name before `=`",
            ));
            cursor.advance();
            continue;
        }
        let key = Spanned::new(text[key_start..key_end].to_string(), base + key_start..base + key_end);

        let value = if cursor.current() == Some('=') {
            cursor.advance();
            let value_start = cursor.pos();
            match cursor.current() {
                Some('"' | '\'') => {
                    let Some(quoted) = read_quoted(&mut cursor, text, base, line, &mut raw.problems) else {
                        break;
                    };
                    Some(RawValue::Quoted(quoted))
                }
                None => Some(RawValue::Empty(base + value_start..base + value_start)),
                Some(ch) if ch.is_whitespace() => {
                    Some(RawValue::Empty(base + value_start..base + value_start))
                }
                Some(_) => match cursor.read_value(line) {
                    Ok(value) => Some(RawValue::Bare(Spanned::new(
                        value,
                        base + value_start..base + cursor.pos(),
                    ))),
                    Err(error) => {
                        raw.problems.push(HeaderProblem::malformed(
                            base + value_start..base + text.len(),
                            error.message,
                        ));
                        break;
                    }
                },
            }
        } else {
            None
        };
        raw.options.push(RawOption { key, value });
    }

    raw
}

fn read_quoted(
    cursor: &mut Cursor<'_>,
    text: &str,
    base: usize,
    line: usize,
    problems: &mut Vec<HeaderProblem>,
) -> Option<QuotedExpression> {
    let start = cursor.pos();
    match cursor.read_quoted_value_mapped(line) {
        Ok(mapped) => Some(QuotedExpression {
            text: mapped.decoded.clone(),
            span: base + start..base + cursor.pos(),
            mapped,
            origin: base,
        }),
        Err(error) => {
            problems.push(HeaderProblem::malformed(
                base + start..base + text.len(),
                error.message,
            ));
            None
        }
    }
}

/// Whether `token` is exactly one lexer variable path (dashed segments allowed).
fn is_source_path(token: &str) -> bool {
    matches!(
        lex_spanned(token, ParseMode::Interpolation).as_deref(),
        Ok([first, last])
            if matches!(&first.value, Token::Variable(name) if name == token)
                && matches!(last.value, Token::Eof)
    )
}

/// Checks the grammar of one quoted expression; `condition` selects condition
/// mode, otherwise the value must be a `pattern => expression` callback.
fn check_expression(quoted: &QuotedExpression, what: &str, condition: bool) -> Result<(), HeaderProblem> {
    if quoted.text.trim().is_empty() {
        return Err(HeaderProblem::malformed(quoted.span.clone(), format!("{what} must not be empty")));
    }
    let outcome = if condition {
        parse_condition(&quoted.text).map(|_| ())
    } else {
        parse_callback_spanned(&quoted.text).map(|_| ())
    };
    outcome.map_err(|error| {
        let len = quoted.text.len();
        let at = error.position.min(len);
        let span = quoted
            .authored_range(at..(at + 1).min(len))
            .unwrap_or_else(|| quoted.span.clone());
        HeaderProblem {
            span,
            message: format!("invalid {what}: {}", error.message),
            callback: !condition,
        }
    })
}

/// Validates a read header; returns the first problem in line order.
fn validate(raw: RawHeader) -> Result<CollectionHeader, HeaderProblem> {
    check(raw).map_err(|problems| problems.into_iter().next().expect("check returns at least one problem"))
}

/// Validates a read header; returns every problem in line order.
fn check(raw: RawHeader) -> Result<CollectionHeader, Vec<HeaderProblem>> {
    let RawHeader { source, callback, options, mut problems, .. } = raw;

    if let Some(callback) = &callback
        && let Err(problem) = check_expression(callback, "callback", false)
    {
        problems.push(problem);
    }

    let mut when = None;
    let mut filter = None;
    let mut sort = None;
    let mut while_predicate = None;
    let mut until_predicate = None;
    let mut sort_order: Option<Spanned<SortOrder>> = None;
    let mut sort_order_key: Option<Range<usize>> = None;
    let mut seen: Vec<&str> = Vec::new();

    for option in &options {
        let name = option.key.value.as_str();
        let key_span = option.key.span.clone();

        if name == "fn" {
            problems.push(HeaderProblem::malformed(
                key_span,
                "`fn=` is not supported: write the callback as the quoted positional argument",
            ));
            continue;
        }
        if !OPTION_NAMES.contains(&name) {
            let message = match &option.value {
                None if name.starts_with('-') => format!("unknown flag `{name}`: options take a quoted value"),
                None => format!("unexpected positional argument `{name}`: a region takes one quoted callback"),
                Some(_) => format!("unknown option `{name}`"),
            };
            problems.push(HeaderProblem::malformed(key_span, message));
            continue;
        }
        if seen.contains(&name) {
            problems.push(HeaderProblem::malformed(key_span, format!("option `{name}` is given more than once")));
            continue;
        }
        seen.push(name);

        match &option.value {
            None => {
                problems.push(HeaderProblem::malformed(
                    key_span,
                    format!("`{name}` is a bare flag: write `{name}=\"...\"`"),
                ));
            }
            Some(RawValue::Empty(span)) => {
                problems.push(HeaderProblem::malformed(span.clone(), format!("`{name}` needs a quoted value")));
            }
            Some(RawValue::Bare(value)) if name != "sort-order" => {
                problems.push(HeaderProblem::malformed(
                    value.span.clone(),
                    format!("the value of `{name}` must be quoted"),
                ));
            }
            Some(RawValue::Bare(value)) => {
                match parse_order(&value.value) {
                    Some(order) => sort_order = Some(Spanned::new(order, value.span.clone())),
                    None => problems.push(HeaderProblem::malformed(
                        value.span.clone(),
                        "`sort-order` must be exactly `asc` or `desc`",
                    )),
                }
                sort_order_key = Some(key_span);
            }
            Some(RawValue::Quoted(quoted)) if name == "sort-order" => {
                match parse_order(&quoted.text) {
                    Some(order) => sort_order = Some(Spanned::new(order, quoted.span.clone())),
                    None => problems.push(HeaderProblem::malformed(
                        quoted.span.clone(),
                        "`sort-order` must be exactly `asc` or `desc`",
                    )),
                }
                sort_order_key = Some(key_span);
            }
            Some(RawValue::Quoted(quoted)) => {
                let condition = name == "when";
                let what = if condition { "condition" } else { "callback" };
                if let Err(mut problem) = check_expression(quoted, &format!("`{name}` {what}"), condition) {
                    problem.callback = !condition;
                    problems.push(problem);
                }
                let slot = match name {
                    "when" => &mut when,
                    "filter" => &mut filter,
                    "sort" => &mut sort,
                    "while" => &mut while_predicate,
                    _ => &mut until_predicate,
                };
                *slot = Some(quoted.clone());
            }
        }
    }

    if while_predicate.is_some() && until_predicate.is_some() {
        let second = options
            .iter()
            .filter(|option| matches!(option.key.value.as_str(), "while" | "until"))
            .nth(1)
            .map(|option| option.key.span.clone())
            .unwrap_or_default();
        problems.push(HeaderProblem::malformed(second, "`while` and `until` are mutually exclusive"));
    }
    if let Some(key_span) = sort_order_key
        && !seen.contains(&"sort")
    {
        problems.push(HeaderProblem::malformed(key_span, "`sort-order` requires `sort`"));
    }

    if !problems.is_empty() {
        problems.sort_by_key(|problem| problem.span.start);
        return Err(problems);
    }
    let Some(source) = source else {
        unreachable!("a missing source is recorded as a problem");
    };
    Ok(CollectionHeader {
        source,
        callback,
        when,
        filter,
        sort,
        sort_order,
        while_predicate,
        until_predicate,
    })
}

fn parse_order(text: &str) -> Option<SortOrder> {
    match text {
        "asc" => Some(SortOrder::Asc),
        "desc" => Some(SortOrder::Desc),
        _ => None,
    }
}

fn collection_kind(kind: BlockOpenKind) -> Option<CollectionKind> {
    match kind {
        BlockOpenKind::Map => Some(CollectionKind::Map),
        BlockOpenKind::Filter => Some(CollectionKind::Filter),
        BlockOpenKind::Page | BlockOpenKind::Shell => None,
    }
}

/// The opener's directive text (container markers stripped), its offset, and
/// its keyword length.
fn opener_directive<'a>(source: &'a str, pair: &BlockPair) -> (&'a str, usize, usize) {
    let line = &source[pair.opener_span.clone()];
    let trimmed = line.trim();
    let first_non_ws = pair.opener_span.start + (line.len() - line.trim_start().len());
    let directive = strip_blockquote_prefix(trimmed);
    let kind = collection_kind(pair.kind).unwrap_or(CollectionKind::Map);
    (directive, first_non_ws + trimmed.len() - directive.len(), 2 + kind.opener().len())
}

fn region_from_pair(source: &str, pair: &BlockPair) -> Result<CollectionRegion, HeaderProblem> {
    let kind = collection_kind(pair.kind).unwrap_or(CollectionKind::Map);
    let (directive, start, kw_len) = opener_directive(source, pair);
    let raw = read_raw_header(directive, kw_len, start, pair.start_line);
    let header_span = raw.header_span.clone();
    let header = validate(raw)?;
    Ok(CollectionRegion {
        kind,
        span: pair.span.clone(),
        header_span,
        header,
        body_span: pair.body_span.clone(),
        start_line: pair.start_line,
        end_line: pair.end_line,
        children: Vec::new(),
    })
}

/// Nests regions by containment; `flat` may be in any order.
fn nest(mut flat: Vec<CollectionRegion>) -> Vec<CollectionRegion> {
    flat.sort_by_key(|region| region.span.start);
    let mut roots = Vec::new();
    let mut stack: Vec<CollectionRegion> = Vec::new();

    fn close(stack: &mut Vec<CollectionRegion>, roots: &mut Vec<CollectionRegion>) {
        if let Some(done) = stack.pop() {
            match stack.last_mut() {
                Some(parent) => parent.children.push(done),
                None => roots.push(done),
            }
        }
    }

    for region in flat {
        while stack.last().is_some_and(|top| region.span.start >= top.span.end) {
            close(&mut stack, &mut roots);
        }
        stack.push(region);
    }
    while !stack.is_empty() {
        close(&mut stack, &mut roots);
    }
    roots
}

fn line_span(source: &str, line: usize) -> Range<usize> {
    let mut start = 0;
    for (index, text) in source.split('\n').enumerate() {
        if index + 1 == line {
            return start..start + text.trim_end_matches('\r').len();
        }
        start += text.len() + 1;
    }
    source.len()..source.len()
}

pub(super) fn site(source: &str, path: Option<&Path>, span: Range<usize>) -> CollectionSite {
    // `AuthoredSpan` cannot locate an empty range; widen it by one character.
    let span = if span.is_empty() {
        match source[span.start.min(source.len())..].chars().next() {
            Some(next) => span.start..span.start + next.len_utf8(),
            None => source[..span.start.min(source.len())]
                .char_indices()
                .next_back()
                .map_or(span.clone(), |(index, _)| index..span.start),
        }
    } else {
        span
    };
    let located = path.and_then(|path| {
        let located = AuthoredSpan::locate(source, span.clone())?;
        let display = path.file_name().map_or_else(|| path.to_path_buf(), Into::into);
        Some(SourceRef::OnDiskSpan {
            context: SourceContext::new(path.to_path_buf(), display, source),
            span: located,
        })
    });
    let source_ref = located.unwrap_or_else(|| {
        let from = source[..span.start.min(source.len())].rfind('\n').map_or(0, |index| index + 1);
        let line = &source[from..];
        SourceRef::Effective {
            rendered: line.lines().next().unwrap_or_default().to_string(),
            origin_key: None,
        }
    });
    CollectionSite { source: Box::new(source_ref), iteration_path: Default::default() }
}

fn problem_error(source: &str, path: Option<&Path>, problem: HeaderProblem) -> CollectionError {
    let site = site(source, path, problem.span);
    if problem.callback {
        CollectionError::InvalidCallback { site, message: problem.message }
    } else {
        CollectionError::Malformed { site, message: problem.message }
    }
}

fn structural_error(source: &str, path: Option<&Path>, error: &BlockPairError) -> CollectionError {
    CollectionError::Malformed {
        site: site(source, path, line_span(source, error.line())),
        message: error.to_string(),
    }
}

/// Parses every enabled `::map` / `::filter` region of `source`.
///
/// Returns the outermost regions in document order, each with its nested
/// regions in `children`. Nothing is evaluated.
///
/// ## Examples
///
/// ```ignore
/// let regions = parse_collection_regions("::map items \"i => i.name\"\nbody\n::end-map\n", None)?;
/// assert_eq!(regions[0].header.source.value, "items");
/// ```
///
/// ## Errors
///
/// [`CollectionError::Malformed`] locates an unpaired, crossed, or
/// mis-nested wrapper, a `::map`/`::filter` inside a shell block, and any
/// header defect: bad source, unknown or duplicate option, bare flag, extra
/// positional, unquoted or empty expression value, `fn=`, `while` with
/// `until`, an invalid `sort-order`, or `sort-order` without `sort`.
/// [`CollectionError::InvalidCallback`] locates a callback or `filter` /
/// `sort` / `while` / `until` value that does not parse. Spans are authored
/// offsets; with `source_path` the site carries the file.
pub fn parse_collection_regions(
    source: &str,
    source_path: Option<&Path>,
) -> Result<Vec<CollectionRegion>, CollectionError> {
    parse_collection_regions_in(source, None, &ComposeOperationSet::all(), source_path)
}

/// [`parse_collection_regions`] over a body with inserted `data` bytes and an
/// explicit operation set. When [`ComposeOperation::CollectionBlocks`] is not
/// enabled the wrappers are inert text and the result is empty, even for
/// malformed wrappers.
pub(crate) fn parse_collection_regions_in(
    source: &str,
    data: Option<&DataRanges>,
    operations: &ComposeOperationSet,
    source_path: Option<&Path>,
) -> Result<Vec<CollectionRegion>, CollectionError> {
    if !operations.contains(ComposeOperation::CollectionBlocks) {
        return Ok(Vec::new());
    }
    let pairs = scan_block_pairs_in(source, data)
        .map_err(|error| structural_error(source, source_path, &error))?;
    let mut collections: Vec<&BlockPair> = pairs.iter().filter(|pair| pair.kind.is_collection()).collect();
    collections.sort_by_key(|pair| pair.span.start);
    let mut flat = Vec::with_capacity(collections.len());
    for pair in collections {
        flat.push(region_from_pair(source, pair).map_err(|problem| problem_error(source, source_path, problem))?);
    }
    Ok(nest(flat))
}

/// One opener line as the lenient scan sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LenientCollectionHeader {
    /// The best-effort directive from the shared directive scanner.
    pub directive: ParsedDirective,
    /// Every defect strict parsing would report for this header, in line order.
    pub problems: Vec<HeaderProblem>,
}

/// The result of [`scan_collection_regions_lenient`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LenientCollectionScan {
    /// Every `::map` / `::filter` opener line, paired or not.
    pub headers: Vec<LenientCollectionHeader>,
    /// Paired regions whose headers validate, nested; empty when pairing failed.
    pub regions: Vec<CollectionRegion>,
    /// Why pairing failed, when it did.
    pub structural: Option<BlockScanError>,
}

/// Best-effort scan for editors: never fails and never evaluates.
///
/// Openers come from the public lenient directive scanner, so an incomplete
/// header still appears with whatever was recovered. A region that cannot be
/// paired, or whose header does not validate, is absent from `regions` and
/// explained through `headers` and `structural`.
pub fn scan_collection_regions_lenient(source: &str) -> LenientCollectionScan {
    scan_collection_regions_lenient_in(source, None)
}

pub(crate) fn scan_collection_regions_lenient_in(
    source: &str,
    data: Option<&DataRanges>,
) -> LenientCollectionScan {
    let headers = scan_darkmatter_directives_in(source, data)
        .into_iter()
        .filter(|directive| matches!(directive.kind, DirectiveKind::Map | DirectiveKind::Filter))
        .map(|directive| {
            let text = &source[directive.keyword_span.start..directive.span.end];
            let raw = read_raw_header(text, directive.keyword_span.len(), directive.keyword_span.start, directive.line);
            let problems = check(raw).err().unwrap_or_default();
            LenientCollectionHeader { directive, problems }
        })
        .collect();

    match scan_block_pairs_in(source, data) {
        Ok(pairs) => {
            let flat = pairs
                .iter()
                .filter(|pair| pair.kind.is_collection())
                .filter_map(|pair| region_from_pair(source, pair).ok())
                .collect();
            LenientCollectionScan { headers, regions: nest(flat), structural: None }
        }
        Err(error) => LenientCollectionScan {
            headers,
            regions: Vec::new(),
            structural: Some(error.into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "/doc.md";

    fn strict(source: &str) -> Result<Vec<CollectionRegion>, CollectionError> {
        parse_collection_regions(source, Some(Path::new(PATH)))
    }

    fn wrap(header: &str) -> String {
        format!("::map {header}\nbody\n::end-map\n")
    }

    /// The (line, authored text) a located error points at.
    fn located<'a>(source: &'a str, error: &CollectionError) -> (usize, &'a str) {
        let site = error.site().expect("a located error");
        match site.source.as_ref() {
            SourceRef::OnDiskSpan { span, .. } => (span.line(), &source[span.range()]),
            other => panic!("expected an authored span, got {other:?}"),
        }
    }

    fn reject(header: &str, fragment: &str, message: &str) {
        let source = wrap(header);
        let error = strict(&source).expect_err("header must be rejected");
        let (line, text) = located(&source, &error);
        assert_eq!(line, 1, "{header}");
        // A zero-width problem (a missing token) is widened by one character.
        assert!(
            text == fragment || (fragment.is_empty() && text.chars().count() == 1),
            "{header}: {text:?}"
        );
        assert!(error.to_string().contains(message), "{header}: {error}");
    }

    #[test]
    fn control_row_parses_both_kinds_with_every_option() {
        let source = concat!(
            "::map team.members \"p => p.name\" when=\"on\" filter=\"p => p.active\" sort=\"p => p.age\" sort-order=\"desc\" until=\"p => p.done\"\n",
            "m\n",
            "::end-map\n",
            "::filter local.my-items while='i => i > 0'\n",
            "f\n",
            "::end-filter\n",
        );
        let regions = strict(source).unwrap();

        assert_eq!(regions.len(), 2);
        let map = &regions[0];
        assert_eq!(map.kind, CollectionKind::Map);
        assert_eq!(map.header.source.value, "team.members");
        assert_eq!(&source[map.header.source.span.clone()], "team.members");
        assert_eq!(map.header.callback.as_ref().unwrap().text, "p => p.name");
        assert_eq!(map.header.when.as_ref().unwrap().text, "on");
        assert_eq!(map.header.filter.as_ref().unwrap().text, "p => p.active");
        assert_eq!(map.header.sort.as_ref().unwrap().text, "p => p.age");
        assert_eq!(map.header.sort_order.as_ref().unwrap().value, SortOrder::Desc);
        assert_eq!(map.header.until_predicate.as_ref().unwrap().text, "p => p.done");
        assert!(map.header.while_predicate.is_none());
        assert_eq!(&source[map.body_span.clone()], "m\n");
        assert_eq!((map.start_line, map.end_line), (1, 3));

        let filter = &regions[1];
        assert_eq!(filter.kind, CollectionKind::Filter);
        assert_eq!(filter.header.source.value, "local.my-items");
        assert!(filter.header.callback.is_none());
        assert_eq!(filter.header.while_predicate.as_ref().unwrap().text, "i => i > 0");
        assert_eq!(&source[filter.header.while_predicate.as_ref().unwrap().span.clone()], "'i => i > 0'");
    }

    #[test]
    fn escaped_quotes_and_multibyte_text_keep_authored_offsets() {
        let source = "::map items \"é => é.name == \\\"ü\\\"\"\nx\n::end-map\n";
        let regions = strict(source).unwrap();
        let callback = regions[0].header.callback.as_ref().unwrap();

        assert_eq!(callback.text, "é => é.name == \"ü\"");
        let decoded = callback.text.find("\"ü\"").unwrap();
        // Three decoded bytes (`"ü"` is quote + 2-byte ü + quote = 4) map across
        // the two-byte escapes back to the authored text.
        let authored = callback.authored_range(decoded..decoded + 4).unwrap();
        assert_eq!(&source[authored], "\\\"ü\\\"");
        let name = callback.text.find("name").unwrap();
        assert_eq!(&source[callback.authored_range(name..name + 4).unwrap()], "name");
    }

    #[test]
    fn callback_error_points_through_the_offset_map() {
        let source = "::map items \"é => é.name == \\\"x\\\" )\"\nx\n::end-map\n";
        let error = strict(source).unwrap_err();

        assert!(matches!(error, CollectionError::InvalidCallback { .. }), "{error}");
        let (line, text) = located(source, &error);
        assert_eq!(line, 1);
        assert_eq!(text, ")");
    }

    #[test]
    fn rejects_bad_sources() {
        reject("", "", "source");
        reject("items.", "items.", "not a valid source");
        reject("items[0]", "items[0]", "not a valid source");
        reject("f(x)", "f(x)", "not a valid source");
        reject("{{items}}", "{{items}}", "not a valid source");
        reject("\"i => i\"", "\"i => i\"", "source must be a variable path");
    }

    #[test]
    fn rejects_unknown_and_duplicate_options() {
        reject("items bogus=\"x\"", "bogus", "unknown option `bogus`");
        reject("items when=\"a\" when=\"b\"", "when", "more than once");
        reject("items sort=\"i => i\" sort=\"i => i\"", "sort", "more than once");
    }

    #[test]
    fn rejects_bare_flags_and_extra_positionals() {
        reject("items when", "when", "bare flag");
        reject("items --dir", "--dir", "unknown flag");
        reject("items extra", "extra", "unexpected positional");
        reject("items \"i => i\" \"j => j\"", "\"j => j\"", "extra positional");
        reject("items when=\"a\" \"i => i\"", "\"i => i\"", "must precede");
    }

    #[test]
    fn rejects_unquoted_and_empty_values() {
        reject("items when=x", "x", "must be quoted");
        reject("items filter=i", "i", "must be quoted");
        reject("items when=", "", "quoted value");
        reject("items \"\"", "\"\"", "must not be empty");
        reject("items when=\"  \"", "\"  \"", "must not be empty");
        reject("items filter=\"\"", "\"\"", "must not be empty");
    }

    #[test]
    fn rejects_fn_option() {
        reject("items fn=\"i => i\"", "fn", "`fn=` is not supported");
    }

    #[test]
    fn rejects_while_with_until() {
        reject("items while=\"i => i\" until=\"i => i\"", "until", "mutually exclusive");
        reject("items until=\"i => i\" while=\"i => i\"", "while", "mutually exclusive");
    }

    #[test]
    fn sort_order_must_be_exact_and_requires_sort() {
        reject("items sort-order=\"asc\"", "sort-order", "requires `sort`");
        reject("items sort=\"i => i\" sort-order=\"up\"", "\"up\"", "exactly `asc` or `desc`");
        reject("items sort=\"i => i\" sort-order=\"ASC\"", "\"ASC\"", "exactly `asc` or `desc`");
        let asc = strict(&wrap("items sort=\"i => i\" sort-order=\"asc\"")).unwrap();
        assert_eq!(asc[0].header.sort_order.as_ref().unwrap().value, SortOrder::Asc);
    }

    #[test]
    fn malformed_callback_and_condition_grammar() {
        let source = wrap("items \"i i\"");
        let error = strict(&source).unwrap_err();
        assert!(matches!(error, CollectionError::InvalidCallback { .. }), "{error}");
        assert_eq!(located(&source, &error).0, 1);

        let source = wrap("items when=\"a ==\"");
        let error = strict(&source).unwrap_err();
        assert!(matches!(error, CollectionError::Malformed { .. }), "{error}");

        let source = wrap("items \"i => i");
        let error = strict(&source).unwrap_err();
        assert!(matches!(error, CollectionError::Malformed { .. }), "{error}");
    }

    #[test]
    fn false_and_empty_regions_are_still_validated() {
        let source = "::filter items \"i => true\" when=\"false\" bogus=\"x\"\n::end-filter\n";
        let error = strict(source).unwrap_err();
        assert_eq!(located(source, &error), (1, "bogus"));

        let valid = strict("::filter items when=\"false\"\n::end-filter\n").unwrap();
        assert_eq!(valid[0].body_span, valid[0].header_span.end + 1..valid[0].header_span.end + 1);
    }

    #[test]
    fn disabled_operation_leaves_wrappers_inert() {
        let mut operations = ComposeOperationSet::all();
        operations.remove(ComposeOperation::CollectionBlocks);
        let source = "::map bogus=\"x\"\nnever closed\n";

        let regions = parse_collection_regions_in(source, None, &operations, None).unwrap();

        assert!(regions.is_empty());
        assert!(parse_collection_regions(source, None).is_err());
    }

    #[test]
    fn structural_errors_are_located() {
        let unterminated = "text\n::map items\nbody\n";
        let error = strict(unterminated).unwrap_err();
        assert_eq!(located(unterminated, &error), (2, "::map items"));

        let stray = "a\n::end-filter\n";
        let error = strict(stray).unwrap_err();
        assert_eq!(located(stray, &error), (2, "::end-filter"));

        let crossed = "::map a\n::filter b\n::end-map\n::end-filter\n";
        let error = strict(crossed).unwrap_err();
        assert_eq!(located(crossed, &error), (3, "::end-map"));

        let trailing = "::map a\n::end-map extra\n";
        let error = strict(trailing).unwrap_err();
        assert_eq!(located(trailing, &error), (2, "::end-map extra"));

        let shell = "::shell-block\n::map a\n::end-map\n::end-block\n";
        let error = strict(shell).unwrap_err();
        assert_eq!(located(shell, &error), (2, "::map a"));
        assert!(matches!(error, CollectionError::Malformed { .. }));
    }

    #[test]
    fn without_a_path_the_site_carries_the_line() {
        let error = parse_collection_regions("::map\nx\n::end-map\n", None).unwrap_err();
        match error.site().unwrap().source.as_ref() {
            SourceRef::Effective { rendered, .. } => assert_eq!(rendered, "::map"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn nested_regions_land_in_children() {
        let source = concat!(
            "::map a\n",
            "  ::filter b\n",
            "    ::map c\n",
            "    ::end-map\n",
            "  ::end-filter\n",
            "::end-map\n",
            "::filter d\n",
            "::end-filter\n",
        );
        let regions = strict(source).unwrap();

        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].children.len(), 1);
        assert_eq!(regions[0].children[0].kind, CollectionKind::Filter);
        assert_eq!(regions[0].children[0].children[0].header.source.value, "c");
        assert!(regions[1].children.is_empty());
    }

    #[test]
    fn page_blocks_do_not_hide_nested_collections() {
        let source = "::block when=\"x\"\n::map a\n::end-map\n::end-block\n";
        let regions = strict(source).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_line, 2);
    }

    #[test]
    fn quoted_and_list_contained_regions_are_active() {
        let quoted = "> ::map items\n> body\n> ::end-map\n";
        let regions = strict(quoted).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(&quoted[regions[0].header.source.span.clone()], "items");

        let list = "- item\n\n  ::filter xs \"i => i\"\n  text\n  ::end-filter\n";
        let regions = strict(list).unwrap();
        assert_eq!(regions[0].kind, CollectionKind::Filter);
        assert_eq!(&list[regions[0].header.source.span.clone()], "xs");

        let broken = "> ::map items bogus=\"x\"\n> ::end-map\n";
        let error = strict(broken).unwrap_err();
        assert_eq!(located(broken, &error), (1, "bogus"));
    }

    #[test]
    fn code_regions_are_not_collections() {
        let source = "```\n::map\n```\n";
        assert!(strict(source).unwrap().is_empty());
    }

    #[test]
    fn lenient_scan_reports_incomplete_headers_without_failing() {
        let source = "::map items \"p => p.\" when=\n";
        let scan = scan_collection_regions_lenient(source);

        assert_eq!(scan.headers.len(), 1);
        let header = &scan.headers[0];
        assert_eq!(header.directive.kind, DirectiveKind::Map);
        assert_eq!(header.directive.target.as_ref().unwrap().value, "items");
        assert_eq!(header.directive.callback.as_ref().unwrap().value, "p => p.");
        assert!(header.problems.len() >= 2, "{:?}", header.problems);
        assert!(header.problems.windows(2).all(|pair| pair[0].span.start <= pair[1].span.start));
        assert!(scan.regions.is_empty());
        assert!(matches!(scan.structural, Some(BlockScanError::UnterminatedBlock { line: 1, .. })));
    }

    #[test]
    fn lenient_scan_keeps_valid_regions_and_skips_invalid_ones() {
        let source = "::map a\n::end-map\n::map b bogus=\"1\"\n::end-map\n";
        let scan = scan_collection_regions_lenient(source);

        assert_eq!(scan.headers.len(), 2);
        assert!(scan.headers[0].problems.is_empty());
        assert_eq!(scan.headers[1].problems[0].span, source.find("bogus").unwrap()..source.find("bogus").unwrap() + 5);
        assert_eq!(scan.regions.len(), 1);
        assert_eq!(scan.regions[0].header.source.value, "a");
        assert!(scan.structural.is_none());
    }
}
