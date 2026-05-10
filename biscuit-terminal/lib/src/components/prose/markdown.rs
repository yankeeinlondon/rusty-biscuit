//! Markdown pre-processing for [`Prose`](super::Prose) input.
//!
//! Converts a curated subset of Markdown syntax into the existing block-tag
//! grammar that [`super::tokens::parse_tokens_inner`] already understands:
//!
//! | Markdown          | Pre-processed form                |
//! |-------------------|-----------------------------------|
//! | `[desc](ref)`     | `<a href="ref">desc</a>`          |
//! | `**text**`        | `<b>text</b>`                     |
//! | `_text_`          | `<i>text</i>`                     |
//!
//! The conversion order is fixed: links → bold → italics. Each phase
//! respects backslash escapes (`\*`, `\_`, `\[`, `\]`, `\(`, `\)`) so
//! literal Markdown characters survive untouched and reach the token
//! parser as-is.
//!
//! Link conversion is *atomic*: the reference (URL) portion of a link is
//! lifted out of the input stream into a placeholder during link parsing
//! and re-inserted only after bold/italics phases have run. This guarantees
//! that `*` or `_` characters inside a URL — for example,
//! `https://example.com/path_with_underscores` — are never re-interpreted
//! as Markdown emphasis markers.

const HREF_PLACEHOLDER_MARK: char = '\u{0001}';

/// Returns `true` when `ch` is an alphanumeric "word" character for the
/// purposes of flanking rules. Backslash, `<`, whitespace, punctuation,
/// and end-of-input are all non-word neighbours and therefore form a
/// boundary at which an emphasis delimiter may open or close.
fn is_word_neighbour(ch: Option<char>) -> bool {
    matches!(ch, Some(c) if c.is_alphanumeric())
}

/// Returns `true` when an emphasis delimiter sits between two word
/// characters and must therefore be treated as a literal rather than a
/// delimiter. This is the rule that prevents identifiers like
/// `OPENCODE_CONFIG_CONTENT` or `foo**bar**baz` from being chewed up by
/// the bold/italics pre-processor.
///
/// We deliberately use a simple "alphanumeric on both sides" predicate
/// rather than the full CommonMark left/right-flanking rules: terminal
/// markup is overwhelmingly ASCII identifiers, predictability matters
/// more than spec parity, and the simpler rule covers every documented
/// acceptance case in
/// [`biscuit-terminal/features/2026-05-05-prose-plus/spec.md`].
fn is_intra_word(prev: Option<char>, next: Option<char>) -> bool {
    is_word_neighbour(prev) && is_word_neighbour(next)
}

const CODE_BLOCK_PLACEHOLDER_MARK: char = '\u{0002}';

/// Phase 0: lift fenced code blocks (` ```lang\n...\n``` `) into opaque
/// placeholders so that inner backticks, asterisks, and underscores are
/// never interpreted as Markdown.
///
/// Returns the text with placeholders and a vector of (language, body)
/// tuples for later restoration.
fn convert_fenced_code_blocks(input: &str) -> (String, Vec<(String, String)>) {
    let lines: Vec<&str> = input.lines().collect();
    let mut output_lines: Vec<String> = Vec::new();
    let mut code_blocks: Vec<(String, String)> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            let lang = trimmed[3..].trim().to_string();
            let mut body_lines = Vec::new();
            i += 1;

            while i < lines.len() {
                let body_line = lines[i];
                if body_line.trim_start() == "```" {
                    i += 1;
                    break;
                }
                body_lines.push(body_line);
                i += 1;
            }

            let placeholder = format!(
                "{m}CODE{n}{m}",
                m = CODE_BLOCK_PLACEHOLDER_MARK,
                n = code_blocks.len()
            );
            code_blocks.push((lang, body_lines.join("\n")));
            output_lines.push(placeholder);
        } else {
            output_lines.push(line.to_string());
            i += 1;
        }
    }

    (output_lines.join("\n"), code_blocks)
}

/// Restore each `\u{0002}CODE<n>\u{0002}` placeholder with a
/// `<code-block lang="...">...</code-block>` tag that the token parser
/// recognises and renders as a dim, indented code block.
fn restore_code_blocks(input: &str, blocks: &[(String, String)]) -> String {
    if blocks.is_empty() {
        return input.to_string();
    }
    let mut output = input.to_string();
    for (i, (lang, body)) in blocks.iter().enumerate() {
        let placeholder = format!("{m}CODE{i}{m}", m = CODE_BLOCK_PLACEHOLDER_MARK);
        let tag = format!(
            "<code-block lang=\"{}\">{}</code-block>",
            html_escape(lang),
            body
        );
        output = output.replace(&placeholder, &tag);
    }
    output
}

/// Minimal HTML escape for code-block language attributes.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Apply the full Markdown pre-processing pipeline.
///
/// Order: fenced code blocks → links → bold → italics. The output is fed
/// verbatim into the existing block-tag parser. Backslash escapes for `*`,
/// `_`, `[`, `]`, `(`, `)` are preserved end-to-end so the downstream parser
/// converts them to literal characters via Phase 1's escape handling.
pub(super) fn preprocess_markdown(input: &str) -> String {
    let (with_code, code_blocks) = convert_fenced_code_blocks(input);
    let (with_links, hrefs) = convert_links(&with_code);
    let with_bold = convert_bold(&with_links);
    let with_italics = convert_italics(&with_bold);
    let restored = restore_hrefs(&with_italics, &hrefs);
    restore_code_blocks(&restored, &code_blocks)
}

/// Characters that participate in backslash escape sequences during
/// Markdown pre-processing. Mirrors the set recognised by
/// [`super::tokens::parse_tokens_inner`].
fn is_escapable(c: char) -> bool {
    matches!(
        c,
        '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '{' | '\\'
    )
}

/// Copy a block-tag declaration `<…>` from the input to the output
/// without interpreting any Markdown inside it. Returns the index just
/// past the closing `>`, or `None` if no closing `>` exists (in which
/// case the caller should fall through to the default per-character
/// path).
///
/// This preserves attribute values like `href="path_with_underscores"`
/// from being chewed up by the italics phase, and keeps any literal
/// Markdown inside an `href`/title attribute opaque to the pre-processor.
fn copy_tag_declaration(chars: &[char], start: usize, output: &mut String) -> Option<usize> {
    debug_assert_eq!(chars[start], '<');
    let mut i = start + 1;
    while i < chars.len() {
        if chars[i] == '>' {
            for c in &chars[start..=i] {
                output.push(*c);
            }
            return Some(i + 1);
        }
        if chars[i] == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            i += 2;
            continue;
        }
        i += 1;
    }
    None
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
/// the closing `)`. Backslash escapes inside the description are passed
/// through verbatim. Inside the href, only `\(` and `\)` are interpreted
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

        if ch == '\\' && j + 1 < chars.len() && (chars[j + 1] == '(' || chars[j + 1] == ')') {
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
            // Flanking rule: `**` is a bold opener only when it does NOT sit
            // between two word characters. This keeps identifiers like
            // `foo**bar**baz` from triggering bold.
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 2 < chars.len() {
                Some(chars[i + 2])
            } else {
                None
            };
            if !is_intra_word(prev, next)
                && let Some(end) = find_closing_double(&chars, i + 2, '*')
            {
                output.push_str("<b>");
                for c in &chars[i + 2..end] {
                    output.push(*c);
                }
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
            // Flanking rule: `_` is an italics opener only when it does NOT
            // sit between two word characters. This keeps identifiers like
            // `OPENCODE_CONFIG_CONTENT` from being italicised mid-word.
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 1 < chars.len() {
                Some(chars[i + 1])
            } else {
                None
            };
            if !is_intra_word(prev, next)
                && let Some(end) = find_closing_single(&chars, i + 1, '_')
            {
                output.push_str("<i>");
                for c in &chars[i + 1..end] {
                    output.push(*c);
                }
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

/// Locate a closing `marker`+`marker` pair (e.g. `**`) at or after `start`,
/// honouring backslash escapes, skipping over `<…>` tag declarations
/// embedded in the prose, and applying the same intra-word inhibition as
/// the opener: a doubled marker that sits between two word characters
/// cannot close. Returns the index of the first marker of the closing
/// pair, or `None` if no closer is found before EOF.
fn find_closing_double(chars: &[char], start: usize, marker: char) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        let ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            i += 2;
            continue;
        }
        if ch == '<'
            && let Some(end) = scan_past_tag_declaration(chars, i)
        {
            i = end;
            continue;
        }
        if ch == marker && chars[i + 1] == marker {
            if i == start {
                return None;
            }
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 2 < chars.len() {
                Some(chars[i + 2])
            } else {
                None
            };
            if is_intra_word(prev, next) {
                // Doubled marker is intra-word — keep scanning, this is
                // not a real closer.
                i += 2;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Locate a closing single-character `marker` at or after `start`,
/// honouring backslash escapes, skipping over `<…>` tag declarations,
/// treating doubled `marker` runs (`__`) as opaque so a single `_`
/// inside a `__…__` sequence cannot terminate an outer italics run, and
/// applying the same intra-word inhibition as the opener: a single
/// marker that sits between two word characters cannot close.
/// Returns the index of the closing marker, or `None` if no closer is
/// found before EOF.
fn find_closing_single(chars: &[char], start: usize, marker: char) -> Option<usize> {
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
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 1 < chars.len() {
                Some(chars[i + 1])
            } else {
                None
            };
            if is_intra_word(prev, next) {
                // Intra-word single marker — keep scanning.
                i += 1;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Variant of [`copy_tag_declaration`] that only advances the index
/// without copying anything. Used inside the closer-search helpers,
/// which must not mutate output state.
fn scan_past_tag_declaration(chars: &[char], start: usize) -> Option<usize> {
    debug_assert_eq!(chars[start], '<');
    let mut i = start + 1;
    while i < chars.len() {
        if chars[i] == '>' {
            return Some(i + 1);
        }
        if chars[i] == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            i += 2;
            continue;
        }
        i += 1;
    }
    None
}

/// Final phase: replace each `\u{0001}HREF<n>\u{0001}` placeholder with
/// the corresponding stored href value.
fn restore_hrefs(input: &str, hrefs: &[String]) -> String {
    if hrefs.is_empty() {
        return input.to_string();
    }
    let mut output = input.to_string();
    for (i, href) in hrefs.iter().enumerate() {
        let placeholder = format!("{m}HREF{i}{m}", m = HREF_PLACEHOLDER_MARK);
        output = output.replace(&placeholder, href);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_basic_conversion() {
        assert_eq!(
            preprocess_markdown("[click here](https://example.com)"),
            "<a href=\"https://example.com\">click here</a>",
        );
    }

    #[test]
    fn link_url_with_underscores_preserved() {
        assert_eq!(
            preprocess_markdown("[link](https://example.com/path_with_underscores)"),
            "<a href=\"https://example.com/path_with_underscores\">link</a>",
        );
    }

    #[test]
    fn link_url_with_asterisks_preserved() {
        assert_eq!(
            preprocess_markdown("[link](https://example.com/a*b*c)"),
            "<a href=\"https://example.com/a*b*c\">link</a>",
        );
    }

    #[test]
    fn link_url_with_double_asterisks_preserved() {
        assert_eq!(
            preprocess_markdown("[link](https://example.com/a**b)"),
            "<a href=\"https://example.com/a**b\">link</a>",
        );
    }

    #[test]
    fn escaped_brackets_render_as_literal_text() {
        // Both `\[` and `\]` escapes survive pre-processing; downstream
        // tokens.rs converts them to literal `[` and `]`.
        assert_eq!(
            preprocess_markdown(r"\[not a link\](url)"),
            r"\[not a link\](url)"
        );
    }

    #[test]
    fn escaped_paren_inside_description_does_not_terminate_link() {
        // `[desc\]](url)` -> description preserves `\]` as escape, link
        // closes at the unescaped `]`.
        assert_eq!(
            preprocess_markdown(r"[desc\]](url)"),
            "<a href=\"url\">desc\\]</a>",
        );
    }

    #[test]
    fn link_with_escaped_paren_in_url() {
        // `\)` inside the href should be folded to a literal `)`.
        assert_eq!(
            preprocess_markdown(r"[link](http://e.com/p\)q)"),
            "<a href=\"http://e.com/p)q\">link</a>",
        );
    }

    #[test]
    fn malformed_link_no_paren_left_alone() {
        assert_eq!(preprocess_markdown("[desc] no paren"), "[desc] no paren");
    }

    #[test]
    fn malformed_link_unclosed_paren_left_alone() {
        assert_eq!(preprocess_markdown("[desc](no close"), "[desc](no close");
    }

    #[test]
    fn bold_basic_conversion() {
        assert_eq!(preprocess_markdown("**bold text**"), "<b>bold text</b>");
    }

    #[test]
    fn bold_escaped_asterisks_left_literal() {
        assert_eq!(
            preprocess_markdown(r"\*\*not bold\*\*"),
            r"\*\*not bold\*\*",
        );
    }

    #[test]
    fn bold_underscore_double_form_unsupported() {
        // Per spec, `__bold__` is NOT bold — left as-is.
        assert_eq!(preprocess_markdown("__bold__"), "__bold__");
    }

    #[test]
    fn italics_basic_conversion() {
        assert_eq!(preprocess_markdown("_italic text_"), "<i>italic text</i>");
    }

    #[test]
    fn italics_escaped_underscores_left_literal() {
        assert_eq!(preprocess_markdown(r"\_not italic\_"), r"\_not italic\_");
    }

    #[test]
    fn italics_asterisk_form_unsupported() {
        // Per spec, `*italics*` is NOT italic — left as-is.
        assert_eq!(preprocess_markdown("*italics*"), "*italics*");
    }

    #[test]
    fn nested_bold_with_inner_italics() {
        // Bold phase wraps `**...**`; italics phase then converts the
        // inner `_..._`.
        assert_eq!(
            preprocess_markdown("**bold _and italics_**"),
            "<b>bold <i>and italics</i></b>",
        );
    }

    #[test]
    fn nested_bold_italics() {
        assert_eq!(
            preprocess_markdown("**_bold italics_**"),
            "<b><i>bold italics</i></b>",
        );
    }

    #[test]
    fn link_protects_url_underscores_during_italics_phase() {
        // The URL contains underscores at positions that would normally
        // trigger italics; the placeholder shields them.
        assert_eq!(
            preprocess_markdown("[link](https://e.com/_path_)"),
            "<a href=\"https://e.com/_path_\">link</a>",
        );
    }

    #[test]
    fn mixed_markdown_combination() {
        assert_eq!(
            preprocess_markdown("**bold** and _italics_ and [link](url)"),
            "<b>bold</b> and <i>italics</i> and <a href=\"url\">link</a>",
        );
    }

    #[test]
    fn bold_inside_link_description() {
        assert_eq!(
            preprocess_markdown("[**bold link**](url)"),
            "<a href=\"url\"><b>bold link</b></a>",
        );
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(preprocess_markdown(""), "");
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        assert_eq!(
            preprocess_markdown("just some plain prose"),
            "just some plain prose",
        );
    }

    #[test]
    fn empty_link_description_still_converts() {
        assert_eq!(
            preprocess_markdown("[](https://example.com)"),
            "<a href=\"https://example.com\"></a>",
        );
    }

    #[test]
    fn empty_link_url_still_converts() {
        assert_eq!(preprocess_markdown("[desc]()"), "<a href=\"\">desc</a>");
    }

    #[test]
    fn adjacent_bold_runs() {
        assert_eq!(preprocess_markdown("**a****b**"), "<b>a</b><b>b</b>",);
    }

    #[test]
    fn adjacent_italics_runs() {
        // `__` is opaque (doubled markers are not italics openers/closers),
        // so the outer single underscores wrap the entire `a__b` slice.
        assert_eq!(preprocess_markdown("_a__b_"), "<i>a__b</i>");
    }

    #[test]
    fn separate_italics_runs_with_text_between() {
        assert_eq!(preprocess_markdown("_a_ and _b_"), "<i>a</i> and <i>b</i>",);
    }

    #[test]
    fn unclosed_bold_left_alone() {
        assert_eq!(preprocess_markdown("**unfinished"), "**unfinished");
    }

    #[test]
    fn unclosed_italics_left_alone() {
        assert_eq!(preprocess_markdown("_unfinished"), "_unfinished");
    }

    #[test]
    fn empty_bold_run_left_alone() {
        // `****` is not a valid empty bold (would yield `<b></b>` which
        // is meaningless). The scanner refuses zero-width runs.
        assert_eq!(preprocess_markdown("****"), "****");
    }

    #[test]
    fn empty_italics_run_left_alone() {
        assert_eq!(preprocess_markdown("__"), "__");
    }

    #[test]
    fn backslash_escape_for_existing_special_chars_preserved() {
        // Phase 1 of prose-plus added `\*` `\_` `\[` `\]` `\(` `\)` to
        // the escape set; legacy escapes (`\<`, `\>`, `\{`, `\\`) must
        // still pass through pre-processing untouched.
        assert_eq!(preprocess_markdown(r"\<env\>"), r"\<env\>");
        assert_eq!(preprocess_markdown(r"\\path"), r"\\path");
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
        assert_eq!(
            preprocess_markdown("OPENCODE_CONFIG_CONTENT"),
            "OPENCODE_CONFIG_CONTENT",
        );
        assert_eq!(preprocess_markdown("foo_bar"), "foo_bar");
        assert_eq!(
            preprocess_markdown("CLAUDINE_SESSION_ID"),
            "CLAUDINE_SESSION_ID",
        );
    }

    #[test]
    fn italics_outer_marks_capture_inner_word_underscore() {
        // `_foo_bar_`: outer `_`s are at word boundaries (start/end), the
        // middle `_` is intra-word. Result: a single italic span over the
        // full `foo_bar` slice.
        assert_eq!(preprocess_markdown("_foo_bar_"), "<i>foo_bar</i>");
    }

    #[test]
    fn bold_intra_word_double_asterisks_are_literal() {
        // Same rule for `**` doubled markers: word-on-both-sides means
        // literal text.
        assert_eq!(preprocess_markdown("foo**bar**baz"), "foo**bar**baz");
    }

    #[test]
    fn bold_outer_marks_capture_inner_word_double_asterisks() {
        // Outer `**` are flanked by start/end (boundaries); inner `**`
        // pairs are intra-word and therefore literal. The outer pair
        // wraps the whole inner slice as bold.
        assert_eq!(
            preprocess_markdown("**foo**bar**baz**"),
            "<b>foo**bar**baz</b>",
        );
    }

    #[test]
    fn italics_punctuation_neighbour_is_a_boundary() {
        // Non-alphanumeric neighbours (parens, brackets, period, slash,
        // colon) form boundaries — emphasis still triggers around them.
        assert_eq!(preprocess_markdown("(_text_)"), "(<i>text</i>)");
        assert_eq!(preprocess_markdown("hit _Esc_."), "hit <i>Esc</i>.");
    }

    #[test]
    fn italics_inside_block_tag_body() {
        // Inside `<dim>…</dim>` the body is processed normally; the
        // tag declarations themselves are copied verbatim. The outer
        // `_` flanks against space and `<`, both non-word.
        assert_eq!(
            preprocess_markdown("<dim>one _two_</dim>"),
            "<dim>one <i>two</i></dim>",
        );
    }

    #[test]
    fn italics_intra_word_cannot_close() {
        // Opener `_` at start has a word boundary; the inner `_` between
        // letters is intra-word and must not close. With no further `_`
        // available, the opener is left as a literal.
        assert_eq!(preprocess_markdown("_foo_bar"), "_foo_bar");
    }

    #[test]
    fn dynamic_interpolation_with_styling_around_identifier() {
        // The original claudine-output regression: a structural tag
        // wraps a dynamic value containing intra-word underscores. The
        // value must render unmodified.
        assert_eq!(
            preprocess_markdown("<dim>=OPENCODE_CONFIG_CONTENT</dim>"),
            "<dim>=OPENCODE_CONFIG_CONTENT</dim>",
        );
    }

    // -- Fenced code blocks --------------------------------------------------

    #[test]
    fn fenced_code_block_basic() {
        let input = "```yaml\nkey: value\n```";
        let out = preprocess_markdown(input);
        assert!(out.contains("<code-block lang=\"yaml\">"));
        assert!(out.contains("key: value"));
        assert!(out.contains("</code-block>"));
    }

    #[test]
    fn fenced_code_block_no_lang() {
        let input = "```\nplain text\n```";
        let out = preprocess_markdown(input);
        assert!(out.contains("<code-block lang=\"\">"));
        assert!(out.contains("plain text"));
    }

    #[test]
    fn fenced_code_block_preserves_inner_markdown() {
        // Asterisks and underscores inside code blocks must NOT be
        // interpreted as emphasis.
        let input = "```\n**not bold**\n_not italic_\n```";
        let out = preprocess_markdown(input);
        assert!(!out.contains("<b>"));
        assert!(!out.contains("<i>"));
        assert!(out.contains("**not bold**"));
        assert!(out.contains("_not italic_"));
    }
}
