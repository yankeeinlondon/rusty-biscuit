//! Owner identity for repeated owner names (review 1 of
//! `2026-10-03-symbol-filters-and-reports`): a member's qualified name keeps
//! the qualification that tells same-named owners apart (a Rust `impl
//! right::Repeated`, stacked Scala package clauses), and its
//! `relations.parent` is that owner's record by ID, also when the member is
//! declared outside the owner's body (Rust `impl`, Swift extension, Lua
//! table function). Same-language controls with repeated nested names pin the
//! paths that were already right.

use tree_hugger::{SymbolRecord, TreeFile};

use crate::symbols_fixtures::in_file_name;

/// `(member in-file qualified name, owner in-file qualified name)`; each name
/// must be unique in the source.
type OwnerCase = (&'static str, &'static str);

struct OwnerRow {
    label: &'static str,
    extension: &'static str,
    source: &'static str,
    cases: &'static [OwnerCase],
}

const OWNER_MATRIX: &[OwnerRow] = &[
    OwnerRow {
        label: "Rust same-named types with impls in their own modules",
        extension: "rs",
        source: "mod left { pub struct Boxed; impl Boxed { pub fn work(&self) {} } }\n\
                 mod right { pub struct Boxed; impl Boxed { pub fn work(&self) {} } }\n",
        cases: &[
            ("left::Boxed::work", "left::Boxed"),
            ("right::Boxed::work", "right::Boxed"),
            ("right::Boxed::work::self", "right::Boxed"),
        ],
    },
    OwnerRow {
        label: "Rust impls of qualified self types",
        extension: "rs",
        source: "mod left { pub struct Repeated; }\n\
                 mod right { pub struct Repeated; }\n\
                 impl left::Repeated { fn first(&self) {} }\n\
                 impl right::Repeated { fn second(&self) {} }\n\
                 impl<T> self::right::Repeated { fn generic(&self) {} }\n\
                 mod outer {\n\
                     pub struct Inner;\n\
                     mod nested {\n\
                         impl super::Inner { fn up(&self) {} }\n\
                         impl crate::left::Repeated { fn rooted(&self) {} }\n\
                     }\n\
                 }\n",
        cases: &[
            ("left::Repeated::first", "left::Repeated"),
            ("right::Repeated::second", "right::Repeated"),
            ("right::Repeated::generic", "right::Repeated"),
            ("outer::Inner::up", "outer::Inner"),
            ("left::Repeated::rooted", "left::Repeated"),
        ],
    },
    OwnerRow {
        label: "Swift extensions of same-named nested types",
        extension: "swift",
        source: "class Left { class Repeated {} }\n\
                 class Right { class Repeated {} }\n\
                 extension Left.Repeated { func onLeft() {} }\n\
                 extension Right.Repeated { func onRight() {} }\n",
        cases: &[
            ("Left.Repeated.onLeft", "Left.Repeated"),
            ("Right.Repeated.onRight", "Right.Repeated"),
        ],
    },
    OwnerRow {
        label: "Swift same-named nested types in their bodies",
        extension: "swift",
        source: "class Left { class Repeated { func inLeft() {} } }\n\
                 class Right { class Repeated { func inRight() {} } }\n",
        cases: &[
            ("Left.Repeated.inLeft", "Left.Repeated"),
            ("Right.Repeated.inRight", "Right.Repeated"),
        ],
    },
    OwnerRow {
        label: "Scala stacked package clauses nest",
        extension: "scala",
        source: "package outer\npackage inner\n\nobject PackageProbe { def work(): Unit = () }\n",
        cases: &[("outer.inner.PackageProbe.work", "outer.inner.PackageProbe")],
    },
    OwnerRow {
        label: "PHP consecutive namespaces replace each other",
        extension: "php",
        source: "<?php\nnamespace A;\nclass W { function f() {} }\nnamespace B;\nclass W { function g() {} }\n",
        cases: &[("A\\W::f", "A\\W"), ("B\\W::g", "B\\W")],
    },
    OwnerRow {
        label: "C++ same-named classes in two namespaces",
        extension: "cpp",
        source: "namespace a { class W { void f(); }; }\nnamespace b { class W { void g(); }; }\n",
        cases: &[("a::W::f", "a::W"), ("b::W::g", "b::W")],
    },
    OwnerRow {
        label: "C# same-named classes in two namespaces",
        extension: "cs",
        source: "namespace A { class W { void F() {} } }\nnamespace B { class W { void G() {} } }\n",
        cases: &[("A.W.F", "A.W"), ("B.W.G", "B.W")],
    },
    OwnerRow {
        label: "Java same-named nested classes",
        extension: "java",
        source: "package p;\nclass A { class In { void f() {} } }\nclass B { class In { void g() {} } }\n",
        cases: &[("p.A.In.f", "p.A.In"), ("p.B.In.g", "p.B.In")],
    },
    OwnerRow {
        label: "Python same-named nested classes",
        extension: "py",
        source: "class A:\n    class In:\n        def f(self): pass\nclass B:\n    class In:\n        def g(self): pass\n",
        cases: &[("A.In.f", "A.In"), ("B.In.g", "B.In")],
    },
    OwnerRow {
        label: "TypeScript same-named classes in two namespaces",
        extension: "ts",
        source: "namespace A { export class W { f() {} } }\nnamespace B { export class W { g() {} } }\n",
        cases: &[("A.W.f", "A.W"), ("B.W.g", "B.W")],
    },
    OwnerRow {
        label: "Go pointer receiver",
        extension: "go",
        source: "package p\ntype W struct{}\nfunc (w *W) F() {}\n",
        cases: &[("p.W.F", "p.W")],
    },
];

fn records_of(source: &str, extension: &str) -> Vec<SymbolRecord> {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join(format!("owners.{extension}"));
    std::fs::write(&path, source).expect("write source");
    TreeFile::new(&path)
        .expect("parse source")
        .symbol_index_v2()
        .expect("symbol index")
        .symbols
}

fn named<'a>(records: &'a [SymbolRecord], label: &str, qualified: &str) -> &'a SymbolRecord {
    let found: Vec<&SymbolRecord> = records.iter().filter(|record| in_file_name(record) == qualified).collect();
    assert_eq!(
        found.len(),
        1,
        "{label}: expected one `{qualified}`, have {:?}",
        records.iter().map(in_file_name).collect::<Vec<_>>()
    );
    found[0]
}

fn assert_owned(records: &[SymbolRecord], label: &str, member: &str, owner: &str) {
    let child = named(records, label, member);
    let owner_record = named(records, label, owner);
    let parent = child
        .relations
        .parent
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: `{member}` has no parent"));
    assert_eq!(parent.id.as_ref(), Some(&owner_record.id), "{label}: `{member}` parent is {parent:?}, not `{owner}`");
    assert_eq!(parent.qualified_name.as_deref(), Some(owner), "{label}: `{member}` parent name");
    assert!(
        owner_record.relations.members.iter().any(|reference| reference.id.as_ref() == Some(&child.id)),
        "{label}: `{owner}` does not list `{member}`"
    );
}

#[test]
fn members_attach_to_their_own_same_named_owner() {
    for row in OWNER_MATRIX {
        let records = records_of(row.source, row.extension);
        for (member, owner) in row.cases {
            assert_owned(&records, row.label, member, owner);
        }
    }
}

#[test]
fn stacked_scala_package_clauses_root_the_name() {
    let records = records_of("package outer\npackage inner\n\nobject PackageProbe { def work(): Unit = () }\n", "scala");
    let work = named(&records, "scala", "outer.inner.PackageProbe.work");
    assert!(work.identity.is_namespace_rooted);
}

#[test]
fn same_named_lua_tables_own_the_functions_after_them() {
    let source = "do\n  local M = {}\n  function M.first() end\nend\n\
                  do\n  local M = {}\n  function M:second() end\nend\n";
    let records = records_of(source, "lua");
    let tables: Vec<&SymbolRecord> = records.iter().filter(|record| record.identity.name == "M").collect();
    assert_eq!(tables.len(), 2, "two `M` tables");
    for (function, table) in [("M.first", tables[0]), ("M:second", tables[1])] {
        let child = named(&records, "lua", function);
        assert_eq!(
            child.relations.parent.as_ref().and_then(|parent| parent.id.as_ref()),
            Some(&table.id),
            "`{function}` belongs to the `M` before it"
        );
        assert!(table.relations.members.iter().any(|reference| reference.id.as_ref() == Some(&child.id)));
    }
}

#[test]
fn an_unqualified_impl_of_an_ambiguous_name_links_to_no_owner() {
    // `Boxed` is declared in two modules; a file-level `impl Boxed` names
    // neither, so linking either would be a guess.
    let source = "mod left { pub struct Boxed; }\nmod right { pub struct Boxed; }\nimpl Boxed { fn guess(&self) {} }\n";
    let records = records_of(source, "rs");
    let guess = named(&records, "rust", "Boxed::guess");
    assert!(guess.relations.parent.is_none(), "{:?}", guess.relations.parent);
}
