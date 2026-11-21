//! The EditorConfig opinion: section matching and the indentation fold.
//!
//! Sections come from the shared ordered scan
//! (`formatting::scan_editorconfig`). A section applies when its glob matches
//! the worktree-relative path, written with `/` separators on every platform;
//! a glob without a `/` matches at any directory level. Matching is
//! case-sensitive. Matching sections fold `indent_style`, `indent_size`, and
//! `tab_width` in file order: a valid value replaces the earlier one, `unset`
//! clears it, and an invalid value is diagnosed and leaves the earlier value
//! in place.

use std::num::NonZeroU8;
use std::path::PathBuf;

use super::catalog::EDITORCONFIG;
use super::inputs::Inputs;
use super::{Diagnostics, IndentAuthority, IndentOpinion, IndentStyle, Target, TargetKind, normalize_extension};
use crate::filesystem::formatting::EditorConfigScan;

/// The merged opinion for `target`, or `None` when the matching sections
/// state neither a style nor a width.
pub(super) fn opinion(inputs: &Inputs, target: &Target, diagnostics: &mut Diagnostics) -> Option<IndentOpinion> {
    if let Some(reason) = inputs.failure(EDITORCONFIG) {
        diagnostics.push(EDITORCONFIG, None, reason);
        return None;
    }
    let scan = inputs.editorconfig()?;
    let match_path = target.match_path();
    let mut fold = Fold::default();
    let mut via = Vec::new();

    for section in &scan.sections {
        let glob = match Glob::parse(&section.pattern) {
            Ok(glob) => glob,
            Err(reason) => {
                diagnostics.push(
                    EDITORCONFIG,
                    Some(&format!("[{}]", section.pattern)),
                    format!("line {}: the section pattern is invalid: {reason}", section.line),
                );
                continue;
            }
        };
        let applies = match &target.kind {
            TargetKind::Generic => is_generic(&section.pattern),
            TargetKind::Extension(_) => is_extension_wide(&section.pattern) && glob.matches(&match_path),
            TargetKind::Path(_) => glob.matches(&match_path),
        };
        if !applies {
            continue;
        }
        let mut contributed = false;
        for property in &section.properties {
            let outcome = match property.key.as_str() {
                "indent_style" => fold.style(&property.value),
                "indent_size" => fold.size(&property.value),
                "tab_width" => fold.tab_width(&property.value),
                _ => continue,
            };
            match outcome {
                Ok(()) => contributed = true,
                Err(reason) => diagnostics.push(
                    EDITORCONFIG,
                    Some(&property.key),
                    format!("line {} in [{}]: {reason}", property.line, section.pattern),
                ),
            }
        }
        if contributed {
            via.push(section.pattern.clone());
        }
    }

    let (style, width) = fold.resolve();
    if style.is_none() && width.is_none() {
        return None;
    }
    Some(IndentOpinion {
        source: IndentAuthority::EditorConfig,
        path: Some(PathBuf::from(EDITORCONFIG)),
        via,
        indent_using: style,
        indent: width,
    })
}

/// Extensions named by sections such as `[*.rs]` and `[*.{yaml,yml}]`.
///
/// A wildcard, class, or range inside the suffix names no extension.
pub(super) fn literal_suffixes(scan: Option<&EditorConfigScan>) -> Vec<String> {
    let Some(scan) = scan else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for section in &scan.sections {
        let Some(suffix) = section.pattern.trim_start_matches('*').strip_prefix('.') else {
            continue;
        };
        if !section.pattern.starts_with('*') {
            continue;
        }
        let names: Vec<&str> = match suffix.strip_prefix('{').and_then(|inner| inner.strip_suffix('}')) {
            Some(inner) => inner.split(',').collect(),
            None => vec![suffix],
        };
        if names.iter().all(|name| normalize_extension(name).is_ok_and(|ext| ext == *name)) {
            found.extend(names.into_iter().map(str::to_string));
        }
    }
    found
}

/// `[*]` and `[**]`: the sections a summary's `default` uses.
fn is_generic(pattern: &str) -> bool {
    matches!(pattern, "*" | "**")
}

/// Whether a pattern depends on nothing but a file's extension: `*`, or a
/// suffix pattern such as `*.yaml` or `*.{yaml,yml}`, optionally under
/// `**/`. Filename and directory patterns are left to path lookup.
pub(super) fn is_extension_wide(pattern: &str) -> bool {
    let mut rest = pattern;
    while let Some(stripped) = rest.strip_prefix("**/") {
        rest = stripped;
    }
    if rest.contains('/') || !rest.starts_with('*') {
        return false;
    }
    let suffix = rest.trim_start_matches('*');
    suffix.is_empty() || suffix.starts_with('.') || suffix.starts_with('{')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Size {
    Columns(NonZeroU8),
    /// `indent_size = tab`: the width is `tab_width`.
    Tab,
}

/// The three indentation properties after folding; `None` is missing or
/// cleared by `unset`.
#[derive(Debug, Default)]
struct Fold {
    style: Option<IndentStyle>,
    size: Option<Size>,
    tab_width: Option<NonZeroU8>,
}

impl Fold {
    fn style(&mut self, value: &str) -> Result<(), String> {
        self.style = match value.to_ascii_lowercase().as_str() {
            "unset" => None,
            "space" => Some(IndentStyle::Spaces),
            "tab" => Some(IndentStyle::Tabs),
            _ => return Err(format!("`{value}` is not `space`, `tab`, or `unset`")),
        };
        Ok(())
    }

    fn size(&mut self, value: &str) -> Result<(), String> {
        self.size = match value.to_ascii_lowercase().as_str() {
            "unset" => None,
            "tab" => Some(Size::Tab),
            _ => Some(Size::Columns(
                exact_width(value).ok_or_else(|| format!("`{value}` is not a width from 1 through 255, `tab`, or `unset`"))?,
            )),
        };
        Ok(())
    }

    fn tab_width(&mut self, value: &str) -> Result<(), String> {
        self.tab_width = if value.eq_ignore_ascii_case("unset") {
            None
        } else {
            Some(exact_width(value).ok_or_else(|| format!("`{value}` is not a width from 1 through 255 or `unset`"))?)
        };
        Ok(())
    }

    /// Missing or unset `indent_size` takes `tab_width` only with tab style;
    /// `indent_size = tab` takes it whatever the style.
    fn resolve(&self) -> (Option<IndentStyle>, Option<NonZeroU8>) {
        let width = match self.size {
            Some(Size::Columns(width)) => Some(width),
            Some(Size::Tab) => self.tab_width,
            None if self.style == Some(IndentStyle::Tabs) => self.tab_width,
            None => None,
        };
        (self.style, width)
    }
}

/// An exact positive integer through 255: digits only, no sign, no leading
/// zero.
fn exact_width(value: &str) -> Option<NonZeroU8> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) || value.starts_with('0') {
        return None;
    }
    value.parse::<u8>().ok().and_then(NonZeroU8::new)
}

/// A compiled EditorConfig glob.
#[derive(Debug)]
pub(super) struct Glob {
    nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
enum Node {
    Literal(char),
    /// `*`: any run of characters except `/`.
    Star,
    /// `**`: any run of characters, `/` included.
    AnyPath,
    /// `**` between two separators (or at the start before one): zero or
    /// more whole directories, so `a/**/b` matches `a/b`.
    Directories,
    /// `?`: one character except `/`.
    One,
    Class { negated: bool, items: Vec<(char, char)> },
    Alternatives(Vec<Vec<Node>>),
    /// `{lo..hi}`: an integer in the inclusive range, signed.
    Range(i64, i64),
}

impl Glob {
    pub(super) fn parse(pattern: &str) -> Result<Self, String> {
        if pattern.is_empty() {
            return Err("it is empty".to_string());
        }
        let text: Vec<char> = pattern.strip_prefix('/').unwrap_or(pattern).chars().collect();
        let mut nodes = parse_sequence(&text)?;
        // A glob without a `/` may match at any level, as if written `**/glob`.
        if !(pattern.starts_with('/') || contains_separator(&nodes)) {
            nodes.insert(0, Node::Directories);
        }
        Ok(Self { nodes })
    }

    pub(super) fn matches(&self, relative: &str) -> bool {
        let chars: Vec<char> = relative.chars().collect();
        sequence_matches(&self.nodes, &chars, &|rest| rest.is_empty())
    }
}

fn contains_separator(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::Literal('/') | Node::Directories => true,
        Node::Alternatives(branches) => branches.iter().any(|branch| contains_separator(branch)),
        _ => false,
    })
}

fn parse_sequence(text: &[char]) -> Result<Vec<Node>, String> {
    let mut nodes = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let c = text[index];
        match c {
            '\\' => {
                let Some(escaped) = text.get(index + 1) else {
                    return Err("it ends with an unescaped `\\`".to_string());
                };
                nodes.push(Node::Literal(*escaped));
                index += 2;
            }
            '*' if text.get(index + 1) == Some(&'*') => {
                let after_separator = index == 0 || text[index - 1] == '/';
                if after_separator && text.get(index + 2) == Some(&'/') {
                    nodes.push(Node::Directories);
                    index += 3;
                } else {
                    nodes.push(Node::AnyPath);
                    index += 2;
                }
            }
            '*' => {
                nodes.push(Node::Star);
                index += 1;
            }
            '?' => {
                nodes.push(Node::One);
                index += 1;
            }
            '[' => match class_end(text, index) {
                Some(end) if !text[index + 1..end].contains(&'/') => {
                    nodes.push(parse_class(&text[index + 1..end]));
                    index = end + 1;
                }
                _ => {
                    nodes.push(Node::Literal('['));
                    index += 1;
                }
            },
            '{' => match brace_end(text, index) {
                Some(end) => {
                    let inner = &text[index + 1..end];
                    if let Some(range) = parse_range(inner)? {
                        nodes.push(range);
                    } else {
                        let branches = split_alternatives(inner);
                        if branches.len() < 2 {
                            // `{single}` is literal text.
                            nodes.push(Node::Literal('{'));
                            index += 1;
                            continue;
                        }
                        nodes.push(Node::Alternatives(
                            branches.into_iter().map(parse_sequence).collect::<Result<_, _>>()?,
                        ));
                    }
                    index = end + 1;
                }
                None => {
                    nodes.push(Node::Literal('{'));
                    index += 1;
                }
            },
            _ => {
                nodes.push(Node::Literal(c));
                index += 1;
            }
        }
    }
    Ok(nodes)
}

fn class_end(text: &[char], open: usize) -> Option<usize> {
    let mut index = open + 1;
    if matches!(text.get(index), Some('!' | '^')) {
        index += 1;
    }
    // A `]` right after the opening (or its negation) is a member.
    if text.get(index) == Some(&']') {
        index += 1;
    }
    while index < text.len() {
        match text[index] {
            '\\' => index += 2,
            ']' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

fn parse_class(body: &[char]) -> Node {
    let (negated, body) = match body.first() {
        Some('!' | '^') => (true, &body[1..]),
        _ => (false, body),
    };
    let mut members = Vec::new();
    let mut index = 0;
    while index < body.len() {
        let mut c = body[index];
        if c == '\\' && index + 1 < body.len() {
            index += 1;
            c = body[index];
        }
        if body.get(index + 1) == Some(&'-') && index + 2 < body.len() {
            members.push((c, body[index + 2]));
            index += 3;
        } else {
            members.push((c, c));
            index += 1;
        }
    }
    Node::Class { negated, items: members }
}

fn brace_end(text: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while index < text.len() {
        match text[index] {
            '\\' => index += 1,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn split_alternatives(inner: &[char]) -> Vec<&[char]> {
    let mut branches = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut index = 0;
    while index < inner.len() {
        match inner[index] {
            '\\' => index += 1,
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                branches.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    branches.push(&inner[start..]);
    branches
}

/// `{lo..hi}` with signed integer bounds, or `None` when the braces hold
/// anything else.
fn parse_range(inner: &[char]) -> Result<Option<Node>, String> {
    let text: String = inner.iter().collect();
    let Some((low, high)) = text.split_once("..") else {
        return Ok(None);
    };
    let is_integer = |bound: &str| {
        let digits = bound.strip_prefix(['+', '-']).unwrap_or(bound);
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    };
    if !is_integer(low) || !is_integer(high) {
        return Ok(None);
    }
    let parse = |bound: &str| {
        bound.parse::<i64>().map_err(|_| format!("the range bound `{bound}` does not fit a 64-bit integer"))
    };
    let (low, high) = (parse(low)?, parse(high)?);
    Ok(Some(Node::Range(low.min(high), low.max(high))))
}

fn sequence_matches(nodes: &[Node], input: &[char], then: &dyn Fn(&[char]) -> bool) -> bool {
    let Some((first, rest)) = nodes.split_first() else {
        return then(input);
    };
    let next = |after: &[char]| sequence_matches(rest, after, then);
    match first {
        Node::Literal(c) => input.first() == Some(c) && next(&input[1..]),
        Node::One => input.first().is_some_and(|c| *c != '/') && next(&input[1..]),
        Node::Star => {
            for split in 0..=input.len() {
                if split > 0 && input[split - 1] == '/' {
                    break;
                }
                if next(&input[split..]) {
                    return true;
                }
            }
            false
        }
        Node::AnyPath => (0..=input.len()).any(|split| next(&input[split..])),
        Node::Directories => {
            next(input) || (1..=input.len()).any(|split| input[split - 1] == '/' && next(&input[split..]))
        }
        Node::Class { negated, items } => input.first().is_some_and(|c| {
            *c != '/' && items.iter().any(|(low, high)| (*low..=*high).contains(c)) != *negated && next(&input[1..])
        }),
        Node::Alternatives(branches) => branches.iter().any(|branch| sequence_matches(branch, input, &next)),
        Node::Range(low, high) => {
            let sign = usize::from(matches!(input.first(), Some('-' | '+')));
            let digits = input[sign..].iter().take_while(|c| c.is_ascii_digit()).count();
            if digits == 0 {
                return false;
            }
            // The whole digit run is the number, as in the reference cores.
            let end = sign + digits;
            let number: String = input[..end].iter().collect();
            number.parse::<i64>().is_ok_and(|value| (*low..=*high).contains(&value)) && next(&input[end..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, path: &str) -> bool {
        Glob::parse(pattern).unwrap().matches(path)
    }

    #[test]
    fn a_pattern_without_a_slash_matches_the_basename_at_any_depth() {
        assert!(matches("*.rs", "main.rs"));
        assert!(matches("*.rs", "src/deep/main.rs"));
        assert!(matches("Makefile", "build/Makefile"));
        assert!(!matches("*.rs", "main.rst"));
    }

    #[test]
    fn a_slash_anchors_the_pattern_to_the_root() {
        assert!(matches("/docs/*.md", "docs/a.md"));
        assert!(!matches("/docs/*.md", "x/docs/a.md"));
        assert!(matches("docs/*.md", "docs/a.md"));
        assert!(!matches("docs/*.md", "docs/sub/a.md"));
        assert!(matches("docs/**.md", "docs/sub/a.md"));
        assert!(matches("/*.yaml", "a.yaml"));
        assert!(!matches("/*.yaml", "sub/a.yaml"));
    }

    #[test]
    fn a_double_star_between_separators_matches_zero_or_more_directories() {
        assert!(matches("a/**/z.c", "a/z.c"));
        assert!(matches("a/**/z.c", "a/b/c/z.c"));
        assert!(matches("**/z.c", "z.c"));
        assert!(!matches("a/**/z.c", "ab/z.c"));
    }

    #[test]
    fn star_and_question_mark_never_cross_a_separator() {
        assert!(!matches("a*z", "a/z"));
        assert!(!matches("a?z", "a/z"));
        assert!(matches("a?z", "abz"));
        assert!(matches("a**z", "a/b/z"));
    }

    #[test]
    fn braces_hold_alternatives_and_a_single_item_is_literal() {
        assert!(matches("*.{js,ts}", "a.ts"));
        assert!(matches("*.{js,{ts,tsx}}", "a.tsx"));
        assert!(!matches("*.{js,ts}", "a.rs"));
        assert!(matches("{single}.txt", "{single}.txt"));
        assert!(!matches("{single}.txt", "single.txt"));
        assert!(matches("{a,}b", "b"));
    }

    #[test]
    fn numeric_ranges_are_signed_inclusive_predicates() {
        assert!(matches("file{1..3}.txt", "file1.txt"));
        assert!(matches("file{1..3}.txt", "file3.txt"));
        assert!(!matches("file{1..3}.txt", "file4.txt"));
        assert!(!matches("file{1..3}.txt", "file12.txt"));
        assert!(matches("f{-5..-2}", "f-3"));
        assert!(!matches("f{-5..-2}", "f-1"));
        assert!(matches("f{+2..10}", "f10"));
        assert!(matches("*{10..20}", "x15"));
        // Bounds given high to low are the same range.
        assert!(matches("f{3..1}", "f2"));
        // A range of two hundred thousand values is a predicate, not a list.
        assert!(matches("v{0..200000}", "v199999"));
    }

    #[test]
    fn classes_and_escapes_match_literally() {
        assert!(matches("[ab].c", "a.c"));
        assert!(!matches("[!ab].c", "a.c"));
        assert!(matches("[!ab].c", "x.c"));
        assert!(matches("[a-c]x", "bx"));
        assert!(matches("\\*.md", "*.md"));
        assert!(!matches("\\*.md", "a.md"));
        assert!(matches("a\\{b,c\\}", "a{b,c}"));
        assert!(matches("[", "["));
    }

    #[test]
    fn matching_is_case_sensitive() {
        assert!(!matches("*.MD", "a.md"));
        assert!(!matches("makefile", "Makefile"));
    }

    #[test]
    fn malformed_patterns_are_errors() {
        assert!(Glob::parse("").is_err());
        assert!(Glob::parse("a\\").is_err());
        assert!(Glob::parse("{1..99999999999999999999}").is_err());
    }

    #[test]
    fn extension_wide_patterns_depend_only_on_the_extension() {
        for pattern in ["*", "**", "*.yaml", "*.{yaml,yml}", "**/*.ts"] {
            assert!(is_extension_wide(pattern), "{pattern}");
        }
        for pattern in ["Makefile", "docs/**.md", "/*.yaml", "file.*", "src/*.rs"] {
            assert!(!is_extension_wide(pattern), "{pattern}");
        }
    }

    #[test]
    fn literal_suffixes_ignore_wildcards_inside_the_suffix() {
        let scan = crate::filesystem::formatting::scan_editorconfig(
            "[*]\n[*.rs]\n[*.{yaml,yml}]\n[*.t?ml]\n[*.{a*,b}]\n[Makefile]\n[docs/*.md]\n[**.py]\n",
        );
        assert_eq!(literal_suffixes(Some(&scan)), ["rs", "yaml", "yml", "py"]);
    }

    #[test]
    fn exact_width_accepts_only_canonical_integers_through_255() {
        assert_eq!(exact_width("4").map(NonZeroU8::get), Some(4));
        assert_eq!(exact_width("255").map(NonZeroU8::get), Some(255));
        for invalid in ["0", "256", "04", "+4", "-4", "4.0", " 4", ""] {
            assert_eq!(exact_width(invalid), None, "{invalid:?}");
        }
    }
}
