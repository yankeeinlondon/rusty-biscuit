//! Attached documentation: the doc comments before a declaration, or a
//! Python docstring. Ordinary comments are never documentation.

use super::*;
use crate::file::parents::IndexedParent as _;

/// The cleaned documentation attached to the declaration `node`, or `None`
/// when it has none (or only blank doc comments).
///
/// Which comments document a declaration is per language (see
/// [`is_doc_comment_text`]); ordinary comments between them and the
/// declaration are skipped. Python reads the docstring of a `def` or
/// `class`. Lua, Perl, Bash and Zsh have no doc-comment detection and always
/// return `None`.
///
/// A declaration with no doc comments of its own is documented by those
/// before its enclosing wrapper (see [`documents_through`]): the nearest
/// comments win, so a Go spec's own doc inside a `var (…)`, `const (…)` or
/// `type (…)` group is kept and the group's doc documents only the specs
/// without one. In Go "without one" means no attached comment group at all
/// ([`Attachment::Absent`]): a spec whose own group reads as no text (blank
/// `//`, an empty block, only directives) is undocumented and does not
/// inherit the group's doc or its `Deprecated:` paragraph. In every other
/// language an empty own doc still falls back to the wrapper's.
pub(crate) fn extract_doc_comment(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    match language {
        ProgrammingLanguage::Python => python_docstring(node, source),
        ProgrammingLanguage::Lua
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => None,
        _ => {
            let mut current = node;
            loop {
                match collect_doc_comments(current, language, source) {
                    Attachment::Text(doc) => return Some(doc),
                    // `go/doc` falls back to the group's doc only when a
                    // spec's comment-group pointer is nil, before reading its
                    // text: an attached group that reads as nothing is still
                    // the spec's own doc. No other language's doc tool is
                    // known to draw that line, so their wrappers keep
                    // documenting a declaration whose own doc is empty.
                    Attachment::Empty if language == ProgrammingLanguage::Go => return None,
                    Attachment::Empty | Attachment::Absent => {}
                }
                match current.indexed_parent() {
                    Some(parent) if documents_through(parent, language) => current = parent,
                    _ => return None,
                }
            }
        }
    }
}

/// Whether the comments before `wrapper` document the declarations directly
/// inside it:
///
/// - `export`, a JS/TS variable statement or ambient `declare`, and a C++
///   `template<…>` header
/// - Go `var`, `const` and `type` declarations, alone or grouped: every spec
///   of a group with no attached comment group shares the group's doc
///   (`go/doc`)
/// - a C/C++ `extern "…"` linkage specification of one declaration (a
///   braced block's members sit in a `declaration_list` and never climb)
/// - a Scala `extension` of a single method; a collective extension's doc
///   documents none of its methods
fn documents_through(wrapper: Node<'_>, language: ProgrammingLanguage) -> bool {
    match wrapper.kind() {
        "export_statement"
        | "export_declaration"
        | "ambient_declaration"
        | "lexical_declaration"
        | "variable_declarator"
        | "variable_declaration"
        | "template_declaration" => true,
        "var_declaration" | "var_spec_list" | "const_declaration" | "type_declaration" => {
            language == ProgrammingLanguage::Go
        }
        "linkage_specification" => matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp),
        "extension_definition" => {
            let mut cursor = wrapper.walk();
            language == ProgrammingLanguage::Scala
                && wrapper
                    .children_by_field_name("body", &mut cursor)
                    .filter(|body| body.is_named())
                    .count()
                    == 1
        }
        _ => false,
    }
}

/// The doc comments attached directly before one declaration or wrapper.
enum Attachment {
    /// No doc-form comment is attached: none at all, only ordinary comments,
    /// a Go group separated by a blank line, or only comments trailing the
    /// preceding code.
    Absent,
    /// Doc-form comments are attached, but their cleaned text is blank (a
    /// bare `//`, an empty block, Go directives only).
    Empty,
    /// The cleaned, nonblank documentation.
    Text(String),
}

/// Collects the doc comments in the run of comments (and, in Rust,
/// attributes) directly before `node`, keeping whether any were attached
/// apart from whether their text is blank ([`Attachment`]).
///
/// In Go, JavaScript and TypeScript a comment that starts on the line where
/// the preceding code ends trails that code and documents nothing after it
/// (see [`trailing_comment_count`]); it and the comments chained to it on
/// the same line are dropped before the doc-form filter.
fn collect_doc_comments(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Attachment {
    let mut comments = Vec::new();
    let mut preceding_code = None;
    let mut prev = node.prev_sibling();
    // Go documents a declaration only with the comment group that ends on
    // the line before it.
    let mut next_start_row = node.start_position().row;

    while let Some(sibling) = prev {
        let kind = sibling.kind();
        if language == ProgrammingLanguage::Rust
            && matches!(kind, "attribute_item" | "inner_attribute_item")
        {
            // Attributes (`#[derive(...)]`) may sit between docs and the item.
        } else if is_comment_node_kind(kind) {
            if language == ProgrammingLanguage::Go
                && next_start_row.saturating_sub(sibling.end_position().row) > 1
            {
                break;
            }
            comments.push(sibling);
        } else {
            preceding_code = Some(sibling);
            break;
        }
        next_start_row = sibling.start_position().row;
        prev = sibling.prev_sibling();
    }

    comments.reverse();
    let trailing = match preceding_code {
        Some(code) if trailing_comments_belong_to_code(language) => {
            trailing_comment_count(&comments, code.end_position().row)
        }
        _ => 0,
    };
    let comments: Vec<(&str, usize)> = comments[trailing..]
        .iter()
        .filter_map(|comment| {
            let text = comment.utf8_text(source.as_bytes()).ok()?;
            Some((text, comment.start_position().column))
        })
        .filter(|(text, _)| is_doc_comment_text(text, language))
        .collect();
    if comments.is_empty() {
        return Attachment::Absent;
    }
    let joined = if language == ProgrammingLanguage::Go {
        let texts: Vec<&str> = comments.iter().map(|(text, _)| *text).collect();
        go_comment_group_text(&texts)
    } else {
        comments
            .iter()
            .map(|(comment, column)| clean_doc_comment(comment, language, *column))
            .collect::<Vec<_>>()
            .join("\n")
    };
    if joined.trim().is_empty() {
        Attachment::Empty
    } else {
        Attachment::Text(joined)
    }
}

/// Whether `language`'s doc reader gives a comment on the same line as the
/// code before it to that code: Go's parser makes it the previous line's
/// line comment (`go/parser`), and TypeScript's leading-comment scan, which
/// finds JSDoc for JavaScript and TypeScript, starts after the first line
/// break following the previous token.
fn trailing_comments_belong_to_code(language: ProgrammingLanguage) -> bool {
    matches!(
        language,
        ProgrammingLanguage::Go | ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript
    )
}

/// How many of `comments` (in source order) trail code ending on
/// `code_end_row`: the first comment starts on that row, and each next one
/// starts on the row where the previous one ends, so a block comment
/// spanning lines carries the comments after it on its last line.
fn trailing_comment_count(comments: &[Node<'_>], code_end_row: usize) -> usize {
    let mut end_row = code_end_row;
    comments
        .iter()
        .take_while(|comment| {
            let trails = comment.start_position().row <= end_row;
            end_row = comment.end_position().row;
            trails
        })
        .count()
}

fn is_comment_node_kind(kind: &str) -> bool {
    matches!(
        kind,
        "comment" | "line_comment" | "block_comment" | "multiline_comment"
    )
}

/// Whether a comment's text is a doc comment in `language`:
///
/// - Rust: `///` and `/** */` (outer docs; `//!` documents the enclosing
///   module, not the next item)
/// - Go: every comment (Go's doc comments are the plain comments directly
///   before a declaration); directive lines are removed afterwards by
///   [`go_comment_group_text`]
/// - Java, JavaScript, TypeScript, PHP, Scala: `/** */`
/// - C#, Swift: `///` and `/** */`
/// - C, C++: Doxygen's `///`, `//!`, `/** */` and `/*! */`, except their
///   `<` forms (`///<`, `//!<`, `/**<`, `/*!<`), which document the member
///   before them
pub(crate) fn is_doc_comment_text(text: &str, language: ProgrammingLanguage) -> bool {
    let text = text.trim_start();
    let triple_slash = text.starts_with("///") && !text.starts_with("////");
    let javadoc = text.starts_with("/**") && !text.starts_with("/***") && !text.starts_with("/**/");
    match language {
        ProgrammingLanguage::Rust => triple_slash || javadoc,
        ProgrammingLanguage::Go => true,
        ProgrammingLanguage::Java
        | ProgrammingLanguage::JavaScript
        | ProgrammingLanguage::TypeScript
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::Scala => javadoc,
        ProgrammingLanguage::CSharp | ProgrammingLanguage::Swift => triple_slash || javadoc,
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => {
            // Every Doxygen opener is three bytes long.
            (triple_slash || javadoc || text.starts_with("//!") || text.starts_with("/*!"))
                && !text[3..].starts_with('<')
        }
        ProgrammingLanguage::Python
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => false,
    }
}

/// The docstring of a Python `def` or `class` (decorated or not): the
/// first statement of its body when that statement is a nonempty string
/// constant, as Python itself decides.
///
/// A docstring is one ordinary string literal (no prefix, `r`, `u`; any
/// quoting) or an adjacent concatenation of them, optionally parenthesized.
/// A bytes (`b`) or formatted (`f`, `t`) literal is an expression, not a
/// docstring, and so is a concatenation containing one. The text is the
/// literal parts' source content joined and trimmed, never evaluated: `""`
/// and whitespace are undocumented, an escape such as `"\u0041"` is not.
fn python_docstring(node: Node<'_>, source: &str) -> Option<String> {
    let definition = if node.kind() == "decorated_definition" {
        node.child_by_field_name("definition")?
    } else {
        node
    };
    if !matches!(definition.kind(), "function_definition" | "class_definition") {
        return None;
    }
    let first = definition.child_by_field_name("body")?.named_child(0)?;
    if first.kind() != "expression_statement" || first.named_child_count() != 1 {
        return None;
    }
    let mut expression = first.named_child(0)?;
    while expression.kind() == "parenthesized_expression" {
        expression = single_non_comment_child(expression)?;
    }
    let parts: Vec<Node<'_>> = match expression.kind() {
        "string" => vec![expression],
        "concatenated_string" => {
            let mut cursor = expression.walk();
            expression
                .named_children(&mut cursor)
                .filter(|child| child.kind() != "comment")
                .collect()
        }
        _ => return None,
    };
    let mut content = String::new();
    for part in parts {
        if part.kind() != "string" || !is_unicode_python_literal(part, source) {
            return None;
        }
        let mut cursor = part.walk();
        for child in part.named_children(&mut cursor) {
            if child.kind() == "string_content" {
                content.push_str(child.utf8_text(source.as_bytes()).ok()?);
            }
        }
    }
    let content = content.trim();
    (!content.is_empty()).then(|| content.to_string())
}

/// The only named child of `node` other than comments.
fn single_non_comment_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let mut children = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment");
    let only = children.next()?;
    children.next().is_none().then_some(only)
}

/// Whether a Python `string` node is an ordinary (`str`) literal: its
/// prefix, read from the `string_start` token before the quote, has neither
/// a bytes (`b`) nor a formatted (`f`, `t`) letter.
fn is_unicode_python_literal(string: Node<'_>, source: &str) -> bool {
    let Some(start) = string
        .child(0)
        .filter(|child| child.kind() == "string_start")
        .and_then(|child| child.utf8_text(source.as_bytes()).ok())
    else {
        return false;
    };
    start
        .chars()
        .take_while(|character| !matches!(character, '"' | '\''))
        .all(|character| matches!(character.to_ascii_lowercase(), 'r' | 'u'))
}

/// Removes a doc comment's delimiters and the decoration its language's
/// doc tool removes, then trims every line and drops blank ones. Go
/// comments never come here: see [`go_comment_group_text`].
///
/// A line comment loses its `///`, `//!` or `//` marker only; a `*` after it
/// is text (`/// *` is documentation). A block comment loses `/*` plus a
/// third opening `*` or `!`, and `*/`; its leading-`*` decoration is then
/// removed by the rule of `language`'s doc tool, which deletes a star only
/// where that tool does and keeps every other star as text
/// (`column` is the comment's byte column, which Swift's rule reads):
///
/// - Rust: rustdoc's `beautify_doc_string` ([`rustdoc_block_lines`])
/// - C#: the compiler's common line pattern ([`csharp_block_lines`])
/// - Swift: the compiler's comment-column decoration ([`swift_block_lines`])
/// - Java, JavaScript, TypeScript, PHP, Scala, C and C++: one leading `*`
///   on every line, whatever the other lines hold (Javadoc, JSDoc,
///   phpDoc, Scaladoc and Doxygen)
fn clean_doc_comment(comment: &str, language: ProgrammingLanguage, column: usize) -> String {
    let trimmed = comment.trim();
    if trimmed.starts_with("/*") && trimmed.ends_with("*/") && trimmed.len() >= 4 {
        let opener = if trimmed.starts_with("/**") || trimmed.starts_with("/*!") { 3 } else { 2 };
        let body = &trimmed[opener.min(trimmed.len() - 2)..trimmed.len() - 2];
        let lines: Vec<&str> = match language {
            ProgrammingLanguage::Rust => rustdoc_block_lines(body),
            ProgrammingLanguage::CSharp => csharp_block_lines(trimmed),
            ProgrammingLanguage::Swift => swift_block_lines(body, column),
            _ => body
                .lines()
                .map(|line| {
                    let line = line.trim();
                    line.strip_prefix('*').unwrap_or(line)
                })
                .collect(),
        };
        return lines
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }
    let line = ["///", "//!", "//"]
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
        .unwrap_or(trimmed);
    line.trim().to_string()
}

/// The lines of a Rust `/** … */` body (between `/**` and `*/`) as
/// rustdoc's `beautify_doc_string` (`rustc_ast::util::comments`) leaves
/// them. A one-line body is unchanged (`/** * */` documents `*`). Otherwise
/// a first or last line of only stars is dropped, and a `[ \t]*` prefix plus
/// one star is removed only when every line considered (the first line too
/// when it starts with a star, interior blank lines included, leading and
/// trailing blank lines not) has its first star at the same character
/// column with only spaces or tabs before it. The prefix removed is the
/// first line's text before that column, so a line indented differently
/// keeps its star; a stripped line loses the star itself only when nothing,
/// a space or another star follows it (` *b*` keeps `*b*`).
fn rustdoc_block_lines(body: &str) -> Vec<&str> {
    if !body.contains('\n') {
        return vec![body];
    }
    let mut lines: Vec<&str> = body.lines().collect();
    let only_stars = |line: &str| line.chars().all(|character| character == '*');
    let mut end = lines.len();
    let start = usize::from(lines.first().is_some_and(|line| only_stars(line)));
    if end > start && !lines[end - 1].is_empty() && only_stars(lines[end - 1]) {
        end -= 1;
    }
    let lines = &mut lines[start..end];
    if let Some(prefix) = rustdoc_horizontal_trim(lines) {
        for line in lines.iter_mut() {
            if let Some(rest) = line.strip_prefix(prefix.as_str()) {
                *line = if rest == "*" || rest.starts_with("* ") || rest.starts_with("**") {
                    &rest[1..]
                } else {
                    rest
                };
            }
        }
    }
    lines.to_vec()
}

/// rustdoc's `get_horizontal_trim` for a block comment: the text before
/// the star column shared by every considered line, or `None` when the
/// lines disagree.
fn rustdoc_horizontal_trim(lines: &[&str]) -> Option<String> {
    let mut start = lines
        .first()
        .map_or(0, |line| usize::from(!line.trim_start().starts_with('*')));
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    let lines = &lines[start..end];
    let mut star_column = usize::MAX;
    let mut first = true;
    for line in lines {
        for (column, character) in line.chars().enumerate() {
            if column > star_column || !matches!(character, '*' | ' ' | '\t') {
                return None;
            }
            if character == '*' {
                if first {
                    star_column = column;
                    first = false;
                } else if star_column != column {
                    return None;
                }
                break;
            }
        }
        // rustc compares a character column with a byte length here; the
        // characters before the star are ASCII, so the two agree.
        if star_column >= line.len() {
            return None;
        }
    }
    Some(lines.first()?.get(..star_column)?.to_string())
}

/// The lines of a C# `/** … */` comment (the whole comment) as the C#
/// compiler's `WriteFormattedMultiLineComment` writes them. A first line of
/// only `/**` and a last line of only `*/` are dropped; the first line
/// otherwise loses `/** ` or `/**` and is never undecorated. The pattern
/// removed from the other lines is the longest common prefix of the second
/// line's `[whitespace]*[*][whitespace]*` with each later line, and nothing
/// is removed when that prefix is whitespace only (lines indented
/// differently, a tab against spaces, a blank line, or an unstarred line).
fn csharp_block_lines(comment: &str) -> Vec<&str> {
    let lines: Vec<&str> = comment.lines().collect();
    let Some(&first) = lines.first() else {
        return Vec::new();
    };
    let skip_first = first.trim() == "/**";
    let skip_last = lines.len() > 1 && lines[lines.len() - 1].trim() == "*/";
    let count = lines.len() - usize::from(skip_last);
    let mut skip = 0;
    if count > 1
        && let Some(mut pattern) = csharp_line_pattern(lines[1])
    {
        let mut all_match = true;
        for line in &lines[2..count] {
            pattern = &pattern[..common_prefix_len(pattern, line)];
            if pattern.trim().is_empty() {
                all_match = false;
                break;
            }
        }
        if all_match {
            skip = pattern.len();
        }
    }
    let mut out = Vec::with_capacity(count);
    if !skip_first {
        let mut text = first.trim_start();
        if !skip_last && count == 1 {
            text = before_comment_end(text);
        }
        out.push(text.strip_prefix("/** ").or_else(|| text.strip_prefix("/**")).unwrap_or(text));
    }
    for (index, line) in lines.iter().enumerate().take(count).skip(1) {
        let mut text = &line[skip..];
        if !skip_last && index == count - 1 {
            text = before_comment_end(text);
        }
        out.push(text);
    }
    out
}

/// `line` up to its first `*/` (the C# compiler's `TrimEndOfMultiLineComment`).
fn before_comment_end(line: &str) -> &str {
    line.find("*/").map_or(line, |index| &line[..index])
}

/// The C# compiler's `FindMultiLineCommentPattern`: the longest prefix of
/// `line` matching `[whitespace]*[*][whitespace]*`, or `None` without a star.
fn csharp_line_pattern(line: &str) -> Option<&str> {
    let mut seen_star = false;
    let end = line
        .char_indices()
        .find(|&(_, character)| {
            if character.is_whitespace() {
                false
            } else if !seen_star && character == '*' {
                seen_star = true;
                false
            } else {
                true
            }
        })
        .map_or(line.len(), |(index, _)| index);
    seen_star.then(|| &line[..end])
}

/// The byte length of the longest common prefix of `left` and `right`, on
/// a character boundary of both.
fn common_prefix_len(left: &str, right: &str) -> usize {
    left.char_indices()
        .zip(right.chars())
        .find(|&((_, left), right)| left != right)
        .map_or_else(|| left.len().min(right.len()), |((index, _), _)| index)
}

/// The lines of a Swift `/** … */` body (between `/**` and `*/`) as the
/// Swift compiler's `MarkupContext::getLineList` reads them. Decoration
/// exists only when the body starts with a line break and its second line
/// is decorated: exactly `column` spaces (the comment's own column), then
/// ` * ` or a lone ` *` ending its line. Each line is then undecorated only
/// when it has that same decoration; any other line keeps its star, and a
/// comment whose first line holds text (`/** * */`, `/** Intro`) has no
/// decoration at all.
fn swift_block_lines(body: &str, column: usize) -> Vec<&str> {
    let (body, decorated) = match body.strip_prefix("\r\n").or_else(|| body.strip_prefix('\n')) {
        Some(rest) => (rest, swift_decoration_len(rest, column).is_some()),
        None => (body, false),
    };
    body.lines()
        .map(|line| {
            let decoration = decorated
                .then(|| swift_decoration_len(&body[offset_in(body, line)..], column))
                .flatten();
            decoration.map_or(line, |length| line.get(length..).unwrap_or(""))
        })
        .collect()
}

/// The byte offset of `line`, a subslice of `text`, within it.
fn offset_in(text: &str, line: &str) -> usize {
    line.as_ptr() as usize - text.as_ptr() as usize
}

/// Swift's `measureASCIIArt`: the length of `column` spaces followed by
/// ` * `, or by ` *` and a line break, at the start of `text`.
fn swift_decoration_len(text: &str, column: usize) -> Option<usize> {
    let spaces = text.get(..column)?;
    if !spaces.bytes().all(|byte| byte == b' ') {
        return None;
    }
    let rest = &text[column..];
    if rest.starts_with(" * ") {
        Some(column + 3)
    } else if rest.starts_with(" *\n") || rest.starts_with(" *\r\n") {
        Some(column + 2)
    } else {
        None
    }
}

/// The text of a Go comment group as `go/ast`'s `CommentGroup.Text` and
/// `go/doc/comment` read it. Go defines no decoration: a `//` comment loses
/// its marker and one following space, a block loses exactly `/*` and `*/`
/// (plus the whitespace that follows `/*` on its line), and every other
/// character, a third opening `*` or a leading `*` on a line included, is
/// text. A `//` directive ([`is_go_directive`]) is not text at all: it is
/// dropped before anything else is decided, leaves no blank line behind (so
/// it never separates paragraphs), and a group of only directives is
/// empty. Trailing whitespace and the group's leading and trailing blank
/// lines are dropped, then the whitespace prefix common to every nonblank
/// line is removed; any indentation left marks a code block, which the
/// `Deprecated:` paragraph rule depends on.
fn go_comment_group_text(comments: &[&str]) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for raw in comments {
        let comment = raw.trim();
        if let Some(body) = comment.strip_prefix("/*").and_then(|rest| rest.strip_suffix("*/")) {
            // The space after `/*` separates the text from the delimiter, so
            // the first line's indentation is not compared with the others'
            // (`/* Intro\nDeprecated: …` is one paragraph, not a code line
            // and a marker). Only whitespace goes: a `*` there is text.
            let mut body_lines = body.lines().map(str::trim_end);
            if let Some(first) = body_lines.next() {
                lines.push(first.trim_start_matches([' ', '\t']));
            }
            lines.extend(body_lines);
        } else {
            let line = comment.strip_prefix("//").unwrap_or(comment);
            // Read before trimming: `//line ` is a directive, `//line` is not.
            let untrimmed = raw.trim_start().strip_prefix("//").unwrap_or(line).trim_end_matches(['\r', '\n']);
            if is_go_directive(untrimmed) {
                continue;
            }
            lines.push(line.strip_prefix(' ').unwrap_or(line).trim_end());
        }
    }
    let Some(first) = lines.iter().position(|line| !line.is_empty()) else {
        return String::new();
    };
    let last = lines.iter().rposition(|line| !line.is_empty()).unwrap_or(first);
    let lines = &lines[first..=last];
    let indentation = |line: &str| line.len() - line.trim_start_matches([' ', '\t']).len();
    let common = lines
        .iter()
        .filter(|line| !line.is_empty())
        .map(|line| &line[..indentation(line)])
        .reduce(|common, prefix| {
            let shared = common
                .bytes()
                .zip(prefix.bytes())
                .take_while(|(left, right)| left == right)
                .count();
            &common[..shared]
        })
        .unwrap_or("");
    lines
        .iter()
        .map(|line| line.get(common.len()..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether the text of a `//` comment after its marker is a directive
/// (`go/ast`'s `isDirective`): `line `, `extern ` or `export ` at its
/// start, or `[a-z0-9]+:[a-z0-9]` (`go:noinline`, `a1:b2`). A space after
/// `//`, an uppercase letter or nothing after the colon makes it prose.
fn is_go_directive(text: &str) -> bool {
    if ["line ", "extern ", "export "].iter().any(|prefix| text.starts_with(prefix)) {
        return true;
    }
    let bytes = text.as_bytes();
    let Some(colon) = text.find(':') else {
        return false;
    };
    colon > 0
        && colon + 1 < bytes.len()
        && bytes[..colon]
            .iter()
            .chain(std::iter::once(&bytes[colon + 1]))
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}
