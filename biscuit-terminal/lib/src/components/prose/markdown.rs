//! Markdown pre-processing for [`Prose`](super::Prose) and
//! [`InlineProse`](super::InlineProse) input.
//!
//! Converts a curated subset of Markdown syntax into the block-tag grammar
//! that [`super::tokens::parse_render_nodes`] understands:
//!
//! | Markdown          | Pre-processed form                |
//! |-------------------|-----------------------------------|
//! | `[desc](ref)`     | `<a href="ref">desc</a>`          |
//! | `**text**`        | `<b>text</b>`                     |
//! | `_text_`          | `<i>text</i>`                     |
//! | `` `code` ``      | a lifted-content placeholder resolved to `InlineCode` |
//!
//! Opaque regions are recognized once for the whole input ([`lift_opaque`]),
//! before paragraphs are split or anything is rewritten. Each paragraph then
//! runs the inline phases in a fixed order ([`preprocess_inline`]): links →
//! bold → italics. Each phase respects backslash escapes (`\*`, `\_`, `\[`,
//! `\]`, `\(`, `\)`) so literal Markdown characters survive untouched and
//! reach the token parser as-is.
//!
//! ## Opaque regions
//!
//! Fenced code blocks, code spans, HTML comments, and explicit
//! `<code-block>` bodies are moved into a [`Lifted`] table and replaced by a
//! `\u{0002}<n>\u{0002}` placeholder that survives every later phase (the
//! sentinel is non-word punctuation) and is resolved by the token parser
//! straight into a `Code` or `InlineCode` node, or into nothing for a
//! comment, never re-expanded into string markup. Recognized tag
//! declarations, quoted attribute values included, are copied through
//! unread. A later phase therefore never sees code contents, and never finds
//! a fence or code span inside an attribute value.
//!
//! Literal sentinel characters in user input are backslash-escaped by
//! [`lift_opaque`] outside code before anything else runs, so an unescaped
//! sentinel is always one the parser issued and input can never impersonate
//! a lifted reference or a link-target placeholder.
//!
//! ## Code spans
//!
//! A backtick run opens a span only when a later run of the same length
//! closes it, before the next blank line or fence line; an unmatched run is
//! literal. The span's value follows CommonMark: backslashes are literal,
//! line endings become spaces, and one space is stripped from each side when
//! the content both begins and ends with a space and is not all spaces.
//!
//! Emphasis delimiters follow the CommonMark left- and right-flanking rules
//! (`_` additionally may not open or close inside a word, and `**` keeps the
//! stricter Prose rule that it never opens or closes inside a word), and a
//! closing delimiter is accepted only at the tag nesting depth of its opener.
//! A delimiter that cannot form valid emphasis stays literal text, so the
//! pre-processor never emits an unbalanced tag.
//!
//! Link conversion is *atomic*: the reference (URL) portion of a link is
//! lifted out of the input stream into a placeholder during link parsing
//! and re-inserted only after bold/italics phases have run. This guarantees
//! that `*` or `_` characters inside a URL — for example,
//! `https://example.com/path_with_underscores` — are never re-interpreted
//! as Markdown emphasis markers.

use super::tokens::{is_recognized_opening_tag, parse_opening_tag};

const HREF_PLACEHOLDER_MARK: char = '\u{0001}';

/// Returns `true` when `ch` is an alphanumeric "word" character.
fn is_word_neighbour(ch: Option<char>) -> bool {
    matches!(ch, Some(c) if c.is_alphanumeric())
}

/// Returns `true` when an emphasis delimiter sits between two word
/// characters. Prose never lets such a delimiter open or close emphasis, so
/// identifiers like `OPENCODE_CONFIG_CONTENT` or `foo**bar**baz` stay
/// literal.
fn is_intra_word(prev: Option<char>, next: Option<char>) -> bool {
    is_word_neighbour(prev) && is_word_neighbour(next)
}

/// CommonMark punctuation, approximated as any character that is neither
/// alphanumeric nor whitespace (tag brackets and placeholder sentinels
/// included).
fn is_punctuation(ch: char) -> bool {
    !ch.is_alphanumeric() && !ch.is_whitespace()
}

/// Start and end of input count as whitespace for flanking purposes.
fn is_space_or_edge(ch: Option<char>) -> bool {
    ch.is_none_or(char::is_whitespace)
}

/// CommonMark left-flanking delimiter run: not followed by whitespace, and
/// either not followed by punctuation or preceded by whitespace/punctuation.
fn is_left_flanking(prev: Option<char>, next: Option<char>) -> bool {
    match next {
        None => false,
        Some(n) if n.is_whitespace() => false,
        Some(n) => !is_punctuation(n) || is_space_or_edge(prev) || prev.is_some_and(is_punctuation),
    }
}

/// CommonMark right-flanking delimiter run: not preceded by whitespace, and
/// either not preceded by punctuation or followed by whitespace/punctuation.
fn is_right_flanking(prev: Option<char>, next: Option<char>) -> bool {
    match prev {
        None => false,
        Some(p) if p.is_whitespace() => false,
        Some(p) => !is_punctuation(p) || is_space_or_edge(next) || next.is_some_and(is_punctuation),
    }
}

/// Which way a delimiter run may act.
#[derive(Clone, Copy)]
enum Delimiter {
    /// `**` — opens when left-flanking, closes when right-flanking.
    Bold,
    /// `_` — the CommonMark underscore rule: a run that is both left- and
    /// right-flanking opens only after punctuation and closes only before it.
    Italic,
}

impl Delimiter {
    fn can_open(self, prev: Option<char>, next: Option<char>) -> bool {
        if is_intra_word(prev, next) || !is_left_flanking(prev, next) {
            return false;
        }
        match self {
            Delimiter::Bold => true,
            Delimiter::Italic => !is_right_flanking(prev, next) || prev.is_some_and(is_punctuation),
        }
    }

    fn can_close(self, prev: Option<char>, next: Option<char>) -> bool {
        if is_intra_word(prev, next) || !is_right_flanking(prev, next) {
            return false;
        }
        match self {
            Delimiter::Bold => true,
            Delimiter::Italic => !is_left_flanking(prev, next) || next.is_some_and(is_punctuation),
        }
    }
}

/// The characters on either side of `chars[start..start + len]`.
fn neighbours(chars: &[char], start: usize, len: usize) -> (Option<char>, Option<char>) {
    let prev = start.checked_sub(1).map(|i| chars[i]);
    (prev, chars.get(start + len).copied())
}

/// Sentinel character delimiting a lifted-content placeholder
/// (`\u{0002}<n>\u{0002}`, an index into the [`Lifted`] table). It is a C0
/// control character; a literal one in user input is backslash-escaped by
/// [`lift_opaque`], so an unescaped occurrence is always parser-issued.
pub(super) const LIFT_MARK: char = '\u{0002}';

/// Whether `c` is one of the parser's placeholder sentinels.
pub(super) fn is_sentinel(c: char) -> bool {
    c == LIFT_MARK || c == HREF_PLACEHOLDER_MARK
}

/// A fenced code block lifted out of the input during pre-processing.
///
/// The body is opaque: no Prose markup is ever parsed from it.
#[derive(Debug, Clone)]
pub(super) struct FencedCode {
    /// Language hint from the opening fence (may be empty).
    pub lang: String,
    /// Verbatim code-block body.
    pub body: String,
}

/// Opaque content moved out of the text stream; placeholders index into a
/// `Vec<Lifted>` shared by every paragraph of one input.
#[derive(Debug, Clone)]
pub(super) enum Lifted {
    /// A fenced code block.
    Fence(FencedCode),
    /// A code span's final value (CommonMark adjustments already applied).
    Span(String),
    /// An explicit `<code-block>` body, verbatim; its placeholder stays
    /// between the opening and closing tags.
    CodeBody(String),
    /// An HTML comment. It contributes no node; the placeholder keeps the
    /// comment opaque to the later phases and reads as punctuation to the
    /// emphasis flanking rules, as the comment's `<` and `>` would.
    Comment,
}

/// Placeholder text referring to `lifted[index]`.
fn lift_placeholder(index: usize) -> String {
    format!("{LIFT_MARK}{index}{LIFT_MARK}")
}

/// Parse a lifted-content placeholder starting at `chars[start]` (which must
/// be [`LIFT_MARK`]). Returns the index and the position just past the
/// closing mark.
pub(super) fn parse_lift_placeholder(chars: &[char], start: usize) -> Option<(usize, usize)> {
    debug_assert_eq!(chars[start], LIFT_MARK);
    let digits_end = start + 1 + chars[start + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
    if digits_end == start + 1 || chars.get(digits_end) != Some(&LIFT_MARK) {
        return None;
    }
    let index: String = chars[start + 1..digits_end].iter().collect();
    Some((index.parse().ok()?, digits_end + 1))
}

/// Stage 0 for the whole input: normalize line endings, lift every opaque
/// region into `lifted`, and backslash-escape every literal sentinel
/// character outside them (rule R1).
///
/// CRLF and lone CR become LF first, so the paragraph and break rules see
/// the same structure whatever platform produced the text. One left-to-right
/// scan then recognizes, whichever starts first:
///
/// - a fenced block: a line starting (after indentation) with three
///   backticks, up to a line holding only three backticks; an unclosed fence
///   consumes the rest of the input. The body is lifted verbatim.
/// - a code span: a backtick run closed by a later run of the same length
///   before the next blank line or fence line. Its CommonMark value is lifted.
/// - an HTML comment (`<!-->`, `<!--->`, or `<!--` … `-->`) closed before the
///   next blank line or fence line. It is lifted as [`Lifted::Comment`] and
///   contributes no content, so Markdown that separates two code spans with
///   `<!-- -->` reads back as the two values alone.
/// - a recognized tag declaration, scanned quote-aware, so a quoted attribute
///   value (newlines, backticks, fence lines, `>` and all) is copied through
///   unread. For `<code-block>` the body up to the first `</code-block>` (or
///   the end of input) is lifted verbatim as well.
///
/// No later phase can therefore rewrite code or an attribute value, and a
/// placeholder never lands inside one.
pub(super) fn lift_opaque(input: &str) -> (String, Vec<Lifted>) {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let chars: Vec<char> = normalized.chars().collect();
    let mut output = String::with_capacity(normalized.len());
    let mut lifted: Vec<Lifted> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if (i == 0 || chars[i - 1] == '\n')
            && let Some(next) = lift_fence(&chars, i, &mut output, &mut lifted)
        {
            i = next;
            continue;
        }

        let ch = chars[i];
        match ch {
            '\\' => match chars.get(i + 1) {
                // A lone backslash before a literal sentinel is literal too:
                // escape it so the sentinel keeps its own escape.
                Some(&next) if is_sentinel(next) => {
                    output.push_str("\\\\");
                    i += 1;
                }
                Some(&next) if is_escapable(next) => {
                    output.push(ch);
                    output.push(next);
                    i += 2;
                }
                _ => {
                    output.push(ch);
                    i += 1;
                }
            },
            c if is_sentinel(c) => {
                output.push('\\');
                output.push(c);
                i += 1;
            }
            '`' => {
                let run = backtick_run_len(&chars, i);
                match closing_backtick_run(&chars, i + run, run) {
                    Some(close) => {
                        output.push_str(&lift_placeholder(lifted.len()));
                        lifted.push(Lifted::Span(code_span_value(&chars[i + run..close])));
                        i = close + run;
                    }
                    None => {
                        output.extend(&chars[i..i + run]);
                        i += run;
                    }
                }
            }
            '<' => match html_comment_end(&chars, i) {
                Some(end) => {
                    output.push_str(&lift_placeholder(lifted.len()));
                    lifted.push(Lifted::Comment);
                    i = end;
                }
                None => i = copy_opaque_tag(&chars, i, &mut output, &mut lifted),
            },
            _ => {
                output.push(ch);
                i += 1;
            }
        }
    }

    (output, lifted)
}

/// Lift the fenced block opening on the line at `chars[start]`, if that line
/// is a fence. Returns the index of the line ending after the closing fence
/// (or the end of input), leaving that line ending in place.
fn lift_fence(chars: &[char], start: usize, output: &mut String, lifted: &mut Vec<Lifted>) -> Option<usize> {
    let line_end = |from: usize| chars[from..].iter().position(|&c| c == '\n').map_or(chars.len(), |p| from + p);
    let opening_end = line_end(start);
    let opening: String = chars[start..opening_end].iter().collect();
    let lang = opening.trim_start().strip_prefix("```")?.trim().to_string();

    let mut body_lines: Vec<String> = Vec::new();
    let mut i = opening_end;
    while i < chars.len() {
        let line_start = i + 1;
        let end = line_end(line_start);
        let line: String = chars[line_start..end].iter().collect();
        i = end;
        if line.trim_start() == "```" {
            break;
        }
        body_lines.push(line);
    }

    output.push_str(&lift_placeholder(lifted.len()));
    lifted.push(Lifted::Fence(FencedCode {
        lang,
        body: body_lines.join("\n"),
    }));
    Some(i)
}

/// Copy the tag declaration at `chars[start]` (a `<`) when it opens a tag
/// the parser recognizes, escaping literal sentinels but reading nothing
/// else inside it; a `<code-block>` body is lifted as [`Lifted::CodeBody`].
/// Any other `<` is copied alone. Returns the index to resume scanning at.
fn copy_opaque_tag(chars: &[char], start: usize, output: &mut String, lifted: &mut Vec<Lifted>) -> usize {
    let Some(end) = tag_declaration_end(chars, start) else {
        output.push('<');
        return start + 1;
    };
    let content: String = chars[start + 1..end - 1].iter().collect();
    if !is_recognized_opening_tag(&content) {
        output.push('<');
        return start + 1;
    }
    push_declaration(&chars[start..end], output);
    if !parse_opening_tag(&content).is_some_and(|(name, _)| name == "code-block") {
        return end;
    }

    let (body_end, resume) = match find_ci(chars, end, CODE_BLOCK_CLOSE) {
        Some(close) => (close, close + CODE_BLOCK_CLOSE.chars().count()),
        None => (chars.len(), chars.len()),
    };
    output.push_str(&lift_placeholder(lifted.len()));
    lifted.push(Lifted::CodeBody(chars[end..body_end].iter().collect()));
    output.extend(&chars[body_end..resume]);
    resume
}

/// Index just past the HTML comment opening at `chars[start]` (a `<`), using
/// CommonMark's comment syntax: `<!-->`, `<!--->`, or `<!--` then text without
/// `-->` then `-->`. Like a code span, a comment never crosses a blank line or
/// a fence line; an unclosed `<!--` is literal text.
fn html_comment_end(chars: &[char], start: usize) -> Option<usize> {
    let open = start + 4;
    if !chars[start..].starts_with(&['<', '!', '-', '-']) {
        return None;
    }
    if chars.get(open) == Some(&'>') {
        return Some(open + 1);
    }
    if chars[open..].starts_with(&['-', '>']) {
        return Some(open + 2);
    }
    let mut i = open;
    while i < chars.len() {
        if chars[i] == '\n' && (blank_run_end(chars, i).is_some() || is_fence_line(chars, i + 1)) {
            return None;
        }
        if chars[i..].starts_with(&['-', '-', '>']) {
            return Some(i + 3);
        }
        i += 1;
    }
    None
}

/// The closing tag that ends an explicit code-block body.
pub(super) const CODE_BLOCK_CLOSE: &str = "</code-block>";

/// Copy a tag declaration verbatim apart from literal sentinels, which are
/// escaped (a backslash right before one is escaped too). Escape pairs are
/// copied whole so a sentinel's escape never pairs with an authored one.
fn push_declaration(declaration: &[char], output: &mut String) {
    let mut i = 0;
    while i < declaration.len() {
        let c = declaration[i];
        match declaration.get(i + 1) {
            Some(&next) if c == '\\' && is_sentinel(next) => output.push_str("\\\\"),
            Some(&next) if c == '\\' => {
                output.push(c);
                output.push(next);
                i += 1;
            }
            _ if is_sentinel(c) => {
                output.push('\\');
                output.push(c);
            }
            _ => output.push(c),
        }
        i += 1;
    }
}

/// Index just past the `>` of the tag declaration opening at `chars[start]`,
/// scanning quote- and escape-aware like the token parser, so a newline or
/// `>` inside a quoted attribute belongs to the declaration. An unquoted
/// blank line ends the attempt: the `<` is then literal text.
pub(super) fn tag_declaration_end(chars: &[char], start: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '\n' if quote.is_none() && blank_run_end(chars, i).is_some() => return None,
            '\\' => i += 2,
            c @ ('"' | '\'') => {
                match quote {
                    Some(q) if q == c => quote = None,
                    None => quote = Some(c),
                    Some(_) => {}
                }
                i += 1;
            }
            '>' if quote.is_none() => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// Index of the first case-insensitive occurrence of `needle` at or after
/// `start`.
pub(super) fn find_ci(chars: &[char], start: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (start..=chars.len().checked_sub(needle.len())?).find(|&i| {
        chars[i..i + needle.len()]
            .iter()
            .zip(&needle)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

/// Apply the inline Markdown phases to one paragraph of [`lift_opaque`]
/// output.
///
/// Order: links → bold → italics. Every placeholder survives untouched for
/// the token parser.
pub(super) fn preprocess_inline(paragraph: &str) -> String {
    let (with_links, hrefs) = convert_links(paragraph);
    let with_bold = convert_bold(&with_links);
    let with_italics = convert_italics(&with_bold);
    restore_hrefs(&with_italics, &hrefs)
}

/// The CommonMark value of a code span's raw contents.
///
/// Backslashes are literal. Line endings become spaces, and one space is
/// stripped from each side when the content begins and ends with a space
/// and is not all spaces.
fn code_span_value(contents: &[char]) -> String {
    let mut value: String = contents.iter().map(|&c| if c == '\n' { ' ' } else { c }).collect();
    if value.len() >= 2 && value.starts_with(' ') && value.ends_with(' ') && !value.chars().all(|c| c == ' ') {
        value = value[1..value.len() - 1].to_string();
    }
    value
}

/// Index just past a run of two or more newlines starting at `start`
/// (`chars[start]` must be `\n`) separated only by spaces or tabs, or `None`
/// when the newline at `start` stands alone. Trailing spaces after the last
/// newline are not part of the run.
pub(super) fn blank_run_end(chars: &[char], start: usize) -> Option<usize> {
    let (count, end) = newline_run(chars, start);
    (count >= 2).then_some(end)
}

/// The number of newlines in the run starting at `chars[start]` (which must
/// be `\n`), separated only by spaces or tabs, and the index just past the
/// last newline of the run.
pub(super) fn newline_run(chars: &[char], start: usize) -> (usize, usize) {
    debug_assert_eq!(chars[start], '\n');
    let mut count = 1;
    let mut end = start + 1;
    let mut j = end;
    loop {
        while j < chars.len() && matches!(chars[j], ' ' | '\t') {
            j += 1;
        }
        if j < chars.len() && chars[j] == '\n' {
            count += 1;
            j += 1;
            end = j;
        } else {
            break;
        }
    }
    (count, end)
}

/// Length of the backtick run starting at `start`.
fn backtick_run_len(chars: &[char], start: usize) -> usize {
    chars[start..].iter().take_while(|&&c| c == '`').count()
}

/// Index of the first backtick run of exactly `len` at or after `start`,
/// searching no further than the next blank line or fence line (a span never
/// crosses a paragraph boundary or a fenced block).
fn closing_backtick_run(chars: &[char], start: usize, len: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '\n' && (blank_run_end(chars, i).is_some() || is_fence_line(chars, i + 1)) {
            return None;
        }
        if chars[i] == '`' {
            let run = backtick_run_len(chars, i);
            if run == len {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

/// Whether the line starting at `chars[start]` opens a fenced block.
fn is_fence_line(chars: &[char], start: usize) -> bool {
    let rest = chars.get(start..).unwrap_or_default();
    let indent = rest.iter().take_while(|&&c| c != '\n' && c.is_whitespace()).count();
    rest[indent..].starts_with(&['`', '`', '`'])
}

/// Characters that participate in backslash escape sequences during
/// Markdown pre-processing. Mirrors the set recognised by
/// [`super::tokens::parse_render_nodes`]; the sentinels are included because
/// [`lift_opaque`] escapes literal ones.
pub(super) fn is_escapable(c: char) -> bool {
    matches!(
        c,
        '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '{' | '`' | '\\'
    ) || is_sentinel(c)
}

/// Copy a tag declaration `<…>` from the input to the output without
/// interpreting any Markdown inside it. Returns the index just past the
/// closing `>`, or `None` when [`tag_declaration_end`] finds none (the caller
/// then falls through to the default per-character path).
///
/// The scan is quote-aware like the token parser's, so the two agree on
/// where a declaration ends: attribute values such as
/// `href="path_with_underscores"` or `href="a>**b**"` are never read as
/// Markdown.
fn copy_tag_declaration(chars: &[char], start: usize, output: &mut String) -> Option<usize> {
    let end = tag_declaration_end(chars, start)?;
    output.extend(&chars[start..end]);
    Some(end)
}

/// Phase 1: lift `[desc](ref)` into `<a href="...">desc</a>`.
///
/// The `ref` value is replaced by an opaque placeholder of the form
/// `\u{0001}HREF<index>\u{0001}` and stored in the returned `Vec`. This
/// shields URL contents from later bold/italics phases.
fn convert_links(input: &str) -> (String, Vec<String>) {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::new();
    let mut hrefs: Vec<String> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            output.push(ch);
            output.push(chars[i + 1]);
            i += 2;
            continue;
        }

        if ch == '<'
            && let Some(next) = copy_tag_declaration(&chars, i, &mut output)
        {
            i = next;
            continue;
        }

        if ch == '['
            && let Some((desc, href, end)) = try_parse_link(&chars, i)
        {
            let placeholder = format!("{m}HREF{n}{m}", m = HREF_PLACEHOLDER_MARK, n = hrefs.len());
            hrefs.push(href);
            output.push_str("<a href=\"");
            output.push_str(&placeholder);
            output.push_str("\">");
            output.push_str(&desc);
            output.push_str("</a>");
            i = end;
            continue;
        }

        output.push(ch);
        i += 1;
    }

    (output, hrefs)
}

/// Try to parse `[desc](ref)` starting at `start` (which must point at the
/// opening `[`). Returns the description, raw href, and the index *after*
/// the closing `)`. Backslash escapes and tag declarations inside the
/// description are passed through verbatim, so a `]` in a quoted attribute
/// never ends it. Inside the href, only `\(` and `\)` are interpreted
/// (the backslash is dropped and the paren is kept as a literal); all
/// other characters are taken literally.
fn try_parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    debug_assert_eq!(chars[start], '[');

    let mut i = start + 1;
    let mut desc = String::new();

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            desc.push(ch);
            desc.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if ch == '<'
            && let Some(next) = copy_tag_declaration(chars, i, &mut desc)
        {
            i = next;
            continue;
        }
        if ch == ']' {
            break;
        }
        desc.push(ch);
        i += 1;
    }

    if i >= chars.len() || chars[i] != ']' {
        return None;
    }

    let after_bracket = i + 1;
    if after_bracket >= chars.len() || chars[after_bracket] != '(' {
        return None;
    }

    let mut j = after_bracket + 1;
    let mut href = String::new();

    while j < chars.len() {
        let ch = chars[j];

        if ch == '\\'
            && j + 1 < chars.len()
            && (chars[j + 1] == '(' || chars[j + 1] == ')' || is_sentinel(chars[j + 1]))
        {
            href.push(chars[j + 1]);
            j += 2;
            continue;
        }
        if ch == ')' {
            break;
        }
        href.push(ch);
        j += 1;
    }

    if j >= chars.len() || chars[j] != ')' {
        return None;
    }

    Some((desc, href, j + 1))
}

/// Phase 2: convert `**text**` into `<b>text</b>`. Single `*` (and the
/// `__bold__` form) are intentionally left untouched. Markdown inside
/// `<…>` tag declarations is preserved verbatim so attribute values are
/// never re-interpreted.
fn convert_bold(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::new();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            output.push(ch);
            output.push(chars[i + 1]);
            i += 2;
            continue;
        }

        if ch == '<'
            && let Some(next) = copy_tag_declaration(&chars, i, &mut output)
        {
            i = next;
            continue;
        }

        if ch == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let (prev, next) = neighbours(&chars, i, 2);
            if Delimiter::Bold.can_open(prev, next)
                && let Some(end) = find_closing_double(&chars, i + 2, '*')
            {
                output.push_str("<b>");
                output.extend(&chars[i + 2..end]);
                output.push_str("</b>");
                i = end + 2;
                continue;
            }
        }

        output.push(ch);
        i += 1;
    }

    output
}

/// Phase 3: convert `_text_` into `<i>text</i>`. The `*italics*` form
/// and `__double underscore__` are intentionally left untouched. URL
/// contents are protected by the placeholder substitution from
/// [`convert_links`]; Markdown inside `<…>` tag declarations is also
/// preserved verbatim.
fn convert_italics(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::new();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            output.push(ch);
            output.push(chars[i + 1]);
            i += 2;
            continue;
        }

        if ch == '<'
            && let Some(next) = copy_tag_declaration(&chars, i, &mut output)
        {
            i = next;
            continue;
        }

        // Doubled underscores (`__…__`) are explicitly NOT bold per spec.
        // Skip a `__` pair as literal text so neither underscore can act
        // as an italics opener.
        if ch == '_' && i + 1 < chars.len() && chars[i + 1] == '_' {
            output.push('_');
            output.push('_');
            i += 2;
            continue;
        }

        if ch == '_' {
            let (prev, next) = neighbours(&chars, i, 1);
            if Delimiter::Italic.can_open(prev, next)
                && let Some(end) = find_closing_single(&chars, i + 1, '_')
            {
                output.push_str("<i>");
                output.extend(&chars[i + 1..end]);
                output.push_str("</i>");
                i = end + 1;
                continue;
            }
        }

        output.push(ch);
        i += 1;
    }

    output
}

/// Tracks how deeply the closer search has descended into recognized tags
/// since the opener. A closer is valid only at depth zero, and the search
/// fails once it walks out of the tag that contains the opener, so emphasis
/// can never straddle a tag boundary and unbalance the tag stream.
struct TagDepth(usize);

impl TagDepth {
    /// Account for the tag declaration at `chars[start..end]`. Returns
    /// `false` when it closes a tag the opener sits inside.
    fn step(&mut self, chars: &[char], start: usize, end: usize) -> bool {
        let content: String = chars[start + 1..end - 1].iter().collect();
        if content.starts_with('/') {
            if self.0 == 0 {
                return false;
            }
            self.0 -= 1;
        } else if is_recognized_opening_tag(&content) {
            self.0 += 1;
        }
        true
    }
}

/// Locate a closing `marker`+`marker` pair (e.g. `**`) at or after `start`,
/// honouring backslash escapes and tag nesting ([`TagDepth`]). Only a pair
/// that [`Delimiter::Bold`] allows to close counts. Returns the index of the
/// first marker of the closing pair, or `None` if no closer is found.
fn find_closing_double(chars: &[char], start: usize, marker: char) -> Option<usize> {
    let mut depth = TagDepth(0);
    let mut i = start;
    while i + 1 < chars.len() {
        let ch = chars[i];
        if ch == '\\' && is_escapable(chars[i + 1]) {
            i += 2;
            continue;
        }
        if ch == '<'
            && let Some(end) = scan_past_tag_declaration(chars, i)
        {
            if !depth.step(chars, i, end) {
                return None;
            }
            i = end;
            continue;
        }
        if ch == marker && chars[i + 1] == marker {
            if i == start {
                return None;
            }
            let (prev, next) = neighbours(chars, i, 2);
            if depth.0 == 0 && Delimiter::Bold.can_close(prev, next) {
                return Some(i);
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    None
}

/// Locate a closing single-character `marker` at or after `start`,
/// honouring backslash escapes and tag nesting ([`TagDepth`]), and treating
/// doubled `marker` runs (`__`) as opaque so a single `_` inside a `__…__`
/// sequence cannot terminate an outer italics run. Only a marker that
/// [`Delimiter::Italic`] allows to close counts. Returns the index of the
/// closing marker, or `None` if no closer is found.
fn find_closing_single(chars: &[char], start: usize, marker: char) -> Option<usize> {
    let mut depth = TagDepth(0);
    let mut i = start;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            i += 2;
            continue;
        }
        if ch == '<'
            && let Some(end) = scan_past_tag_declaration(chars, i)
        {
            if !depth.step(chars, i, end) {
                return None;
            }
            i = end;
            continue;
        }
        if ch == marker && i + 1 < chars.len() && chars[i + 1] == marker {
            i += 2;
            continue;
        }
        if ch == marker {
            if i == start {
                return None;
            }
            let (prev, next) = neighbours(chars, i, 1);
            if depth.0 == 0 && Delimiter::Italic.can_close(prev, next) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Variant of [`copy_tag_declaration`] that only advances the index
/// without copying anything. Used inside the closer-search helpers,
/// which must not mutate output state.
fn scan_past_tag_declaration(chars: &[char], start: usize) -> Option<usize> {
    tag_declaration_end(chars, start)
}

/// Final phase: replace each `\u{0001}HREF<n>\u{0001}` placeholder with
/// the corresponding stored href value.
///
/// A single left-to-right pass, skipping escape pairs, so neither an
/// escaped literal sentinel nor a restored href value is ever read as a
/// placeholder.
fn restore_hrefs(input: &str, hrefs: &[String]) -> String {
    if hrefs.is_empty() {
        return input.to_string();
    }
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            output.push(ch);
            output.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if ch == HREF_PLACEHOLDER_MARK
            && let Some((href, next)) = href_placeholder(&chars, i, hrefs)
        {
            output.push_str(href);
            i = next;
            continue;
        }
        output.push(ch);
        i += 1;
    }
    output
}

/// Resolve the `\u{0001}HREF<n>\u{0001}` placeholder at `chars[start]`.
fn href_placeholder<'a>(chars: &[char], start: usize, hrefs: &'a [String]) -> Option<(&'a str, usize)> {
    let prefix = ['H', 'R', 'E', 'F'];
    if chars.get(start + 1..start + 5)? != prefix {
        return None;
    }
    let digits_start = start + 5;
    let digits_end = digits_start + chars[digits_start..].iter().take_while(|c| c.is_ascii_digit()).count();
    if digits_end == digits_start || chars.get(digits_end) != Some(&HREF_PLACEHOLDER_MARK) {
        return None;
    }
    let index: usize = chars[digits_start..digits_end].iter().collect::<String>().parse().ok()?;
    Some((hrefs.get(index)?.as_str(), digits_end + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the pre-processor and return only the converted text.
    fn pp(input: &str) -> String {
        preprocess_inline(&lift_opaque(input).0)
    }

    /// Run the pre-processor and return the text plus every lifted span value.
    fn pp_spans(input: &str) -> (String, Vec<String>) {
        let (text, lifted) = lift_opaque(input);
        let out = preprocess_inline(&text);
        let spans = lifted
            .into_iter()
            .filter_map(|l| match l {
                Lifted::Span(v) => Some(v),
                _ => None,
            })
            .collect();
        (out, spans)
    }

    fn fences(input: &str) -> (String, Vec<FencedCode>) {
        let (text, lifted) = lift_opaque(input);
        let blocks = lifted
            .into_iter()
            .filter_map(|l| match l {
                Lifted::Fence(f) => Some(f),
                _ => None,
            })
            .collect();
        (text, blocks)
    }

    #[test]
    fn link_basic_conversion() {
        assert_eq!(
            pp("[click here](https://example.com)"),
            "<a href=\"https://example.com\">click here</a>",
        );
    }

    #[test]
    fn link_url_with_underscores_preserved() {
        assert_eq!(
            pp("[link](https://example.com/path_with_underscores)"),
            "<a href=\"https://example.com/path_with_underscores\">link</a>",
        );
    }

    #[test]
    fn link_url_with_asterisks_preserved() {
        assert_eq!(
            pp("[link](https://example.com/a*b*c)"),
            "<a href=\"https://example.com/a*b*c\">link</a>",
        );
    }

    #[test]
    fn link_url_with_double_asterisks_preserved() {
        assert_eq!(
            pp("[link](https://example.com/a**b)"),
            "<a href=\"https://example.com/a**b\">link</a>",
        );
    }

    #[test]
    fn escaped_brackets_render_as_literal_text() {
        // Both `\[` and `\]` escapes survive pre-processing; downstream
        // tokens.rs converts them to literal `[` and `]`.
        assert_eq!(pp(r"\[not a link\](url)"), r"\[not a link\](url)");
    }

    #[test]
    fn escaped_paren_inside_description_does_not_terminate_link() {
        // `[desc\]](url)` -> description preserves `\]` as escape, link
        // closes at the unescaped `]`.
        assert_eq!(pp(r"[desc\]](url)"), "<a href=\"url\">desc\\]</a>",);
    }

    #[test]
    fn link_with_escaped_paren_in_url() {
        // `\)` inside the href should be folded to a literal `)`.
        assert_eq!(
            pp(r"[link](http://e.com/p\)q)"),
            "<a href=\"http://e.com/p)q\">link</a>",
        );
    }

    #[test]
    fn malformed_link_no_paren_left_alone() {
        assert_eq!(pp("[desc] no paren"), "[desc] no paren");
    }

    #[test]
    fn malformed_link_unclosed_paren_left_alone() {
        assert_eq!(pp("[desc](no close"), "[desc](no close");
    }

    #[test]
    fn bold_basic_conversion() {
        assert_eq!(pp("**bold text**"), "<b>bold text</b>");
    }

    #[test]
    fn bold_escaped_asterisks_left_literal() {
        assert_eq!(pp(r"\*\*not bold\*\*"), r"\*\*not bold\*\*",);
    }

    #[test]
    fn bold_underscore_double_form_unsupported() {
        // Per spec, `__bold__` is NOT bold — left as-is.
        assert_eq!(pp("__bold__"), "__bold__");
    }

    #[test]
    fn italics_basic_conversion() {
        assert_eq!(pp("_italic text_"), "<i>italic text</i>");
    }

    #[test]
    fn italics_escaped_underscores_left_literal() {
        assert_eq!(pp(r"\_not italic\_"), r"\_not italic\_");
    }

    #[test]
    fn italics_asterisk_form_unsupported() {
        // Per spec, `*italics*` is NOT italic — left as-is.
        assert_eq!(pp("*italics*"), "*italics*");
    }

    #[test]
    fn nested_bold_with_inner_italics() {
        // Bold phase wraps `**...**`; italics phase then converts the
        // inner `_..._`.
        assert_eq!(
            pp("**bold _and italics_**"),
            "<b>bold <i>and italics</i></b>",
        );
    }

    #[test]
    fn nested_bold_italics() {
        assert_eq!(pp("**_bold italics_**"), "<b><i>bold italics</i></b>",);
    }

    #[test]
    fn link_protects_url_underscores_during_italics_phase() {
        // The URL contains underscores at positions that would normally
        // trigger italics; the placeholder shields them.
        assert_eq!(
            pp("[link](https://e.com/_path_)"),
            "<a href=\"https://e.com/_path_\">link</a>",
        );
    }

    #[test]
    fn mixed_markdown_combination() {
        assert_eq!(
            pp("**bold** and _italics_ and [link](url)"),
            "<b>bold</b> and <i>italics</i> and <a href=\"url\">link</a>",
        );
    }

    #[test]
    fn bold_inside_link_description() {
        assert_eq!(
            pp("[**bold link**](url)"),
            "<a href=\"url\"><b>bold link</b></a>",
        );
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(pp(""), "");
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        assert_eq!(pp("just some plain prose"), "just some plain prose",);
    }

    #[test]
    fn empty_link_description_still_converts() {
        assert_eq!(
            pp("[](https://example.com)"),
            "<a href=\"https://example.com\"></a>",
        );
    }

    #[test]
    fn empty_link_url_still_converts() {
        assert_eq!(pp("[desc]()"), "<a href=\"\">desc</a>");
    }

    #[test]
    fn adjacent_bold_runs() {
        assert_eq!(pp("**a****b**"), "<b>a</b><b>b</b>",);
    }

    #[test]
    fn adjacent_italics_runs() {
        // `__` is opaque (doubled markers are not italics openers/closers),
        // so the outer single underscores wrap the entire `a__b` slice.
        assert_eq!(pp("_a__b_"), "<i>a__b</i>");
    }

    #[test]
    fn separate_italics_runs_with_text_between() {
        assert_eq!(pp("_a_ and _b_"), "<i>a</i> and <i>b</i>",);
    }

    #[test]
    fn unclosed_bold_left_alone() {
        assert_eq!(pp("**unfinished"), "**unfinished");
    }

    #[test]
    fn unclosed_italics_left_alone() {
        assert_eq!(pp("_unfinished"), "_unfinished");
    }

    #[test]
    fn empty_bold_run_left_alone() {
        // `****` is not a valid empty bold (would yield `<b></b>` which
        // is meaningless). The scanner refuses zero-width runs.
        assert_eq!(pp("****"), "****");
    }

    #[test]
    fn empty_italics_run_left_alone() {
        assert_eq!(pp("__"), "__");
    }

    #[test]
    fn backslash_escape_for_existing_special_chars_preserved() {
        // Phase 1 of prose-plus added `\*` `\_` `\[` `\]` `\(` `\)` to
        // the escape set; legacy escapes (`\<`, `\>`, `\{`, `\\`) must
        // still pass through pre-processing untouched.
        assert_eq!(pp(r"\<env\>"), r"\<env\>");
        assert_eq!(pp(r"\\path"), r"\\path");
    }

    // -- Flanking rules --------------------------------------------------------
    //
    // These cases verify the "intra-word inhibition" rule: a `_` or `**`
    // delimiter sitting between two word characters cannot open or close
    // emphasis. The rule keeps identifiers like `OPENCODE_CONFIG_CONTENT`
    // and `foo**bar**baz` from being chewed up by the pre-processor.

    #[test]
    fn italics_intra_word_underscores_are_literal() {
        // The canonical regression case: env-var-style identifiers must
        // pass through pre-processing unchanged. Every `_` here has a
        // word character on both sides and therefore cannot open or close.
        assert_eq!(pp("OPENCODE_CONFIG_CONTENT"), "OPENCODE_CONFIG_CONTENT",);
        assert_eq!(pp("foo_bar"), "foo_bar");
        assert_eq!(pp("CLAUDINE_SESSION_ID"), "CLAUDINE_SESSION_ID",);
    }

    #[test]
    fn italics_outer_marks_capture_inner_word_underscore() {
        // `_foo_bar_`: outer `_`s are at word boundaries (start/end), the
        // middle `_` is intra-word. Result: a single italic span over the
        // full `foo_bar` slice.
        assert_eq!(pp("_foo_bar_"), "<i>foo_bar</i>");
    }

    #[test]
    fn bold_intra_word_double_asterisks_are_literal() {
        // Same rule for `**` doubled markers: word-on-both-sides means
        // literal text.
        assert_eq!(pp("foo**bar**baz"), "foo**bar**baz");
    }

    #[test]
    fn bold_outer_marks_capture_inner_word_double_asterisks() {
        // Outer `**` are flanked by start/end (boundaries); inner `**`
        // pairs are intra-word and therefore literal. The outer pair
        // wraps the whole inner slice as bold.
        assert_eq!(pp("**foo**bar**baz**"), "<b>foo**bar**baz</b>",);
    }

    #[test]
    fn italics_punctuation_neighbour_is_a_boundary() {
        // Non-alphanumeric neighbours (parens, brackets, period, slash,
        // colon) form boundaries — emphasis still triggers around them.
        assert_eq!(pp("(_text_)"), "(<i>text</i>)");
        assert_eq!(pp("hit _Esc_."), "hit <i>Esc</i>.");
    }

    #[test]
    fn italics_inside_block_tag_body() {
        // Inside `<dim>…</dim>` the body is processed normally; the
        // tag declarations themselves are copied verbatim. The outer
        // `_` flanks against space and `<`, both non-word.
        assert_eq!(pp("<dim>one _two_</dim>"), "<dim>one <i>two</i></dim>",);
    }

    #[test]
    fn italics_intra_word_cannot_close() {
        // Opener `_` at start has a word boundary; the inner `_` between
        // letters is intra-word and must not close. With no further `_`
        // available, the opener is left as a literal.
        assert_eq!(pp("_foo_bar"), "_foo_bar");
    }

    #[test]
    fn dynamic_interpolation_with_styling_around_identifier() {
        // The original claudine-output regression: a structural tag
        // wraps a dynamic value containing intra-word underscores. The
        // value must render unmodified.
        assert_eq!(
            pp("<dim>=OPENCODE_CONFIG_CONTENT</dim>"),
            "<dim>=OPENCODE_CONFIG_CONTENT</dim>",
        );
    }

    // -- Fenced code blocks --------------------------------------------------

    #[test]
    fn fenced_code_block_basic() {
        // The fence is lifted out as an opaque code block; the converted
        // text keeps only the sentinel placeholder, never a `<code-block>`
        // tag re-injected into the string grammar.
        let (text, blocks) = fences("```yaml\nkey: value\n```");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, "yaml");
        assert_eq!(blocks[0].body, "key: value");
        assert_eq!(text, format!("{LIFT_MARK}0{LIFT_MARK}"));
    }

    #[test]
    fn fenced_code_block_no_lang() {
        let (_, blocks) = fences("```\nplain text\n```");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, "");
        assert_eq!(blocks[0].body, "plain text");
    }

    #[test]
    fn fenced_code_block_preserves_inner_markdown() {
        let (text, blocks) = fences("```\n**not bold**\n_not italic_\n```");
        assert!(!pp(&text).contains("<b>"));
        assert_eq!(blocks[0].body, "**not bold**\n_not italic_");
    }

    #[test]
    fn fenced_code_block_with_closing_tag_in_body_is_opaque() {
        let (text, blocks) = fences("```\n</code-block><red>x</red>\n```");
        assert_eq!(blocks[0].body, "</code-block><red>x</red>");
        assert!(!text.contains("<red>"));
    }

    #[test]
    fn line_endings_are_normalized_before_fences_are_found() {
        let (text, blocks) = fences("a\r\n```\r\nx\ry\r\n```\rb");
        assert_eq!(blocks[0].body, "x\ny");
        assert_eq!(text, format!("a\n{LIFT_MARK}0{LIFT_MARK}\nb"));
    }

    #[test]
    fn literal_sentinels_are_escaped_outside_fences_only() {
        let (text, blocks) = fences("a\u{0002}0\u{0002}\u{0001}\n```\n\u{0002}\n```");
        assert_eq!(text, format!("a\\\u{0002}0\\\u{0002}\\\u{0001}\n{LIFT_MARK}0{LIFT_MARK}"));
        assert_eq!(blocks[0].body, "\u{0002}");
    }

    // ── CommonMark flanking, code spans, and tag balance ─────────────

    #[test]
    fn underscore_after_bold_opener_with_no_valid_closer_is_literal() {
        let input = "1. **_pr/open.md** opens it.\n2. **_pr/triage.md** triages it.";
        assert_eq!(
            pp(input),
            "1. <b>_pr/open.md</b> opens it.\n2. <b>_pr/triage.md</b> triages it."
        );
    }

    #[test]
    fn underscore_preceded_by_space_cannot_close() {
        assert_eq!(
            pp("unknown root '_loop_countx' in '{{ _loop_countx }}'"),
            "unknown root '_loop_countx' in '{{ _loop_countx }}'"
        );
    }

    #[test]
    fn underscore_followed_by_space_cannot_open() {
        assert_eq!(pp("a_ b_"), "a_ b_");
        assert_eq!(pp("x _ y _"), "x _ y _");
    }

    #[test]
    fn underscore_between_punctuation_opens_and_closes() {
        assert_eq!(pp("\"_quoted_\""), "\"<i>quoted</i>\"");
    }

    fn ph(n: usize) -> String {
        format!("{LIFT_MARK}{n}{LIFT_MARK}")
    }

    #[test]
    fn code_span_is_lifted_with_literal_contents() {
        let (text, spans) = pp_spans("See `_pr/_report.md`.");
        assert_eq!(text, format!("See {}.", ph(0)));
        assert_eq!(spans, ["_pr/_report.md"]);
        let (_, spans) = pp_spans("`**x** <red>y</red>`");
        assert_eq!(spans, ["**x** <red>y</red>"]);
    }

    #[test]
    fn code_span_backslashes_are_literal() {
        assert_eq!(pp_spans(r"`a\_b`").1, [r"a\_b"]);
        assert_eq!(pp_spans(r"`a\\`").1, [r"a\\"]);
    }

    #[test]
    fn code_span_value_follows_commonmark_spacing() {
        assert_eq!(pp_spans("`` `a` ``").1, ["`a`"]);
        assert_eq!(pp_spans("` a `").1, ["a"]);
        assert_eq!(pp_spans("`  `").1, ["  "]);
        assert_eq!(pp_spans("`  a  `").1, [" a "]);
        assert_eq!(pp_spans("` a`").1, [" a"]);
        assert_eq!(pp_spans("`a\nb`").1, ["a b"]);
    }

    #[test]
    fn code_span_needs_matching_backtick_run() {
        assert_eq!(pp_spans("``a`b``").1, ["a`b"]);
        assert_eq!(pp("`unclosed _x_"), "`unclosed <i>x</i>");
    }

    #[test]
    fn code_span_never_crosses_a_blank_line() {
        assert_eq!(pp_spans("`a\n\nb`").1, Vec::<String>::new());
        assert_eq!(pp_spans("`a\n  \nb").1, Vec::<String>::new());
        // An unmatched opener stays literal, so a later run may pair up.
        assert_eq!(pp_spans("`a\n\nb` c`").1, [" c"]);
    }

    #[test]
    fn code_span_holding_a_link_stays_literal() {
        let (text, spans) = pp_spans("The `[a_b.md](/x/a_b.md)` plan");
        assert_eq!(text, format!("The {} plan", ph(0)));
        assert_eq!(spans, ["[a_b.md](/x/a_b.md)"]);
    }

    #[test]
    fn code_span_inside_link_text_survives_as_placeholder() {
        let (text, spans) = pp_spans("[`x_y`](u)");
        assert_eq!(text, format!("<a href=\"u\">{}</a>", ph(0)));
        assert_eq!(spans, ["x_y"]);
    }

    #[test]
    fn escaped_sentinel_inside_code_span_is_restored() {
        assert_eq!(pp_spans("`a\u{0002}b`").1, ["a\u{0002}b"]);
        assert_eq!(pp_spans("`a\\\u{0002}b`").1, ["a\\\u{0002}b"]);
    }

    #[test]
    fn literal_href_placeholder_is_not_restored() {
        let input = "[a](u) \u{0001}HREF0\u{0001}";
        assert_eq!(pp(input), "<a href=\"u\">a</a> \\\u{0001}HREF0\\\u{0001}");
    }

    #[test]
    fn code_span_shields_emphasis_delimiters_from_outer_pairing() {
        assert_eq!(pp("_a `b_` c_"), format!("<i>a {} c</i>", ph(0)));
    }

    #[test]
    fn emphasis_cannot_close_across_a_tag_boundary() {
        assert_eq!(pp("<b>a _b</b> c_"), "<b>a _b</b> c_");
        assert_eq!(pp("_a <dim>b_ c</dim>"), "_a <dim>b_ c</dim>");
        assert_eq!(pp("**a <dim>b** c</dim>"), "**a <dim>b** c</dim>");
    }

    #[test]
    fn emphasis_may_contain_a_whole_tag() {
        assert_eq!(pp("_a <dim>b</dim> c_"), "<i>a <dim>b</dim> c</i>");
    }

    #[test]
    fn unrecognized_tag_does_not_block_emphasis() {
        assert_eq!(pp("_see Vec<T> here_"), "<i>see Vec<T> here</i>");
    }
}
