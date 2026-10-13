//! The deprecation and test flags (`Capability::Flags`): syntax and
//! file-name heuristics, documented in
//! `docs/symbol-kinds-and-visibility/documentation-flags-and-metrics.md#deprecation-and-test-flags`.

use std::collections::HashMap;
use std::path::Path;

use super::attributes::{SyntacticAttribute, rust_attribute};
use super::*;
use crate::file::parents::IndexedParent as _;
use crate::analysis::counters::count_shared_context_steps;

/// Whether a declaration carries its language's deprecation marker: one of
/// its `attributes` (from [`declaration_attributes`](super::attributes::declaration_attributes),
/// so a C++ name's own declarator attributes count), a `@deprecated` tag in
/// its attached doc block (`doc`, from [`extract_doc_comment`]) or, in Go, a
/// doc paragraph starting `Deprecated:`. Ordinary comments are never read.
pub(crate) fn is_deprecated(
    attributes: &[SyntacticAttribute],
    doc: Option<&str>,
    language: ProgrammingLanguage,
) -> bool {
    match language {
        ProgrammingLanguage::Rust | ProgrammingLanguage::Cpp => {
            attributes.iter().any(|attribute| attribute.name == "deprecated")
        }
        ProgrammingLanguage::Java => {
            attributes
                .iter()
                .any(|attribute| matches!(attribute.name.as_str(), "Deprecated" | "java.lang.Deprecated"))
                || doc.is_some_and(has_deprecated_tag)
        }
        ProgrammingLanguage::CSharp => attributes
            .iter()
            .any(|attribute| csharp_attribute_is(attribute, &["Obsolete"])),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript | ProgrammingLanguage::Php => {
            doc.is_some_and(has_deprecated_tag)
        }
        ProgrammingLanguage::Go => doc.is_some_and(has_deprecated_paragraph),
        ProgrammingLanguage::Swift => attributes
            .iter()
            .any(is_swift_unconditional_deprecation),
        ProgrammingLanguage::Scala => attributes
            .iter()
            .any(|attribute| matches!(attribute.name.as_str(), "deprecated" | "scala.deprecated")),
        ProgrammingLanguage::Python => attributes.iter().any(|attribute| {
            matches!(
                attribute.name.as_str(),
                "deprecated" | "typing_extensions.deprecated" | "warnings.deprecated"
            )
        }),
        ProgrammingLanguage::C
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => false,
    }
}

/// Whether a declaration's `attributes` hold a test attribute: Rust
/// `#[test]` or `#[<path>::test]`, Java `@Test` (qualified or not), C#
/// `[Test]`, `[Fact]` or `[TestMethod]`.
pub(crate) fn has_test_attribute(attributes: &[SyntacticAttribute], language: ProgrammingLanguage) -> bool {
    match language {
        ProgrammingLanguage::Rust => attributes
            .iter()
            .any(|attribute| attribute.name == "test" || attribute.name.ends_with("::test")),
        ProgrammingLanguage::Java => attributes
            .iter()
            .any(|attribute| attribute.name == "Test" || attribute.name.ends_with(".Test")),
        ProgrammingLanguage::CSharp => attributes
            .iter()
            .any(|attribute| csharp_attribute_is(attribute, &["Test", "Fact", "TestMethod"])),
        _ => false,
    }
}

/// Whether a Rust declaration is, or is inside, an item marked
/// `#[cfg(test)]`, or a module or file whose inner attributes say
/// `#![cfg(test)]`. `cfg(any(test, …))` and other compound predicates do not
/// count.
///
/// `inner_cfg_test` remembers each file root's and module body's inner
/// attribute answer by node id; pass one map for every declaration of a
/// file, so each root or module body has its children read at most once.
pub(crate) fn is_in_rust_cfg_test(
    declaration: Node<'_>,
    source: &str,
    inner_cfg_test: &mut HashMap<usize, bool>,
) -> bool {
    let mut current = Some(declaration);
    while let Some(node) = current {
        let mut outer = Vec::new();
        super::attributes::rust_outer_attributes(node, source, &mut outer);
        if outer.iter().any(is_cfg_test) {
            return true;
        }
        // A module's inner attributes sit in its body.
        let inner_host = match node.kind() {
            "source_file" => Some(node),
            "mod_item" => node.child_by_field_name("body"),
            _ => None,
        };
        if let Some(host) = inner_host {
            count_shared_context_steps(1);
            let has_inner = *inner_cfg_test
                .entry(host.id())
                .or_insert_with(|| has_inner_cfg_test(host, source));
            if has_inner {
                return true;
            }
        }
        current = node.indexed_parent();
    }
    false
}

fn has_inner_cfg_test(host: Node<'_>, source: &str) -> bool {
    let mut cursor = host.walk();
    let mut steps = 0;
    let found = host
        .children(&mut cursor)
        .inspect(|_| steps += 1)
        .filter(|child| child.kind() == "inner_attribute_item")
        .filter_map(|item| find_child_by_kind(item, "attribute"))
        .any(|attribute| is_cfg_test(&rust_attribute(attribute, source)));
    count_shared_context_steps(steps);
    found
}

/// `cfg(test)`: the one argument is the bare `test` predicate (comments
/// inside the parentheses are not tokens, so `cfg(/* why */ test)` counts).
fn is_cfg_test(attribute: &SyntacticAttribute) -> bool {
    attribute.name == "cfg"
        && attribute
            .arguments
            .as_deref()
            .is_some_and(|arguments| arguments == ["test"])
}

/// Whether every declaration of the file at `path` is test-only code by its
/// name: Go `*_test.go`; Python `test_*.py`, `*_test.py` and `conftest.py`;
/// JavaScript and TypeScript `*.test.*` and `*.spec.*`; Rust files below the
/// `tests/` directory of the nearest enclosing Cargo package.
///
/// The Rust rule reads the filesystem, so its answer can change while the
/// file does not: call it after cache lookup (`SymbolQueryResult::finish`),
/// never while building records that are stored.
pub(crate) fn is_test_file(path: &Path, language: ProgrammingLanguage) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    match language {
        ProgrammingLanguage::Go => name.ends_with("_test.go"),
        ProgrammingLanguage::Python => {
            name == "conftest.py"
                || (name.starts_with("test_") && name.ends_with(".py"))
                || name.ends_with("_test.py")
        }
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            let segments: Vec<&str> = name.split('.').collect();
            segments.len() >= 3
                && segments[1..segments.len() - 1]
                    .iter()
                    .any(|segment| matches!(*segment, "test" | "spec"))
        }
        ProgrammingLanguage::Rust => is_in_cargo_tests_dir(path),
        _ => false,
    }
}

/// Whether `path`, relative to the nearest ancestor directory holding a
/// `Cargo.toml`, starts with a `tests` component. A file with no such
/// ancestor is not in a package's `tests/`.
fn is_in_cargo_tests_dir(path: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    for root in parent.ancestors() {
        if root.join("Cargo.toml").is_file() {
            return path
                .strip_prefix(root)
                .ok()
                .and_then(|relative| relative.components().next())
                .is_some_and(|first| first.as_os_str() == "tests");
        }
    }
    false
}

/// A C# attribute by its last name segment, with or without the
/// `Attribute` suffix C# lets a use omit (`[System.ObsoleteAttribute]` is
/// `Obsolete`).
fn csharp_attribute_is(attribute: &SyntacticAttribute, names: &[&str]) -> bool {
    let last = attribute.name.rsplit('.').next().unwrap_or(&attribute.name);
    let last = last.strip_suffix("Attribute").unwrap_or(last);
    names.contains(&last)
}

/// `@available(*, deprecated…)`: deprecated on every platform. Only a
/// top-level argument that is `deprecated` or `deprecated: …` counts, never
/// the word inside a `message:` string.
fn is_swift_unconditional_deprecation(attribute: &SyntacticAttribute) -> bool {
    if attribute.name != "available" {
        return false;
    }
    let Some(arguments) = attribute.arguments.as_deref() else {
        return false;
    };
    arguments.first().is_some_and(|first| first == "*")
        && arguments[1..].iter().any(|argument| {
            argument
                .strip_prefix("deprecated")
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(':'))
        })
}

/// A doc line (after the comment delimiters) that is a `@deprecated` tag.
/// Lines inside a fenced code block are example text, not tags (see
/// [`lines_outside_fences`]).
fn has_deprecated_tag(doc: &str) -> bool {
    lines_outside_fences(doc).any(|line| {
        line.strip_prefix("@deprecated")
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    })
}

/// The trimmed lines of `doc` outside every CommonMark fenced code block
/// (the fence lines themselves are inside).
///
/// A fence opens on a run of three or more backticks or tildes (a backtick
/// fence's info string cannot contain a backtick) and closes only on a line
/// holding a run of the same character at least as long, followed by nothing
/// but whitespace. Any other line, including a shorter run, the other
/// character or a run followed by text, is content. An unclosed fence runs to
/// the end of `doc`. Indentation is not read: the doc cleanup has already
/// removed the comment's leading `*` and spaces.
fn lines_outside_fences(doc: &str) -> impl Iterator<Item = &str> {
    let mut open: Option<Fence> = None;
    doc.lines().map(str::trim).filter(move |line| match open {
        Some(fence) => {
            if fence.closed_by(line) {
                open = None;
            }
            false
        }
        None => {
            open = Fence::opened_by(line);
            open.is_none()
        }
    })
}

#[derive(Clone, Copy)]
struct Fence {
    marker: char,
    length: usize,
}

impl Fence {
    fn opened_by(line: &str) -> Option<Self> {
        let marker = line.chars().next().filter(|first| matches!(first, '`' | '~'))?;
        // Both markers are one byte, so the run length is a byte offset.
        let length = line.chars().take_while(|&character| character == marker).count();
        let info = &line[length..];
        (length >= 3 && !(marker == '`' && info.contains('`'))).then_some(Self { marker, length })
    }

    fn closed_by(self, line: &str) -> bool {
        let length = line.chars().take_while(|&character| character == self.marker).count();
        length >= self.length && line[length..].trim().is_empty()
    }
}

/// A Go doc paragraph (the first line, or a line after a blank one or after
/// a code block) that starts with `Deprecated:`.
///
/// `doc` is the group's text as Go reads it (`go_comment_group_text`):
/// directive lines are already gone without leaving a blank line, so a
/// directive neither starts nor ends a paragraph, and the text is
/// unindented by its common prefix (as `go/doc/comment` does), so a line
/// that is still indented belongs to a code block, never to a paragraph.
fn has_deprecated_paragraph(doc: &str) -> bool {
    let mut paragraph_start = true;
    for line in doc.lines() {
        if line.trim().is_empty() || line.starts_with([' ', '\t']) {
            paragraph_start = true;
            continue;
        }
        if paragraph_start && line.starts_with("Deprecated:") {
            return true;
        }
        paragraph_start = false;
    }
    false
}
