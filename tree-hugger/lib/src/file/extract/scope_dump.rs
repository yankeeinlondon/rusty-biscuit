//! Test-only prototype of the presentation scope classifier (`Item`,
//! `Member`, `Local`) and a dump of its verdicts over every symbol fixture.
//!
//! The dump is the review artifact that fixes the classifier's rules before
//! the real one is written: set `TREE_HUGGER_SCOPE_DUMP=<file>` to write it as
//! Markdown (fixtures, then the [`SNIPPETS`] for spec cases no fixture has).
//! Without the variable the tests still classify every declaration and hold
//! the invariants and snippet expectations the rules promise.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::*;
use crate::file::tree_file::{MetadataSelection, TreeFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposedScope {
    Item,
    Member,
    Local,
}

/// The nearest semantic declaration decides: walking out from the
/// declaration's member slot, a function body (or a non-function local body)
/// makes it `Local`, a type, trait, interface, object or `impl` makes it
/// `Member`, and reaching the file root makes it `Item`. Namespace, package
/// and module bodies are neither, so their contents stay `Item`; a body shared
/// by modules and types (Rust and C# `declaration_list`) is decided by the
/// declaration that owns it, since the walk looks past every body.
fn proposed_scope(
    declaration: Node<'_>,
    name: Node<'_>,
    kind: SymbolKind,
    language: ProgrammingLanguage,
) -> ProposedScope {
    // A C++ template type parameter is captured as a Type; like a function
    // parameter it names nothing outside its declaration.
    if kind == SymbolKind::Parameter || is_type_parameter_kind(declaration.kind()) {
        return ProposedScope::Local;
    }
    // A Go method is declared at file level but names its receiver type; a
    // Lua `function M:m()` / `function M.f()` names its table. Both are
    // members of what they name, wherever they are written.
    if language == ProgrammingLanguage::Go && member_slot(declaration).kind() == "method_declaration" {
        return ProposedScope::Member;
    }
    if language == ProgrammingLanguage::Lua
        && name
            .parent()
            .is_some_and(|parent| matches!(parent.kind(), "method_index_expression" | "dot_index_expression"))
    {
        return ProposedScope::Member;
    }

    let mut current = member_slot(declaration).parent();
    while let Some(node) = current {
        let node_kind = node.kind();
        if is_function_node_kind(node_kind) || is_local_scope_kind(node_kind, language) {
            return ProposedScope::Local;
        }
        if is_scope_owner_kind(node_kind) {
            return ProposedScope::Member;
        }
        current = node.parent();
    }
    ProposedScope::Item
}

/// Declarations whose bodies hold members. Starts from the member owners the
/// extractor already knows, minus the bodies (decided by their parent), plus
/// owners with no member-kind captures today: C/C++ `enum_specifier`, Go
/// struct and interface types, and the Java annotation type.
fn is_scope_owner_kind(kind: &str) -> bool {
    let is_body = kind.ends_with("_body")
        || matches!(kind, "block" | "declaration_list" | "field_declaration_list" | "template_body");
    (is_member_owner_kind(kind) && !is_body)
        || matches!(
            kind,
            "enum_specifier" | "struct_type" | "interface_type" | "annotation_type_declaration"
        )
}

fn is_type_parameter_kind(kind: &str) -> bool {
    matches!(
        kind,
        "type_parameter_declaration"
            | "optional_type_parameter_declaration"
            | "variadic_type_parameter_declaration"
            | "template_template_parameter_declaration"
    )
}

fn is_member_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Field
            | SymbolKind::Property
            | SymbolKind::EnumVariant
    )
}

/// A labeled file and its classified declarations.
type Classified = (String, ProgrammingLanguage, Vec<Row>);

struct Row {
    line: usize,
    name: String,
    kind: SymbolKind,
    container: Option<String>,
    container_kind: Option<SymbolKind>,
    declaration_kind: &'static str,
    is_local_today: bool,
    scope: ProposedScope,
}

impl Row {
    /// Cases a reviewer should look at first: a kind that disagrees with its
    /// scope, or a verdict that disagrees with today's `is_local_declaration`.
    fn note(&self) -> Option<&'static str> {
        if is_member_kind(self.kind) && self.scope == ProposedScope::Item {
            return Some("member kind at item scope");
        }
        if matches!(self.kind, SymbolKind::Function) && self.scope == ProposedScope::Member {
            return Some("function at member scope");
        }
        if self.kind != SymbolKind::Parameter && self.is_local_today != (self.scope == ProposedScope::Local) {
            return Some("differs from is_local_declaration");
        }
        None
    }
}

fn fixture_paths() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/symbols");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read tests/fixtures/symbols")
        .map(|entry| entry.expect("read fixture entry").path())
        .filter(|path| path.is_file() && ProgrammingLanguage::from_path(path).is_some())
        .collect();
    paths.sort();
    paths
}

fn classify_fixture(path: &Path) -> (ProgrammingLanguage, Vec<Row>) {
    let tree_file = TreeFile::new(path).expect("load fixture");
    let language = ProgrammingLanguage::from_path(path).expect("fixture language");
    let rows = tree_file
        .definitions(&MetadataSelection::ALL)
        .expect("extract definitions")
        .into_iter()
        .map(|extracted| {
            let symbol = extracted.symbol;
            let name_node = name_node_at(extracted.declaration, symbol.range.start_byte, symbol.range.end_byte)
                .unwrap_or(extracted.declaration);
            Row {
                line: symbol.range.start_line,
                scope: proposed_scope(extracted.declaration, name_node, symbol.kind, language),
                is_local_today: is_local_declaration(extracted.declaration, language),
                declaration_kind: member_slot(extracted.declaration).kind(),
                name: symbol.name,
                kind: symbol.kind,
                container: symbol.container_name,
                container_kind: symbol.container_kind,
            }
        })
        .collect();
    (language, rows)
}

/// The named node spanning exactly the symbol's name range inside its
/// declaration (the captured name node is not kept on `ExtractedSymbol`).
fn name_node_at(declaration: Node<'_>, start: usize, end: usize) -> Option<Node<'_>> {
    let root = {
        let mut node = declaration;
        while let Some(parent) = node.parent() {
            node = parent;
        }
        node
    };
    root.named_descendant_for_byte_range(start, end)
}

fn render(fixtures: &[Classified]) -> String {
    let mut out = String::new();
    out.push_str("# Scope dump\n\n");
    out.push_str(
        "Every declaration in `lib/tests/fixtures/symbols/*`, then in the inline \
snippets for spec cases no fixture has, with its current kind and container and \
the scope the prototype classifier proposes. `local today` is \
`is_local_declaration`; `note` flags rows to review first.\n",
    );
    for (file, language, rows) in fixtures {
        let count = |scope| rows.iter().filter(|row| row.scope == scope).count();
        let _ = write!(
            out,
            "\n## `{file}` ({})\n\nItem {}, Member {}, Local {}\n\n",
            language.name(),
            count(ProposedScope::Item),
            count(ProposedScope::Member),
            count(ProposedScope::Local),
        );
        out.push_str("| line | name | kind | container | declaration node | local today | proposed scope | note |\n");
        out.push_str("|---:|---|---|---|---|---|---|---|\n");
        for row in rows {
            let container = match (&row.container, row.container_kind) {
                (Some(name), Some(kind)) => format!("`{name}` ({kind:?})"),
                (Some(name), None) => format!("`{name}`"),
                (None, _) => "—".to_string(),
            };
            let _ = writeln!(
                out,
                "| {} | `{}` | {:?} | {} | `{}` | {} | {:?} | {} |",
                row.line,
                row.name.replace('|', "\\|"),
                row.kind,
                container.replace('|', "\\|"),
                row.declaration_kind,
                if row.is_local_today { "yes" } else { "no" },
                row.scope,
                row.note().unwrap_or(""),
            );
        }
    }
    out
}

/// Spec cases the per-language fixtures do not contain yet, each with the
/// scope every listed declaration must get.
/// Each declaration name a snippet must contain, with its expected scope.
type Expected = &'static [(&'static str, ProposedScope)];

const SNIPPETS: &[(&str, &str, Expected)] = &[
    (
        "nested.sh",
        "outer() {\n  inner() { echo hi; }\n}\n",
        &[("outer", ProposedScope::Item), ("inner", ProposedScope::Local)],
    ),
    (
        "nested.zsh",
        "outer() {\n  inner() { echo hi; }\n}\n",
        &[("outer", ProposedScope::Item), ("inner", ProposedScope::Local)],
    ),
    (
        "nested.rs",
        "mod m { pub fn f() {} }\nfn g() {\n    struct S { x: i32 }\n    impl S { fn method(&self) {} }\n    mod inner { pub fn h() {} }\n}\n",
        &[
            ("m", ProposedScope::Item),
            ("f", ProposedScope::Item),
            ("g", ProposedScope::Item),
            ("S", ProposedScope::Local),
            ("x", ProposedScope::Member),
            ("method", ProposedScope::Member),
            ("inner", ProposedScope::Local),
            ("h", ProposedScope::Local),
        ],
    ),
    (
        "nested.js",
        "const api = { get() { return 1; } };\nfunction f() {\n  const Klass = class { m() {} };\n  const o = { n() {} };\n}\n",
        &[
            ("api", ProposedScope::Item),
            ("get", ProposedScope::Member),
            ("f", ProposedScope::Item),
            ("Klass", ProposedScope::Local),
            ("m", ProposedScope::Member),
            ("o", ProposedScope::Local),
            ("n", ProposedScope::Member),
        ],
    ),
    (
        "nested.py",
        "def outer():\n    class Inner:\n        def m(self):\n            pass\n    def helper():\n        pass\n",
        &[
            ("outer", ProposedScope::Item),
            ("Inner", ProposedScope::Local),
            ("m", ProposedScope::Member),
            ("helper", ProposedScope::Local),
        ],
    ),
    (
        "nested.scala",
        "object Holder {\n  given intOrdering: Ordering[Int] with {\n    def compare(a: Int, b: Int): Int = 0\n  }\n}\n",
        &[("Holder", ProposedScope::Item), ("compare", ProposedScope::Member)],
    ),
    (
        "nested.cpp",
        "namespace ns { void f(); }\nvoid g() {\n  struct L { int v; };\n}\n",
        &[
            ("ns", ProposedScope::Item),
            ("f", ProposedScope::Item),
            ("g", ProposedScope::Item),
            ("L", ProposedScope::Local),
            ("v", ProposedScope::Member),
        ],
    ),
    (
        "Nested.java",
        "class A {\n  void m() {\n    class LocalType { int v; }\n  }\n}\n",
        &[
            ("A", ProposedScope::Item),
            ("m", ProposedScope::Member),
            ("LocalType", ProposedScope::Local),
            ("v", ProposedScope::Member),
        ],
    ),
];

/// Classifies every snippet in a temporary directory.
fn snippet_fixtures() -> Vec<Classified> {
    let dir = tempfile::tempdir().expect("temp dir");
    SNIPPETS
        .iter()
        .map(|(file, source, _)| {
            let path = dir.path().join(file);
            std::fs::write(&path, source).expect("write snippet");
            let (language, rows) = classify_fixture(&path);
            (format!("{file} (snippet)"), language, rows)
        })
        .collect()
}

#[test]
fn spec_cases_missing_from_the_fixtures_get_their_scope() {
    let mut failures = Vec::new();
    for ((file, _, expected), (_, _, rows)) in SNIPPETS.iter().zip(snippet_fixtures()) {
        for (name, scope) in *expected {
            let found: Vec<ProposedScope> =
                rows.iter().filter(|row| row.name == *name).map(|row| row.scope).collect();
            if found != [*scope] {
                failures.push(format!("{file}: `{name}` expected [{scope:?}], got {found:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "scope mismatches:\n{}", failures.join("\n"));
}

#[test]
fn every_fixture_declaration_gets_a_proposed_scope() {
    let fixtures: Vec<Classified> = fixture_paths()
        .iter()
        .map(|path| {
            let (language, rows) = classify_fixture(path);
            let file = path.file_name().expect("file name").to_string_lossy().into_owned();
            (file, language, rows)
        })
        .collect();

    for language in ProgrammingLanguage::all() {
        let rows: Vec<&Row> = fixtures
            .iter()
            .filter(|(_, fixture_language, _)| *fixture_language == language)
            .flat_map(|(_, _, rows)| rows)
            .collect();
        assert!(!rows.is_empty(), "{} has no fixture declarations", language.name());
        assert!(
            rows.iter().any(|row| row.scope == ProposedScope::Item),
            "{} has no item-scoped declaration",
            language.name()
        );
        for row in &rows {
            if row.kind == SymbolKind::Parameter {
                assert_eq!(row.scope, ProposedScope::Local, "{}: parameter `{}`", language.name(), row.name);
            }
            // A member kind is validated against its owner at extraction, so
            // it never lands at file level or in a namespace.
            assert!(
                !(is_member_kind(row.kind) && row.scope == ProposedScope::Item),
                "{}: {:?} `{}` (line {}) proposed Item",
                language.name(),
                row.kind,
                row.name,
                row.line
            );
        }
    }

    if let Some(target) = std::env::var_os("TREE_HUGGER_SCOPE_DUMP") {
        let mut all = fixtures;
        all.extend(snippet_fixtures());
        std::fs::write(&target, render(&all)).expect("write scope dump");
    }
}
