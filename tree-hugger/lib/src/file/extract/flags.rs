//! The deprecation and test flags (`Capability::Flags`): syntax and
//! file-name heuristics, documented in `docs/symbol-kinds-and-visibility.md`
//! ("Deprecation and test flags").

use std::path::Path;

use super::attributes::{SyntacticAttribute, declaration_attributes, rust_attribute};
use super::*;

/// Whether `declaration` carries its language's deprecation marker: an
/// attribute, annotation or decorator, a `@deprecated` tag in its attached
/// doc block (`doc`, from [`extract_doc_comment`]) or, in Go, a doc paragraph
/// starting `Deprecated:`. Ordinary comments are never read.
pub(crate) fn is_deprecated(
    declaration: Node<'_>,
    doc: Option<&str>,
    language: ProgrammingLanguage,
    source: &str,
) -> bool {
    let attributes = || declaration_attributes(declaration, language, source);
    match language {
        ProgrammingLanguage::Rust | ProgrammingLanguage::Cpp => {
            attributes().iter().any(|attribute| attribute.name == "deprecated")
        }
        ProgrammingLanguage::Java => {
            attributes()
                .iter()
                .any(|attribute| matches!(attribute.name.as_str(), "Deprecated" | "java.lang.Deprecated"))
                || doc.is_some_and(has_deprecated_tag)
        }
        ProgrammingLanguage::CSharp => attributes()
            .iter()
            .any(|attribute| csharp_attribute_is(attribute, &["Obsolete"])),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript | ProgrammingLanguage::Php => {
            doc.is_some_and(has_deprecated_tag)
        }
        ProgrammingLanguage::Go => doc.is_some_and(has_deprecated_paragraph),
        ProgrammingLanguage::Swift => attributes()
            .iter()
            .any(is_swift_unconditional_deprecation),
        ProgrammingLanguage::Scala => attributes()
            .iter()
            .any(|attribute| matches!(attribute.name.as_str(), "deprecated" | "scala.deprecated")),
        ProgrammingLanguage::Python => attributes().iter().any(|attribute| {
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

/// Whether `declaration` carries a test attribute: Rust `#[test]` or
/// `#[<path>::test]`, Java `@Test` (qualified or not), C# `[Test]`,
/// `[Fact]` or `[TestMethod]`.
pub(crate) fn has_test_attribute(
    declaration: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> bool {
    let attributes = || declaration_attributes(declaration, language, source);
    match language {
        ProgrammingLanguage::Rust => attributes()
            .iter()
            .any(|attribute| attribute.name == "test" || attribute.name.ends_with("::test")),
        ProgrammingLanguage::Java => attributes()
            .iter()
            .any(|attribute| attribute.name == "Test" || attribute.name.ends_with(".Test")),
        ProgrammingLanguage::CSharp => attributes()
            .iter()
            .any(|attribute| csharp_attribute_is(attribute, &["Test", "Fact", "TestMethod"])),
        _ => false,
    }
}

/// Whether a Rust declaration is, or is inside, an item marked
/// `#[cfg(test)]`, or a module or file whose inner attributes say
/// `#![cfg(test)]`. `cfg(any(test, …))` and other compound predicates do not
/// count.
pub(crate) fn is_in_rust_cfg_test(declaration: Node<'_>, source: &str) -> bool {
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
        if inner_host.is_some_and(|host| has_inner_cfg_test(host, source)) {
            return true;
        }
        current = node.parent();
    }
    false
}

fn has_inner_cfg_test(host: Node<'_>, source: &str) -> bool {
    let mut cursor = host.walk();
    host.children(&mut cursor)
        .filter(|child| child.kind() == "inner_attribute_item")
        .filter_map(|item| find_child_by_kind(item, "attribute"))
        .any(|attribute| is_cfg_test(&rust_attribute(attribute, source)))
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
/// Lines inside a fenced code block (`` ``` `` or `~~~`) are example text,
/// not tags.
fn has_deprecated_tag(doc: &str) -> bool {
    let mut in_fence = false;
    doc.lines().any(|line| {
        let line = line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_fence = !in_fence;
            return false;
        }
        !in_fence
            && line
                .strip_prefix("@deprecated")
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    })
}

/// A Go doc paragraph (the first line, or a line after a blank one) that
/// starts with `Deprecated:`.
fn has_deprecated_paragraph(doc: &str) -> bool {
    let mut paragraph_start = true;
    for line in doc.lines() {
        let line = line.trim();
        if line.is_empty() {
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
