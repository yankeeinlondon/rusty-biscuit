//! Owner identity for repeated owner names (review 1 of
//! `2026-10-03-symbol-filters-and-reports`): a member's qualified name keeps
//! the qualification that tells same-named owners apart (a Rust `impl
//! right::Repeated`, stacked Scala package clauses), and its
//! `relations.parent` is that owner's record by ID, also when the member is
//! declared outside the owner's body (Rust `impl`, Swift extension, Lua
//! table function). Same-language controls with repeated nested names pin the
//! paths that were already right. A sole same-named record never becomes the
//! owner of a member whose explicit qualification names another owner
//! (`ABSENT_OWNER_MATRIX`).

use tree_hugger::ProgrammingLanguage;
use tree_hugger::{SymbolKindV2, SymbolRecord, TreeFile};

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
        // An imported self type (`use a::V; impl V`) needs import
        // resolution and stays unlinked: `symbols_relations` holds that row.
        label: "Rust sole type with an unqualified or rooted impl",
        extension: "rs",
        source: "pub struct W;\n\
                 impl W { pub fn direct(&self) {} }\n\
                 mod inner { impl crate::W { pub fn rooted(&self) {} } }\n",
        cases: &[
            ("W::direct", "W"),
            ("W::rooted", "W"),
            ("W::rooted::self", "W"),
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
        // An extension is written at file scope: a function's same-named
        // type is out of its view, while that type keeps its own members.
        label: "Swift extension beside a function-local same-named type",
        extension: "swift",
        source: "class Repeated {}\n\
                 func f() { class Repeated { func inLocal() {} } }\n\
                 extension Repeated { func probe() {} }\n",
        cases: &[("Repeated.probe", "Repeated"), ("f.Repeated.inLocal", "f.Repeated")],
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

/// A member whose explicit qualification names an owner declared in another
/// file, beside a sole same-named local record that is not its owner.
struct AbsentOwnerRow {
    label: &'static str,
    extension: &'static str,
    source: &'static str,
    /// In-file qualified names of the members that must have no container.
    members: &'static [&'static str],
    /// In-file qualified name of the unrelated local record.
    local: &'static str,
}

const ABSENT_OWNER_MATRIX: &[AbsentOwnerRow] = &[
    AbsentOwnerRow {
        label: "Rust impl of a type in an out-of-line module",
        extension: "rs",
        source: "pub struct W;\nmod other;\nimpl other::W { pub fn probe(&self) {} }\n",
        members: &["other::W::probe", "other::W::probe::self"],
        local: "W",
    },
    AbsentOwnerRow {
        label: "Rust impl of an external crate's type beside a local enum",
        extension: "rs",
        source: "pub enum UniqueOwner { A }\nimpl external::UniqueOwner { fn probe(&self) {} }\n",
        members: &["external::UniqueOwner::probe"],
        local: "UniqueOwner",
    },
    AbsentOwnerRow {
        label: "Swift extension of an external qualified type",
        extension: "swift",
        source: "class UniqueOwner {}\nextension External.UniqueOwner { func probe() {} }\n",
        members: &["External.UniqueOwner.probe"],
        local: "UniqueOwner",
    },
    AbsentOwnerRow {
        label: "Swift extension of a name only a function declares",
        extension: "swift",
        source: "func f() { class UniqueOwner {} }\nextension UniqueOwner { func probe() {} }\n",
        members: &["UniqueOwner.probe"],
        local: "f.UniqueOwner",
    },
    AbsentOwnerRow {
        label: "Swift extension of a path only a function's type declares",
        extension: "swift",
        source: "func f() { class Outer { class UniqueOwner {} } }\nextension Outer.UniqueOwner { func probe() {} }\n",
        members: &["Outer.UniqueOwner.probe"],
        local: "f.Outer.UniqueOwner",
    },
    AbsentOwnerRow {
        label: "Swift extension of a name only a closure declares",
        extension: "swift",
        source: "let reviewValue: Int = {\n    class UniqueOwner {}\n    return 0\n}()\n\
                 extension UniqueOwner { func probe() {} }\n",
        members: &["UniqueOwner.probe"],
        local: "UniqueOwner",
    },
    AbsentOwnerRow {
        label: "Swift extension of a two-segment path only a closure declares",
        extension: "swift",
        source: "let reviewClosure = { struct ReviewHolder { struct ReviewInner {} } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        members: &["ReviewHolder.ReviewInner.reviewMethod"],
        local: "ReviewHolder",
    },
    AbsentOwnerRow {
        label: "Swift extension of a three-segment path only a closure declares",
        extension: "swift",
        source: "let reviewClosure = { struct ReviewHolder { struct ReviewMiddle { struct ReviewInner {} } } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewMiddle.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        members: &["ReviewHolder.ReviewMiddle.ReviewInner.reviewMethod"],
        local: "ReviewHolder",
    },
    AbsentOwnerRow {
        label: "Lua member of an external qualified table",
        extension: "lua",
        source: "local UniqueOwner = {}\n\
                 function External.UniqueOwner.review_probe() end\n\
                 return UniqueOwner\n",
        members: &["External.UniqueOwner.review_probe"],
        local: "UniqueOwner",
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

crate::language_cases::language_tests!(members_attach_to_their_own_same_named_owner => check_members_attach_to_their_own_same_named_owner);

fn check_members_attach_to_their_own_same_named_owner(language: ProgrammingLanguage) {
    for row in OWNER_MATRIX.iter().filter(|row| ProgrammingLanguage::from_extension(row.extension) == Some(language)) {
        let records = records_of(row.source, row.extension);
        for (member, owner) in row.cases {
            assert_owned(&records, row.label, member, owner);
        }
    }
}

#[test]
fn a_sole_same_named_record_never_overrides_an_explicit_owner() {
    for row in ABSENT_OWNER_MATRIX {
        let records = records_of(row.source, row.extension);
        let local = named(&records, row.label, row.local);
        for member in row.members {
            let child = named(&records, row.label, member);
            assert!(child.relations.parent.is_none(), "{}: `{member}` parent {:?}", row.label, child.relations.parent);
            // A Rust impl member keeps a textual container named by the self
            // type as written, with no ID.
            assert!(
                child.relations.container.as_ref().is_none_or(|container| container.id.is_none()),
                "{}: `{member}` container {:?}",
                row.label,
                child.relations.container
            );
            assert!(
                child.identity.container_kind.is_none() || row.extension == "swift",
                "{}: `{member}` takes the kind of the local `{}`: {:?}",
                row.label,
                row.local,
                child.identity.container_kind
            );
            assert!(
                !local.relations.members.iter().any(|reference| reference.id.as_ref() == Some(&child.id)),
                "{}: local `{}` lists `{member}`",
                row.label,
                row.local
            );
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

/// Two Go types of one name are competing owners, not a table each in
/// force until the next: a receiver method names neither by proximity.
#[test]
fn a_receiver_method_of_a_repeated_go_type_links_to_no_owner() {
    let source = "package shapes\n\ntype ReviewBase struct{}\ntype ReviewBase struct{}\n\nfunc (r ReviewBase) M() {}\n";
    let records = records_of(source, "go");
    let method = named(&records, "go", "shapes.ReviewBase.M");
    assert!(method.relations.parent.is_none(), "{:?}", method.relations.parent);
    assert!(method.relations.container.is_none(), "{:?}", method.relations.container);
    for record in records.iter().filter(|record| in_file_name(record) == "shapes.ReviewBase") {
        assert!(
            !record.relations.members.iter().any(|member| member.id.as_ref() == Some(&method.id)),
            "line {} lists the method",
            record.source.declaration_span.start.line
        );
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

/// A Swift extension of a nested file-level type beside other declarations
/// whose displayed names share its path: the extension's method belongs to
/// the file-level type declared on `owner_line`, which also takes the
/// extension's conformance to `ReviewP`; no other type does.
pub struct NestedShadowRow {
    pub label: &'static str,
    pub source: &'static str,
    /// The extended type's in-file qualified name.
    pub owner: &'static str,
    /// The file-level owner's declaration line.
    pub owner_line: u32,
    /// The extension method's in-file qualified name.
    pub member: &'static str,
}

/// The two- and three-segment extension paths a closure's same-named
/// owner shadows by display name only, and the controls at each depth (no
/// shadow, a differently named closure owner, a same-named owner in a named
/// function) plus the one-segment control.
pub const NESTED_SHADOW_MATRIX: &[NestedShadowRow] = &[
    NestedShadowRow {
        label: "two segments, closure-local outer type of the same name",
        source: SWIFT_NESTED_SHADOW,
        owner: "ReviewHolder.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "three segments, closure-local outer type of the same name",
        source: "struct ReviewHolder { struct ReviewMiddle { struct ReviewInner {} } }\n\
                 let reviewClosure = { struct ReviewHolder { struct ReviewMiddle { struct ReviewInner {} } } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewMiddle.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewMiddle.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewMiddle.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "one segment, closure-local type of the same name",
        source: "struct ReviewHolder {}\n\
                 let reviewClosure = { struct ReviewHolder {} }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder",
        owner_line: 1,
        member: "ReviewHolder.reviewMethod",
    },
    NestedShadowRow {
        label: "two segments, no shadow",
        source: "struct ReviewHolder { struct ReviewInner {} }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "two segments, differently named closure owner",
        source: "struct ReviewHolder { struct ReviewInner {} }\n\
                 let reviewClosure = { struct ReviewOther { struct ReviewInner {} } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "two segments, same-named owner in a named function",
        source: "struct ReviewHolder { struct ReviewInner {} }\n\
                 func reviewProbe() { struct ReviewHolder { struct ReviewInner {} } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "two segments, selective function import of the outer name",
        source: "import func Lib.ReviewHolder\n\
                 struct ReviewHolder { struct ReviewInner {} }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewInner",
        owner_line: 2,
        member: "ReviewHolder.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "three segments, no shadow",
        source: "struct ReviewHolder { struct ReviewMiddle { struct ReviewInner {} } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewMiddle.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewMiddle.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewMiddle.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "three segments, differently named closure owner",
        source: "struct ReviewHolder { struct ReviewMiddle { struct ReviewInner {} } }\n\
                 let reviewClosure = { struct ReviewOther { struct ReviewMiddle { struct ReviewInner {} } } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewMiddle.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewMiddle.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewMiddle.ReviewInner.reviewMethod",
    },
    NestedShadowRow {
        label: "three segments, same-named owner in a named function",
        source: "struct ReviewHolder { struct ReviewMiddle { struct ReviewInner {} } }\n\
                 func reviewProbe() { struct ReviewHolder { struct ReviewMiddle { struct ReviewInner {} } } }\n\
                 protocol ReviewP {}\n\
                 extension ReviewHolder.ReviewMiddle.ReviewInner: ReviewP { func reviewMethod() {} }\n",
        owner: "ReviewHolder.ReviewMiddle.ReviewInner",
        owner_line: 1,
        member: "ReviewHolder.ReviewMiddle.ReviewInner.reviewMethod",
    },
];

/// The two-segment nested-shadow source, shared with the cache and CLI
/// tests.
pub const SWIFT_NESTED_SHADOW: &str = "struct ReviewHolder { struct ReviewInner {} }\n\
    let reviewClosure = { struct ReviewHolder { struct ReviewInner {} } }\n\
    protocol ReviewP {}\n\
    extension ReviewHolder.ReviewInner: ReviewP { func reviewMethod() {} }\n";

/// Checks a [`NestedShadowRow`] against `records` (a complete index or a
/// containment result): the method's parent and container are the
/// file-level owner by ID, that owner lists it and implements `ReviewP` by
/// ID, and no other same-named type does either.
pub fn check_nested_shadow(label: &str, row: &NestedShadowRow, records: &[SymbolRecord]) {
    let label = format!("{label}: {}", row.label);
    let types: Vec<&SymbolRecord> =
        records.iter().filter(|record| in_file_name(record) == row.owner && matches!(record.kind, SymbolKindV2::Type | SymbolKindV2::Class)).collect();
    let owner = types
        .iter()
        .find(|record| record.source.declaration_span.start.line == row.owner_line)
        .unwrap_or_else(|| panic!("{label}: no `{}` on line {}", row.owner, row.owner_line));
    let member = named(records, &label, row.member);
    for (relation, link) in [("parent", &member.relations.parent), ("container", &member.relations.container)] {
        assert_eq!(
            link.as_ref().and_then(|link| link.id.as_ref()),
            Some(&owner.id),
            "{label}: `{}` {relation} is {link:?}",
            row.member
        );
    }
    let protocol = named(records, &label, "ReviewP");
    for record in &types {
        let lists = record.relations.members.iter().any(|reference| reference.id.as_ref() == Some(&member.id));
        let conforms = record.relations.implements.iter().any(|target| target.id.as_ref() == Some(&protocol.id));
        let is_owner = record.id == owner.id;
        assert_eq!(lists, is_owner, "{label}: `{}` line {} lists the method", row.owner, record.source.declaration_span.start.line);
        assert_eq!(conforms, is_owner, "{label}: `{}` line {} implements `ReviewP`", row.owner, record.source.declaration_span.start.line);
    }
}

#[test]
fn swift_extension_members_belong_to_the_file_level_type_beside_same_named_local_owners() {
    for row in NESTED_SHADOW_MATRIX {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("shadow.swift");
        std::fs::write(&path, row.source).expect("write source");
        let file = TreeFile::new(&path).expect("parse source");
        assert!(file.syntax_diagnostics().is_empty(), "{}: {:?}", row.label, file.syntax_diagnostics());
        check_nested_shadow("complete", row, &file.symbol_index_v2().expect("symbol index").symbols);
        let contained = file
            .query(&tree_hugger::SymbolQuery::new().capabilities([tree_hugger::Capability::Containment]))
            .expect("containment query");
        check_nested_shadow("declarations with containment", row, &contained.index.symbols);
    }
}

/// A Swift extension whose owner the relation lookup does not prove: two
/// same-named direct, final nested or intermediate owners, a type alias or
/// a selective type import of the direct or intermediate name. The
/// extension's conformance attaches nowhere, and under Containment its
/// method takes the same decision: a container named by the extended type
/// as written, with no ID, no parent and no local membership, however a
/// local record's name or position reads.
pub struct UnprovedOwnerRow {
    pub label: &'static str,
    pub source: &'static str,
    /// The extension method's in-file qualified name.
    pub member: &'static str,
    /// The extended type as written: the method's textual container.
    pub written: &'static str,
    /// Local types (in-file qualified name, declaration line) a by-name
    /// lookup could take for the owner.
    pub locals: &'static [(&'static str, u32)],
}

pub const UNPROVED_OWNER_MATRIX: &[UnprovedOwnerRow] = &[
    UnprovedOwnerRow {
        label: "duplicate direct owner",
        source: "protocol ReviewP {}\n\
                 struct ReviewBase {}\n\
                 struct ReviewBase {}\n\
                 extension ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewBase.reviewProbe",
        written: "ReviewBase",
        locals: &[("ReviewBase", 2), ("ReviewBase", 3)],
    },
    UnprovedOwnerRow {
        label: "duplicate final nested owner",
        source: "protocol ReviewP {}\n\
                 struct ReviewHolder {\n\
                 \x20   struct ReviewBase {}\n\
                 \x20   struct ReviewBase {}\n\
                 }\n\
                 extension ReviewHolder.ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewHolder.ReviewBase.reviewProbe",
        written: "ReviewHolder.ReviewBase",
        locals: &[("ReviewHolder.ReviewBase", 3), ("ReviewHolder.ReviewBase", 4)],
    },
    UnprovedOwnerRow {
        label: "duplicate intermediate owner",
        source: "protocol ReviewP {}\n\
                 struct ReviewHolder { struct ReviewBase {} }\n\
                 struct ReviewHolder {}\n\
                 extension ReviewHolder.ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewHolder.ReviewBase.reviewProbe",
        written: "ReviewHolder.ReviewBase",
        locals: &[("ReviewHolder.ReviewBase", 2)],
    },
    UnprovedOwnerRow {
        label: "alias of the direct owner's name",
        source: "protocol ReviewP {}\n\
                 typealias ReviewBase = External\n\
                 struct ReviewOther { struct ReviewBase {} }\n\
                 extension ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewBase.reviewProbe",
        written: "ReviewBase",
        locals: &[("ReviewOther.ReviewBase", 3)],
    },
    UnprovedOwnerRow {
        label: "alias of the intermediate owner's name",
        source: "protocol ReviewP {}\n\
                 typealias ReviewHolder = External\n\
                 struct ReviewOther { struct ReviewHolder { struct ReviewBase {} } }\n\
                 extension ReviewHolder.ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewHolder.ReviewBase.reviewProbe",
        written: "ReviewHolder.ReviewBase",
        locals: &[("ReviewOther.ReviewHolder.ReviewBase", 3)],
    },
    UnprovedOwnerRow {
        label: "alias beside the intermediate owner",
        source: "protocol ReviewP {}\n\
                 struct ReviewHolder { struct ReviewBase {} }\n\
                 typealias ReviewHolder = External\n\
                 extension ReviewHolder.ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewHolder.ReviewBase.reviewProbe",
        written: "ReviewHolder.ReviewBase",
        locals: &[("ReviewHolder.ReviewBase", 2)],
    },
    UnprovedOwnerRow {
        label: "selective import of the direct owner's name",
        source: "import struct Lib.ReviewBase\n\
                 protocol ReviewP {}\n\
                 struct ReviewOther { struct ReviewBase {} }\n\
                 extension ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewBase.reviewProbe",
        written: "ReviewBase",
        locals: &[("ReviewOther.ReviewBase", 3)],
    },
    UnprovedOwnerRow {
        label: "selective import of the intermediate owner's name",
        source: "import struct Lib.ReviewHolder\n\
                 protocol ReviewP {}\n\
                 struct ReviewOther { struct ReviewHolder { struct ReviewBase {} } }\n\
                 extension ReviewHolder.ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewHolder.ReviewBase.reviewProbe",
        written: "ReviewHolder.ReviewBase",
        locals: &[("ReviewOther.ReviewHolder.ReviewBase", 3)],
    },
    UnprovedOwnerRow {
        label: "selective import beside the intermediate owner",
        source: "import struct Lib.ReviewHolder\n\
                 protocol ReviewP {}\n\
                 struct ReviewHolder { struct ReviewBase {} }\n\
                 extension ReviewHolder.ReviewBase: ReviewP { func reviewProbe() {} }\n",
        member: "ReviewHolder.ReviewBase.reviewProbe",
        written: "ReviewHolder.ReviewBase",
        locals: &[("ReviewHolder.ReviewBase", 3)],
    },
    UnprovedOwnerRow {
        label: "external owner",
        source: "protocol ReviewP {}\n\
                 struct ReviewBase {}\n\
                 extension External: ReviewP { func reviewProbe() {} }\n",
        member: "External.reviewProbe",
        written: "External",
        locals: &[("ReviewBase", 2)],
    },
];

/// Checks an [`UnprovedOwnerRow`] against `records` (a complete index or a
/// containment result): the method has no parent and a container with no
/// ID named as written; no record lists it, and no record (the local
/// candidates included) implements `ReviewP`.
pub fn check_unproved_owner(label: &str, row: &UnprovedOwnerRow, records: &[SymbolRecord]) {
    let label = format!("{label}: {}", row.label);
    let member = named(records, &label, row.member);
    assert!(member.relations.parent.is_none(), "{label}: parent {:?}", member.relations.parent);
    let container = member.relations.container.as_ref().unwrap_or_else(|| panic!("{label}: no container"));
    assert_eq!((container.name.as_str(), container.id.as_ref()), (row.written, None), "{label}: container");
    for &(local, line) in row.locals {
        assert!(
            records.iter().any(|record| in_file_name(record) == local
                && record.source.declaration_span.start.line == line),
            "{label}: no local `{local}` on line {line}"
        );
    }
    let protocol = named(records, &label, "ReviewP");
    for record in records {
        let lists = record.relations.members.iter().any(|reference| reference.id.as_ref() == Some(&member.id));
        let conforms = record.relations.implements.iter().any(|target| target.id.as_ref() == Some(&protocol.id));
        let at = (in_file_name(record), record.source.declaration_span.start.line);
        assert!(!lists, "{label}: `{}` line {} lists the method", at.0, at.1);
        assert!(!conforms, "{label}: `{}` line {} implements `ReviewP`", at.0, at.1);
    }
}

/// Conformance and membership agree for an unproved extension owner, on
/// the complete index, a declarations query with Containment and a
/// method-only query with Containment.
#[test]
fn swift_extension_members_of_an_unproved_owner_join_no_local_type() {
    use tree_hugger::{Capability, SymbolQuery};
    let check = |row: &UnprovedOwnerRow| {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("unproved.swift");
        std::fs::write(&path, row.source).expect("write source");
        let file = TreeFile::new(&path).expect("parse source");
        assert!(file.syntax_diagnostics().is_empty(), "{}: {:?}", row.label, file.syntax_diagnostics());
        check_unproved_owner("complete", row, &file.symbol_index_v2().expect("symbol index").symbols);
        let contained = file.query(&SymbolQuery::new().capabilities([Capability::Containment])).expect("containment");
        check_unproved_owner("declarations with containment", row, &contained.index.symbols);
        let methods = file
            .query(&SymbolQuery::new().kinds([SymbolKindV2::Method]).capabilities([Capability::Containment]))
            .expect("method-only containment");
        let member = named(&methods.index.symbols, row.label, row.member);
        assert!(member.relations.parent.is_none(), "{}: method-only parent", row.label);
        assert_eq!(
            member.relations.container.as_ref().map(|container| (container.name.as_str(), container.id.is_none())),
            Some((row.written, true)),
            "{}: method-only container",
            row.label
        );
    };
    let failed: Vec<&str> = UNPROVED_OWNER_MATRIX
        .iter()
        .filter(|row| std::panic::catch_unwind(|| check(row)).is_err())
        .map(|row| row.label)
        .collect();
    assert!(failed.is_empty(), "rows failed (messages above): {failed:#?}");
}
