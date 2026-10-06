//! The declared relations matrix: `relations.extends` / `relations.implements`
//! hold each base a declaration writes in its own header, as written (outer
//! whitespace trimmed, generic arguments kept, access and `virtual` keywords
//! dropped), in source order with equal expressions once. A target carries a
//! same-file `id` and in-file `qualified_name` only when its path names
//! exactly one eligible declaration of the file under the language's lexical
//! rule: an external path that shares a local short name, an alias, a type
//! parameter, an ambiguous name and a dynamic expression keep their text
//! alone. Each language has one row, checked on the complete index and on a
//! declarations-only query of the owner's kind (whose targets are of other,
//! unselected kinds). Rust impls have their own tests below.

use std::io::Write;

use tree_hugger::{
    FileSymbolIndex, ProgrammingLanguage, SymbolKindV2, SymbolQuery, SymbolRecord, SymbolRef, TreeFile,
};

use crate::symbols_fixtures::in_file_name;

use ProgrammingLanguage as L;

/// One written target: its text and, when it resolves, the in-file
/// qualified name of the declaration it names.
pub struct Target {
    pub name: &'static str,
    pub resolves_to: Option<&'static str>,
}

const fn to(name: &'static str, resolves_to: &'static str) -> Target {
    Target { name, resolves_to: Some(resolves_to) }
}

const fn text(name: &'static str) -> Target {
    Target { name, resolves_to: None }
}

/// An owner (by in-file qualified name) and its relations.
pub struct Owner {
    pub qualified_name: &'static str,
    pub extends: &'static [Target],
    pub implements: &'static [Target],
}

pub enum Support {
    Supported,
    /// The base list mixes inheritance and conformance; nothing is read
    /// until the author decides how to classify it.
    Pending(&'static str),
    /// The language declares no bases in this feature's scope; the row's
    /// source holds the forms that must not be inferred.
    Absent(&'static str),
}

pub struct RelationsRow {
    pub language: ProgrammingLanguage,
    pub support: Support,
    pub extension: &'static str,
    pub source: &'static str,
    pub owners: &'static [Owner],
}

pub const RELATIONS_MATRIX: &[RelationsRow] = &[
    RelationsRow {
        language: L::Rust,
        support: Support::Supported,
        extension: "rs",
        source: "pub trait Base {}\n\
                 #[cfg(a)]\npub trait Amb {}\n#[cfg(not(a))]\npub trait Amb {}\n\
                 pub trait Child: Base + other::Base + 'static + Clone + ?Sized + for<'x> Fn(&'x u8) {}\n\
                 pub trait Sub: Child + Child + Amb {}\n\
                 mod a { pub trait Dup {} pub trait UsesDup: Dup + super::Base + crate::Base {} }\n\
                 mod b { pub trait UsesOuter: Base {} }\n",
        owners: &[
            Owner {
                qualified_name: "Child",
                extends: &[to("Base", "Base"), text("other::Base"), text("Clone"), text("for<'x> Fn(&'x u8)")],
                implements: &[],
            },
            Owner { qualified_name: "Sub", extends: &[to("Child", "Child"), text("Amb")], implements: &[] },
            Owner {
                qualified_name: "a::UsesDup",
                extends: &[to("Dup", "a::Dup"), to("super::Base", "Base"), to("crate::Base", "Base")],
                implements: &[],
            },
            // A Rust item does not see its parent module's names.
            Owner { qualified_name: "b::UsesOuter", extends: &[text("Base")], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::TypeScript,
        support: Support::Supported,
        extension: "ts",
        source: "interface Base {}\ninterface Mixin<T> {}\nclass Root {}\nclass T {}\ntype Alias = Root;\n\
                 namespace ns { export class Inner {} export interface Dup {} }\ninterface Dup {}\n\
                 class Child extends Root implements Base, Mixin<string>, Mixin<number>, Base {}\n\
                 export class Q extends ns.Inner implements ns.Dup, Dup {}\n\
                 class Ext extends other.Root implements Alias {}\n\
                 class Dyn extends mix(Root) {}\n\
                 class Gen<T> extends T<string> {}\n\
                 interface Sub extends Base, ns.Dup, lib.Base {}\n",
        owners: &[
            Owner {
                qualified_name: "Child",
                extends: &[to("Root", "Root")],
                implements: &[to("Base", "Base"), to("Mixin<string>", "Mixin"), to("Mixin<number>", "Mixin")],
            },
            Owner {
                qualified_name: "Q",
                extends: &[to("ns.Inner", "ns.Inner")],
                implements: &[to("ns.Dup", "ns.Dup"), to("Dup", "Dup")],
            },
            Owner { qualified_name: "Ext", extends: &[text("other.Root")], implements: &[text("Alias")] },
            Owner { qualified_name: "Dyn", extends: &[text("mix(Root)")], implements: &[] },
            Owner { qualified_name: "Gen", extends: &[text("T<string>")], implements: &[] },
            Owner {
                qualified_name: "Sub",
                extends: &[to("Base", "Base"), to("ns.Dup", "ns.Dup"), text("lib.Base")],
                implements: &[],
            },
        ],
    },
    RelationsRow {
        language: L::JavaScript,
        support: Support::Supported,
        extension: "js",
        source: "class Root {}\nclass Child extends Root {}\nclass Dyn extends mix(Root) {}\n\
                 class Ext extends lib.Root {}\n",
        owners: &[
            Owner { qualified_name: "Child", extends: &[to("Root", "Root")], implements: &[] },
            Owner { qualified_name: "Dyn", extends: &[text("mix(Root)")], implements: &[] },
            Owner { qualified_name: "Ext", extends: &[text("lib.Root")], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::Go,
        support: Support::Supported,
        extension: "go",
        source: "package shapes\n\n\
                 type Base interface{ M() }\n\n\
                 type Num interface{ ~int | ~float64 }\n\n\
                 type G[T any] interface{ Get() T }\n\n\
                 type Iface interface {\n\tBase\n\tfmt.Stringer\n\tNum\n\tint\n\t~string\n\tG[int]\n\terror\n}\n\n\
                 type S struct {\n\tBase\n\tx int\n}\n",
        owners: &[
            Owner {
                qualified_name: "shapes.Iface",
                extends: &[
                    to("Base", "shapes.Base"),
                    text("fmt.Stringer"),
                    to("Num", "shapes.Num"),
                    to("G[int]", "shapes.G"),
                    text("error"),
                ],
                implements: &[],
            },
            // An embedded struct field is composition.
            Owner { qualified_name: "shapes.S", extends: &[], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::Python,
        support: Support::Supported,
        extension: "py",
        source: "class Base:\n    pass\n\n\
                 class Meta(type):\n    pass\n\n\
                 class Child(Base, mod.Base, Generic[T], *rest, metaclass=Meta, **options):\n    pass\n\n\
                 class Outer:\n    class Base:\n        pass\n\n    class Inner(Base):\n        pass\n\n\
                 class Dyn(make()):\n    pass\n\n\
                 class Twice(Base, Base):\n    pass\n",
        owners: &[
            Owner { qualified_name: "Meta", extends: &[text("type")], implements: &[] },
            Owner {
                qualified_name: "Child",
                extends: &[to("Base", "Base"), text("mod.Base"), text("Generic[T]"), text("*rest")],
                implements: &[],
            },
            Owner { qualified_name: "Outer.Inner", extends: &[to("Base", "Outer.Base")], implements: &[] },
            Owner { qualified_name: "Dyn", extends: &[text("make()")], implements: &[] },
            Owner { qualified_name: "Twice", extends: &[to("Base", "Base")], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::Java,
        support: Support::Supported,
        extension: "java",
        source: "package p;\n\
                 interface Base {}\ninterface Gen<T> {}\nclass Root {}\n\
                 class Child extends Root implements Base, Gen<String>, Gen<Integer> {}\n\
                 class Ext extends other.Root implements p.Base {}\n\
                 interface Sub extends Base, java.io.Serializable {}\n\
                 enum E implements Base { A }\n\
                 record R(int x) implements Base {}\n\
                 class Outer { interface Base {} class In implements Base {} }\n\
                 class Param<Root> extends Root {}\n",
        owners: &[
            Owner {
                qualified_name: "p.Child",
                extends: &[to("Root", "p.Root")],
                implements: &[to("Base", "p.Base"), to("Gen<String>", "p.Gen"), to("Gen<Integer>", "p.Gen")],
            },
            Owner { qualified_name: "p.Ext", extends: &[text("other.Root")], implements: &[to("p.Base", "p.Base")] },
            Owner {
                qualified_name: "p.Sub",
                extends: &[to("Base", "p.Base"), text("java.io.Serializable")],
                implements: &[],
            },
            Owner { qualified_name: "p.E", extends: &[], implements: &[to("Base", "p.Base")] },
            Owner { qualified_name: "p.R", extends: &[], implements: &[to("Base", "p.Base")] },
            Owner { qualified_name: "p.Outer.In", extends: &[], implements: &[to("Base", "p.Outer.Base")] },
            Owner { qualified_name: "p.Param", extends: &[text("Root")], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::CSharp,
        support: Support::Pending("C# base lists mix a base class with interfaces (ruling 1)"),
        extension: "cs",
        source: "interface IBase {}\nclass Root {}\nclass Child : Root, IBase {}\n",
        owners: &[],
    },
    RelationsRow {
        language: L::C,
        support: Support::Absent("C declares no inheritance"),
        extension: "c",
        source: "struct Base { int x; };\nstruct Child { struct Base base; int y; };\ntypedef struct Child Alias;\n",
        owners: &[],
    },
    RelationsRow {
        language: L::Cpp,
        support: Support::Supported,
        extension: "cpp",
        source: "namespace ns { struct B {}; template <class T> struct TB {}; }\n\
                 struct B {};\nstruct Root {};\nusing Alias = Root;\n\
                 struct D : public virtual ns::B, private B, Root, ns::TB<int>, ns::TB<long>, Alias, Root {};\n\
                 template <class T> struct P : T, B {};\n\
                 namespace ns { struct N : B {}; }\n\
                 class E final : other::B {};\n",
        owners: &[
            Owner {
                qualified_name: "D",
                extends: &[
                    to("ns::B", "ns::B"),
                    to("B", "B"),
                    to("Root", "Root"),
                    to("ns::TB<int>", "ns::TB"),
                    to("ns::TB<long>", "ns::TB"),
                    text("Alias"),
                ],
                implements: &[],
            },
            Owner { qualified_name: "P", extends: &[text("T"), to("B", "B")], implements: &[] },
            Owner { qualified_name: "ns::N", extends: &[to("B", "ns::B")], implements: &[] },
            Owner { qualified_name: "E", extends: &[text("other::B")], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::Swift,
        support: Support::Pending("Swift inheritance and conformance lists, extensions included (ruling 1)"),
        extension: "swift",
        source: "protocol P {}\nclass Root {}\nclass Child: Root, P {}\nstruct S: P {}\nextension Root: P {}\n",
        owners: &[],
    },
    RelationsRow {
        language: L::Scala,
        support: Support::Pending("Scala `extends`/`with` lists mix classes and traits (ruling 1)"),
        extension: "scala",
        source: "trait T\nclass Root\nclass Child extends Root with T\n",
        owners: &[],
    },
    RelationsRow {
        language: L::Php,
        support: Support::Supported,
        extension: "php",
        source: "<?php\nnamespace App;\n\
                 interface Base {}\ninterface Other {}\ntrait Shared {}\nclass Root {}\n\
                 class Child extends Root implements Base, \\App\\Other, \\Lib\\Base { use Shared; }\n\
                 interface Sub extends Base, Other {}\n\
                 enum E: string implements Base { case A = 'a'; }\n",
        owners: &[
            Owner {
                qualified_name: "App\\Child",
                extends: &[to("Root", "App\\Root")],
                implements: &[to("Base", "App\\Base"), to("\\App\\Other", "App\\Other"), text("\\Lib\\Base")],
            },
            Owner {
                qualified_name: "App\\Sub",
                extends: &[to("Base", "App\\Base"), to("Other", "App\\Other")],
                implements: &[],
            },
            Owner { qualified_name: "App\\E", extends: &[], implements: &[to("Base", "App\\Base")] },
        ],
    },
    RelationsRow {
        language: L::Perl,
        support: Support::Absent("`@ISA` and `use parent` are not inferred"),
        extension: "pl",
        source: "package Base;\nsub new { return bless {}, shift; }\n\
                 package Child;\nuse parent -norequire, 'Base';\nour @ISA = ('Base');\nsub run { return 1; }\n1;\n",
        owners: &[],
    },
    RelationsRow {
        language: L::Bash,
        support: Support::Absent("Bash declares no types"),
        extension: "sh",
        source: "base() {\n  echo base\n}\nchild() {\n  base\n}\n",
        owners: &[],
    },
    RelationsRow {
        language: L::Zsh,
        support: Support::Absent("Zsh declares no types"),
        extension: "zsh",
        source: "base() {\n  echo base\n}\nchild() {\n  base\n}\n",
        owners: &[],
    },
    RelationsRow {
        language: L::Lua,
        support: Support::Absent("metatables are not inferred"),
        extension: "lua",
        source: "local Base = {}\nBase.__index = Base\nlocal Child = setmetatable({}, { __index = Base })\n\
                 function Child.new() return setmetatable({}, Child) end\nreturn Child\n",
        owners: &[],
    },
];

pub fn relations_matrix_languages() -> Vec<ProgrammingLanguage> {
    RELATIONS_MATRIX.iter().map(|row| row.language).collect()
}

/// One source written for the declaration-fact matrices.
pub struct FactSource {
    pub label: String,
    pub extension: &'static str,
    pub source: String,
}

/// Every source of the generics, modifiers, body-lines (LF and CRLF) and
/// relations matrices, plus the Rust impl source: the forms whose generic
/// parameters, modifiers, attributes, relations and `metrics.lines` the
/// query and cache equivalence tests compare.
pub fn declaration_fact_sources() -> Vec<FactSource> {
    use crate::symbols_generics::GENERICS_MATRIX;
    use crate::symbols_metrics::BODY_LINES_MATRIX;
    use crate::symbols_modifiers::MODIFIERS_MATRIX;

    let mut sources = Vec::new();
    let mut push = |matrix: &str, language: ProgrammingLanguage, extension: &'static str, source: String| {
        sources.push(FactSource { label: format!("{matrix} {}", language.name()), extension, source });
    };
    for row in GENERICS_MATRIX {
        push("generics", row.language, row.extension, row.source.to_string());
    }
    for row in MODIFIERS_MATRIX {
        push("modifiers", row.language, row.extension, row.source.to_string());
    }
    for row in BODY_LINES_MATRIX {
        push("body lines", row.language, row.extension, row.source.to_string());
        push("body lines CRLF", row.language, row.extension, row.source.replace('\n', "\r\n"));
    }
    for row in RELATIONS_MATRIX {
        push("relations", row.language, row.extension, row.source.to_string());
    }
    push("rust impls", ProgrammingLanguage::Rust, "rs", RUST_IMPLS.to_string());
    sources
}

/// Writes each of [`declaration_fact_sources`] into `dir`, one file each.
pub fn write_declaration_fact_sources(dir: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    declaration_fact_sources()
        .into_iter()
        .enumerate()
        .map(|(idx, fact)| {
            let path = dir.join(format!("fact_{idx}.{}", fact.extension));
            std::fs::write(&path, &fact.source).expect("write fact source");
            (fact.label, path)
        })
        .collect()
}

fn write_temp(source: &str, extension: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .expect("create temp source");
    file.write_all(source.as_bytes()).expect("write temp source");
    file
}

fn is_type_like(kind: SymbolKindV2) -> bool {
    matches!(
        kind,
        SymbolKindV2::Class
            | SymbolKindV2::Interface
            | SymbolKindV2::Trait
            | SymbolKindV2::Type
            | SymbolKindV2::Enum
    )
}

/// The one type-like record of `index` with this in-file qualified name.
fn type_record<'a>(index: &'a FileSymbolIndex, label: &str, qualified_name: &str) -> &'a SymbolRecord {
    let found: Vec<&SymbolRecord> = index
        .symbols
        .iter()
        .filter(|record| in_file_name(record) == qualified_name && is_type_like(record.kind))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "{label}: expected one type `{qualified_name}`, found {} among {:?}",
        found.len(),
        index.symbols.iter().map(in_file_name).collect::<Vec<_>>()
    );
    found[0]
}

fn assert_targets(
    complete: &FileSymbolIndex,
    label: &str,
    relation: &str,
    actual: &[SymbolRef],
    expected: &[Target],
) {
    let actual: Vec<(&str, Option<&str>)> = actual
        .iter()
        .map(|target| (target.name.as_str(), target.qualified_name.as_deref()))
        .collect();
    let wanted: Vec<(&str, Option<&str>)> =
        expected.iter().map(|target| (target.name, target.resolves_to)).collect();
    assert_eq!(actual, wanted, "{label}: {relation}");
    for target in expected {
        if let Some(qualified_name) = target.resolves_to {
            let _ = type_record(complete, label, qualified_name);
        }
    }
}

/// Each row's owners report exactly their written bases on the complete
/// index and on a declarations-only query of the owner's kind; a resolved
/// target's ID is the named declaration's. Pending and absent rows report
/// no declared base on any record.
#[test]
fn relations_matrix_holds_on_complete_and_declarations_only_paths() {
    for row in RELATIONS_MATRIX {
        let label = row.language.name();
        let file = write_temp(row.source, row.extension);
        let tree_file = TreeFile::new(file.path()).expect("parse row source");
        assert!(
            tree_file.syntax_diagnostics().is_empty(),
            "{label}: row source must parse cleanly: {:?}",
            tree_file.syntax_diagnostics()
        );
        let complete = tree_file.symbol_index_v2().expect("complete index");
        for owner in row.owners {
            let record = type_record(&complete, label, owner.qualified_name);
            let selective = tree_file
                .query(&SymbolQuery::new().kinds([record.kind]))
                .expect("declarations-only query")
                .index;
            assert!(
                selective.completed_passes.iter().all(|pass| pass.as_str() == "parse"),
                "{label}: declarations only"
            );
            let narrow = type_record(&selective, label, owner.qualified_name);
            for (path, record) in [("complete", record), ("declarations-only", narrow)] {
                let label = format!("{label} {path} `{}`", owner.qualified_name);
                assert_targets(&complete, &label, "extends", &record.relations.extends, owner.extends);
                assert_targets(&complete, &label, "implements", &record.relations.implements, owner.implements);
                for (target, expected) in record.relations.extends.iter().chain(&record.relations.implements).zip(
                    owner.extends.iter().chain(owner.implements),
                ) {
                    let expected_id = expected
                        .resolves_to
                        .map(|qualified_name| type_record(&complete, &label, qualified_name).id.clone());
                    assert_eq!(target.id, expected_id, "{label}: `{}` target ID", target.name);
                }
            }
        }
        if let Support::Pending(reason) | Support::Absent(reason) = row.support {
            for record in &complete.symbols {
                assert!(
                    record.relations.extends.is_empty() && record.relations.implements.is_empty(),
                    "{label}: {reason}: `{}` has {:?} {:?}",
                    record.identity.qualified_name,
                    record.relations.extends,
                    record.relations.implements
                );
            }
        }
    }
}

const RUST_IMPLS: &str = "use std::fmt;\n\
    pub trait Base {}\n\
    pub trait Other {}\n\
    pub trait Conv<T> {}\n\
    pub struct Local;\n\
    pub struct Gen<T>(T);\n\
    pub struct T;\n\
    type Alias = Local;\n\
    mod m { pub struct V; }\n\
    use m::V;\n\
    impl Base for Local {}\n\
    impl !Other for Local {}\n\
    impl Conv<u8> for Local {}\n\
    impl Conv<u16> for Local {}\n\
    impl fmt::Display for Local {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { Ok(()) }\n}\n\
    impl Local {\n    pub fn inherent(&self) {}\n}\n\
    impl<T> Other for Gen<T> {\n    fn generic(&self) {}\n}\n\
    impl Other for &Local {\n    fn by_reference(&self) {}\n}\n\
    impl Other for other::Local {\n    fn external(&self) {}\n}\n\
    impl Other for Alias {\n    fn aliased(&self) {}\n}\n\
    impl Other for V {\n    fn imported(&self) {}\n}\n\
    impl<T> Base for T {\n    fn blanket(&self) {}\n}\n\
    impl Base for crate::m::V {}\n";

fn rust_impl_index(query: &SymbolQuery) -> FileSymbolIndex {
    let file = write_temp(RUST_IMPLS, "rs");
    let tree_file = TreeFile::new(file.path()).expect("parse");
    assert!(tree_file.syntax_diagnostics().is_empty(), "{:?}", tree_file.syntax_diagnostics());
    tree_file.query(query).expect("query").index
}

fn method<'a>(index: &'a FileSymbolIndex, name: &str) -> &'a SymbolRecord {
    let found: Vec<&SymbolRecord> = index
        .symbols
        .iter()
        .filter(|record| record.identity.name == name && record.kind == SymbolKindV2::Method)
        .collect();
    assert_eq!(found.len(), 1, "one method `{name}`");
    found[0]
}

fn names(refs: &[SymbolRef]) -> Vec<(&str, Option<&str>)> {
    refs.iter()
        .map(|target| (target.name.as_str(), target.qualified_name.as_deref()))
        .collect()
}

/// Every positive trait impl, empty ones included, aggregates its trait as
/// written onto the uniquely resolved self type, in file order: distinct
/// instantiations stay apart, a negative impl adds nothing, and a reference,
/// alias, imported, external or type-parameter self type attaches to no
/// local type.
#[test]
fn rust_trait_impls_aggregate_on_the_resolved_self_type() {
    let index = rust_impl_index(&SymbolQuery::complete_index());
    let local = type_record(&index, "rust", "Local");
    assert_eq!(
        names(&local.relations.implements),
        vec![("Base", Some("Base")), ("Conv<u8>", Some("Conv")), ("Conv<u16>", Some("Conv")), ("fmt::Display", None)]
    );
    let base_id = type_record(&index, "rust", "Base").id.clone();
    assert_eq!(local.relations.implements[0].id.as_ref(), Some(&base_id));
    assert_eq!(names(&type_record(&index, "rust", "Gen").relations.implements), vec![("Other", Some("Other"))]);
    assert!(type_record(&index, "rust", "T").relations.implements.is_empty(), "a blanket impl's `T` is its parameter");
    assert_eq!(
        names(&type_record(&index, "rust", "m::V").relations.implements),
        vec![("Base", Some("Base"))],
        "only the rooted `crate::m::V` impl names `m::V`; `use m::V; impl Other for V` needs import resolution"
    );
}

/// Each member of a trait impl keeps the trait as written, also when a
/// query selects methods alone; inherent members have none.
#[test]
fn rust_impl_members_keep_their_trait_on_a_method_only_query() {
    for query in [SymbolQuery::complete_index(), SymbolQuery::new().kinds([SymbolKindV2::Method])] {
        let index = rust_impl_index(&query);
        assert_eq!(names(&method(&index, "fmt").relations.implements), vec![("fmt::Display", None)]);
        assert_eq!(names(&method(&index, "generic").relations.implements), vec![("Other", Some("Other"))]);
        assert_eq!(names(&method(&index, "by_reference").relations.implements), vec![("Other", Some("Other"))]);
        assert!(method(&index, "inherent").relations.implements.is_empty());
        assert_eq!(names(&method(&index, "blanket").relations.implements), vec![("Base", Some("Base"))]);
    }
}

/// Under Containment each impl member belongs to its resolved self type,
/// beside the type's other members; an unresolved self type gives a
/// container named as written, with no ID, no parent and no local
/// membership. Qualified names do not change.
#[test]
fn rust_impl_members_join_the_resolved_self_type_only() {
    let index = rust_impl_index(&SymbolQuery::complete_index());
    let local = type_record(&index, "rust", "Local");
    let gen_type = type_record(&index, "rust", "Gen");
    // Parameters of impl methods belong to the impl's type too; the methods
    // are what this test is about.
    let member_methods: Vec<&str> = local
        .relations
        .members
        .iter()
        .map(|member| member.name.as_str())
        .filter(|name| !matches!(*name, "self" | "f"))
        .collect();
    assert_eq!(member_methods, vec!["fmt", "inherent"]);
    for (name, owner) in [("fmt", local), ("inherent", local), ("generic", gen_type)] {
        let record = method(&index, name);
        for link in [&record.relations.container, &record.relations.parent] {
            assert_eq!(link.as_ref().and_then(|link| link.id.as_ref()), Some(&owner.id), "`{name}` owner");
        }
    }
    for (name, written, qualified_name) in [
        ("by_reference", "&Local", "Local::by_reference"),
        ("external", "other::Local", "other::Local::external"),
        ("aliased", "Alias", "Alias::aliased"),
        ("imported", "V", "V::imported"),
        ("blanket", "T", "T::blanket"),
    ] {
        let record = method(&index, name);
        let container = record.relations.container.as_ref().expect("textual container");
        assert_eq!((container.name.as_str(), container.id.as_ref()), (written, None), "`{name}` container");
        assert!(record.relations.parent.is_none(), "`{name}` has no parent");
        assert_eq!(in_file_name(record), qualified_name);
        for owner in &index.symbols {
            assert!(
                !owner.relations.members.iter().any(|member| member.id.as_ref() == Some(&record.id)),
                "`{name}` is listed by `{}`",
                owner.identity.qualified_name
            );
        }
    }
}

/// The base-list readers and impl ownership read only the declaration's own
/// header: Rust relations need no references, semantics, docs or
/// diagnostics, so a declarations-only query of the trait kind runs the
/// parse pass alone and still resolves supertraits of other modules.
#[test]
fn relations_need_only_the_parse_pass() {
    let index = rust_impl_index(&SymbolQuery::new().kinds([SymbolKindV2::Type]));
    assert!(index.completed_passes.iter().all(|pass| pass.as_str() == "parse"));
    let local = type_record(&index, "rust", "Local");
    assert_eq!(local.relations.implements.len(), 4);
    assert!(local.relations.members.is_empty(), "containment is not computed");
}

/// A generated file whose declarations each write bases or impls: header,
/// then `body(i)` for every `i < n`.
struct RelationShape {
    label: &'static str,
    extension: &'static str,
    header: &'static str,
    body: fn(usize) -> String,
}

const RELATION_SHAPES: &[RelationShape] = &[
    RelationShape {
        label: "Rust modules with a supertrait, a trait impl and an external impl each",
        extension: "rs",
        header: "pub trait Base {}\n",
        body: |i| {
            format!(
                "mod m{i} {{ pub trait T: super::Base + Clone {{}} pub struct S;\n    \
                 impl T for S {{ fn m(&self) {{}} }} impl super::Base for S {{}} impl other::X for S {{}} }}\n"
            )
        },
    },
    RelationShape {
        label: "TypeScript classes extending and implementing",
        extension: "ts",
        header: "interface I {}\nclass Root {}\n",
        body: |i| format!("class C{i} extends Root implements I, lib.J<number> {{}}\n"),
    },
    RelationShape {
        label: "Java nested classes extending an outer type",
        extension: "java",
        header: "interface I {}\n",
        body: |i| format!("class O{i} {{ static class Base {{}} static class C extends Base implements I {{}} }}\n"),
    },
    RelationShape {
        label: "C++ namespaces with a local and an external base",
        extension: "cpp",
        header: "struct B {};\n",
        body: |i| format!("namespace n{i} {{ struct B {{}}; struct C : public B, virtual ext::D {{}}; }}\n"),
    },
    RelationShape {
        label: "Python classes with positional bases",
        extension: "py",
        header: "class Base:\n    pass\n",
        body: |i| format!("class C{i}(Base, mod.Other, metaclass=M):\n    pass\n"),
    },
];

/// Resolving written bases and Rust impls stays linear in the declarations:
/// the declaration index is built once per file and each target is looked
/// up in it, never by rescanning the file per declaration. Results are
/// checked at both sizes so the shapes are known to produce relations.
#[test]
fn relation_resolution_grows_linearly_with_declarations() {
    use crate::symbols_shared_context::{LARGE, SMALL, grows_linearly};

    let dir = tempfile::TempDir::new().expect("temp");
    let mut failures = Vec::new();
    for shape in RELATION_SHAPES {
        let mut measured = Vec::new();
        for declarations in [SMALL, LARGE] {
            let mut source = shape.header.to_string();
            for idx in 0..declarations {
                source.push_str(&(shape.body)(idx));
            }
            let path = dir.path().join(format!("shape_{declarations}.{}", shape.extension));
            std::fs::write(&path, source).expect("write shape");
            let file = TreeFile::new(&path).expect("parse");
            let before = tree_hugger::work_counters();
            let index = file.symbol_index_v2().expect("complete index");
            let work = tree_hugger::work_counters().since(before);
            assert_eq!(work.relation_links, 1, "{}: one relation link", shape.label);
            let resolved = index
                .symbols
                .iter()
                .flat_map(|record| record.relations.extends.iter().chain(&record.relations.implements))
                .filter(|target| target.id.is_some())
                .count();
            assert!(resolved >= declarations, "{}: {resolved} resolved targets for {declarations}", shape.label);
            measured.push((work.shared_context_steps, index.symbols.len()));
        }
        let [(small_steps, _), (large_steps, large_records)] = measured[..] else {
            unreachable!("two sizes")
        };
        if !grows_linearly(small_steps, large_steps, large_records) {
            failures.push(format!(
                "{}: {small_steps} steps for {SMALL} declarations, {large_steps} for {LARGE} ({large_records} records)",
                shape.label
            ));
        }
    }
    assert!(failures.is_empty(), "relation resolution grew faster than the declarations:\n{}", failures.join("\n"));
}
