//! Namespaces written as a block (`package p { … }`, `namespace N { … }`,
//! `mod m { … }`) qualify every declaration in their body, so same-named
//! declarations in sibling blocks keep distinct qualified names, and
//! qualified selectors tell them apart. Scala block packages are never
//! extracted as records; they still name and root what they enclose.

use std::collections::HashSet;
use std::path::PathBuf;

use tree_hugger::{
    Capability, NamePattern, SymbolKindV2, SymbolQuery, SymbolRecord, TreeFile, VisibilityLevel,
};

use crate::symbols_fixtures::{extract_inline, in_file_name, primary_fixture};

use tree_hugger::ProgrammingLanguage as L;

/// Appended to the primary Scala fixture, which opens with
/// `package example.symbols`.
const SCALA_BLOCKS: &str = "
package R9Left { class R9W { def probe(): Unit = () } }
package R9Right { class R9W { def probe(): Unit = () } }
package R9Outer {
  package R9Inner { class R9Nested { def probe(): Unit = () } }
}
class R9After
";

/// A scratch copy of the primary Scala fixture with [`SCALA_BLOCKS`]
/// appended; the directory lives as long as the returned guard.
fn scala_fixture_with_blocks() -> (tempfile::TempDir, PathBuf) {
    let fixture = primary_fixture(L::Scala).expect("Scala fixture");
    let mut source = std::fs::read_to_string(&fixture).expect("read Scala fixture");
    source.push_str(SCALA_BLOCKS);
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("scala.scala");
    std::fs::write(&path, source).expect("write Scala copy");
    (dir, path)
}

fn query(path: &PathBuf, query: &SymbolQuery) -> Vec<SymbolRecord> {
    TreeFile::new(path).expect("parse").query(query).expect("query").index.symbols
}

fn only<'a>(records: &'a [SymbolRecord], qualified: &str) -> &'a SymbolRecord {
    let found: Vec<&SymbolRecord> = records
        .iter()
        .filter(|record| record.identity.qualified_name == qualified)
        .collect();
    assert_eq!(found.len(), 1, "expected one `{qualified}`, found {}", found.len());
    found[0]
}

#[test]
fn scala_block_packages_qualify_and_root_their_declarations() {
    let (_dir, path) = scala_fixture_with_blocks();
    let records = query(&path, &SymbolQuery::new().capabilities([Capability::Containment]));

    for (qualified, kind) in [
        ("example.symbols.R9Left.R9W", SymbolKindV2::Class),
        ("example.symbols.R9Right.R9W", SymbolKindV2::Class),
        ("example.symbols.R9Outer.R9Inner.R9Nested", SymbolKindV2::Class),
        ("example.symbols.R9After", SymbolKindV2::Class),
        ("example.symbols.R9Left.R9W.probe", SymbolKindV2::Method),
        ("example.symbols.R9Right.R9W.probe", SymbolKindV2::Method),
        ("example.symbols.R9Outer.R9Inner.R9Nested.probe", SymbolKindV2::Method),
    ] {
        let record = only(&records, qualified);
        assert_eq!(record.kind, kind, "`{qualified}` kind");
        assert!(record.identity.is_namespace_rooted, "`{qualified}` is rooted");
    }
    assert!(
        !records.iter().any(|record| record.identity.qualified_name == "example.symbols.R9W"),
        "a block package segment was dropped"
    );

    for (method, owner) in [
        ("example.symbols.R9Left.R9W.probe", "example.symbols.R9Left.R9W"),
        ("example.symbols.R9Right.R9W.probe", "example.symbols.R9Right.R9W"),
        ("example.symbols.R9Outer.R9Inner.R9Nested.probe", "example.symbols.R9Outer.R9Inner.R9Nested"),
    ] {
        let owner = only(&records, owner);
        let parent = only(&records, method).relations.parent.as_ref().expect("probe has a parent");
        assert_eq!(parent.id.as_ref(), Some(&owner.id), "`{method}` parent");
    }
}

#[test]
fn qualified_selectors_tell_scala_block_packages_apart() {
    let (_dir, path) = scala_fixture_with_blocks();
    let selected = |query: SymbolQuery| -> Vec<String> {
        self::query(&path, &query)
            .into_iter()
            .map(|record| record.identity.qualified_name)
            .collect()
    };

    let by_name = selected(SymbolQuery::new().name(NamePattern::exact("example.symbols.R9Left.R9W.probe")));
    assert_eq!(by_name, ["example.symbols.R9Left.R9W.probe"]);

    let in_class = selected(
        SymbolQuery::new()
            .kinds([SymbolKindV2::Method])
            .in_container(NamePattern::exact("example.symbols.R9Left.R9W")),
    );
    assert_eq!(in_class, ["example.symbols.R9Left.R9W.probe"]);

    let class = selected(
        SymbolQuery::new()
            .kinds([SymbolKindV2::Class])
            .name(NamePattern::exact("example.symbols.R9Right.R9W")),
    );
    assert_eq!(class, ["example.symbols.R9Right.R9W"]);

    let nested = selected(SymbolQuery::new().name(NamePattern::exact("example.symbols.R9Outer.R9Inner.R9Nested")));
    assert_eq!(nested, ["example.symbols.R9Outer.R9Inner.R9Nested"]);
}

/// `(name, container, in-file qualified name, is_namespace_rooted)`.
type Case = (&'static str, Option<&'static str>, &'static str, bool);

/// `(extension, source, cases)`: sibling blocks declaring the same names,
/// plus nesting where the language allows it.
const BLOCK_NAMESPACE_CASES: &[(&str, &str, &[Case])] = &[
    // Without a preceding statement package a block package still roots
    // the name; a dotted block name keeps every segment.
    (
        "scala",
        "package R9Left { class R9W { def probe(): Unit = () } }\n\
         package R9Right { class R9W { def probe(): Unit = () } }\n\
         package r9.dotted { class R9D }\n",
        &[
            ("R9W", None, "R9Left.R9W", true),
            ("probe", Some("R9W"), "R9Left.R9W.probe", true),
            ("R9D", None, "r9.dotted.R9D", true),
        ],
    ),
    // Stacked statement clauses prefix a block package; a declaration after
    // the block is back in the statement packages.
    (
        "scala",
        "package r9a\npackage r9b\n\
         package R9Block { class R9W { def probe(): Unit = () } }\n\
         class R9After\n",
        &[
            ("R9W", None, "r9a.r9b.R9Block.R9W", true),
            ("probe", Some("R9W"), "r9a.r9b.R9Block.R9W.probe", true),
            ("R9After", None, "r9a.r9b.R9After", true),
        ],
    ),
    (
        "rs",
        "mod r9_left { pub struct R9W; impl R9W { pub fn probe() {} } }\n\
         mod r9_right { pub struct R9W; impl R9W { pub fn probe() {} } }\n\
         mod r9_outer { mod r9_inner { pub fn probe() {} } }\n",
        &[
            ("R9W", Some("r9_left"), "r9_left::R9W", false),
            ("R9W", Some("r9_right"), "r9_right::R9W", false),
            ("probe", Some("r9_inner"), "r9_outer::r9_inner::probe", false),
        ],
    ),
    (
        "ts",
        "namespace R9Left { export class R9W { probe(): void {} } }\n\
         namespace R9Right { export class R9W { probe(): void {} } }\n\
         namespace R9Outer { export namespace R9Inner { export function probe(): void {} } }\n",
        &[
            ("R9W", Some("R9Left"), "R9Left.R9W", false),
            ("R9W", Some("R9Right"), "R9Right.R9W", false),
            ("probe", Some("R9Inner"), "R9Outer.R9Inner.probe", false),
        ],
    ),
    (
        "cpp",
        "namespace R9Left { class R9W { public: void probe() {} }; }\n\
         namespace R9Right { class R9W { public: void probe() {} }; }\n\
         namespace R9Outer { namespace R9Inner { void probe() {} } }\n",
        &[
            ("R9W", Some("R9Left"), "R9Left::R9W", true),
            ("R9W", Some("R9Right"), "R9Right::R9W", true),
            ("probe", Some("R9Inner"), "R9Outer::R9Inner::probe", true),
        ],
    ),
    (
        "cs",
        "namespace R9Left { class R9W { void Probe() {} } }\n\
         namespace R9Right { class R9W { void Probe() {} } }\n\
         namespace R9Outer { namespace R9Inner { class R9Nested {} } }\n",
        &[
            ("R9W", Some("R9Left"), "R9Left.R9W", true),
            ("R9W", Some("R9Right"), "R9Right.R9W", true),
            ("R9Nested", Some("R9Inner"), "R9Outer.R9Inner.R9Nested", true),
        ],
    ),
    (
        "php",
        "<?php\nnamespace R9Left { class R9W { function probe() {} } }\n\
         namespace R9Right { class R9W { function probe() {} } }\n",
        &[
            ("R9W", Some("R9Left"), "R9Left\\R9W", true),
            ("probe", Some("R9W"), "R9Left\\R9W::probe", true),
        ],
    ),
    // Perl package names are absolute, nested blocks included.
    (
        "pl",
        "package R9Left { sub probe { 1 } }\n\
         package R9Right { sub probe { 1 } }\n\
         package R9Outer { package R9Inner { sub nested { 1 } } }\n",
        &[
            ("probe", Some("R9Left"), "R9Left::probe", true),
            ("probe", Some("R9Right"), "R9Right::probe", true),
            ("nested", Some("R9Inner"), "R9Inner::nested", true),
        ],
    ),
];

#[test]
fn block_namespaces_keep_sibling_names_distinct() {
    for (extension, source, cases) in BLOCK_NAMESPACE_CASES {
        let extracted = extract_inline(source, extension);
        let label = format!("{} ({})", extracted.label, source.lines().next().unwrap_or_default());
        for (name, container, qualified, rooted) in *cases {
            let found: Vec<&SymbolRecord> = extracted
                .records
                .iter()
                .filter(|record| record.identity.is_namespace_rooted == *rooted && in_file_name(record) == *qualified)
                .collect();
            assert_eq!(found.len(), 1, "{label}: expected one `{qualified}` (rooted: {rooted}), found {}", found.len());
            let identity = &found[0].identity;
            assert_eq!(identity.name, *name, "{label}: `{qualified}` name");
            assert_eq!(identity.container_name.as_deref(), *container, "{label}: `{qualified}` container");
        }
        // Every same-named declaration has its own qualified name and ID.
        for name in ["R9W", "probe", "Probe"] {
            let same: Vec<&SymbolRecord> = extracted
                .records
                .iter()
                .filter(|record| record.identity.name == name)
                .collect();
            let names: HashSet<&str> = same.iter().map(|record| record.identity.qualified_name.as_str()).collect();
            let ids: HashSet<&str> = same.iter().map(|record| record.id.0.as_str()).collect();
            assert_eq!(names.len(), same.len(), "{label}: `{name}` qualified names collide: {names:?}");
            assert_eq!(ids.len(), same.len(), "{label}: `{name}` IDs collide");
        }
    }
}

/// `private[p]` naming a block package, nested ones included, is package
/// access.
#[test]
fn scala_access_qualified_by_a_block_package_is_package_access() {
    let extracted = extract_inline(
        "package R9Outer {\n  package R9Inner {\n    class R9W {\n      private[R9Inner] def inner(): Unit = ()\n      private[R9Outer] def outer(): Unit = ()\n    }\n  }\n}\n",
        "scala",
    );
    for name in ["inner", "outer"] {
        assert_eq!(
            extracted.record(name, Some("R9W")).visibility.level,
            VisibilityLevel::Package,
            "`{name}` visibility"
        );
    }
}
