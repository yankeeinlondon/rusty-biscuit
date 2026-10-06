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

/// Every source of the generics, modifiers, body-lines (LF and CRLF),
/// relations and path-meaning matrices, plus the Rust impl source: the forms whose generic
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
    for row in PATH_MEANING_MATRIX {
        sources.push(FactSource {
            label: format!("path meaning {}: {}", row.language.name(), row.label),
            extension: row.extension,
            source: row.source.to_string(),
        });
    }
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

/// Parses `source` (which must parse cleanly) and checks that each owner
/// reports exactly its written bases on the complete index and on a
/// declarations-only query of the owner's kind, a resolved target's ID being
/// the named declaration's. Returns the complete index.
fn check_owners(label: &str, extension: &str, source: &str, owners: &[Owner]) -> FileSymbolIndex {
    let file = write_temp(source, extension);
    let tree_file = TreeFile::new(file.path()).expect("parse row source");
    assert!(
        tree_file.syntax_diagnostics().is_empty(),
        "{label}: row source must parse cleanly: {:?}",
        tree_file.syntax_diagnostics()
    );
    let complete = tree_file.symbol_index_v2().expect("complete index");
    for owner in owners {
        let record = type_record(&complete, label, owner.qualified_name);
        let selective = tree_file
            .query(&SymbolQuery::new().kinds([record.kind]))
            .expect("declarations-only query")
            .index;
        assert!(
            selective.completed_passes.iter().all(|pass| pass.as_str() == "parse"),
            "{label}: declarations only"
        );
        check_owner_targets(&format!("{label} declarations-only"), &complete, &selective, std::slice::from_ref(owner));
    }
    check_owner_targets(&format!("{label} complete"), &complete, &complete, owners);
    complete
}

/// Checks that each owner in `index` reports exactly its written bases, a
/// resolved target's ID being the named declaration's in `complete`.
pub fn check_owner_targets(label: &str, complete: &FileSymbolIndex, index: &FileSymbolIndex, owners: &[Owner]) {
    for owner in owners {
        let record = type_record(index, label, owner.qualified_name);
        let label = format!("{label} `{}`", owner.qualified_name);
        assert_targets(complete, &label, "extends", &record.relations.extends, owner.extends);
        assert_targets(complete, &label, "implements", &record.relations.implements, owner.implements);
        for (target, expected) in record
            .relations
            .extends
            .iter()
            .chain(&record.relations.implements)
            .zip(owner.extends.iter().chain(owner.implements))
        {
            let expected_id =
                expected.resolves_to.map(|qualified_name| type_record(complete, &label, qualified_name).id.clone());
            assert_eq!(target.id, expected_id, "{label}: `{}` target ID", target.name);
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
        let complete = check_owners(label, row.extension, row.source, row.owners);
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

/// One Rust impl member and what its impl's self type decides for it.
pub struct ImplMember {
    pub method: &'static str,
    /// The self type as written, which names the member's container.
    pub container: &'static str,
    /// The in-file qualified name of the type the self type resolves to.
    pub owner: Option<&'static str>,
    /// The trait the member keeps in `implements` (an inherent impl: none).
    pub implements: Option<Target>,
    /// `identity.container_kind`: the resolved self type's kind, else none
    /// (never the kind of a same-named declaration the path does not name).
    pub container_kind: Option<SymbolKindV2>,
}

/// A written path whose meaning depends on its root, the binding of its
/// first segment, the owners it passes through or the generic parameters
/// in view, in one language. Rows of one language share no source, so a
/// language may have several.
pub struct PathMeaningRow {
    pub label: &'static str,
    pub language: ProgrammingLanguage,
    pub extension: &'static str,
    pub source: &'static str,
    pub owners: &'static [Owner],
    pub impl_members: &'static [ImplMember],
    /// Types (in-file qualified names) an impl of the row must not attach
    /// to: no `implements` and none of the row's impl members.
    pub untouched: &'static [&'static str],
}

/// `probe`, a trait impl member whose self type `T` stays unresolved.
const fn unresolved_probe(implements: Target) -> ImplMember {
    ImplMember { method: "probe", container: "T", owner: None, implements: Some(implements), container_kind: None }
}

const fn bases(qualified_name: &'static str, extends: &'static [Target]) -> Owner {
    Owner { qualified_name, extends, implements: &[] }
}

/// A row of written bases alone.
const fn row(
    label: &'static str,
    language: ProgrammingLanguage,
    extension: &'static str,
    source: &'static str,
    owners: &'static [Owner],
) -> PathMeaningRow {
    PathMeaningRow { label, language, extension, source, owners, impl_members: &[], untouched: &[] }
}

/// A target resolves only when its root, every intermediate owner and the
/// first segment's nearest binding all lead to the declaration: a leading
/// `::` is global in C++ and external in Rust, a PHP relative name stays in
/// its namespace, a parameter, import, nearer namespace or inherited member
/// type decides its first segment, a function's local class is not reachable
/// through the function's name, and an enclosing generic parameter is never
/// a file-level type.
pub const PATH_MEANING_MATRIX: &[PathMeaningRow] = &[
    row(
        "Rust supertrait through an external `::` path",
        L::Rust,
        "rs",
        "mod a { pub trait Dup {} }\npub trait ReviewAbsolute: ::a::Dup {}\n",
        &[bases("ReviewAbsolute", &[text("::a::Dup")])],
    ),
    PathMeaningRow {
        label: "Rust impl of an external `::` self type",
        language: L::Rust,
        extension: "rs",
        source: "mod ext { pub struct B; }\nimpl ::ext::B { fn probe(&self) {} }\n\
                 impl crate::ext::B { fn rooted(&self) {} }\n",
        owners: &[],
        impl_members: &[
            ImplMember { method: "probe", container: "::ext::B", owner: None, implements: None, container_kind: None },
            ImplMember {
                method: "rooted",
                container: "ext::B",
                owner: Some("ext::B"),
                implements: None,
                container_kind: Some(SymbolKindV2::Type),
            },
        ],
        untouched: &[],
    },
    PathMeaningRow {
        label: "Rust impl of an enclosing function's generic parameter",
        language: L::Rust,
        extension: "rs",
        source: "pub trait Base {}\npub struct T;\nfn outer<T>() { impl Base for T { fn probe(&self) {} } }\n",
        owners: &[],
        impl_members: &[unresolved_probe(to("Base", "Base"))],
        untouched: &["T"],
    },
    PathMeaningRow {
        label: "Rust impl of an enclosing impl's generic parameter",
        language: L::Rust,
        extension: "rs",
        source: "pub trait Base {}\npub struct T;\npub struct Wrap<X>(X);\n\
                 impl<T> Wrap<T> { fn outer() { impl Base for T { fn probe(&self) {} } } }\n",
        owners: &[],
        impl_members: &[unresolved_probe(to("Base", "Base"))],
        untouched: &["T"],
    },
    PathMeaningRow {
        label: "Rust impl of a name imported in its block",
        language: L::Rust,
        extension: "rs",
        source: "pub trait Base {}\npub struct T;\n\
                 fn outer() { use other::T; impl Base for T { fn probe(&self) {} } }\n",
        owners: &[],
        impl_members: &[unresolved_probe(to("Base", "Base"))],
        untouched: &["T"],
    },
    PathMeaningRow {
        label: "Rust impl under a glob import in its block",
        language: L::Rust,
        extension: "rs",
        source: "pub trait Base {}\npub struct T;\n\
                 fn outer() { use other::*; impl Base for T { fn probe(&self) {} } }\n",
        owners: &[],
        // The glob may supply the trait as well as the self type.
        impl_members: &[unresolved_probe(text("Base"))],
        untouched: &["T"],
    },
    row(
        "Rust qualified import control",
        L::Rust,
        "rs",
        "mod inner { use other::ns; pub trait Child: ns::Base {} }\nmod ns { pub trait Base {} }\n",
        &[bases("inner::Child", &[text("ns::Base")])],
    ),
    row(
        "Rust explicit local root control",
        L::Rust,
        "rs",
        "mod ext { pub trait Base {} }\npub trait Child: crate::ext::Base {}\n",
        &[bases("Child", &[to("crate::ext::Base", "ext::Base")])],
    ),
    row(
        "TypeScript parameter shadowing a namespace",
        L::TypeScript,
        "ts",
        "namespace ns { export class Inner {} }\nfunction f(ns: any) { class Child extends ns.Inner {} }\n\
         class Control extends ns.Inner {}\n",
        &[bases("f.Child", &[text("ns.Inner")]), bases("Control", &[to("ns.Inner", "ns.Inner")])],
    ),
    row(
        "TypeScript import alias in a namespace",
        L::TypeScript,
        "ts",
        "class Base {}\nnamespace q { import Base = lib.Base; class Child extends Base {} }\n",
        &[bases("q.Child", &[text("Base")])],
    ),
    row(
        "TypeScript class body is no lexical scope",
        L::TypeScript,
        "ts",
        "class Base {}\nclass Outer { Base = 1; m() { class Child extends Base {} } }\n",
        &[bases("Outer.m.Child", &[to("Base", "Base")])],
    ),
    row(
        "JavaScript class local to a function",
        L::JavaScript,
        "js",
        "function outer() { class Local {} }\nclass Child extends outer.Local {}\n",
        &[bases("Child", &[text("outer.Local")])],
    ),
    row(
        "Python parameter shadowing a class",
        L::Python,
        "py",
        "class Outer:\n    class Base:\n        pass\n\n\
         def f(Outer):\n    class Child(Outer.Base):\n        pass\n\n\
         class Control(Outer.Base):\n    pass\n",
        &[bases("f.Child", &[text("Outer.Base")]), bases("Control", &[to("Outer.Base", "Outer.Base")])],
    ),
    row(
        "Python class local to a function",
        L::Python,
        "py",
        "def outer():\n    class Local:\n        pass\n\nclass Child(outer.Local):\n    pass\n",
        &[bases("Child", &[text("outer.Local")])],
    ),
    row(
        "Python class scope seen from a method",
        L::Python,
        "py",
        "class Base:\n    pass\n\nclass Outer:\n    class Base:\n        pass\n\n\
         \x20   class Direct(Base):\n        pass\n\n\
         \x20   def m(self):\n        class FromMethod(Base):\n            pass\n",
        &[bases("Outer.Direct", &[to("Base", "Outer.Base")]), bases("Outer.m.FromMethod", &[to("Base", "Base")])],
    ),
    row(
        "Python import in a function",
        L::Python,
        "py",
        "class Base:\n    pass\n\ndef g():\n    from lib import Base\n    class Child(Base):\n        pass\n",
        &[bases("g.Child", &[text("Base")])],
    ),
    row(
        "Python nested class skips an enclosing class's class",
        L::Python,
        "py",
        "class Base:\n    pass\n\nclass Outer:\n    class Base:\n        pass\n\n\
         \x20   class Middle:\n        class Leaf(Base):\n            pass\n",
        &[bases("Outer.Middle.Leaf", &[to("Base", "Base")])],
    ),
    row(
        "Python nested class skips an enclosing class's qualified path",
        L::Python,
        "py",
        "class NS:\n    class B:\n        pass\n\nclass Outer:\n    class NS:\n        class B:\n            pass\n\n\
         \x20   class Middle:\n        class Leaf(NS.B):\n            pass\n",
        &[bases("Outer.Middle.Leaf", &[to("NS.B", "NS.B")])],
    ),
    row(
        "Python nested class skips an enclosing class's variable",
        L::Python,
        "py",
        "class Base:\n    pass\n\nclass Outer:\n    Base = object\n\n\
         \x20   class Middle:\n        class Leaf(Base):\n            pass\n",
        &[bases("Outer.Middle.Leaf", &[to("Base", "Base")])],
    ),
    row(
        "Python nested class skips an enclosing class's import",
        L::Python,
        "py",
        "class Base:\n    pass\n\nclass Outer:\n    from builtins import object as Base\n\n\
         \x20   class Middle:\n        class Leaf(Base):\n            pass\n",
        &[bases("Outer.Middle.Leaf", &[to("Base", "Base")])],
    ),
    row(
        "Python nested class with only an enclosing class's binding",
        L::Python,
        "py",
        "class Outer:\n    class Base:\n        pass\n\n\
         \x20   class Middle:\n        class Leaf(Base):\n            pass\n",
        &[bases("Outer.Middle.Leaf", &[text("Base")])],
    ),
    row(
        "Python nested class inside a function",
        L::Python,
        "py",
        "def factory():\n    class Base:\n        pass\n\n\
         \x20   class Outer:\n        class Base:\n            pass\n\n\
         \x20       class Middle:\n            class Leaf(Base):\n                pass\n",
        &[bases("factory.Outer.Middle.Leaf", &[to("Base", "factory.Base")])],
    ),
    row(
        "Python immediate class body control",
        L::Python,
        "py",
        "class Base:\n    pass\n\nclass Outer:\n    class Base:\n        pass\n\n\
         \x20   class Middle:\n        class Base:\n            pass\n\n\
         \x20       class Leaf(Base):\n            pass\n",
        &[bases("Outer.Middle.Leaf", &[to("Base", "Outer.Middle.Base")])],
    ),
    row(
        "Python direct nested header control",
        L::Python,
        "py",
        "class Base:\n    pass\n\nclass Outer:\n    class Base:\n        pass\n\n\
         \x20   class Leaf(Base):\n        pass\n",
        &[bases("Outer.Leaf", &[to("Base", "Outer.Base")])],
    ),
    row(
        "Python method body control",
        L::Python,
        "py",
        "class Base:\n    pass\n\nclass Outer:\n    class Base:\n        pass\n\n\
         \x20   def m(self):\n        class Leaf(Base):\n            pass\n",
        &[bases("Outer.m.Leaf", &[to("Base", "Base")])],
    ),
    row(
        "Python enclosing class's type parameter in a nested class",
        L::Python,
        "py",
        "class T:\n    pass\n\nclass Outer[T]:\n    class Middle:\n        class Leaf(T):\n            pass\n",
        &[bases("Outer.Middle.Leaf", &[text("T")])],
    ),
    row(
        "C++ global `::` anchor",
        L::Cpp,
        "cpp",
        "struct B {};\nnamespace ns { struct B {}; struct Child : ::B {}; }\n",
        &[bases("ns::Child", &[to("::B", "B")])],
    ),
    row(
        "C++ intermediate namespace shadowing",
        L::Cpp,
        "cpp",
        "namespace ns { struct B {}; }\nnamespace inner { namespace ns {} struct Child : ns::B {}; }\n",
        &[bases("inner::Child", &[text("ns::B")])],
    ),
    row(
        "C++ global anchor control",
        L::Cpp,
        "cpp",
        "struct B {};\nnamespace ns { struct Child : ::B {}; }\n",
        &[bases("ns::Child", &[to("::B", "B")])],
    ),
    row(
        "C++ using-declaration",
        L::Cpp,
        "cpp",
        "struct B {};\nnamespace inner { using other::B; struct Child : B {}; }\n",
        &[bases("inner::Child", &[text("B")])],
    ),
    row(
        "C++ using-directive",
        L::Cpp,
        "cpp",
        "struct B {};\nnamespace inner { using namespace other; struct Child : B {}; }\n",
        &[bases("inner::Child", &[text("B")])],
    ),
    row(
        "C++ namespace alias",
        L::Cpp,
        "cpp",
        "namespace ns { struct B {}; }\nnamespace inner { namespace ns = other; struct Child : ns::B {}; }\n",
        &[bases("inner::Child", &[text("ns::B")])],
    ),
    row(
        "C++ inherited member type",
        L::Cpp,
        "cpp",
        "struct B {};\nstruct Sup { struct B {}; };\nstruct Der : Sup { struct In : B {}; };\n\
         struct Plain { struct In : B {}; };\n",
        &[bases("Der::In", &[text("B")]), bases("Plain::In", &[to("B", "B")])],
    ),
    row(
        "PHP relative qualified name",
        L::Php,
        "php",
        "<?php\nnamespace App;\nclass Root {}\nnamespace Other;\nclass Child extends App\\Root {}\n",
        &[bases("Other\\Child", &[text("App\\Root")])],
    ),
    row(
        "PHP import alias",
        L::Php,
        "php",
        "<?php\nnamespace ns;\nclass B {}\nnamespace Other;\nuse other\\ns;\nclass Child extends ns\\B {}\n",
        &[bases("Other\\Child", &[text("ns\\B")])],
    ),
    row(
        "PHP import in the namespace of a local match",
        L::Php,
        "php",
        "<?php\nnamespace App\\ns;\nclass B {}\nnamespace App;\nuse Lib\\ns;\nclass Child extends ns\\B {}\n",
        &[bases("App\\Child", &[text("ns\\B")])],
    ),
    row(
        "PHP explicit root control",
        L::Php,
        "php",
        "<?php\nnamespace ns;\nclass B {}\nnamespace Other;\nclass Child extends \\ns\\B {}\n",
        &[bases("Other\\Child", &[to("\\ns\\B", "ns\\B")])],
    ),
    row(
        "Java generic shadowing control",
        L::Java,
        "java",
        "class Root {}\nclass Outer<Root> { class Child extends Root {} }\n",
        &[bases("Outer.Child", &[text("Root")])],
    ),
    row(
        "Java inherited member type",
        L::Java,
        "java",
        "class Base {}\nclass A { static class Base {} }\nclass Sub extends A { class Child extends Base {} }\n\
         class Plain { class Child extends Base {} }\n",
        &[bases("Sub.Child", &[text("Base")]), bases("Plain.Child", &[to("Base", "Base")])],
    ),
    row(
        "Go generic shadowing control",
        L::Go,
        "go",
        "package shapes\n\ntype Base interface{ M() }\n\ntype Iface[Base any] interface {\n\tBase\n}\n",
        &[bases("shapes.Iface", &[text("Base")])],
    ),
];

/// Every [`PATH_MEANING_MATRIX`] row's owners hold on the complete index and
/// on a declarations-only query of the owner's kind.
#[test]
fn path_meaning_matrix_holds_on_complete_and_declarations_only_paths() {
    let failed = failing_rows(PATH_MEANING_MATRIX.iter(), |row| {
        let label = format!("{}: {}", row.language.name(), row.label);
        check_owners(&label, row.extension, row.source, row.owners);
    });
    assert!(failed.is_empty(), "rows failed (messages above): {failed:#?}");
}

/// The labels of the rows `check` panics on; every row runs.
fn failing_rows<'a>(
    rows: impl Iterator<Item = &'a PathMeaningRow>,
    check: impl Fn(&PathMeaningRow),
) -> Vec<&'static str> {
    rows.filter(|row| std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(row))).is_err())
        .map(|row| row.label)
        .collect()
}

/// Checks the row's impl members and untouched types in `index`, which was
/// produced by a query asking for `containment` or not.
pub fn check_impl_facts(
    label: &str,
    row: &PathMeaningRow,
    complete: &FileSymbolIndex,
    index: &FileSymbolIndex,
    containment: bool,
) {
    for member in row.impl_members {
        let record = method(index, member.method);
        let label = format!("{label} `{}`", member.method);
        let implements: Vec<(&str, Option<&str>)> =
            member.implements.iter().map(|target| (target.name, target.resolves_to)).collect();
        assert_eq!(names(&record.relations.implements), implements, "{label}: implements");
        assert_eq!(record.identity.container_kind, member.container_kind, "{label}: container kind");
        if !containment {
            continue;
        }
        let owner_id = member.owner.map(|owner| type_record(complete, &label, owner).id.clone());
        let container = record.relations.container.as_ref().expect("an impl member has a container");
        assert_eq!(container.id, owner_id, "{label}: container ID");
        if owner_id.is_none() {
            assert_eq!(container.name, member.container, "{label}: textual container");
        }
        assert_eq!(record.relations.parent.as_ref().and_then(|parent| parent.id.clone()), owner_id, "{label}: parent");
        for other in &index.symbols {
            let lists = other.relations.members.iter().any(|listed| listed.id.as_ref() == Some(&record.id));
            let owns = Some(&other.id) == owner_id.as_ref();
            assert_eq!(lists, owns, "{label}: listed by `{}`", other.identity.qualified_name);
        }
    }
    for untouched in row.untouched {
        if let Some(record) = index.symbols.iter().find(|record| in_file_name(record) == *untouched) {
            assert!(record.relations.implements.is_empty(), "{label}: `{untouched}` implements nothing");
            assert!(record.relations.members.is_empty(), "{label}: `{untouched}` has no impl members");
        }
    }
}

/// An impl row's members keep their trait on the complete index, on a
/// method-only query and on a method-only query with Containment, which
/// links each member to its resolved self type only; a self type that does
/// not resolve leaves a textual container, no parent, no membership and no
/// container kind (in the older model too), and the unrelated same-named
/// type gains nothing.
#[test]
fn path_meaning_impl_rows_hold_on_complete_and_method_only_queries() {
    let impl_rows = || PATH_MEANING_MATRIX.iter().filter(|row| !row.impl_members.is_empty());
    let failed = failing_rows(impl_rows(), |row| {
        let file = write_temp(row.source, row.extension);
        let tree_file = TreeFile::new(file.path()).expect("parse");
        assert!(tree_file.syntax_diagnostics().is_empty(), "{}: {:?}", row.label, tree_file.syntax_diagnostics());
        let complete = tree_file.symbol_index_v2().expect("complete index");
        let methods = SymbolQuery::new().kinds([SymbolKindV2::Method]);
        let contained = methods.clone().capabilities([tree_hugger::Capability::Containment]);
        for (path, query, containment) in [
            ("complete", None, true),
            ("method-only", Some(methods), false),
            ("method-only with containment", Some(contained), true),
        ] {
            let index = match query {
                Some(query) => tree_file.query(&query).expect("query").index,
                None => complete.clone(),
            };
            check_impl_facts(&format!("{} ({path})", row.label), row, &complete, &index, containment);
        }
        // The older model reads the same decision.
        let symbols = tree_file.symbols().expect("symbols");
        for member in row.impl_members {
            let found: Vec<_> = symbols.iter().filter(|symbol| symbol.name == member.method).collect();
            assert_eq!(found.len(), 1, "{}: one `{}` symbol", row.label, member.method);
            assert_eq!(
                found[0].container_kind.map(SymbolKindV2::from),
                member.container_kind,
                "{}: `{}` older-model container kind",
                row.label,
                member.method
            );
        }
    });
    assert!(failed.is_empty(), "rows failed (messages above): {failed:#?}");
    assert!(impl_rows().count() >= 5, "the impl rows ran");
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
