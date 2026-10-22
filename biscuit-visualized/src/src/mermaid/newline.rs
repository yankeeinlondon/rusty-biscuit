//! Interim `\n` → `<br/>` rewrite for the label text of four Mermaid families.
//!
//! Merman breaks a label at a literal `\n` in class, ER, mindmap, and
//! requirement diagrams but sizes its box for one line, so the second line is
//! clipped; `<br/>` breaks the line and sizes the box. Every other family is
//! returned unchanged: flowchart, state, and kanban already break correctly,
//! and the rest either keep `\n` literal (as Mermaid.js does) or would draw
//! `<br/>` as text. The rewrite is removed once Merman normalizes `\n` itself.
//!
//! The rewrite is line-oriented and never global. Within a supported family it
//! leaves these untouched, whole:
//!
//! - YAML frontmatter, `%%{init}%%` directives (including multi-line ones), and
//!   `%%` comments;
//! - `click`, `link`, and `callback` statements (URLs and tooltips);
//! - `classDef`, `style`, `cssClass`, and `linkStyle` statements;
//! - `accTitle` and `accDescr`, including the `accDescr { … }` block form.
//!
//! A `\\n` (escaped backslash followed by `n`) is not a line break and is kept.

use std::borrow::Cow;

/// The families whose label text gets the rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Class,
    Er,
    Mindmap,
    Requirement,
}

impl Family {
    fn from_header(keyword: &str) -> Option<Self> {
        match keyword {
            "classDiagram" | "classDiagram-v2" => Some(Self::Class),
            "erDiagram" => Some(Self::Er),
            "mindmap" => Some(Self::Mindmap),
            "requirementDiagram" => Some(Self::Requirement),
            _ => None,
        }
    }

    /// Statement keywords whose whole line is left alone.
    fn excluded_keywords(self) -> &'static [&'static str] {
        const SHARED: &[&str] = &["accTitle", "accDescr", "classDef", "style"];
        match self {
            Self::Class => &[
                "accTitle", "accDescr", "classDef", "style", "cssClass", "click", "link", "callback",
            ],
            Self::Er | Self::Requirement => SHARED,
            Self::Mindmap => &["accTitle", "accDescr", "classDef", "style", "click"],
        }
    }
}

/// Rewrites `\n` to `<br/>` inside label text for class, ER, mindmap, and
/// requirement diagrams; returns any other source unchanged (borrowed).
pub(crate) fn rewrite_label_newlines(source: &str) -> Cow<'_, str> {
    if !source.contains("\\n") {
        return Cow::Borrowed(source);
    }

    let mut out = String::with_capacity(source.len() + 16);
    let mut state = State::Preamble;
    let mut family = None;
    let mut changed = false;

    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        state = match state {
            State::Frontmatter => {
                out.push_str(line);
                if trimmed.trim_end() == "---" { State::Preamble } else { State::Frontmatter }
            }
            State::Directive(next) => {
                out.push_str(line);
                if line.contains("}%%") { *next } else { State::Directive(next) }
            }
            State::AccDescrBlock => {
                out.push_str(line);
                if line.contains('}') { State::Body } else { State::AccDescrBlock }
            }
            State::Preamble | State::Body => {
                let in_body = matches!(state, State::Body);
                if let Some(next) = skipped_line(trimmed, in_body) {
                    out.push_str(line);
                    match next {
                        Skip::Line => state,
                        Skip::Directive => State::Directive(Box::new(state)),
                        Skip::Frontmatter => State::Frontmatter,
                    }
                } else if !in_body {
                    out.push_str(line);
                    family = trimmed.split_whitespace().next().and_then(Family::from_header);
                    if family.is_none() {
                        // Nothing before the header is ever rewritten.
                        return Cow::Borrowed(source);
                    }
                    State::Body
                } else {
                    let family = family.expect("body lines follow a recognized header");
                    let keyword = statement_keyword(trimmed);
                    if family.excluded_keywords().contains(&keyword) {
                        out.push_str(line);
                        if keyword == "accDescr" && opens_block(trimmed) {
                            State::AccDescrBlock
                        } else {
                            State::Body
                        }
                    } else {
                        changed |= push_rewritten(&mut out, line);
                        State::Body
                    }
                }
            }
        };
    }

    if changed { Cow::Owned(out) } else { Cow::Borrowed(source) }
}

#[derive(Debug)]
enum State {
    /// Before the diagram header: blank lines, comments, directives, frontmatter.
    Preamble,
    /// Inside a `---` frontmatter block.
    Frontmatter,
    /// Inside a multi-line `%%{ … }%%` directive; holds the state to return to.
    Directive(Box<State>),
    /// Inside an `accDescr { … }` block.
    AccDescrBlock,
    /// After the header.
    Body,
}

enum Skip {
    Line,
    Directive,
    Frontmatter,
}

/// Lines copied verbatim regardless of family: blanks, `%%` comments, and
/// directives anywhere; a frontmatter opener only before the header.
fn skipped_line(trimmed: &str, in_body: bool) -> Option<Skip> {
    if trimmed.trim().is_empty() {
        return Some(Skip::Line);
    }
    if trimmed.starts_with("%%{") {
        return Some(if trimmed.contains("}%%") { Skip::Line } else { Skip::Directive });
    }
    if trimmed.starts_with("%%") {
        return Some(Skip::Line);
    }
    if !in_body && trimmed.trim_end() == "---" {
        return Some(Skip::Frontmatter);
    }
    None
}

/// The statement's leading keyword: the text before the first whitespace,
/// `:`, or `{` (`accTitle: x` and `accDescr{` both yield the keyword).
fn statement_keyword(trimmed: &str) -> &str {
    let end = trimmed
        .find(|c: char| c.is_whitespace() || c == ':' || c == '{')
        .unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// `accDescr {` opens a multi-line block unless it closes on the same line.
fn opens_block(trimmed: &str) -> bool {
    let rest = trimmed["accDescr".len()..].trim_start();
    rest.starts_with('{') && !rest.contains('}')
}

/// Appends `line` with every unescaped `\n` replaced by `<br/>`; reports
/// whether anything was replaced.
fn push_rewritten(out: &mut String, line: &str) -> bool {
    let mut changed = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('n') => {
                chars.next();
                out.push_str("<br/>");
                changed = true;
            }
            Some('\\') => {
                chars.next();
                out.push_str("\\\\");
            }
            _ => out.push('\\'),
        }
    }
    changed
}
