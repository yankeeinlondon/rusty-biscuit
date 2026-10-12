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
/// qualified name of the declaration it names, plus that declaration's line
/// when several declarations share the name.
#[derive(Clone, Copy)]
pub struct Target {
    pub name: &'static str,
    pub resolves_to: Option<&'static str>,
    pub line: Option<u32>,
}

const fn to(name: &'static str, resolves_to: &'static str) -> Target {
    Target { name, resolves_to: Some(resolves_to), line: None }
}

/// A target among same-named declarations: the one declared on `line`.
const fn to_at(name: &'static str, resolves_to: &'static str, line: u32) -> Target {
    Target { name, resolves_to: Some(resolves_to), line: Some(line) }
}

const fn text(name: &'static str) -> Target {
    Target { name, resolves_to: None, line: None }
}

/// An owner (by in-file qualified name) and its relations.
pub struct Owner {
    pub qualified_name: &'static str,
    pub extends: &'static [Target],
    pub implements: &'static [Target],
    pub unclassified: &'static [Target],
}

pub enum Support {
    Supported,
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
            Owner { unclassified: &[],
                qualified_name: "Child",
                extends: &[to("Base", "Base"), text("other::Base"), text("Clone"), text("for<'x> Fn(&'x u8)")],
                implements: &[],
            },
            Owner { unclassified: &[], qualified_name: "Sub", extends: &[to("Child", "Child"), text("Amb")], implements: &[] },
            Owner { unclassified: &[],
                qualified_name: "a::UsesDup",
                extends: &[to("Dup", "a::Dup"), to("super::Base", "Base"), to("crate::Base", "Base")],
                implements: &[],
            },
            // A Rust item does not see its parent module's names.
            Owner { unclassified: &[], qualified_name: "b::UsesOuter", extends: &[text("Base")], implements: &[] },
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
            Owner { unclassified: &[],
                qualified_name: "Child",
                extends: &[to("Root", "Root")],
                implements: &[to("Base", "Base"), to("Mixin<string>", "Mixin"), to("Mixin<number>", "Mixin")],
            },
            Owner { unclassified: &[],
                qualified_name: "Q",
                extends: &[to("ns.Inner", "ns.Inner")],
                implements: &[to("ns.Dup", "ns.Dup"), to("Dup", "Dup")],
            },
            Owner { unclassified: &[], qualified_name: "Ext", extends: &[text("other.Root")], implements: &[text("Alias")] },
            Owner { unclassified: &[], qualified_name: "Dyn", extends: &[text("mix(Root)")], implements: &[] },
            Owner { unclassified: &[], qualified_name: "Gen", extends: &[text("T<string>")], implements: &[] },
            Owner { unclassified: &[],
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
            Owner { unclassified: &[], qualified_name: "Child", extends: &[to("Root", "Root")], implements: &[] },
            Owner { unclassified: &[], qualified_name: "Dyn", extends: &[text("mix(Root)")], implements: &[] },
            Owner { unclassified: &[], qualified_name: "Ext", extends: &[text("lib.Root")], implements: &[] },
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
            Owner { unclassified: &[],
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
            Owner { unclassified: &[], qualified_name: "shapes.S", extends: &[], implements: &[] },
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
            Owner { unclassified: &[], qualified_name: "Meta", extends: &[text("type")], implements: &[] },
            Owner { unclassified: &[],
                qualified_name: "Child",
                extends: &[to("Base", "Base"), text("mod.Base"), text("Generic[T]"), text("*rest")],
                implements: &[],
            },
            Owner { unclassified: &[], qualified_name: "Outer.Inner", extends: &[to("Base", "Outer.Base")], implements: &[] },
            Owner { unclassified: &[], qualified_name: "Dyn", extends: &[text("make()")], implements: &[] },
            Owner { unclassified: &[], qualified_name: "Twice", extends: &[to("Base", "Base")], implements: &[] },
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
            Owner { unclassified: &[],
                qualified_name: "p.Child",
                extends: &[to("Root", "p.Root")],
                implements: &[to("Base", "p.Base"), to("Gen<String>", "p.Gen"), to("Gen<Integer>", "p.Gen")],
            },
            Owner { unclassified: &[], qualified_name: "p.Ext", extends: &[text("other.Root")], implements: &[to("p.Base", "p.Base")] },
            Owner { unclassified: &[],
                qualified_name: "p.Sub",
                extends: &[to("Base", "p.Base"), text("java.io.Serializable")],
                implements: &[],
            },
            Owner { unclassified: &[], qualified_name: "p.E", extends: &[], implements: &[to("Base", "p.Base")] },
            Owner { unclassified: &[], qualified_name: "p.R", extends: &[], implements: &[to("Base", "p.Base")] },
            Owner { unclassified: &[], qualified_name: "p.Outer.In", extends: &[], implements: &[to("Base", "p.Outer.Base")] },
            Owner { unclassified: &[], qualified_name: "p.Param", extends: &[text("Root")], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::CSharp,
        support: Support::Supported,
        extension: "cs",
        source: "interface I<T> {}\ninterface J : I<int>, External {}\nclass Root<T> {}\n\
                 class Child : Root<int>, I<int>, External {}\nclass Unknown : Foreign {}\n\
                 struct Value : External {}\nrecord struct Rec : External;\nenum E : int { A }\n\
                 class T {}\nclass Generic<T> : T {}\nclass Duplicate {}\nclass Duplicate {}\nclass Amb : Duplicate {}\n",
        owners: &[
            Owner { qualified_name: "J", extends: &[to("I<int>", "I"), text("External")], implements: &[], unclassified: &[] },
            Owner { qualified_name: "Child", extends: &[to("Root<int>", "Root")], implements: &[to("I<int>", "I"), text("External")], unclassified: &[] },
            Owner { qualified_name: "Unknown", extends: &[], implements: &[], unclassified: &[text("Foreign")] },
            Owner { qualified_name: "Value", extends: &[], implements: &[text("External")], unclassified: &[] },
            Owner { qualified_name: "Rec", extends: &[], implements: &[text("External")], unclassified: &[] },
            Owner { qualified_name: "E", extends: &[], implements: &[], unclassified: &[] },
            Owner { qualified_name: "Generic", extends: &[], implements: &[], unclassified: &[text("T")] },
            Owner { qualified_name: "Amb", extends: &[], implements: &[], unclassified: &[text("Duplicate")] },
        ],
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
            Owner { unclassified: &[],
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
            Owner { unclassified: &[], qualified_name: "P", extends: &[text("T"), to("B", "B")], implements: &[] },
            Owner { unclassified: &[], qualified_name: "ns::N", extends: &[to("B", "ns::B")], implements: &[] },
            Owner { unclassified: &[], qualified_name: "E", extends: &[text("other::B")], implements: &[] },
        ],
    },
    RelationsRow {
        language: L::Swift,
        support: Support::Supported,
        extension: "swift",
        source: "protocol P {}\nprotocol Q: P, External {}\nclass Root {}\nclass Child: Root, P, External {}\n\
                 struct S: P, Codable {}\nclass Unknown: Foreign {}\nenum E: Int, P { case a = 1 }\n\
                 enum F: P { case a }\nstruct G<T> {}\nextension Root: P {}\nextension G: Equatable where T: Equatable {}\n\
                 extension other.Root: Q {}\nextension Root: P, Codable {}\n",
        owners: &[
            Owner { qualified_name: "Q", extends: &[to("P", "P"), text("External")], implements: &[], unclassified: &[] },
            Owner { qualified_name: "Child", extends: &[to("Root", "Root")], implements: &[to("P", "P"), text("External")], unclassified: &[] },
            Owner { qualified_name: "S", extends: &[], implements: &[to("P", "P"), text("Codable")], unclassified: &[] },
            Owner { qualified_name: "Unknown", extends: &[], implements: &[], unclassified: &[text("Foreign")] },
            Owner { qualified_name: "E", extends: &[], implements: &[to("P", "P")], unclassified: &[] },
            Owner { qualified_name: "F", extends: &[], implements: &[to("P", "P")], unclassified: &[] },
            Owner { qualified_name: "Root", extends: &[], implements: &[to("P", "P"), text("Codable")], unclassified: &[] },
            Owner { qualified_name: "G", extends: &[], implements: &[text("Equatable")], unclassified: &[] },
        ],
    },
    RelationsRow {
        language: L::Scala,
        support: Support::Supported,
        extension: "scala",
        source: "package sample\ntrait T\nclass Root[A]\nclass Child extends Root[Int] with T with External\n\
                 class Unknown extends Foreign with T\ntrait Sub extends T with External\nclass Mix extends T\n\
                 object Singleton extends Root[Int] with T\n",
        owners: &[
            Owner { qualified_name: "sample.Child", extends: &[to("Root[Int]", "sample.Root")], implements: &[to("T", "sample.T"), text("External")], unclassified: &[] },
            Owner { qualified_name: "sample.Unknown", extends: &[], implements: &[to("T", "sample.T")], unclassified: &[text("Foreign")] },
            Owner { qualified_name: "sample.Sub", extends: &[to("T", "sample.T"), text("External")], implements: &[], unclassified: &[] },
            Owner { qualified_name: "sample.Mix", extends: &[], implements: &[to("T", "sample.T")], unclassified: &[] },
            Owner { qualified_name: "sample.Singleton", extends: &[to("Root[Int]", "sample.Root")], implements: &[to("T", "sample.T")], unclassified: &[] },
        ],
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
            Owner { unclassified: &[],
                qualified_name: "App\\Child",
                extends: &[to("Root", "App\\Root")],
                implements: &[to("Base", "App\\Base"), to("\\App\\Other", "App\\Other"), text("\\Lib\\Base")],
            },
            Owner { unclassified: &[],
                qualified_name: "App\\Sub",
                extends: &[to("Base", "App\\Base"), to("Other", "App\\Other")],
                implements: &[],
            },
            Owner { unclassified: &[], qualified_name: "App\\E", extends: &[], implements: &[to("Base", "App\\Base")] },
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
    /// The corpus group ([`write_declaration_fact_group`]) whose tests check
    /// this source.
    pub group: usize,
}

/// Rust's primary matrix rows and impl source exercise every declaration
/// fact, so the consumers' non-vacuity checks belong to this group.
pub const RUST_FACT_GROUP: usize = 0;

/// Language-specific groups of owner-identity and path-meaning sources;
/// round-robin assignment within a language mixes their matrices.
pub const SHARED_FACT_GROUPS: usize = 44;

const LANGUAGE_FACT_GROUPS: usize = 16;

pub const DECLARATION_FACT_GROUP_COUNT: usize = LANGUAGE_FACT_GROUPS + SHARED_FACT_GROUPS;

/// The most sources one group may hold. Each consumer runs one nextest test
/// per group, so this bounds source work as the matrices grow:
/// when a new row overflows a group, raise [`SHARED_FACT_GROUPS`] and stamp
/// the new group in [`declaration_fact_group_tests`]. Query compilation is
/// isolated by language for the primary matrices.
pub const MAX_FACT_GROUP_SOURCES: usize = 20;

/// Stamps one `#[test]` per declaration-fact group, each calling the
/// sibling function `$check` with its group, inside a module named
/// `$consumer`; `GROUPS` lists the stamped groups for
/// `declaration_fact_groups_partition_the_corpus`. The `shared` form stamps
/// only the shared groups, for consumers of [`path_meaning_group`].
macro_rules! declaration_fact_group_tests {
    ($consumer:ident => $check:ident) => {
        $crate::symbols_relations::declaration_fact_group_tests!(@stamp $consumer, $check, [
            rust_matrices 0,
            typescript_matrices 1,
            javascript_matrices 2,
            go_matrices 3,
            python_matrices 4,
            java_matrices 5,
            csharp_matrices 6,
            c_matrices 7,
            cpp_matrices 8,
            swift_matrices 9,
            scala_matrices 10,
            php_matrices 11,
            perl_matrices 12,
            bash_matrices 13,
            zsh_matrices 14,
            lua_matrices 15,
        ] shared);
    };
    (shared $consumer:ident => $check:ident) => {
        $crate::symbols_relations::declaration_fact_group_tests!(@stamp $consumer, $check, [] shared);
    };
    (@stamp $consumer:ident, $check:ident, [$($first:ident $first_group:literal,)*] shared) => {
        $crate::symbols_relations::declaration_fact_group_tests!(@stamp $consumer, $check, [
            $($first $first_group,)*
            cpp_owners_1 16,
            cpp_owners_2 17,
            csharp_owners_1 18,
            csharp_owners_2 19,
            csharp_owners_3 20,
            csharp_owners_4 21,
            csharp_owners_5 22,
            csharp_owners_6 23,
            csharp_owners_7 24,
            csharp_owners_8 25,
            csharp_owners_9 26,
            csharp_owners_10 27,
            csharp_owners_11 28,
            go_owners_1 29,
            java_owners_1 30,
            javascript_owners_1 31,
            php_owners_1 32,
            python_owners_1 33,
            python_owners_2 34,
            python_owners_3 35,
            python_owners_4 36,
            python_owners_5 37,
            python_owners_6 38,
            python_owners_7 39,
            python_owners_8 40,
            python_owners_9 41,
            rust_owners_1 42,
            rust_owners_2 43,
            rust_owners_3 44,
            scala_owners_1 45,
            scala_owners_2 46,
            scala_owners_3 47,
            scala_owners_4 48,
            swift_owners_1 49,
            swift_owners_2 50,
            swift_owners_3 51,
            swift_owners_4 52,
            swift_owners_5 53,
            swift_owners_6 54,
            swift_owners_7 55,
            swift_owners_8 56,
            swift_owners_9 57,
            typescript_owners_1 58,
            typescript_owners_2 59,
        ]);
    };
    (@stamp $consumer:ident, $check:ident, [$($name:ident $group:literal),+ $(,)?]) => {
        pub(crate) mod $consumer {
            pub const GROUPS: &[usize] = &[$($group),+];
            $(
                #[test]
                fn $name() {
                    super::$check($group);
                }
            )+
        }
    };
}
pub(crate) use declaration_fact_group_tests;

/// Every source of the generics, modifiers, body-lines (LF and CRLF),
/// relations, path-meaning and Swift nested-shadow and unproved owner
/// matrices, plus the
/// Rust impl source: the forms whose generic
/// parameters, modifiers, attributes, relations and `metrics.lines` the
/// query and cache equivalence tests compare.
pub fn declaration_fact_sources() -> Vec<FactSource> {
    use crate::symbols_generics::GENERICS_MATRIX;
    use crate::symbols_metrics::BODY_LINES_MATRIX;
    use crate::symbols_modifiers::MODIFIERS_MATRIX;

    let mut sources = Vec::new();
    let mut push = |matrix: &str, language: ProgrammingLanguage, extension: &'static str, source: String| {
        sources.push(FactSource {
            label: format!("{matrix} {}", language.name()),
            extension,
            source,
            group: language_fact_group(language),
        });
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

    let mut shared = Vec::new();
    // Path-meaning rows must stay last: `path_meaning_group` offsets past
    // the owner-identity rows.
    for row in crate::symbols_owner_identity::NESTED_SHADOW_MATRIX {
        shared.push((format!("owner identity Swift: {}", row.label), "swift", row.source));
    }
    for row in crate::symbols_owner_identity::UNPROVED_OWNER_MATRIX {
        shared.push((format!("owner identity Swift unproved: {}", row.label), "swift", row.source));
    }
    for row in path_meaning_rows() {
        shared.push((format!("path meaning {}: {}", row.language.name(), row.label), row.extension, row.source));
    }
    sources.extend(shared.into_iter().enumerate().map(|(idx, (label, extension, source))| FactSource {
        label,
        extension,
        source: source.to_string(),
        group: shared_fact_group(idx),
    }));
    sources
}

fn language_fact_group(language: ProgrammingLanguage) -> usize {
    let languages = [L::Rust, L::TypeScript, L::JavaScript, L::Go, L::Python, L::Java, L::CSharp, L::C, L::Cpp, L::Swift, L::Scala, L::Php, L::Perl, L::Bash, L::Zsh, L::Lua];
    languages.iter().position(|candidate| *candidate == language).expect("registered language")
}

const SHARED_GROUP_LAYOUT: &[(ProgrammingLanguage, usize)] = &[
    (L::Cpp, 2),
    (L::CSharp, 11),
    (L::Go, 1),
    (L::Java, 1),
    (L::JavaScript, 1),
    (L::Php, 1),
    (L::Python, 9),
    (L::Rust, 3),
    (L::Scala, 4),
    (L::Swift, 9),
    (L::TypeScript, 2),
];

static SHARED_SOURCE_GROUPS: std::sync::LazyLock<Vec<usize>> = std::sync::LazyLock::new(|| {
    let owners = crate::symbols_owner_identity::NESTED_SHADOW_MATRIX.len()
        + crate::symbols_owner_identity::UNPROVED_OWNER_MATRIX.len();
    let mut positions = std::collections::HashMap::<ProgrammingLanguage, usize>::new();
    std::iter::repeat_n(L::Swift, owners)
        .chain(path_meaning_rows().map(|row| row.language))
        .map(|language| {
            let mut start = LANGUAGE_FACT_GROUPS;
            for &(candidate, shards) in SHARED_GROUP_LAYOUT {
                if candidate == language {
                    let ordinal = positions.entry(language).or_default();
                    let group = start + *ordinal % shards;
                    *ordinal += 1;
                    return group;
                }
                start += shards;
            }
            panic!("{language}: no shared fact group");
        })
        .collect()
});

/// The language-specific group of the `idx`-th shared source.
fn shared_fact_group(idx: usize) -> usize {
    SHARED_SOURCE_GROUPS[idx]
}

/// The [`path_meaning_rows`] whose declaration-fact sources are in `group`,
/// for the path-meaning consumers that check rows rather than sources.
pub fn path_meaning_group(group: usize) -> impl Iterator<Item = &'static PathMeaningRow> {
    let offset = crate::symbols_owner_identity::NESTED_SHADOW_MATRIX.len()
        + crate::symbols_owner_identity::UNPROVED_OWNER_MATRIX.len();
    path_meaning_rows().enumerate().filter(move |(idx, _)| shared_fact_group(offset + idx) == group).map(|(_, row)| row)
}

/// Writes the [`declaration_fact_sources`] of `group` into `dir`, one file
/// each, named by the source's corpus position.
pub fn write_declaration_fact_group(dir: &std::path::Path, group: usize) -> Vec<(String, std::path::PathBuf)> {
    declaration_fact_sources()
        .into_iter()
        .enumerate()
        .filter(|(_, fact)| fact.group == group)
        .map(|(idx, fact)| {
            let path = dir.join(format!("fact_{idx}.{}", fact.extension));
            std::fs::write(&path, &fact.source).expect("write fact source");
            (fact.label, path)
        })
        .collect()
}

/// The stamped groups are exactly `0..DECLARATION_FACT_GROUP_COUNT`, every
/// corpus source lands in exactly one of them, and none outgrows
/// [`MAX_FACT_GROUP_SOURCES`].
#[test]
fn declaration_fact_groups_partition_the_corpus() {
    let groups: Vec<usize> = (0..DECLARATION_FACT_GROUP_COUNT).collect();
    for (consumer, stamped) in [
        ("query equivalence", crate::symbols_query::declaration_facts_of_narrow_queries_equal_the_complete_index::GROUPS),
        ("query work", crate::symbols_query::declaration_facts_need_one_parse_one_capture_and_one_relation_link::GROUPS),
        ("cache projection", crate::symbols_cache::declaration_fact_sources_are_warm_hits_and_project_to_uncached_queries::GROUPS),
        ("cache widening", crate::symbols_cache::declaration_fact_entries_widen_by_containment_and_metrics::GROUPS),
        ("adapter loss", crate::symbols_adapter::record_only_declaration_facts_are_lost_through_the_older_model::GROUPS),
    ] {
        assert_eq!(stamped, groups.as_slice(), "{consumer}: stamped groups");
    }
    let shared_groups: Vec<usize> = (LANGUAGE_FACT_GROUPS..DECLARATION_FACT_GROUP_COUNT).collect();
    for (consumer, stamped) in [
        ("path-meaning owners", path_meaning_matrix_holds_on_complete_and_declarations_only_paths::GROUPS),
        ("path-meaning warm hits", crate::symbols_cache::path_meaning_rows_hold_on_warm_hits::GROUPS),
    ] {
        assert_eq!(stamped, shared_groups.as_slice(), "{consumer}: stamped groups");
    }
    let mut row_groups: Vec<&str> = Vec::new();
    for &group in &shared_groups {
        let rows: Vec<&str> = path_meaning_group(group).map(|row| row.label).collect();
        assert!(!rows.is_empty() && rows.len() <= MAX_FACT_GROUP_SOURCES, "path-meaning group {group}: {}", rows.len());
        row_groups.extend(rows);
    }
    assert_eq!(row_groups.len(), path_meaning_rows().count(), "every path-meaning row in exactly one group");
    let mut all: Vec<&str> = path_meaning_rows().map(|row| row.label).collect();
    all.sort_unstable();
    row_groups.sort_unstable();
    assert_eq!(row_groups, all, "the path-meaning groups are the path-meaning rows");
    let written_base_rows = path_meaning_rows().filter(|row| !row.owners.is_empty()).count();
    assert!(written_base_rows >= 30, "the written-base rows the warm-hit test runs ({written_base_rows})");

    let corpus = declaration_fact_sources();
    assert_eq!(SHARED_GROUP_LAYOUT.iter().map(|row| row.1).sum::<usize>(), SHARED_FACT_GROUPS);
    for group in LANGUAGE_FACT_GROUPS..DECLARATION_FACT_GROUP_COUNT {
        let extensions: std::collections::BTreeSet<_> = corpus.iter().filter(|fact| fact.group == group).map(|fact| fact.extension).collect();
        assert_eq!(extensions.len(), 1, "group {group} compiles exactly one language");
    }
    let dir = tempfile::TempDir::new().expect("temp");
    let mut seen = vec![0usize; corpus.len()];
    for group in groups {
        let written = write_declaration_fact_group(dir.path(), group);
        assert!(!written.is_empty(), "group {group} is empty");
        assert!(
            written.len() <= MAX_FACT_GROUP_SOURCES,
            "group {group} holds {} sources; add a group",
            written.len()
        );
        for (label, path) in written {
            let idx: usize = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.strip_prefix("fact_"))
                .and_then(|idx| idx.parse().ok())
                .expect("fact file name");
            assert_eq!(label, corpus[idx].label, "group {group}: source {idx}");
            assert_eq!(std::fs::read_to_string(&path).expect("read"), corpus[idx].source, "{label}");
            seen[idx] += 1;
        }
    }
    let misplaced: Vec<&str> =
        corpus.iter().zip(&seen).filter(|(_, count)| **count != 1).map(|(fact, _)| fact.label.as_str()).collect();
    assert!(misplaced.is_empty(), "sources not in exactly one group: {misplaced:?}");
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

/// The one type-like record of `index` with this in-file qualified name; a
/// Scala `object` only when no class, trait or type shares its name (a
/// companion's base clause names the type).
fn type_record<'a>(index: &'a FileSymbolIndex, label: &str, qualified_name: &str) -> &'a SymbolRecord {
    type_record_at(index, label, qualified_name, None)
}

/// [`type_record`], among the same-named declarations the one declared on
/// `line` when given.
fn type_record_at<'a>(
    index: &'a FileSymbolIndex,
    label: &str,
    qualified_name: &str,
    line: Option<u32>,
) -> &'a SymbolRecord {
    let named = || {
        index.symbols.iter().filter(|record| in_file_name(record) == qualified_name).filter(|record| {
            line.is_none_or(|line| record.source.declaration_span.start.line == line)
        })
    };
    let mut found: Vec<&SymbolRecord> = named().filter(|record| is_type_like(record.kind)).collect();
    if found.is_empty() {
        found = named().filter(|record| record.language == L::Scala && record.kind == SymbolKindV2::Module).collect();
    }
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
            let _ = type_record_at(complete, label, qualified_name, target.line);
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
    check_written_spans(label, source, &complete, owners);
    complete
}

/// Each written target that records a source span (Swift) covers its
/// written text there.
fn check_written_spans(label: &str, source: &str, index: &FileSymbolIndex, owners: &[Owner]) {
    for owner in owners {
        let record = type_record(index, label, owner.qualified_name);
        let relations = &record.relations;
        for target in relations.extends.iter().chain(&relations.implements).chain(&relations.unclassified_bases) {
            let Some(span) = target.source_span.as_ref() else { continue };
            assert_eq!(
                source.get(span.start_byte as usize..span.end_byte as usize),
                Some(target.name.as_str()),
                "{label} `{}`: written span",
                owner.qualified_name
            );
        }
    }
}

/// Checks that each owner in `index` reports exactly its written bases, a
/// resolved target's ID being the named declaration's in `complete`.
pub fn check_owner_targets(label: &str, complete: &FileSymbolIndex, index: &FileSymbolIndex, owners: &[Owner]) {
    for owner in owners {
        let record = type_record(index, label, owner.qualified_name);
        let label = format!("{label} `{}`", owner.qualified_name);
        assert_targets(complete, &label, "extends", &record.relations.extends, owner.extends);
        assert_targets(complete, &label, "implements", &record.relations.implements, owner.implements);
        assert_targets(complete, &label, "unclassified", &record.relations.unclassified_bases, owner.unclassified);
        for (target, expected) in record
            .relations
            .extends
            .iter()
            .chain(&record.relations.implements).chain(&record.relations.unclassified_bases)
            .zip(owner.extends.iter().chain(owner.implements).chain(owner.unclassified))
        {
            let expected_id = expected
                .resolves_to
                .map(|qualified_name| type_record_at(complete, &label, qualified_name, expected.line).id.clone());
            assert_eq!(target.id, expected_id, "{label}: `{}` target ID", target.name);
        }
    }
}

crate::language_cases::language_tests!(relations_matrix_holds_on_complete_and_declarations_only_paths => check_relations_matrix_holds_on_complete_and_declarations_only_paths);

/// Each row's owners report exactly their written bases on the complete
/// index and on a declarations-only query of the owner's kind; a resolved
/// target's ID is the named declaration's. Absent rows report
/// no declared base on any record.
fn check_relations_matrix_holds_on_complete_and_declarations_only_paths(language: ProgrammingLanguage) {
    for row in RELATIONS_MATRIX.iter().filter(|row| row.language == language) {
        let label = row.language.name();
        let complete = check_owners(label, row.extension, row.source, row.owners);
        if let Support::Absent(reason) = row.support {
            for record in &complete.symbols {
                assert!(
                    record.relations.extends.is_empty() && record.relations.implements.is_empty() && record.relations.unclassified_bases.is_empty(),
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
    Owner { unclassified: &[], qualified_name, extends, implements: &[] }
}

/// `ReviewOuter.ReviewChild`, whose single written base lands in `extends`
/// when it resolves to a class and is unclassified otherwise.
const fn scala_child(base: &'static [Target]) -> Owner {
    match base {
        [Target { resolves_to: Some(_), .. }] => {
            Owner { qualified_name: "ReviewOuter.ReviewChild", extends: base, implements: &[], unclassified: &[] }
        }
        _ => Owner { qualified_name: "ReviewOuter.ReviewChild", extends: &[], implements: &[], unclassified: base },
    }
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

const fn implementing(qualified_name: &'static str, implements: &'static [Target]) -> Owner {
    Owner { qualified_name, extends: &[], implements, unclassified: &[] }
}

const fn unclassified(qualified_name: &'static str, unclassified: &'static [Target]) -> Owner {
    Owner { qualified_name, extends: &[], implements: &[], unclassified }
}

/// Invokes `$row!` with every Go predeclared non-interface type name.
macro_rules! go_predeclared_names {
    ($row:ident) => {
        $row!("bool", "byte", "complex64", "complex128", "float32", "float64", "int", "int8", "int16", "int32",
            "int64", "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr")
    };
}

/// Invokes `$row!` with every simple Swift standard raw-value type name;
/// `Unicode.Scalar` is written out by each row.
macro_rules! swift_raw_type_names {
    ($row:ident) => {
        $row!("Int", "Int8", "Int16", "Int32", "Int64", "UInt", "UInt8", "UInt16", "UInt32", "UInt64", "String",
            "Character", "Float", "Double")
    };
}

macro_rules! go_local_interface_row {
    ($($name:literal),+) => {
        row("Go same-file interfaces named like predeclared types", L::Go, "go",
            concat!("package shapes\n", $("\ntype ", $name, " interface{ M() }\ntype Child_", $name, " interface{ ", $name, " }\n"),+),
            &[$(bases(concat!("shapes.Child_", $name), &[to($name, concat!("shapes.", $name))])),+])
    };
}

macro_rules! go_alias_row {
    ($($name:literal),+) => {
        row("Go aliases named like predeclared types", L::Go, "go",
            concat!("package shapes\n\ntype Target interface{ M() }\n",
                $("\ntype ", $name, " = Target\ntype Child_", $name, " interface{ ", $name, " }\n"),+),
            &[$(bases(concat!("shapes.Child_", $name), &[text($name)])),+])
    };
}

macro_rules! go_function_local_row {
    ($($name:literal),+) => {
        row("Go function-local interfaces named like predeclared types", L::Go, "go",
            concat!("package shapes\n\nfunc Scope() {\n",
                $("\ttype ", $name, " interface{ M() }\n\ttype Child_", $name, " interface{ ", $name, " }\n"),+, "}\n"),
            &[$(bases(concat!("shapes.Scope.Child_", $name), &[to($name, concat!("shapes.Scope.", $name))])),+])
    };
}

macro_rules! swift_alias_row {
    ($($name:literal),+) => {
        row("Swift aliases named like standard raw types", L::Swift, "swift",
            concat!("protocol Target {}\n", $("typealias ", $name, " = Target\nenum E_", $name, ": ", $name, " { case a }\n"),+,
                "enum Unicode { typealias Scalar = Target }\nenum E_Scalar: Unicode.Scalar { case a }\n"),
            &[$(unclassified(concat!("E_", $name), &[text($name)])),+,
              unclassified("E_Scalar", &[text("Unicode.Scalar")])])
    };
}

macro_rules! swift_selective_import_row {
    ($($name:literal),+) => {
        row("Swift selective imports named like standard raw types", L::Swift, "swift",
            concat!("import Foundation\n", $("import protocol Lib.", $name, "\n"),+, "import enum Lib.Unicode\n",
                $("enum E_", $name, ": ", $name, " { case a }\n"),+, "enum E_Scalar: Unicode.Scalar { case a }\n"),
            &[$(unclassified(concat!("E_", $name), &[text($name)])),+,
              unclassified("E_Scalar", &[text("Unicode.Scalar")])])
    };
}

macro_rules! swift_local_protocol_row {
    ($($name:literal),+) => {
        row("Swift same-file protocols named like standard raw types", L::Swift, "swift",
            concat!($("protocol ", $name, " {}\nenum E_", $name, ": ", $name, " { case a }\n"),+,
                "enum Unicode { protocol Scalar {} }\nenum E_Scalar: Unicode.Scalar { case a }\n"),
            &[$(implementing(concat!("E_", $name), &[to($name, $name)])),+,
              implementing("E_Scalar", &[to("Unicode.Scalar", "Unicode.Scalar")])])
    };
}

macro_rules! swift_standard_raw_type_row {
    ($($name:literal),+) => {
        row("Swift unshadowed standard raw types, bare and module-qualified", L::Swift, "swift",
            concat!("import Foundation\n",
                $("enum E_", $name, ": ", $name, " { case a }\nenum Q_", $name, ": Swift.", $name, " { case a }\n"),+,
                "enum E_Scalar: Unicode.Scalar { case a }\nenum Q_Scalar: Swift.Unicode.Scalar { case a }\n",
                "enum RawCase: Swift.Int { case a = 1 }\nenum RawScalar: Unicode.Scalar { case a = \"x\" }\n"),
            &[$(bases(concat!("E_", $name), &[]), bases(concat!("Q_", $name), &[])),+,
              bases("E_Scalar", &[]), bases("Q_Scalar", &[]), bases("RawCase", &[]), bases("RawScalar", &[])])
    };
}

macro_rules! go_sibling_block_row {
    ($($name:literal),+) => {
        row("Go finished blocks declare interfaces named like predeclared types", L::Go, "go",
            concat!("package shapes\n\nfunc f() {\n",
                $("\t{\n\t\ttype ", $name, " interface{ M() }\n\t}\n\ttype Child_", $name, " interface{ ", $name, " }\n"),+, "}\n"),
            &[$(bases(concat!("shapes.f.Child_", $name), &[])),+])
    };
}

macro_rules! swift_do_block_alias_row {
    ($($name:literal),+) => {
        row("Swift finished `do` blocks alias standard raw-type names", L::Swift, "swift",
            concat!("func f() {\n",
                $("    do { typealias ", $name, " = String }\n    enum E_", $name, ": ", $name, " { case a }\n"),+,
                "    do { enum Unicode { typealias Scalar = String } }\n    enum E_Scalar: Unicode.Scalar { case a }\n}\n"),
            &[$(bases(concat!("f.E_", $name), &[])),+, bases("f.E_Scalar", &[])])
    };
}

macro_rules! swift_value_import_row {
    ($label:literal, $keyword:literal, $($name:literal),+) => {
        row($label, L::Swift, "swift",
            concat!($("import ", $keyword, " Lib.", $name, "\n"),+, "import ", $keyword, " Lib.Unicode\n",
                $("enum E_", $name, ": ", $name, " { case a }\n"),+, "enum E_Scalar: Unicode.Scalar { case a }\n"),
            &[$(bases(concat!("E_", $name), &[])),+, bases("E_Scalar", &[])])
    };
}

macro_rules! swift_value_import_func_row {
    ($($name:literal),+) => { swift_value_import_row!("Swift `import func` never hides a raw type", "func", $($name),+) };
}

macro_rules! swift_value_import_let_row {
    ($($name:literal),+) => { swift_value_import_row!("Swift `import let` never hides a raw type", "let", $($name),+) };
}

macro_rules! swift_value_import_var_row {
    ($($name:literal),+) => { swift_value_import_row!("Swift `import var` never hides a raw type", "var", $($name),+) };
}

/// A Python `global` probe: `reviewOuter.reviewProbe` declares the
/// directive, then `$body` (lines indented eight spaces), and the readers
/// are a class in the probe, one in `reviewOuter` and one at the module.
macro_rules! python_global_row {
    ($label:literal, $body:literal, $owners:expr) => {
        row(concat!("Python `global` ", $label), L::Python, "py",
            concat!("class ReviewBase:\n    pass\n\n\
                def reviewOuter():\n    def reviewProbe(external):\n        global ReviewBase\n", $body,
                "        class ReviewChild(ReviewBase):\n            pass\n\n\
                \x20   class ReviewOuterChild(ReviewBase):\n        pass\n\n\
                class ReviewModuleChild(ReviewBase):\n    pass\n"),
            $owners)
    };
}

/// The `nonlocal` form of [`python_global_row`]: `reviewOuter` declares
/// its own `ReviewBase`, the directive's destination.
macro_rules! python_nonlocal_row {
    ($label:literal, $body:literal, $owners:expr) => {
        row(concat!("Python `nonlocal` ", $label), L::Python, "py",
            concat!("class ReviewBase:\n    pass\n\n\
                def reviewOuter():\n    class ReviewBase:\n        pass\n\n\
                \x20   def reviewProbe(external):\n        nonlocal ReviewBase\n", $body,
                "        class ReviewChild(ReviewBase):\n            pass\n\n\
                \x20   class ReviewOuterChild(ReviewBase):\n        pass\n\n\
                class ReviewModuleChild(ReviewBase):\n    pass\n"),
            $owners)
    };
}

/// A write under `global` rebinds the module's name: no reader that sees
/// the module binding has a proved target, wherever it is.
const PY_GLOBAL_WRITTEN: &[Owner] = &[bases("reviewOuter.reviewProbe.ReviewChild", &[text("ReviewBase")]),
          bases("reviewOuter.ReviewOuterChild", &[text("ReviewBase")]),
          bases("ReviewModuleChild", &[text("ReviewBase")])];

const PY_GLOBAL_KEPT: &[Owner] = &[bases("reviewOuter.reviewProbe.ReviewChild", &[to_at("ReviewBase", "ReviewBase", 1)]),
          bases("reviewOuter.ReviewOuterChild", &[to_at("ReviewBase", "ReviewBase", 1)]),
          bases("ReviewModuleChild", &[to_at("ReviewBase", "ReviewBase", 1)])];

/// A write under `nonlocal` rebinds `reviewOuter`'s name: its readers lose
/// the target, the module's reader keeps the module class.
const PY_NONLOCAL_WRITTEN: &[Owner] = &[bases("reviewOuter.reviewProbe.ReviewChild", &[text("ReviewBase")]),
          bases("reviewOuter.ReviewOuterChild", &[text("ReviewBase")]),
          bases("ReviewModuleChild", &[to_at("ReviewBase", "ReviewBase", 1)])];

const PY_NONLOCAL_KEPT: &[Owner] = &[
          bases("reviewOuter.reviewProbe.ReviewChild", &[to_at("ReviewBase", "reviewOuter.ReviewBase", 5)]),
          bases("reviewOuter.ReviewOuterChild", &[to_at("ReviewBase", "reviewOuter.ReviewBase", 5)]),
          bases("ReviewModuleChild", &[to_at("ReviewBase", "ReviewBase", 1)])];

/// TypeScript and JavaScript binding patterns, each checked in both languages:
/// a name a pattern binds keeps an outer class from the base, a plain
/// binding of another name does not.
const ECMA_PARAMETER_PATTERNS: &str = "class ReviewBase {}\n\
         function nestedObject({ a: { ReviewBase } }) { class ReviewChild extends ReviewBase {} }\n\
         function nestedArray([[ReviewBase]]) { class ReviewChild extends ReviewBase {} }\n\
         function objectDefault({ ReviewBase = 1 }) { class ReviewChild extends ReviewBase {} }\n\
         function nestedRenamed({ a: { b: ReviewBase } }) { class ReviewChild extends ReviewBase {} }\n\
         function objectRest({ ...ReviewBase }) { class ReviewChild extends ReviewBase {} }\n\
         function arrayRest([...ReviewBase]) { class ReviewChild extends ReviewBase {} }\n\
         function control(other) { class ReviewChild extends ReviewBase {} }\n\
         function flatObject({ ReviewBase }) { class ReviewChild extends ReviewBase {} }\n\
         function flatArray([ReviewBase]) { class ReviewChild extends ReviewBase {} }\n\
         function flatRenamed({ a: ReviewBase }) { class ReviewChild extends ReviewBase {} }\n\
         function withDefault(ReviewBase = 1) { class ReviewChild extends ReviewBase {} }\n\
         function bareRest(...ReviewBase) { class ReviewChild extends ReviewBase {} }\n\
         const assigned = (ReviewBase) => { class ReviewArrowChild extends ReviewBase {} };\n\
         [1].map(ReviewBase => { class ReviewAnonymousChild extends ReviewBase {} });\n";

const ECMA_PARAMETER_OWNERS: &[Owner] = &[bases("nestedObject.ReviewChild", &[text("ReviewBase")]),
          bases("nestedArray.ReviewChild", &[text("ReviewBase")]),
          bases("objectDefault.ReviewChild", &[text("ReviewBase")]),
          bases("nestedRenamed.ReviewChild", &[text("ReviewBase")]),
          bases("objectRest.ReviewChild", &[text("ReviewBase")]),
          bases("arrayRest.ReviewChild", &[text("ReviewBase")]),
          bases("control.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("flatObject.ReviewChild", &[text("ReviewBase")]),
          bases("flatArray.ReviewChild", &[text("ReviewBase")]),
          bases("flatRenamed.ReviewChild", &[text("ReviewBase")]),
          bases("withDefault.ReviewChild", &[text("ReviewBase")]),
          bases("bareRest.ReviewChild", &[text("ReviewBase")]),
          bases("assigned.ReviewArrowChild", &[text("ReviewBase")]),
          bases("ReviewAnonymousChild", &[text("ReviewBase")])];

const ECMA_VARIABLE_PATTERNS: &str = "class ReviewBase {}\n\
         function flatRenamed(x) { const { a: ReviewBase } = x; class ReviewChild extends ReviewBase {} }\n\
         function nestedObject(x) { const { a: { ReviewBase } } = x; class ReviewChild extends ReviewBase {} }\n\
         function flatArray(x) { const [ReviewBase] = x; class ReviewChild extends ReviewBase {} }\n\
         function nestedArray(x) { const [[ReviewBase]] = x; class ReviewChild extends ReviewBase {} }\n\
         function objectDefault(x) { const { ReviewBase = 1 } = x; class ReviewChild extends ReviewBase {} }\n\
         function objectRest(x) { const { ...ReviewBase } = x; class ReviewChild extends ReviewBase {} }\n\
         function arrayRest(x) { const [...ReviewBase] = x; class ReviewChild extends ReviewBase {} }\n\
         function simple(x) { const ReviewBase = x; class ReviewChild extends ReviewBase {} }\n";

const ECMA_VARIABLE_OWNERS: &[Owner] = &[bases("flatRenamed.ReviewChild", &[text("ReviewBase")]),
          bases("nestedObject.ReviewChild", &[text("ReviewBase")]),
          bases("flatArray.ReviewChild", &[text("ReviewBase")]),
          bases("nestedArray.ReviewChild", &[text("ReviewBase")]),
          bases("objectDefault.ReviewChild", &[text("ReviewBase")]),
          bases("objectRest.ReviewChild", &[text("ReviewBase")]),
          bases("arrayRest.ReviewChild", &[text("ReviewBase")]),
          bases("simple.ReviewChild", &[text("ReviewBase")])];

const ECMA_CATCH_AND_FOR_PATTERNS: &str = "class ReviewBase {}\n\
         function catchName() { try {} catch (ReviewBase) { class ReviewChild extends ReviewBase {} } }\n\
         function catchObject() { try {} catch ({ ReviewBase }) { class ReviewChild extends ReviewBase {} } }\n\
         function catchNested() { try {} catch ({ a: { ReviewBase } }) { class ReviewChild extends ReviewBase {} } }\n\
         function forName(xs) { for (const ReviewBase of xs) { class ReviewChild extends ReviewBase {} } }\n\
         function forObject(xs) { for (const { ReviewBase } of xs) { class ReviewChild extends ReviewBase {} } }\n\
         function forNested(xs) { for (const { a: { ReviewBase } } of xs) { class ReviewChild extends ReviewBase {} } }\n\
         function forControl(xs) { for (const other of xs) { class ReviewChild extends ReviewBase {} } }\n";

const ECMA_CATCH_AND_FOR_OWNERS: &[Owner] = &[bases("catchName.ReviewChild", &[text("ReviewBase")]),
          bases("catchObject.ReviewChild", &[text("ReviewBase")]),
          bases("catchNested.ReviewChild", &[text("ReviewBase")]),
          bases("forName.ReviewChild", &[text("ReviewBase")]),
          bases("forObject.ReviewChild", &[text("ReviewBase")]),
          bases("forNested.ReviewChild", &[text("ReviewBase")]),
          bases("forControl.ReviewChild", &[to("ReviewBase", "ReviewBase")])];

/// A target resolves only when its root, every intermediate owner and the
/// first segment's nearest binding all lead to the declaration: a leading
/// `::` is global in C++ and external in Rust, a PHP relative name stays in
/// its namespace, a parameter, import, nearer namespace or inherited member
/// type decides its first segment, a function's local class is not reachable
/// through the function's name, and an enclosing generic parameter is never
/// a file-level type.
pub const PATH_MEANING_MATRIX: &[PathMeaningRow] = &[
    row("C# primary constructor base invocation", L::CSharp, "cs",
        "interface I {}\nclass Base(int x) {}\nclass Child(int x) : Base(x), I {}\nclass Unknown(int x) : External(x) {}\n",
        &[Owner { qualified_name: "Child", extends: &[to("Base(x)", "Base")], implements: &[to("I", "I")], unclassified: &[] },
          Owner { qualified_name: "Unknown", extends: &[text("External(x)")], implements: &[], unclassified: &[] }]),
    row("C# rooted base", L::CSharp, "cs",
        "class Base {}\nnamespace N { class Base {} class Child : global::Base {} }\n",
        &[Owner { qualified_name: "N.Child", extends: &[to("global::Base", "Base")], implements: &[], unclassified: &[] }]),
    row("C# alias in a file-scoped namespace", L::CSharp, "cs",
        "namespace N;\nusing Alias = Lib.Base;\nclass Alias {}\nclass Child : Alias {}\n",
        &[Owner { qualified_name: "N.Child", extends: &[], implements: &[], unclassified: &[text("Alias")] }]),
    row("C# inherited member type", L::CSharp, "cs",
        "class Base {}\nclass Super {}\nclass Outer : Super { class Child : Base {} }\n",
        &[Owner { qualified_name: "Outer.Child", extends: &[], implements: &[], unclassified: &[text("Base")] }]),
    // A C# record and record class are classes, though every record reports
    // the `Type` kind; a struct and a record struct are never superclasses.
    row("C# implicit record class target", L::CSharp, "cs",
        "record ReviewBase;\nrecord ReviewChild : ReviewBase;\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C# explicit record class target", L::CSharp, "cs",
        "record class ReviewBase;\nrecord ReviewChild : ReviewBase;\nclass ReviewClassChild : ReviewBase {}\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("ReviewClassChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C# positional record class target", L::CSharp, "cs",
        "record ReviewBase(int X = 0);\nrecord ReviewChild : ReviewBase;\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C# generic record class target", L::CSharp, "cs",
        "record ReviewBase<T>;\nrecord ReviewChild : ReviewBase<int>;\n",
        &[bases("ReviewChild", &[to("ReviewBase<int>", "ReviewBase")])]),
    row("C# namespace-qualified record class target", L::CSharp, "cs",
        "namespace ReviewNS { record ReviewBase; }\nrecord ReviewChild : ReviewNS.ReviewBase;\n",
        &[bases("ReviewChild", &[to("ReviewNS.ReviewBase", "ReviewNS.ReviewBase")])]),
    row("C# ordinary class and interface targets beside a record", L::CSharp, "cs",
        "class ReviewClass {}\ninterface ReviewInterface {}\nrecord ReviewRecord;\n\
         class ReviewChild : ReviewClass, ReviewInterface {}\nrecord ReviewConforming : ReviewInterface;\n",
        &[Owner { qualified_name: "ReviewChild", extends: &[to("ReviewClass", "ReviewClass")],
                  implements: &[to("ReviewInterface", "ReviewInterface")], unclassified: &[] },
          implementing("ReviewConforming", &[to("ReviewInterface", "ReviewInterface")])]),
    row("C# struct and record struct targets", L::CSharp, "cs",
        "struct ReviewStruct {}\nrecord struct ReviewRecordStruct;\n\
         class ReviewChild : ReviewStruct {}\nrecord ReviewRecordChild : ReviewRecordStruct;\n",
        &[unclassified("ReviewChild", &[to("ReviewStruct", "ReviewStruct")]),
          unclassified("ReviewRecordChild", &[to("ReviewRecordStruct", "ReviewRecordStruct")])]),
    row("C# aliased and duplicate record targets", L::CSharp, "cs",
        "using ReviewAlias = Lib.ReviewBase;\nrecord ReviewAlias;\nrecord ReviewDuplicate;\nrecord ReviewDuplicate;\n\
         record ReviewAliasChild : ReviewAlias;\nrecord ReviewDuplicateChild : ReviewDuplicate;\n",
        &[unclassified("ReviewAliasChild", &[text("ReviewAlias")]),
          unclassified("ReviewDuplicateChild", &[text("ReviewDuplicate")])]),
    row("Swift local protocol shadows a standard raw-type name", L::Swift, "swift",
        "protocol Int {}\nenum E: Int { case a }\n",
        &[Owner { qualified_name: "E", extends: &[], implements: &[to("Int", "Int")], unclassified: &[] }]),
    row("Swift implicit raw-value enum", L::Swift, "swift",
        "enum E: Int { case a }\n",
        &[Owner { qualified_name: "E", extends: &[], implements: &[], unclassified: &[] }]),
    row("Swift explicit external extension owner", L::Swift, "swift",
        "protocol P {}\nstruct Root {}\nextension other.Root: P {}\n",
        &[Owner { qualified_name: "Root", extends: &[], implements: &[], unclassified: &[] }]),
    row("Swift aliased extension owner", L::Swift, "swift",
        "protocol P {}\nstruct Root {}\ntypealias Alias = Root\nextension Alias: P {}\n",
        &[Owner { qualified_name: "Root", extends: &[], implements: &[], unclassified: &[] }]),
    row("Swift qualified base", L::Swift, "swift",
        "enum NS { class Base {} }\nclass Child: NS.Base {}\n",
        &[Owner { qualified_name: "Child", extends: &[to("NS.Base", "NS.Base")], implements: &[], unclassified: &[] }]),
    row("Swift module import keeps local types", L::Swift, "swift",
        "import Foundation\nclass Base {}\nclass Child: Base {}\n",
        &[Owner { qualified_name: "Child", extends: &[to("Base", "Base")], implements: &[], unclassified: &[] }]),
    row("Swift selective import binds the name", L::Swift, "swift",
        "import class Lib.Base\nclass Base {}\nclass Child: Base {}\n",
        &[Owner { qualified_name: "Child", extends: &[], implements: &[], unclassified: &[text("Base")] }]),
    row("Scala imported alias in a package", L::Scala, "scala",
        "package sample\nimport lib.{Other => Base}\nclass Base\nclass Child extends Base\n",
        &[Owner { qualified_name: "sample.Child", extends: &[], implements: &[], unclassified: &[text("Base")] }]),
    row("Scala imported alias in a block package", L::Scala, "scala",
        "package sample { import lib.{Other => Base}; class Base; class Child extends Base }\n",
        &[Owner { qualified_name: "sample.Child", extends: &[], implements: &[], unclassified: &[text("Base")] }]),
    // Every expression of a Scala `import` binds, in both rename forms; a
    // finite rename or exclusion hides nothing else, only a wildcard does.
    row("Scala single import", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.ReviewBase\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala comma-separated imports", L::Scala, "scala",
        "class ReviewBase\ntrait ReviewOther\nobject ReviewOuter {\n  import lib.ReviewBase, lib.ReviewOther\n  \
         class ReviewChild extends ReviewBase with ReviewOther\n}\n",
        &[Owner { qualified_name: "ReviewOuter.ReviewChild", extends: &[], implements: &[text("ReviewOther")],
                  unclassified: &[text("ReviewBase")] }]),
    row("Scala plain import before a selector group", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.ReviewBase, other.{Other}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala second selector group", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other}, lib.{ReviewBase}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala selector group before a plain import", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{ReviewBase}, other.Other\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala arrow rename", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other => ReviewBase}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala unbraced `as` rename", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.Other as ReviewBase\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala braced `as` rename", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other as ReviewBase}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala braced `as` rename of an unrelated name", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other as Renamed}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewBase")])]),
    row("Scala braced `as _` exclusion", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other as _}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewBase")])]),
    row("Scala arrow exclusion", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other => _}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewBase")])]),
    row("Scala unbraced `as _` exclusion", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.Other as _\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewBase")])]),
    row("Scala renamed selectors beside a plain import of an unrelated name", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other as Renamed, Third => _}, other.Fourth\n  \
         class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewBase")])]),
    row("Scala alias with a wildcard and an outer-only base", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.{Other => Alias, _}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala wildcard in a later expression", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.Other, other.*\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("Scala wildcard with a stronger local base", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib._\n  class ReviewBase\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewOuter.ReviewBase")])]),
    row("R8-F2 relation barrier: unbraced operator original", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.++ as ReviewBase\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("R8-F2 relation barrier: symbolic qualifier", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.+.ReviewBase\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("R8-F2 relation barrier: symbolic selector prefix", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.+.{ReviewBase}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("R8-F2 relation barrier: symbolic wildcard prefix", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.+.*\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("R8-F2 relation barrier: commented unbraced operator original", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib. /* n */ ++ /* n */ as /* n */ ReviewBase\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("R8-F2 relation barrier: commented symbolic selector prefix", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib. /* n */ +. /* n */ {ReviewBase}\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[text("ReviewBase")])]),
    row("R8-F2 relation barrier: hidden operator original", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  import lib.++ as _\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewBase")])]),
    row("Scala no-import control", L::Scala, "scala",
        "class ReviewBase\nobject ReviewOuter {\n  class ReviewChild extends ReviewBase\n}\n",
        &[scala_child(&[to("ReviewBase", "ReviewBase")])]),
    row("Scala constructor arguments stay written", L::Scala, "scala",
        "class Base[A](x: Int)\ntrait T\nclass Child extends Base[Int](1) with T\n",
        &[Owner { qualified_name: "Child", extends: &[to("Base[Int](1)", "Base")], implements: &[to("T", "T")], unclassified: &[] }]),

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
    // A built-in spelling is no relation only when no binding in view gives
    // it another meaning: a same-file declaration (package-level or in an
    // enclosing function) keeps and resolves the base, an alias or import
    // keeps it as written.
    go_predeclared_names!(go_local_interface_row),
    go_predeclared_names!(go_alias_row),
    go_predeclared_names!(go_function_local_row),
    row(
        "Go predeclared types and constraint terms",
        L::Go,
        "go",
        "package shapes\n\ntype Plain interface{ int }\ntype Tilde interface{ ~string }\n\
         type Union interface{ int8 | uint16 }\n\nfunc Elsewhere() {\n\ttype rune interface{ M() }\n}\n\n\
         type Outside interface{ rune }\n",
        &[bases("shapes.Plain", &[]), bases("shapes.Tilde", &[]), bases("shapes.Union", &[]),
          bases("shapes.Outside", &[])],
    ),
    swift_raw_type_names!(swift_alias_row),
    swift_raw_type_names!(swift_selective_import_row),
    swift_raw_type_names!(swift_local_protocol_row),
    swift_raw_type_names!(swift_standard_raw_type_row),
    row(
        "Swift module-qualified raw type under a same-file Swift owner",
        L::Swift,
        "swift",
        "enum Swift { protocol Int {} }\nenum E: Swift.Int { case a }\nenum F: Swift.Double { case a }\n",
        &[implementing("E", &[to("Swift.Int", "Swift.Int")]), unclassified("F", &[text("Swift.Double")])],
    ),
    // --- Lexical binding evidence: block boundaries and declaration
    // position. A finished or sibling block's declarations are out of view;
    // a Go, C++ or Java local type is in view only after its name.
    row("Rust blocks", L::Rust, "rs",
        "trait ReviewBase {}\n\
         fn inner() { { trait ReviewBase {} } trait ReviewChild: ReviewBase {} }\n\
         fn sibling() { { trait ReviewBase {} } { trait ReviewChild: ReviewBase {} } }\n\
         fn own_block() { { trait ReviewBase {} trait ReviewChild: ReviewBase {} } }\n\
         fn same_scope() { trait ReviewBase {} trait ReviewChild: ReviewBase {} }\n\
         fn none() { trait ReviewChild: ReviewBase {} }\n",
        &[bases("inner::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("sibling::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("own_block::ReviewChild", &[to("ReviewBase", "own_block::ReviewBase")]),
          bases("same_scope::ReviewChild", &[to("ReviewBase", "same_scope::ReviewBase")]),
          bases("none::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust sibling block holds the only declaration", L::Rust, "rs",
        "fn sibling() { { trait ReviewBase {} } { trait ReviewChild: ReviewBase {} } }\n",
        &[bases("sibling::ReviewChild", &[text("ReviewBase")])]),
    row("Go blocks", L::Go, "go",
        "package shapes\n\ntype ReviewBase interface{ M() }\n\n\
         func inner() {\n\t{\n\t\ttype ReviewBase interface{ M() }\n\t}\n\ttype ReviewChild interface{ ReviewBase }\n}\n\n\
         func sibling() {\n\t{\n\t\ttype ReviewBase interface{ M() }\n\t}\n\t{\n\t\ttype ReviewChild interface{ ReviewBase }\n\t}\n}\n\n\
         func ownBlock() {\n\t{\n\t\ttype ReviewBase interface{ M() }\n\t\ttype ReviewChild interface{ ReviewBase }\n\t}\n}\n\n\
         func sameScope() {\n\ttype ReviewBase interface{ M() }\n\ttype ReviewChild interface{ ReviewBase }\n}\n\n\
         func none() {\n\ttype ReviewChild interface{ ReviewBase }\n}\n",
        &[bases("shapes.inner.ReviewChild", &[to("ReviewBase", "shapes.ReviewBase")]),
          bases("shapes.sibling.ReviewChild", &[to("ReviewBase", "shapes.ReviewBase")]),
          bases("shapes.ownBlock.ReviewChild", &[to("ReviewBase", "shapes.ownBlock.ReviewBase")]),
          bases("shapes.sameScope.ReviewChild", &[to("ReviewBase", "shapes.sameScope.ReviewBase")]),
          bases("shapes.none.ReviewChild", &[to("ReviewBase", "shapes.ReviewBase")])]),
    row("Go sibling block holds the only declaration", L::Go, "go",
        "package shapes\n\nfunc sibling() {\n\t{\n\t\ttype ReviewBase interface{ M() }\n\t}\n\t{\n\t\ttype ReviewChild interface{ ReviewBase }\n\t}\n}\n",
        &[bases("shapes.sibling.ReviewChild", &[text("ReviewBase")])]),
    row("C++ blocks", L::Cpp, "cpp",
        "struct ReviewBase {};\n\
         void inner() { { struct ReviewBase {}; } struct ReviewChild : ReviewBase {}; }\n\
         void sibling() { { struct ReviewBase {}; } { struct ReviewChild : ReviewBase {}; } }\n\
         void own_block() { { struct ReviewBase {}; struct ReviewChild : ReviewBase {}; } }\n\
         void same_scope() { struct ReviewBase {}; struct ReviewChild : ReviewBase {}; }\n\
         void none() { struct ReviewChild : ReviewBase {}; }\n",
        &[bases("inner::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("sibling::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("own_block::ReviewChild", &[to("ReviewBase", "own_block::ReviewBase")]),
          bases("same_scope::ReviewChild", &[to("ReviewBase", "same_scope::ReviewBase")]),
          bases("none::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C++ sibling block holds the only declaration", L::Cpp, "cpp",
        "void sibling() { { struct ReviewBase {}; } { struct ReviewChild : ReviewBase {}; } }\n",
        &[bases("sibling::ReviewChild", &[text("ReviewBase")])]),
    row("Java blocks", L::Java, "java",
        "class ReviewBase {}\nclass Holder {\n\
         void inner() { { class ReviewBase {} } class ReviewChild extends ReviewBase {} }\n\
         void sibling() { { class ReviewBase {} } { class ReviewChild extends ReviewBase {} } }\n\
         void ownBlock() { { class ReviewBase {} class ReviewChild extends ReviewBase {} } }\n\
         void sameScope() { class ReviewBase {} class ReviewChild extends ReviewBase {} }\n\
         void none() { class ReviewChild extends ReviewBase {} }\n}\n",
        &[bases("Holder.inner.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("Holder.sibling.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("Holder.ownBlock.ReviewChild", &[to("ReviewBase", "Holder.ownBlock.ReviewBase")]),
          bases("Holder.sameScope.ReviewChild", &[to("ReviewBase", "Holder.sameScope.ReviewBase")]),
          bases("Holder.none.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Java sibling block holds the only declaration", L::Java, "java",
        "class Holder { void sibling() { { class ReviewBase {} } { class ReviewChild extends ReviewBase {} } } }\n",
        &[bases("Holder.sibling.ReviewChild", &[text("ReviewBase")])]),
    row("TypeScript blocks", L::TypeScript, "ts",
        "class ReviewBase {}\n\
         function inner() { { class ReviewBase {} } class ReviewChild extends ReviewBase {} }\n\
         function sibling() { { class ReviewBase {} } { class ReviewChild extends ReviewBase {} } }\n\
         function ownBlock() { { class ReviewBase {} class ReviewChild extends ReviewBase {} } }\n\
         function sameScope() { class ReviewBase {} class ReviewChild extends ReviewBase {} }\n\
         function none() { class ReviewChild extends ReviewBase {} }\n",
        &[bases("inner.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("sibling.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("ownBlock.ReviewChild", &[to("ReviewBase", "ownBlock.ReviewBase")]),
          bases("sameScope.ReviewChild", &[to("ReviewBase", "sameScope.ReviewBase")]),
          bases("none.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript sibling block holds the only declaration", L::TypeScript, "ts",
        "function sibling() { { class ReviewBase {} } { class ReviewChild extends ReviewBase {} } }\n",
        &[bases("sibling.ReviewChild", &[text("ReviewBase")])]),
    row("JavaScript blocks", L::JavaScript, "js",
        "class ReviewBase {}\n\
         function inner() { { class ReviewBase {} } class ReviewChild extends ReviewBase {} }\n\
         function sibling() { { class ReviewBase {} } { class ReviewChild extends ReviewBase {} } }\n\
         function ownBlock() { { class ReviewBase {} class ReviewChild extends ReviewBase {} } }\n\
         function sameScope() { class ReviewBase {} class ReviewChild extends ReviewBase {} }\n\
         function none() { class ReviewChild extends ReviewBase {} }\n",
        &[bases("inner.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("sibling.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("ownBlock.ReviewChild", &[to("ReviewBase", "ownBlock.ReviewBase")]),
          bases("sameScope.ReviewChild", &[to("ReviewBase", "sameScope.ReviewBase")]),
          bases("none.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("JavaScript sibling block holds the only declaration", L::JavaScript, "js",
        "function sibling() { { class ReviewBase {} } { class ReviewChild extends ReviewBase {} } }\n",
        &[bases("sibling.ReviewChild", &[text("ReviewBase")])]),
    row("Swift `do` blocks", L::Swift, "swift",
        "class ReviewBase {}\n\
         func inner() {\n    do { class ReviewBase {} }\n    class ReviewChild: ReviewBase {}\n}\n\
         func sibling() {\n    do { class ReviewBase {} }\n    do { class ReviewChild: ReviewBase {} }\n}\n\
         func ownBlock() {\n    do {\n        class ReviewBase {}\n        class ReviewChild: ReviewBase {}\n    }\n}\n\
         func sameScope() {\n    class ReviewBase {}\n    class ReviewChild: ReviewBase {}\n}\n\
         func none() {\n    class ReviewChild: ReviewBase {}\n}\n",
        &[bases("inner.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("sibling.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("ownBlock.ReviewChild", &[to("ReviewBase", "ownBlock.ReviewBase")]),
          bases("sameScope.ReviewChild", &[to("ReviewBase", "sameScope.ReviewBase")]),
          bases("none.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Swift sibling `do` block holds the only declaration", L::Swift, "swift",
        "func sibling() {\n    do { class ReviewBase {} }\n    do { class ReviewChild: ReviewBase {} }\n}\n",
        &[unclassified("sibling.ReviewChild", &[text("ReviewBase")])]),
    row("Scala blocks", L::Scala, "scala",
        "class ReviewBase\nobject Holder {\n\
         \x20 def inner(): Unit = {\n    { class ReviewBase }\n    class ReviewChild extends ReviewBase\n  }\n\
         \x20 def sibling(): Unit = {\n    { class ReviewBase }\n    { class ReviewChild extends ReviewBase }\n  }\n\
         \x20 def ownBlock(): Unit = {\n    { class ReviewBase; class ReviewChild extends ReviewBase }\n  }\n\
         \x20 def sameScope(): Unit = {\n    class ReviewBase\n    class ReviewChild extends ReviewBase\n  }\n\
         \x20 def none(): Unit = {\n    class ReviewChild extends ReviewBase\n  }\n}\n",
        &[bases("Holder.inner.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("Holder.sibling.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("Holder.ownBlock.ReviewChild", &[to("ReviewBase", "Holder.ownBlock.ReviewBase")]),
          bases("Holder.sameScope.ReviewChild", &[to("ReviewBase", "Holder.sameScope.ReviewBase")]),
          bases("Holder.none.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala sibling block holds the only declaration", L::Scala, "scala",
        "object Holder {\n  def sibling(): Unit = {\n    { class ReviewBase }\n    { class ReviewChild extends ReviewBase }\n  }\n}\n",
        &[unclassified("Holder.sibling.ReviewChild", &[text("ReviewBase")])]),
    row("Rust import in a finished block", L::Rust, "rs",
        "trait ReviewBase {}\nfn f() { { use other::ReviewBase; } trait ReviewChild: ReviewBase {} }\n",
        &[bases("f::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C++ using-declaration in a finished block", L::Cpp, "cpp",
        "struct ReviewBase {};\nvoid f() { { using other::ReviewBase; } struct ReviewChild : ReviewBase {}; }\n",
        &[bases("f::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala import in a finished block", L::Scala, "scala",
        "class ReviewBase\nobject Holder {\n  def f(): Unit = {\n    { import lib.ReviewBase }\n    class ReviewChild extends ReviewBase\n  }\n}\n",
        &[bases("Holder.f.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust value in a finished block", L::Rust, "rs",
        "trait ReviewBase {}\nfn f() { { let ReviewBase = 1; } trait ReviewChild: ReviewBase {} }\n",
        &[bases("f::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C++ value in a finished block", L::Cpp, "cpp",
        "struct ReviewBase {};\nvoid f() { { int ReviewBase = 0; } struct ReviewChild : ReviewBase {}; }\n",
        &[bases("f::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Java value in a finished block", L::Java, "java",
        "class ReviewBase {}\nclass Holder { void f() { { int ReviewBase = 0; } class ReviewChild extends ReviewBase {} } }\n",
        &[bases("Holder.f.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript value in a finished block", L::TypeScript, "ts",
        "class ReviewBase {}\nfunction f() { { let ReviewBase = 1; } class ReviewChild extends ReviewBase {} }\n",
        &[bases("f.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("JavaScript value in a finished block", L::JavaScript, "js",
        "class ReviewBase {}\nfunction f() { { let ReviewBase = 1; } class ReviewChild extends ReviewBase {} }\n",
        &[bases("f.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Swift value in a finished `do` block", L::Swift, "swift",
        "class ReviewBase {}\nfunc f() {\n    do { let ReviewBase = 1 }\n    class ReviewChild: ReviewBase {}\n}\n",
        &[bases("f.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala value in a finished block", L::Scala, "scala",
        "class ReviewBase\nobject Holder {\n  def f(): Unit = {\n    { val ReviewBase = 1 }\n    class ReviewChild extends ReviewBase\n  }\n}\n",
        &[bases("Holder.f.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    go_predeclared_names!(go_sibling_block_row),
    swift_raw_type_names!(swift_do_block_alias_row),
    row("Go local type declared after its use", L::Go, "go",
        "package shapes\n\ntype ReviewBase interface{ M() }\n\n\
         func f() {\n\ttype ReviewChild interface{ ReviewBase }\n\ttype ReviewBase interface{ M() }\n}\n",
        &[bases("shapes.f.ReviewChild", &[to("ReviewBase", "shapes.ReviewBase")])]),
    row("C++ local type declared after its use", L::Cpp, "cpp",
        "struct ReviewBase {};\nvoid f() { struct ReviewChild : ReviewBase {}; struct ReviewBase {}; }\n",
        &[bases("f::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Java local class declared after its use", L::Java, "java",
        "class ReviewBase {}\nclass Holder { void f() { class ReviewChild extends ReviewBase {} class ReviewBase {} } }\n",
        &[bases("Holder.f.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust local item declared after its use", L::Rust, "rs",
        "trait ReviewBase {}\nfn f() { trait ReviewChild: ReviewBase {} trait ReviewBase {} }\n",
        &[bases("f::ReviewChild", &[to("ReviewBase", "f::ReviewBase")])]),
    row("Swift local type declared after its use", L::Swift, "swift",
        "class ReviewBase {}\nfunc f() {\n    class ReviewChild: ReviewBase {}\n    class ReviewBase {}\n}\n",
        &[bases("f.ReviewChild", &[to("ReviewBase", "f.ReviewBase")])]),
    row("Scala local class declared after its use", L::Scala, "scala",
        "class ReviewBase\nobject Holder {\n  def f(): Unit = {\n    class ReviewChild extends ReviewBase\n    class ReviewBase\n  }\n}\n",
        &[bases("Holder.f.ReviewChild", &[to("ReviewBase", "Holder.f.ReviewBase")])]),
    row("Python suites are no scope; a nested function is", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         def suite(flag):\n    if flag:\n        class ReviewBase:\n            pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def nested():\n    def g():\n        class ReviewBase:\n            pass\n    class ReviewChild(ReviewBase):\n        pass\n",
        &[bases("suite.ReviewChild", &[to("ReviewBase", "suite.ReviewBase")]),
          bases("nested.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C# separate and rooted namespaces", L::CSharp, "cs",
        "namespace A { class ReviewBase {} }\nnamespace B { class ReviewChild : ReviewBase {} }\n",
        &[unclassified("B.ReviewChild", &[text("ReviewBase")])]),
    row("C# rooted name in a file-scoped namespace", L::CSharp, "cs",
        "namespace Example.Symbols;\nclass ReviewBase {}\nclass ReviewChild : global::Example.Symbols.ReviewBase {}\n",
        &[bases("Example.Symbols.ReviewChild", &[to("global::Example.Symbols.ReviewBase", "Example.Symbols.ReviewBase")])]),
    row("PHP class declared in a function names its namespace's class", L::Php, "php",
        "<?php\nnamespace App;\nclass ReviewBase {}\nfunction f() {\n    class ReviewChild extends ReviewBase {}\n}\n\
         if (true) {\n    class ReviewConditional extends ReviewBase {}\n}\n",
        &[bases("App\\f::ReviewChild", &[to("ReviewBase", "App\\ReviewBase")]),
          bases("App\\ReviewConditional", &[to("ReviewBase", "App\\ReviewBase")])]),

    // --- PHP class namespaces: a class, interface, trait or enum declared
    // in a function, method, closure or conditional belongs to the
    // namespace, whatever the function prefix its displayed name carries.
    row("PHP class bases declared in the same function", L::Php, "php",
        "<?php\nnamespace Example\\Symbols;\nfunction reviewProbe() {\n    class ReviewBase {}\n    interface ReviewContract {}\n\
         \x20   class ReviewChild extends ReviewBase implements ReviewContract {}\n\
         \x20   interface ReviewSub extends ReviewContract {}\n\
         \x20   enum ReviewEnum implements ReviewContract { case A; }\n}\n",
        &[Owner { qualified_name: "Example\\Symbols\\reviewProbe::ReviewChild",
                  extends: &[to("ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")],
                  implements: &[to("ReviewContract", "Example\\Symbols\\reviewProbe::ReviewContract")], unclassified: &[] },
          bases("Example\\Symbols\\reviewProbe::ReviewSub",
                &[to("ReviewContract", "Example\\Symbols\\reviewProbe::ReviewContract")]),
          implementing("Example\\Symbols\\reviewProbe::ReviewEnum",
                &[to("ReviewContract", "Example\\Symbols\\reviewProbe::ReviewContract")])]),
    row("PHP class bases declared in a function, named from the namespace", L::Php, "php",
        "<?php\nnamespace Example\\Symbols;\nfunction reviewProbe() {\n    class ReviewBase {}\n    interface ReviewContract {}\n}\n\
         class ReviewChild extends ReviewBase implements ReviewContract {}\n\
         interface ReviewSub extends ReviewContract {}\n\
         enum ReviewEnum implements ReviewContract { case A; }\n\
         function reviewOther() {\n    class ReviewOtherChild extends ReviewBase {}\n}\n",
        &[Owner { qualified_name: "Example\\Symbols\\ReviewChild",
                  extends: &[to("ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")],
                  implements: &[to("ReviewContract", "Example\\Symbols\\reviewProbe::ReviewContract")], unclassified: &[] },
          bases("Example\\Symbols\\ReviewSub", &[to("ReviewContract", "Example\\Symbols\\reviewProbe::ReviewContract")]),
          implementing("Example\\Symbols\\ReviewEnum",
                &[to("ReviewContract", "Example\\Symbols\\reviewProbe::ReviewContract")]),
          bases("Example\\Symbols\\reviewOther::ReviewOtherChild",
                &[to("ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")])]),
    row("PHP rooted and relative qualified names of a class declared in a function", L::Php, "php",
        "<?php\nnamespace Example\\Symbols;\nfunction reviewProbe() {\n    class ReviewBase {}\n}\n\
         class ReviewChild extends \\Example\\Symbols\\ReviewBase {}\n\
         namespace Example;\nclass ReviewRelativeChild extends Symbols\\ReviewBase {}\n\
         namespace Other;\nclass ReviewRootedChild extends \\Example\\Symbols\\ReviewBase {}\n\
         class ReviewWrongChild extends Symbols\\ReviewBase {}\n",
        &[bases("Example\\Symbols\\ReviewChild", &[to("\\Example\\Symbols\\ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")]),
          bases("Example\\ReviewRelativeChild", &[to("Symbols\\ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")]),
          bases("Other\\ReviewRootedChild", &[to("\\Example\\Symbols\\ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")]),
          bases("Other\\ReviewWrongChild", &[text("Symbols\\ReviewBase")])]),
    row("PHP class bases declared in a method, a closure and a nested function", L::Php, "php",
        "<?php\nnamespace Example\\Symbols;\nclass ReviewHolder {\n    function make() {\n        class ReviewMethodBase {}\n    }\n}\n\
         $make = function () {\n    class ReviewClosureBase {}\n};\n\
         function reviewOuter() {\n    function reviewInner() {\n        class ReviewNestedBase {}\n    }\n}\n\
         class ReviewMethodChild extends ReviewMethodBase {}\n\
         class ReviewClosureChild extends ReviewClosureBase {}\n\
         class ReviewNestedChild extends ReviewNestedBase {}\n",
        &[bases("Example\\Symbols\\ReviewMethodChild",
                &[to("ReviewMethodBase", "Example\\Symbols\\ReviewHolder::make::ReviewMethodBase")]),
          bases("Example\\Symbols\\ReviewClosureChild", &[to("ReviewClosureBase", "Example\\Symbols\\ReviewClosureBase")]),
          bases("Example\\Symbols\\ReviewNestedChild",
                &[to("ReviewNestedBase", "Example\\Symbols\\reviewOuter::reviewInner::ReviewNestedBase")])]),
    row("PHP class bases declared in a function of a braced namespace", L::Php, "php",
        "<?php\nnamespace Example\\Symbols {\n    function reviewProbe() {\n        class ReviewBase {}\n\
         \x20       class ReviewChild extends ReviewBase {}\n    }\n    class ReviewOuterChild extends ReviewBase {}\n}\n",
        &[bases("Example\\Symbols\\reviewProbe::ReviewChild", &[to("ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")]),
          bases("Example\\Symbols\\ReviewOuterChild", &[to("ReviewBase", "Example\\Symbols\\reviewProbe::ReviewBase")])]),
    row("PHP competing function-declared classes stay text", L::Php, "php",
        "<?php\nnamespace Example\\Symbols;\nfunction reviewFirst() {\n    class ReviewBase {}\n}\n\
         function reviewSecond() {\n    class ReviewBase {}\n    class ReviewChild extends ReviewBase {}\n}\n\
         class ReviewOuterChild extends ReviewBase {}\n\
         function reviewThird() {\n    class ReviewOther {}\n}\n\
         class ReviewOther {}\nclass ReviewOtherChild extends ReviewOther {}\n",
        &[bases("Example\\Symbols\\reviewSecond::ReviewChild", &[text("ReviewBase")]),
          bases("Example\\Symbols\\ReviewOuterChild", &[text("ReviewBase")]),
          bases("Example\\Symbols\\ReviewOtherChild", &[text("ReviewOther")])]),
    row("PHP function-declared class of another namespace stays text", L::Php, "php",
        "<?php\nnamespace A;\nfunction reviewProbe() {\n    class ReviewBase {}\n}\n\
         namespace B;\nclass ReviewChild extends ReviewBase {}\n\
         function reviewProbe() {\n    class ReviewInner extends ReviewBase {}\n}\n\
         class ReviewRootedChild extends \\A\\ReviewBase {}\n",
        &[bases("B\\ReviewChild", &[text("ReviewBase")]),
          bases("B\\reviewProbe::ReviewInner", &[text("ReviewBase")]),
          bases("B\\ReviewRootedChild", &[to("\\A\\ReviewBase", "A\\reviewProbe::ReviewBase")])]),
    row("PHP conditional class bases control", L::Php, "php",
        "<?php\nnamespace Example\\Symbols;\nif (true) {\n    class ReviewBase {}\n    class ReviewChild extends ReviewBase {}\n}\n",
        &[bases("Example\\Symbols\\ReviewChild", &[to("ReviewBase", "Example\\Symbols\\ReviewBase")])]),
    row("PHP function declared in a function never hides a class", L::Php, "php",
        "<?php\nnamespace Example\\Symbols;\nclass ReviewBase {}\nfunction reviewProbe() {\n    function ReviewBase() {}\n\
         \x20   class ReviewChild extends ReviewBase {}\n}\n",
        &[bases("Example\\Symbols\\reviewProbe::ReviewChild", &[to("ReviewBase", "Example\\Symbols\\ReviewBase")])]),
    // A C++ inline namespace's members are members of the enclosing
    // namespace too, though they display under the inline namespace.
    row("C++ inline namespace members belong to the enclosing namespace", L::Cpp, "cpp",
        "namespace ns {\ninline namespace v1 {\nstruct ReviewBase {};\ninline namespace v2 { struct ReviewDeep {}; }\n\
         namespace v3 { struct ReviewPlain {}; }\n}\n\
         struct ReviewChild : ReviewBase {};\nstruct ReviewDeepChild : ReviewDeep {};\n\
         struct ReviewDeepPathChild : v2::ReviewDeep {};\nstruct ReviewPlainChild : v3::ReviewPlain {};\n\
         struct ReviewHiddenChild : ReviewPlain {};\n}\n\
         struct ReviewRootedChild : ns::ReviewBase {};\nstruct ReviewFullChild : ::ns::v1::ReviewBase {};\n",
        &[bases("ns::ReviewChild", &[to("ReviewBase", "ns::v1::ReviewBase")]),
          bases("ns::ReviewDeepChild", &[to("ReviewDeep", "ns::v1::v2::ReviewDeep")]),
          bases("ns::ReviewDeepPathChild", &[to("v2::ReviewDeep", "ns::v1::v2::ReviewDeep")]),
          bases("ns::ReviewPlainChild", &[to("v3::ReviewPlain", "ns::v1::v3::ReviewPlain")]),
          bases("ns::ReviewHiddenChild", &[text("ReviewPlain")]),
          bases("ReviewRootedChild", &[to("ns::ReviewBase", "ns::v1::ReviewBase")]),
          bases("ReviewFullChild", &[to("::ns::v1::ReviewBase", "ns::v1::ReviewBase")])]),
    row("C++ inline namespace member beside a same-named member stays text", L::Cpp, "cpp",
        "namespace ns {\nstruct ReviewBase {};\ninline namespace v1 { struct ReviewBase {}; }\nstruct ReviewChild : ReviewBase {};\n}\n\
         struct ReviewOuterChild : ns::ReviewBase {};\nstruct ReviewInnerChild : ns::v1::ReviewBase {};\n",
        &[bases("ns::ReviewChild", &[text("ReviewBase")]),
          bases("ReviewOuterChild", &[text("ns::ReviewBase")]),
          bases("ReviewInnerChild", &[to("ns::v1::ReviewBase", "ns::v1::ReviewBase")])]),
    // A Scala `package object q` has no record, and its members display
    // under the enclosing package, but they belong to package `q`.
    row("Scala package object members belong to its package", L::Scala, "scala",
        "package example\nclass ReviewTop\npackage object reviewPkg {\n  class ReviewBase\n  class ReviewChild extends ReviewBase\n\
         \x20 class ReviewUpChild extends ReviewTop\n\
         \x20 object ReviewHolder { class ReviewNested }\n  class ReviewHeldChild extends ReviewHolder.ReviewNested\n}\n\
         class ReviewOuterChild extends reviewPkg.ReviewBase\n\
         class ReviewWrongChild extends ReviewBase\n\
         class ReviewNestedChild extends reviewPkg.ReviewHolder.ReviewNested\n",
        &[bases("example.ReviewChild", &[to("ReviewBase", "example.ReviewBase")]),
          bases("example.ReviewUpChild", &[to("ReviewTop", "example.ReviewTop")]),
          bases("example.ReviewHeldChild", &[to("ReviewHolder.ReviewNested", "example.ReviewHolder.ReviewNested")]),
          bases("example.ReviewOuterChild", &[to("reviewPkg.ReviewBase", "example.ReviewBase")]),
          unclassified("example.ReviewWrongChild", &[text("ReviewBase")]),
          bases("example.ReviewNestedChild",
                &[to("reviewPkg.ReviewHolder.ReviewNested", "example.ReviewHolder.ReviewNested")])]),
    row("Scala package object imports bind in its package", L::Scala, "scala",
        "package example\nclass ReviewBase\npackage object reviewPkg {\n  import lib.ReviewBase\n  class ReviewChild extends ReviewBase\n}\n\
         class ReviewOuterChild extends ReviewBase\n",
        &[unclassified("example.ReviewChild", &[text("ReviewBase")]),
          bases("example.ReviewOuterChild", &[to("ReviewBase", "example.ReviewBase")])]),
    row("Scala package object in a block package", L::Scala, "scala",
        "package example {\n  package object reviewPkg {\n    class ReviewBase\n  }\n  class ReviewChild extends reviewPkg.ReviewBase\n\
         \x20 class ReviewWrongChild extends ReviewBase\n}\n",
        &[bases("example.ReviewChild", &[to("reviewPkg.ReviewBase", "example.ReviewBase")]),
          unclassified("example.ReviewWrongChild", &[text("ReviewBase")])]),

    // --- Type names and value names: a base clause looks up a type, so a
    // value of the same spelling never hides it; a JS/TS class `extends`
    // looks up a value.
    row("Rust values never hide a trait", L::Rust, "rs",
        "trait ReviewBase {}\nconst ReviewBase: usize = 0;\ntrait ReviewChild: ReviewBase {}\n\
         fn param(ReviewBase: u8) { trait ReviewChild: ReviewBase {} }\n\
         fn local() { let ReviewBase = 1; trait ReviewChild: ReviewBase {} }\n\
         fn function() { fn ReviewBase() {} trait ReviewChild: ReviewBase {} }\n\
         fn constant() { const ReviewBase: u8 = 0; trait ReviewChild: ReviewBase {} }\n\
         fn statics() { static ReviewBase: u8 = 0; trait ReviewChild: ReviewBase {} }\n\
         fn generic<const ReviewBase: usize>() { trait ReviewChild: ReviewBase {} }\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("param::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("local::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("function::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("constant::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("statics::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("generic::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust module qualifier beside a same-named function", L::Rust, "rs",
        "mod m { pub trait ReviewBase {} }\nfn m() {}\ntrait ReviewChild: m::ReviewBase {}\n",
        &[bases("ReviewChild", &[to("m::ReviewBase", "m::ReviewBase")])]),
    PathMeaningRow {
        label: "Rust impl in a function whose parameters spell its self type and trait",
        language: L::Rust,
        extension: "rs",
        source: "pub struct ReviewSelf;\npub trait ReviewTrait {}\n\
                 fn f(ReviewSelf: u8, ReviewTrait: u8) { impl ReviewTrait for ReviewSelf { fn probe(&self) {} } }\n",
        owners: &[implementing("ReviewSelf", &[to("ReviewTrait", "ReviewTrait")])],
        impl_members: &[ImplMember {
            method: "probe",
            container: "ReviewSelf",
            owner: Some("ReviewSelf"),
            implements: Some(to("ReviewTrait", "ReviewTrait")),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &[],
    },
    row("C++ non-types never hide a base type", L::Cpp, "cpp",
        "struct ReviewBase {};\nint ReviewBase;\nstruct ReviewChild : ReviewBase {};\n\
         void variable() { int ReviewBase = 0; struct ReviewChild : ReviewBase {}; }\n\
         void function() { void ReviewBase(); struct ReviewChild : ReviewBase {}; }\n\
         template <int ReviewBase> struct ReviewTemplateChild : ReviewBase {};\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("variable::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("function::ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("ReviewTemplateChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C++ type qualifier beside a same-named function", L::Cpp, "cpp",
        "struct Outer { struct ReviewBase {}; };\nvoid Outer(int);\nstruct ReviewChild : Outer::ReviewBase {};\n",
        &[bases("ReviewChild", &[to("Outer::ReviewBase", "Outer::ReviewBase")])]),
    row("Java values never hide a superclass", L::Java, "java",
        "class ReviewBase {}\nclass Holder {\n  int ReviewBase;\n  void ReviewBase() {}\n\
         \x20 class FieldChild extends ReviewBase {}\n\
         \x20 void param(int ReviewBase) { class ReviewChild extends ReviewBase {} }\n\
         \x20 void local() { int ReviewBase = 0; class ReviewChild extends ReviewBase {} }\n}\n",
        &[bases("Holder.FieldChild", &[to("ReviewBase", "ReviewBase")]),
          bases("Holder.param.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("Holder.local.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C# members never hide a base class", L::CSharp, "cs",
        "class ReviewBase {}\ninterface IReviewOther {}\n\
         class FieldHolder { int ReviewBase; class ReviewChild : ReviewBase, IReviewOther {} }\n\
         class PropertyHolder { int ReviewBase { get; set; } class ReviewChild : ReviewBase, IReviewOther {} }\n\
         class MethodHolder { void ReviewBase() {} class ReviewChild : ReviewBase, IReviewOther {} }\n",
        &[Owner { qualified_name: "FieldHolder.ReviewChild", extends: &[to("ReviewBase", "ReviewBase")],
                  implements: &[to("IReviewOther", "IReviewOther")], unclassified: &[] },
          Owner { qualified_name: "PropertyHolder.ReviewChild", extends: &[to("ReviewBase", "ReviewBase")],
                  implements: &[to("IReviewOther", "IReviewOther")], unclassified: &[] },
          Owner { qualified_name: "MethodHolder.ReviewChild", extends: &[to("ReviewBase", "ReviewBase")],
                  implements: &[to("IReviewOther", "IReviewOther")], unclassified: &[] }]),
    row("Swift values never hide a superclass", L::Swift, "swift",
        "class ReviewBase {}\n\
         func param(ReviewBase: Int) {\n    class ReviewChild: ReviewBase {}\n}\n\
         func local() {\n    let ReviewBase = 1\n    class ReviewChild: ReviewBase {}\n}\n",
        &[bases("param.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("local.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala terms never hide a class", L::Scala, "scala",
        "class ReviewBase\n\
         object ValHolder {\n  val ReviewBase = 1\n  class ReviewChild extends ReviewBase\n}\n\
         object DefHolder {\n  def ReviewBase = 1\n  class ReviewChild extends ReviewBase\n}\n",
        &[bases("ValHolder.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("DefHolder.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala companions", L::Scala, "scala",
        "class ReviewBase\nobject ReviewBase\ntrait ReviewTrait\nobject ReviewTrait\n\
         class ReviewChild extends ReviewBase with ReviewTrait\nclass ReviewTraitChild extends ReviewTrait\n",
        &[Owner { qualified_name: "ReviewChild", extends: &[to("ReviewBase", "ReviewBase")],
                  implements: &[to("ReviewTrait", "ReviewTrait")], unclassified: &[] },
          implementing("ReviewTraitChild", &[to("ReviewTrait", "ReviewTrait")])]),
    row("TypeScript type relations never see values", L::TypeScript, "ts",
        "interface ReviewBase {}\n\
         function param(ReviewBase: number) { class ReviewChild implements ReviewBase {} interface ReviewChildType extends ReviewBase {} }\n\
         function local() { let ReviewBase = 1; class ReviewChild implements ReviewBase {} interface ReviewChildType extends ReviewBase {} }\n",
        &[implementing("param.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("param.ReviewChildType", &[to("ReviewBase", "ReviewBase")]),
          implementing("local.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("local.ReviewChildType", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript namespace path of several segments", L::TypeScript, "ts",
        "namespace ReviewOuter { export namespace ReviewInner { export interface ReviewBase {} } }\n\
         namespace ReviewDotted.ReviewPart { export interface ReviewBase {} }\n\
         class ReviewChild implements ReviewOuter.ReviewInner.ReviewBase {}\n\
         class ReviewDottedChild implements ReviewDotted.ReviewPart.ReviewBase {}\n\
         class ReviewMissingChild implements ReviewOuter.ReviewOther.ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewOuter.ReviewInner.ReviewBase", "ReviewOuter.ReviewInner.ReviewBase")]),
          implementing("ReviewDottedChild", &[to("ReviewDotted.ReviewPart.ReviewBase", "ReviewDotted.ReviewPart.ReviewBase")]),
          implementing("ReviewMissingChild", &[text("ReviewOuter.ReviewOther.ReviewBase")])]),
    // A `declare global` interface merges with every global declaration of
    // that name in the program, so one file never proves it.
    row("TypeScript `declare global` declarations stay text", L::TypeScript, "ts",
        "export {};\ndeclare global {\n  interface ReviewGlobal {}\n}\nclass ReviewChild implements ReviewGlobal {}\n",
        &[implementing("ReviewChild", &[text("ReviewGlobal")])]),
    row("TypeScript namespace qualifier in a type beside a value parameter", L::TypeScript, "ts",
        "namespace ns { export interface ReviewBase {} }\nfunction f(ns: number) { class ReviewChild implements ns.ReviewBase {} }\n",
        &[implementing("f.ReviewChild", &[to("ns.ReviewBase", "ns.ReviewBase")])]),
    row("PHP functions never hide a class", L::Php, "php",
        "<?php\nnamespace App;\nclass ReviewBase {}\nfunction ReviewBase() {}\n$ReviewBase = 1;\nclass ReviewChild extends ReviewBase {}\n",
        &[bases("App\\ReviewChild", &[to("ReviewBase", "App\\ReviewBase")])]),
    row("TypeScript class `extends` is a value", L::TypeScript, "ts",
        "class ReviewBase {}\nfunction f(ReviewBase: any) { class ReviewChild extends ReviewBase {} }\n",
        &[bases("f.ReviewChild", &[text("ReviewBase")])]),
    row("JavaScript class `extends` is a value", L::JavaScript, "js",
        "class ReviewBase {}\nfunction f(ReviewBase) { class ReviewChild extends ReviewBase {} }\n",
        &[bases("f.ReviewChild", &[text("ReviewBase")])]),
    row("JavaScript `with` body and direct `eval` may bind the base", L::JavaScript, "js",
        "class ReviewBase {}\n\
         function w(o) { with (o) { class ReviewChild extends ReviewBase {} } }\n\
         function e() { eval(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function n() { class ReviewChild extends ReviewBase {} }\n",
        &[bases("w.ReviewChild", &[text("ReviewBase")]),
          bases("e.ReviewChild", &[text("ReviewBase")]),
          bases("n.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript `with` body and direct `eval` may bind the base", L::TypeScript, "ts",
        "class ReviewBase {}\n\
         function w(o: any) { with (o) { class ReviewChild extends ReviewBase {} } }\n\
         function e() { eval(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function n() { class ReviewChild extends ReviewBase {} }\n",
        &[bases("w.ReviewChild", &[text("ReviewBase")]),
          bases("e.ReviewChild", &[text("ReviewBase")]),
          bases("n.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    // Only a direct `eval` may bind the base: an optional call is an
    // indirect eval, a tagged template evaluates nothing, and `eval`
    // through another value is indirect.
    row("JavaScript optional, tagged and indirect `eval` keep the base", L::JavaScript, "js",
        "class ReviewBase {}\n\
         function o() { eval?.(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function p() { (eval)?.(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function t() { eval`1`; class ReviewChild extends ReviewBase {} }\n\
         function u() { (eval)`1`; class ReviewChild extends ReviewBase {} }\n\
         function s() { (0, eval)(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function m() { globalThis.eval(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function a() { const e = eval; e(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function d() { ((eval))(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function outer() { function f() { eval?.(\"var f = 1\"); class D extends ReviewBase {} f(); } }\n",
        &[bases("o.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("p.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("t.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("u.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("s.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("m.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("a.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("d.ReviewChild", &[text("ReviewBase")]),
          bases("outer.f.D", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript optional, tagged and indirect `eval` keep the base", L::TypeScript, "ts",
        "class ReviewBase {}\n\
         function o() { eval?.(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function p() { (eval as any)?.(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function t() { eval`1`; class ReviewChild extends ReviewBase {} }\n\
         function u() { (eval!)`1`; class ReviewChild extends ReviewBase {} }\n\
         function s() { (0, eval)(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function m() { globalThis.eval(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function a() { const e = eval; e(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function d() { (eval as any)(\"1\"); class ReviewChild extends ReviewBase {} }\n\
         function outer() { function f() { eval`var f = 1`; class D extends ReviewBase {} f(); } }\n",
        &[bases("o.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("p.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("t.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("u.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("s.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("m.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("a.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("d.ReviewChild", &[text("ReviewBase")]),
          bases("outer.f.D", &[to("ReviewBase", "ReviewBase")])]),
    row("JavaScript named function and generator expressions bind their own name", L::JavaScript, "js",
        "class ReviewBase {}\n\
         const g = function ReviewBase() { class ReviewFunction extends ReviewBase {} };\n\
         const h = function* ReviewBase() { class ReviewGenerator extends ReviewBase {} };\n\
         const k = function () { class ReviewUnnamed extends ReviewBase {} };\n\
         const G = class ReviewBase { m() { class ReviewClass extends ReviewBase {} } };\n",
        &[bases("ReviewBase.ReviewFunction", &[text("ReviewBase")]),
          bases("ReviewBase.ReviewGenerator", &[text("ReviewBase")]),
          bases("ReviewUnnamed", &[to("ReviewBase", "ReviewBase")]),
          bases("m.ReviewClass", &[text("ReviewBase")])]),
    row("TypeScript named function and generator expressions bind their own name", L::TypeScript, "ts",
        "class ReviewBase {}\n\
         const g = function ReviewBase() { class ReviewFunction extends ReviewBase {} };\n\
         const h = function* ReviewBase() { class ReviewGenerator extends ReviewBase {} };\n\
         const k = function () { class ReviewUnnamed extends ReviewBase {} };\n\
         const G = class ReviewBase { m() { class ReviewClass extends ReviewBase {} } };\n",
        &[bases("ReviewBase.ReviewFunction", &[text("ReviewBase")]),
          bases("ReviewBase.ReviewGenerator", &[text("ReviewBase")]),
          bases("ReviewUnnamed", &[to("ReviewBase", "ReviewBase")]),
          bases("m.ReviewClass", &[text("ReviewBase")])]),
    row("JavaScript parameter defaults never see the body's `var`", L::JavaScript, "js",
        "class ReviewBase {}\n\
         function g(x = function () { class ReviewDefault extends ReviewBase {} }) { var ReviewBase = 1; }\n\
         function h(x = 1) { var ReviewBase = 1; class ReviewBody extends ReviewBase {} }\n",
        &[bases("g.ReviewDefault", &[to("ReviewBase", "ReviewBase")]),
          bases("h.ReviewBody", &[text("ReviewBase")])]),
    row("TypeScript parameter defaults never see the body's `var`", L::TypeScript, "ts",
        "class ReviewBase {}\n\
         function g(x = function () { class ReviewDefault extends ReviewBase {} }) { var ReviewBase = 1; }\n\
         function h(x = 1) { var ReviewBase = 1; class ReviewBody extends ReviewBase {} }\n",
        &[bases("g.ReviewDefault", &[to("ReviewBase", "ReviewBase")]),
          bases("h.ReviewBody", &[text("ReviewBase")])]),
    row("C++ parameter defaults never see the body's declarations", L::Cpp, "cpp",
        "struct ReviewBase {};\n\
         void g(int x = [] { struct ReviewDefault : ReviewBase {}; return 0; }()) { struct ReviewBase {}; }\n",
        &[bases("g::ReviewDefault", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala parameter defaults never see the body's declarations", L::Scala, "scala",
        "class ReviewBase\n\
         object O {\n  def g(x: Any = { class ReviewDefault extends ReviewBase; 1 }): Int = { class ReviewBase; 2 }\n}\n",
        &[bases("O.g.ReviewDefault", &[to("ReviewBase", "ReviewBase")])]),
    row("Swift parameter defaults never see the body's declarations", L::Swift, "swift",
        "class ReviewBase {}\n\
         func g(x: Int = { class ReviewDefault: ReviewBase {}; return 0 }()) { class ReviewBase {} }\n",
        &[bases("g.ReviewDefault", &[to("ReviewBase", "ReviewBase")])]),
    row("Python simple parameter", L::Python, "py",
        "class ReviewBase:\n    pass\n\ndef f(ReviewBase):\n    class ReviewChild(ReviewBase):\n        pass\n",
        &[bases("f.ReviewChild", &[text("ReviewBase")])]),
    row("Go value binding keeps the base unresolved", L::Go, "go",
        "package shapes\n\ntype ReviewBase interface{ M() }\n\n\
         func f() {\n\tvar ReviewBase = 1\n\t_ = ReviewBase\n\ttype ReviewChild interface{ ReviewBase }\n}\n",
        &[bases("shapes.f.ReviewChild", &[text("ReviewBase")])]),

    // --- Binding patterns no record reports, and anonymous owners.
    row("TypeScript parameter patterns", L::TypeScript, "ts", ECMA_PARAMETER_PATTERNS, ECMA_PARAMETER_OWNERS),
    row("JavaScript parameter patterns", L::JavaScript, "js", ECMA_PARAMETER_PATTERNS, ECMA_PARAMETER_OWNERS),
    row("TypeScript variable patterns", L::TypeScript, "ts", ECMA_VARIABLE_PATTERNS, ECMA_VARIABLE_OWNERS),
    row("JavaScript variable patterns", L::JavaScript, "js", ECMA_VARIABLE_PATTERNS, ECMA_VARIABLE_OWNERS),
    row("TypeScript catch and for-of patterns", L::TypeScript, "ts", ECMA_CATCH_AND_FOR_PATTERNS, ECMA_CATCH_AND_FOR_OWNERS),
    row("JavaScript catch and for-of patterns", L::JavaScript, "js", ECMA_CATCH_AND_FOR_PATTERNS, ECMA_CATCH_AND_FOR_OWNERS),
    row("Python pattern bindings", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         def nested_tuple(x):\n    (a, (ReviewBase, b)) = x\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def nested_list(x):\n    [a, [ReviewBase]] = x\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def starred(x):\n    a, *ReviewBase = x\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def for_tuple(xs):\n    for a, (ReviewBase, b) in xs:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def with_tuple(x):\n    with x as (a, (ReviewBase, b)):\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def match_capture(x):\n    match x:\n        case ReviewBase:\n            pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def match_nested(x):\n    match x:\n        case (a, [ReviewBase]):\n            pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def flat_tuple(x):\n    a, ReviewBase = x\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def except_alias():\n    try:\n        pass\n    except Exception as ReviewBase:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def walrus(x):\n    if (ReviewBase := x):\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def simple_assignment(x):\n    ReviewBase = x\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def simple_for(xs):\n    for ReviewBase in xs:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def simple_with(x):\n    with x as ReviewBase:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def match_value(x):\n    match x:\n        case ReviewBase.attr:\n            pass\n    class ReviewChild(ReviewBase):\n        pass\n",
        &[bases("nested_tuple.ReviewChild", &[text("ReviewBase")]),
          bases("nested_list.ReviewChild", &[text("ReviewBase")]),
          bases("starred.ReviewChild", &[text("ReviewBase")]),
          bases("for_tuple.ReviewChild", &[text("ReviewBase")]),
          bases("with_tuple.ReviewChild", &[text("ReviewBase")]),
          bases("match_capture.ReviewChild", &[text("ReviewBase")]),
          bases("match_nested.ReviewChild", &[text("ReviewBase")]),
          bases("flat_tuple.ReviewChild", &[text("ReviewBase")]),
          bases("except_alias.ReviewChild", &[text("ReviewBase")]),
          bases("walrus.ReviewChild", &[text("ReviewBase")]),
          bases("simple_assignment.ReviewChild", &[text("ReviewBase")]),
          bases("simple_for.ReviewChild", &[text("ReviewBase")]),
          bases("simple_with.ReviewChild", &[text("ReviewBase")]),
          bases("match_value.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Python `global` and `nonlocal` directives", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         def outer():\n    class ReviewBase:\n        pass\n\n\
         \x20   def by_global():\n        global ReviewBase\n        class ReviewChild(ReviewBase):\n            pass\n\n\
         \x20   def by_nonlocal():\n        nonlocal ReviewBase\n        class ReviewChild(ReviewBase):\n            pass\n",
        &[bases("outer.by_global.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("outer.by_nonlocal.ReviewChild", &[to("ReviewBase", "outer.ReviewBase")])]),
    python_global_row!("write: simple assignment", "        ReviewBase = external\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: simple assignment", "        ReviewBase = external\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: nested tuple target", "        (a, (ReviewBase, b)) = external\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: nested tuple target", "        (a, (ReviewBase, b)) = external\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: nested list target", "        [a, [ReviewBase]] = external\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: nested list target", "        [a, [ReviewBase]] = external\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: starred target", "        a, *ReviewBase = external\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: starred target", "        a, *ReviewBase = external\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: augmented assignment", "        ReviewBase += external\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: augmented assignment", "        ReviewBase += external\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: `from` import", "        from external import ReviewBase\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: `from` import", "        from external import ReviewBase\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: aliased import", "        import external as ReviewBase\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: aliased import", "        import external as ReviewBase\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: class declared after the directive", "        class ReviewBase:\n            pass\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: class declared after the directive", "        class ReviewBase:\n            pass\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: function declared after the directive", "        def ReviewBase():\n            pass\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: function declared after the directive", "        def ReviewBase():\n            pass\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: nested `for` target", "        for a, (ReviewBase, b) in external:\n            pass\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: nested `for` target", "        for a, (ReviewBase, b) in external:\n            pass\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: nested `with` target", "        with external as (a, (ReviewBase, b)):\n            pass\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: nested `with` target", "        with external as (a, (ReviewBase, b)):\n            pass\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: `except` alias", "        try:\n            pass\n        except Exception as ReviewBase:\n            pass\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: `except` alias", "        try:\n            pass\n        except Exception as ReviewBase:\n            pass\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: walrus", "        if (ReviewBase := external):\n            pass\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: walrus", "        if (ReviewBase := external):\n            pass\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: walrus in a comprehension", "        xs = [(ReviewBase := y) for y in external]\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: walrus in a comprehension", "        xs = [(ReviewBase := y) for y in external]\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("write: `del`", "        del ReviewBase\n", PY_GLOBAL_WRITTEN),
    python_nonlocal_row!("write: `del`", "        del ReviewBase\n", PY_NONLOCAL_WRITTEN),
    python_global_row!("control: read only", "        external(ReviewBase)\n", PY_GLOBAL_KEPT),
    python_nonlocal_row!("control: read only", "        external(ReviewBase)\n", PY_NONLOCAL_KEPT),
    python_global_row!("control: unrelated assignment", "        other = external\n", PY_GLOBAL_KEPT),
    python_nonlocal_row!("control: unrelated assignment", "        other = external\n", PY_NONLOCAL_KEPT),
    python_global_row!("control: comprehension target", "        xs = [ReviewBase for ReviewBase in external]\n", PY_GLOBAL_KEPT),
    python_nonlocal_row!("control: comprehension target", "        xs = [ReviewBase for ReviewBase in external]\n", PY_NONLOCAL_KEPT),
    python_global_row!("control: write in a nested function", "        def reviewInner():\n            ReviewBase = external\n", PY_GLOBAL_KEPT),
    python_nonlocal_row!("control: write in a nested function", "        def reviewInner():\n            ReviewBase = external\n", PY_NONLOCAL_KEPT),
    row("Python augmented assignment and `del` targets bind without a directive", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         def augmented(x):\n    ReviewBase += x\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def deleted():\n    del ReviewBase\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         def deleted_item(x):\n    del x[ReviewBase]\n    class ReviewChild(ReviewBase):\n        pass\n",
        &[bases("augmented.ReviewChild", &[text("ReviewBase")]),
          bases("deleted.ReviewChild", &[text("ReviewBase")]),
          bases("deleted_item.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Python `global` write in a module-level function", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         def reviewProbe(external):\n    global ReviewBase\n    ReviewBase = external\n    class ReviewChild(ReviewBase):\n        pass\n",
        &[bases("reviewProbe.ReviewChild", &[text("ReviewBase")])]),
    row("Python `global` class with no module declaration", L::Python, "py",
        "def reviewProbe():\n    global ReviewBase\n    class ReviewBase:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\n\
         class ReviewModuleChild(ReviewBase):\n    pass\n",
        &[bases("reviewProbe.ReviewChild", &[text("ReviewBase")]), bases("ReviewModuleChild", &[text("ReviewBase")])]),
    row("Python `global` write in a module-level class body", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         class ReviewHolder:\n    global ReviewBase\n    ReviewBase = 1\n\n\
         class ReviewModuleChild(ReviewBase):\n    pass\n",
        &[bases("ReviewModuleChild", &[text("ReviewBase")])]),
    row("Python `nonlocal` write in a class body", L::Python, "py",
        "def reviewOuter():\n    class ReviewBase:\n        pass\n\n\
         \x20   class ReviewHolder:\n        nonlocal ReviewBase\n        ReviewBase = 1\n\n\
         \x20   class ReviewOuterChild(ReviewBase):\n        pass\n",
        &[bases("reviewOuter.ReviewOuterChild", &[text("ReviewBase")])]),
    row("Python chained `nonlocal` write", L::Python, "py",
        "def reviewOuter():\n    class ReviewBase:\n        pass\n\n\
         \x20   def reviewMiddle():\n        nonlocal ReviewBase\n\n\
         \x20       def reviewInner(external):\n            nonlocal ReviewBase\n            ReviewBase = external\n\n\
         \x20       class ReviewMiddleChild(ReviewBase):\n            pass\n\n\
         \x20   class ReviewOuterChild(ReviewBase):\n        pass\n",
        &[bases("reviewOuter.reviewMiddle.ReviewMiddleChild", &[text("ReviewBase")]),
          bases("reviewOuter.ReviewOuterChild", &[text("ReviewBase")])]),
    row("Python `nonlocal` skips a function that does not bind the name", L::Python, "py",
        "def reviewOuter():\n    class ReviewBase:\n        pass\n\n\
         \x20   def reviewMiddle():\n        class ReviewMiddleChild(ReviewBase):\n            pass\n\n\
         \x20       def reviewInner(external):\n            nonlocal ReviewBase\n            ReviewBase = external\n\n\
         \x20   class ReviewOuterChild(ReviewBase):\n        pass\n",
        &[bases("reviewOuter.reviewMiddle.ReviewMiddleChild", &[text("ReviewBase")]),
          bases("reviewOuter.ReviewOuterChild", &[text("ReviewBase")])]),
    row("Python `global` dictionary `match` capture", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         def reviewProbe(external):\n    global ReviewBase\n    match external:\n        case {\"k\": ReviewBase}:\n\
         \x20           class ReviewChild(ReviewBase):\n                pass\n",
        &[bases("reviewProbe.ReviewChild", &[text("ReviewBase")])]),
    row("Python `nonlocal` dictionary `match` capture", L::Python, "py",
        "class ReviewBase:\n    pass\n\n\
         def reviewOuter():\n    class ReviewBase:\n        pass\n\n\
         \x20   def reviewProbe(external):\n        nonlocal ReviewBase\n        match external:\n            case {\"k\": ReviewBase}:\n\
         \x20               class ReviewChild(ReviewBase):\n                    pass\n\n\
         class ReviewModuleChild(ReviewBase):\n    pass\n",
        &[bases("reviewOuter.reviewProbe.ReviewChild", &[text("ReviewBase")]),
          bases("ReviewModuleChild", &[to_at("ReviewBase", "ReviewBase", 1)])]),
    row("Scala pattern-bound qualifiers", L::Scala, "scala",
        "object reviewNs { class ReviewBase }\nobject Holder {\n\
         \x20 def tuple(): Unit = {\n    val (a, reviewNs) = (1, 2)\n    class ReviewChild extends reviewNs.ReviewBase\n  }\n\
         \x20 def nested(): Unit = {\n    val (a, (b, reviewNs)) = (1, (2, 3))\n    class ReviewChild extends reviewNs.ReviewBase\n  }\n\
         \x20 def extractor(): Unit = {\n    val Some(reviewNs) = Option(1)\n    class ReviewChild extends reviewNs.ReviewBase\n  }\n\
         \x20 def forName(xs: List[Int]): Unit = {\n    for (reviewNs <- xs) { class ReviewChild extends reviewNs.ReviewBase }\n  }\n\
         \x20 def forNested(xs: List[(Int, (Int, Int))]): Unit = {\n    for ((a, (b, reviewNs)) <- xs) { class ReviewChild extends reviewNs.ReviewBase }\n  }\n\
         \x20 def capture(x: Int): Unit = x match {\n    case reviewNs => { class ReviewChild extends reviewNs.ReviewBase }\n  }\n\
         \x20 def nestedCapture(x: (Int, Int)): Unit = x match {\n    case (a, reviewNs) => { class ReviewChild extends reviewNs.ReviewBase }\n  }\n\
         \x20 def simple(): Unit = {\n    val reviewNs = 1\n    class ReviewChild extends reviewNs.ReviewBase\n  }\n\
         \x20 def forTuple(xs: List[(Int, Int)]): Unit = {\n    for ((a, reviewNs) <- xs) { class ReviewChild extends reviewNs.ReviewBase }\n  }\n\
         \x20 def plain(): Unit = {\n    class ReviewChild extends reviewNs.ReviewBase\n  }\n}\n",
        &[unclassified("Holder.tuple.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.nested.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.extractor.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.forName.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.forNested.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.capture.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.nestedCapture.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.simple.ReviewChild", &[text("reviewNs.ReviewBase")]),
          unclassified("Holder.forTuple.ReviewChild", &[text("reviewNs.ReviewBase")]),
          bases("Holder.plain.ReviewChild", &[to("reviewNs.ReviewBase", "reviewNs.ReviewBase")])]),
    row("Scala pattern variables by lexical form, binders by position", L::Scala, "scala",
        "object _reviewNs { class ReviewBase }\nobject ReviewNs { class ReviewBase }\nobject Holder {\n\
         \x20 def underscore(x: Int): Unit = x match {\n    case _reviewNs => { class ReviewChild extends _reviewNs.ReviewBase }\n  }\n\
         \x20 def binder(x: Option[Int]): Unit = x match {\n    case ReviewNs @ Some(_) => { class ReviewChild extends ReviewNs.ReviewBase }\n  }\n\
         \x20 def stable(x: Any): Unit = x match {\n    case ReviewNs => { class ReviewChild extends ReviewNs.ReviewBase }\n  }\n}\n",
        &[unclassified("Holder.underscore.ReviewChild", &[text("_reviewNs.ReviewBase")]),
          unclassified("Holder.binder.ReviewChild", &[text("ReviewNs.ReviewBase")]),
          bases("Holder.stable.ReviewChild", &[to("ReviewNs.ReviewBase", "ReviewNs.ReviewBase")])]),
    row("Scala typed lambda parameter qualifier", L::Scala, "scala",
        "object reviewNs { class ReviewBase }\nobject Holder {\n  val f = (reviewNs: Int) => { class ReviewChild extends reviewNs.ReviewBase }\n}\n",
        &[unclassified("Holder.ReviewChild", &[text("reviewNs.ReviewBase")])]),
    row("TypeScript anonymous generic class", L::TypeScript, "ts",
        "interface ReviewBase {}\n\
         const Box = class<ReviewBase> { method() { class ReviewChild implements ReviewBase {} } };\n\
         const Plain = class { other() { class ReviewPlainChild implements ReviewBase {} } };\n",
        &[implementing("method.ReviewChild", &[text("ReviewBase")]),
          implementing("other.ReviewPlainChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C++ generic lambda template parameter", L::Cpp, "cpp",
        "struct ReviewBase {};\n\
         void f() {\n  auto generic = []<class ReviewBase>() { struct ReviewChild : ReviewBase {}; };\n\
         \x20 auto plain = []() { struct ReviewPlainChild : ReviewBase {}; };\n}\n",
        &[bases("f::ReviewChild", &[text("ReviewBase")]),
          bases("f::ReviewPlainChild", &[to("ReviewBase", "ReviewBase")])]),
    row("C++ template parameter of another template is no target", L::Cpp, "cpp",
        "template <class ReviewBase> struct Wrapper {};\nstruct ReviewChild : ReviewBase {};\n",
        &[bases("ReviewChild", &[text("ReviewBase")])]),
    row("Scala polymorphic lambda", L::Scala, "scala",
        "class ReviewBase\nobject Holder {\n  def f(): Unit = {\n    val g = [ReviewBase] => () => { class ReviewChild extends ReviewBase }\n  }\n}\n",
        &[unclassified("Holder.f.ReviewChild", &[text("ReviewBase")])]),
    row("Swift extension of a generic owner", L::Swift, "swift",
        "class ReviewBase {}\nstruct ReviewBox<ReviewBase> {}\nextension ReviewBox {\n    class ReviewChild: ReviewBase {}\n}\n\
         struct ReviewPlain {}\nextension ReviewPlain {\n    class ReviewChild: ReviewBase {}\n}\n",
        &[unclassified("ReviewBox.ReviewChild", &[text("ReviewBase")]),
          bases("ReviewPlain.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Java anonymous class inherits member types", L::Java, "java",
        "class ReviewBase {}\nclass External {}\n\
         class Holder {\n  Object field = new External() { class ReviewChild extends ReviewBase {} };\n\
         \x20 void m() { Object local = new External() { class ReviewLocalChild extends ReviewBase {} }; }\n}\n",
        &[bases("ReviewChild", &[text("ReviewBase")]), bases("ReviewLocalChild", &[text("ReviewBase")])]),
    row("C++ anonymous struct inherits member types", L::Cpp, "cpp",
        "struct ReviewBase {};\nstruct External {};\n\
         struct : External { struct ReviewChild : ReviewBase {}; } inherited;\n\
         struct { struct ReviewPlainChild : ReviewBase {}; } plain;\n",
        &[bases("ReviewChild", &[text("ReviewBase")]), bases("ReviewPlainChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala anonymous class inherits member types", L::Scala, "scala",
        "class ReviewBase\nclass External\nobject Holder {\n  val o = new External { class ReviewChild extends ReviewBase }\n}\n",
        &[unclassified("ReviewChild", &[text("ReviewBase")])]),

    // --- Imports decide qualifiers and name groups.
    row("Go imported package named like the file's package", L::Go, "go",
        "package symbols\n\nimport symbols \"io\"\n\ntype Reader interface{ M() }\n\ntype Child interface{ symbols.Reader }\n",
        &[bases("symbols.Child", &[text("symbols.Reader")])]),
    row("Java imported type named like the file's package", L::Java, "java",
        "package Entry;\nimport java.util.Map.Entry;\nclass ReviewBase {}\nclass Child extends Entry.ReviewBase {}\n",
        &[bases("Entry.Child", &[text("Entry.ReviewBase")])]),
    row("Java package-qualified name without an import", L::Java, "java",
        "package Entry;\nclass ReviewBase {}\nclass Child extends Entry.ReviewBase {}\n",
        &[bases("Entry.Child", &[to("Entry.ReviewBase", "Entry.ReviewBase")])]),
    row("Java on-demand import keeps local types", L::Java, "java",
        "import java.util.*;\nclass ReviewBase {}\nclass Child extends ReviewBase {}\n",
        &[bases("Child", &[to("ReviewBase", "ReviewBase")])]),
    swift_raw_type_names!(swift_value_import_func_row),
    swift_raw_type_names!(swift_value_import_let_row),
    swift_raw_type_names!(swift_value_import_var_row),
    row("Swift value imports never hide a local class", L::Swift, "swift",
        "import func Lib.ReviewBase\nimport let Lib.ReviewOther\nimport var Lib.ReviewThird\n\
         class ReviewBase {}\nclass ReviewOther {}\nclass ReviewThird {}\n\
         class ReviewChild: ReviewBase {}\nclass ReviewOtherChild: ReviewOther {}\nclass ReviewThirdChild: ReviewThird {}\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("ReviewOtherChild", &[to("ReviewOther", "ReviewOther")]),
          bases("ReviewThirdChild", &[to("ReviewThird", "ReviewThird")])]),
    row("Swift type imports bind the name", L::Swift, "swift",
        "import struct Lib.ReviewS\nimport enum Lib.ReviewE\nimport protocol Lib.ReviewP\nimport typealias Lib.ReviewT\n\
         class ReviewS {}\nclass ReviewE {}\nclass ReviewP {}\nclass ReviewT {}\n\
         class ChildS: ReviewS {}\nclass ChildE: ReviewE {}\nclass ChildP: ReviewP {}\nclass ChildT: ReviewT {}\n",
        &[unclassified("ChildS", &[text("ReviewS")]), unclassified("ChildE", &[text("ReviewE")]),
          unclassified("ChildP", &[text("ReviewP")]), unclassified("ChildT", &[text("ReviewT")])]),

    // --- Rust impl consumers: a sibling block's types never own an impl.
    PathMeaningRow {
        label: "Rust impl beside a sibling block's same-named enum and trait",
        language: L::Rust,
        extension: "rs",
        source: "pub struct ReviewSelf;\npub trait ReviewTrait {}\n\
                 fn f() {\n    { enum ReviewSelf {} trait ReviewTrait {} }\n    \
                 impl ReviewTrait for ReviewSelf { fn probe(&self) {} }\n}\n",
        owners: &[implementing("ReviewSelf", &[to("ReviewTrait", "ReviewTrait")])],
        impl_members: &[ImplMember {
            method: "probe",
            container: "ReviewSelf",
            owner: Some("ReviewSelf"),
            implements: Some(to("ReviewTrait", "ReviewTrait")),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &["f::ReviewSelf"],
    },

    // --- Qualified continuation stays inside the owner the first segment
    // selected: a same-named owner in a sibling block contributes nothing,
    // whether or not it declares the member. Equal names are told apart by
    // the target's declaration line.
    row("C++ local owner paths keep the selected owner", L::Cpp, "cpp",
        "void one() { struct ReviewHolder { struct ReviewBase {}; }; struct ReviewChild : ReviewHolder::ReviewBase {}; }\n\
         void both() {\n\
         { struct ReviewHolder { struct ReviewBase {}; }; }\n\
         { struct ReviewHolder { struct ReviewBase {}; };\n\
         struct ReviewChild : ReviewHolder::ReviewBase {}; }\n\
         }\n\
         void lacks() {\n\
         { struct ReviewHolder { struct ReviewBase {}; }; }\n\
         { struct ReviewHolder {};\n\
         struct ReviewChild : ReviewHolder::ReviewBase {}; }\n\
         }\n\
         void other() { { struct ReviewOther { struct ReviewBase {}; }; } struct ReviewHolder { struct ReviewBase {}; }; struct ReviewChild : ReviewHolder::ReviewBase {}; }\n",
        &[bases("one::ReviewChild", &[to("ReviewHolder::ReviewBase", "one::ReviewHolder::ReviewBase")]),
          bases("both::ReviewChild", &[to_at("ReviewHolder::ReviewBase", "both::ReviewHolder::ReviewBase", 4)]),
          bases("lacks::ReviewChild", &[text("ReviewHolder::ReviewBase")]),
          bases("other::ReviewChild", &[to("ReviewHolder::ReviewBase", "other::ReviewHolder::ReviewBase")])]),
    row("C++ three-segment local owner paths keep the selected owner", L::Cpp, "cpp",
        "void one() { struct ReviewHolder { struct ReviewMiddle { struct ReviewBase {}; }; }; struct ReviewChild : ReviewHolder::ReviewMiddle::ReviewBase {}; }\n\
         void both() {\n\
         { struct ReviewHolder { struct ReviewMiddle { struct ReviewBase {}; }; }; }\n\
         { struct ReviewHolder { struct ReviewMiddle { struct ReviewBase {}; }; };\n\
         struct ReviewChild : ReviewHolder::ReviewMiddle::ReviewBase {}; }\n\
         }\n\
         void lacks() {\n\
         { struct ReviewHolder { struct ReviewMiddle { struct ReviewBase {}; }; }; }\n\
         { struct ReviewHolder { struct ReviewMiddle {}; };\n\
         struct ReviewChild : ReviewHolder::ReviewMiddle::ReviewBase {}; }\n\
         }\n\
         void lacksMiddle() {\n\
         { struct ReviewHolder { struct ReviewMiddle { struct ReviewBase {}; }; }; }\n\
         { struct ReviewHolder {};\n\
         struct ReviewChild : ReviewHolder::ReviewMiddle::ReviewBase {}; }\n\
         }\n\
         void other() { { struct ReviewOther { struct ReviewMiddle { struct ReviewBase {}; }; }; } struct ReviewHolder { struct ReviewMiddle { struct ReviewBase {}; }; }; struct ReviewChild : ReviewHolder::ReviewMiddle::ReviewBase {}; }\n",
        &[bases("one::ReviewChild", &[to("ReviewHolder::ReviewMiddle::ReviewBase", "one::ReviewHolder::ReviewMiddle::ReviewBase")]),
          bases("both::ReviewChild", &[to_at("ReviewHolder::ReviewMiddle::ReviewBase", "both::ReviewHolder::ReviewMiddle::ReviewBase", 4)]),
          bases("lacks::ReviewChild", &[text("ReviewHolder::ReviewMiddle::ReviewBase")]),
          bases("lacksMiddle::ReviewChild", &[text("ReviewHolder::ReviewMiddle::ReviewBase")]),
          bases("other::ReviewChild", &[to("ReviewHolder::ReviewMiddle::ReviewBase", "other::ReviewHolder::ReviewMiddle::ReviewBase")])]),
    row("Java local owner paths keep the selected owner", L::Java, "java",
        "class Probe {\n\
         void one() { class ReviewHolder { class ReviewBase {} } class ReviewChild extends ReviewHolder.ReviewBase {} }\n\
         void both() {\n\
         { class ReviewHolder { class ReviewBase {} } }\n\
         { class ReviewHolder { class ReviewBase {} }\n\
         class ReviewChild extends ReviewHolder.ReviewBase {} }\n\
         }\n\
         void lacks() {\n\
         { class ReviewHolder { class ReviewBase {} } }\n\
         { class ReviewHolder {}\n\
         class ReviewChild extends ReviewHolder.ReviewBase {} }\n\
         }\n\
         void other() { { class ReviewOther { class ReviewBase {} } } class ReviewHolder { class ReviewBase {} } class ReviewChild extends ReviewHolder.ReviewBase {} }\n\
         }\n",
        &[bases("Probe.one.ReviewChild", &[to("ReviewHolder.ReviewBase", "Probe.one.ReviewHolder.ReviewBase")]),
          bases("Probe.both.ReviewChild", &[to_at("ReviewHolder.ReviewBase", "Probe.both.ReviewHolder.ReviewBase", 5)]),
          bases("Probe.lacks.ReviewChild", &[text("ReviewHolder.ReviewBase")]),
          bases("Probe.other.ReviewChild", &[to("ReviewHolder.ReviewBase", "Probe.other.ReviewHolder.ReviewBase")])]),
    row("Java three-segment local owner paths keep the selected owner", L::Java, "java",
        "class Probe {\n\
         void one() { class ReviewHolder { class ReviewMiddle { class ReviewBase {} } } class ReviewChild extends ReviewHolder.ReviewMiddle.ReviewBase {} }\n\
         void both() {\n\
         { class ReviewHolder { class ReviewMiddle { class ReviewBase {} } } }\n\
         { class ReviewHolder { class ReviewMiddle { class ReviewBase {} } }\n\
         class ReviewChild extends ReviewHolder.ReviewMiddle.ReviewBase {} }\n\
         }\n\
         void lacks() {\n\
         { class ReviewHolder { class ReviewMiddle { class ReviewBase {} } } }\n\
         { class ReviewHolder { class ReviewMiddle {} }\n\
         class ReviewChild extends ReviewHolder.ReviewMiddle.ReviewBase {} }\n\
         }\n\
         void other() { { class ReviewOther { class ReviewMiddle { class ReviewBase {} } } } class ReviewHolder { class ReviewMiddle { class ReviewBase {} } } class ReviewChild extends ReviewHolder.ReviewMiddle.ReviewBase {} }\n\
         }\n",
        &[bases("Probe.one.ReviewChild", &[to("ReviewHolder.ReviewMiddle.ReviewBase", "Probe.one.ReviewHolder.ReviewMiddle.ReviewBase")]),
          bases("Probe.both.ReviewChild", &[to_at("ReviewHolder.ReviewMiddle.ReviewBase", "Probe.both.ReviewHolder.ReviewMiddle.ReviewBase", 5)]),
          bases("Probe.lacks.ReviewChild", &[text("ReviewHolder.ReviewMiddle.ReviewBase")]),
          bases("Probe.other.ReviewChild", &[to("ReviewHolder.ReviewMiddle.ReviewBase", "Probe.other.ReviewHolder.ReviewMiddle.ReviewBase")])]),
    row("Swift local owner paths keep the selected owner", L::Swift, "swift",
        "func one() {\n\
         class ReviewHolder { class ReviewBase {} }\n\
         class ReviewChild: ReviewHolder.ReviewBase {}\n\
         }\n\
         func both() {\n\
         do { class ReviewHolder { class ReviewBase {} } }\n\
         do {\n\
         class ReviewHolder { class ReviewBase {} }\n\
         class ReviewChild: ReviewHolder.ReviewBase {}\n\
         }\n\
         }\n\
         func lacks() {\n\
         do { class ReviewHolder { class ReviewBase {} } }\n\
         do {\n\
         class ReviewHolder {}\n\
         class ReviewChild: ReviewHolder.ReviewBase {}\n\
         }\n\
         }\n\
         func other() {\n\
         do { class ReviewOther { class ReviewBase {} } }\n\
         class ReviewHolder { class ReviewBase {} }\n\
         class ReviewChild: ReviewHolder.ReviewBase {}\n\
         }\n",
        &[bases("one.ReviewChild", &[to("ReviewHolder.ReviewBase", "one.ReviewHolder.ReviewBase")]),
          bases("both.ReviewChild", &[to_at("ReviewHolder.ReviewBase", "both.ReviewHolder.ReviewBase", 8)]),
          unclassified("lacks.ReviewChild", &[text("ReviewHolder.ReviewBase")]),
          bases("other.ReviewChild", &[to("ReviewHolder.ReviewBase", "other.ReviewHolder.ReviewBase")])]),
    row("Swift three-segment local owner paths keep the selected owner", L::Swift, "swift",
        "func one() {\n\
         class ReviewHolder { class ReviewMiddle { class ReviewBase {} } }\n\
         class ReviewChild: ReviewHolder.ReviewMiddle.ReviewBase {}\n\
         }\n\
         func both() {\n\
         do { class ReviewHolder { class ReviewMiddle { class ReviewBase {} } } }\n\
         do {\n\
         class ReviewHolder { class ReviewMiddle { class ReviewBase {} } }\n\
         class ReviewChild: ReviewHolder.ReviewMiddle.ReviewBase {}\n\
         }\n\
         }\n\
         func lacks() {\n\
         do { class ReviewHolder { class ReviewMiddle { class ReviewBase {} } } }\n\
         do {\n\
         class ReviewHolder { class ReviewMiddle {} }\n\
         class ReviewChild: ReviewHolder.ReviewMiddle.ReviewBase {}\n\
         }\n\
         }\n\
         func other() {\n\
         do { class ReviewOther { class ReviewMiddle { class ReviewBase {} } } }\n\
         class ReviewHolder { class ReviewMiddle { class ReviewBase {} } }\n\
         class ReviewChild: ReviewHolder.ReviewMiddle.ReviewBase {}\n\
         }\n",
        &[bases("one.ReviewChild", &[to("ReviewHolder.ReviewMiddle.ReviewBase", "one.ReviewHolder.ReviewMiddle.ReviewBase")]),
          bases("both.ReviewChild", &[to_at("ReviewHolder.ReviewMiddle.ReviewBase", "both.ReviewHolder.ReviewMiddle.ReviewBase", 8)]),
          unclassified("lacks.ReviewChild", &[text("ReviewHolder.ReviewMiddle.ReviewBase")]),
          bases("other.ReviewChild", &[to("ReviewHolder.ReviewMiddle.ReviewBase", "other.ReviewHolder.ReviewMiddle.ReviewBase")])]),
    row("Scala local object paths keep the selected owner", L::Scala, "scala",
        "object Probe {\n\
         def one(): Unit = {\n\
         object reviewHolder { class ReviewBase }\n\
         class ReviewChild extends reviewHolder.ReviewBase\n\
         }\n\
         def both(): Unit = {\n\
         { object reviewHolder { class ReviewBase } }\n\
         {\n\
         object reviewHolder { class ReviewBase }\n\
         class ReviewChild extends reviewHolder.ReviewBase\n\
         }\n\
         }\n\
         def lacks(): Unit = {\n\
         { object reviewHolder { class ReviewBase } }\n\
         {\n\
         object reviewHolder\n\
         class ReviewChild extends reviewHolder.ReviewBase\n\
         }\n\
         }\n\
         def other(): Unit = {\n\
         { object reviewOther { class ReviewBase } }\n\
         object reviewHolder { class ReviewBase }\n\
         class ReviewChild extends reviewHolder.ReviewBase\n\
         }\n\
         }\n",
        &[bases("Probe.one.ReviewChild", &[to("reviewHolder.ReviewBase", "Probe.one.reviewHolder.ReviewBase")]),
          bases("Probe.both.ReviewChild", &[to_at("reviewHolder.ReviewBase", "Probe.both.reviewHolder.ReviewBase", 9)]),
          unclassified("Probe.lacks.ReviewChild", &[text("reviewHolder.ReviewBase")]),
          bases("Probe.other.ReviewChild", &[to("reviewHolder.ReviewBase", "Probe.other.reviewHolder.ReviewBase")])]),
    row("Scala three-segment local object paths keep the selected owner", L::Scala, "scala",
        "object Probe {\n\
         def one(): Unit = {\n\
         object reviewHolder { object reviewMiddle { class ReviewBase } }\n\
         class ReviewChild extends reviewHolder.reviewMiddle.ReviewBase\n\
         }\n\
         def both(): Unit = {\n\
         { object reviewHolder { object reviewMiddle { class ReviewBase } } }\n\
         {\n\
         object reviewHolder { object reviewMiddle { class ReviewBase } }\n\
         class ReviewChild extends reviewHolder.reviewMiddle.ReviewBase\n\
         }\n\
         }\n\
         def lacks(): Unit = {\n\
         { object reviewHolder { object reviewMiddle { class ReviewBase } } }\n\
         {\n\
         object reviewHolder { object reviewMiddle }\n\
         class ReviewChild extends reviewHolder.reviewMiddle.ReviewBase\n\
         }\n\
         }\n\
         def other(): Unit = {\n\
         { object reviewOther { object reviewMiddle { class ReviewBase } } }\n\
         object reviewHolder { object reviewMiddle { class ReviewBase } }\n\
         class ReviewChild extends reviewHolder.reviewMiddle.ReviewBase\n\
         }\n\
         }\n",
        &[bases("Probe.one.ReviewChild", &[to("reviewHolder.reviewMiddle.ReviewBase", "Probe.one.reviewHolder.reviewMiddle.ReviewBase")]),
          bases("Probe.both.ReviewChild", &[to_at("reviewHolder.reviewMiddle.ReviewBase", "Probe.both.reviewHolder.reviewMiddle.ReviewBase", 9)]),
          unclassified("Probe.lacks.ReviewChild", &[text("reviewHolder.reviewMiddle.ReviewBase")]),
          bases("Probe.other.ReviewChild", &[to("reviewHolder.reviewMiddle.ReviewBase", "Probe.other.reviewHolder.reviewMiddle.ReviewBase")])]),
    row("Scala three-segment object paths in named scopes", L::Scala, "scala",
        "object reviewHolder { object reviewMiddle { class ReviewBase } }\n\
         class ReviewChild extends reviewHolder.reviewMiddle.ReviewBase\n\
         object reviewOther { object reviewMiddle }\n\
         class ReviewOtherChild extends reviewOther.reviewMiddle.ReviewBase\n",
        &[bases("ReviewChild", &[to("reviewHolder.reviewMiddle.ReviewBase", "reviewHolder.reviewMiddle.ReviewBase")]),
          unclassified("ReviewOtherChild", &[text("reviewOther.reviewMiddle.ReviewBase")])]),
    row("Rust local module paths keep the selected module", L::Rust, "rs",
        "fn one() {\n\
         mod reviewHolder { pub trait ReviewBase {} }\n\
         trait ReviewChild: reviewHolder::ReviewBase {}\n\
         }\n\
         fn both() {\n\
         { mod reviewHolder { pub trait ReviewBase {} } }\n\
         {\n\
         mod reviewHolder { pub trait ReviewBase {} }\n\
         trait ReviewChild: reviewHolder::ReviewBase {}\n\
         }\n\
         }\n\
         fn lacks() {\n\
         { mod reviewHolder { pub trait ReviewBase {} } }\n\
         {\n\
         mod reviewHolder {}\n\
         trait ReviewChild: reviewHolder::ReviewBase {}\n\
         }\n\
         }\n\
         fn deeper() {\n\
         mod reviewHolder { pub mod reviewMiddle { pub trait ReviewBase {} } }\n\
         trait ReviewChild: reviewHolder::reviewMiddle::ReviewBase {}\n\
         }\n\
         fn deeper_lacks() {\n\
         { mod reviewHolder { pub mod reviewMiddle { pub trait ReviewBase {} } } }\n\
         mod reviewHolder { pub mod reviewMiddle {} }\n\
         trait ReviewChild: reviewHolder::reviewMiddle::ReviewBase {}\n\
         }\n\
         fn other() {\n\
         { mod reviewOther { pub trait ReviewBase {} } }\n\
         mod reviewHolder { pub trait ReviewBase {} }\n\
         trait ReviewChild: reviewHolder::ReviewBase {}\n\
         }\n",
        &[bases("one::ReviewChild", &[to("reviewHolder::ReviewBase", "one::reviewHolder::ReviewBase")]),
          bases("both::ReviewChild", &[to_at("reviewHolder::ReviewBase", "both::reviewHolder::ReviewBase", 8)]),
          bases("lacks::ReviewChild", &[text("reviewHolder::ReviewBase")]),
          bases("deeper::ReviewChild", &[to("reviewHolder::reviewMiddle::ReviewBase", "deeper::reviewHolder::reviewMiddle::ReviewBase")]),
          bases("deeper_lacks::ReviewChild", &[text("reviewHolder::reviewMiddle::ReviewBase")]),
          bases("other::ReviewChild", &[to("reviewHolder::ReviewBase", "other::reviewHolder::ReviewBase")])]),
    row("Rust paths inside local modules", L::Rust, "rs",
        "trait ReviewBase {}\n\
         fn self_path() {\n\
         mod reviewHolder { pub trait ReviewBase {} pub trait ReviewChild: self::ReviewBase {} }\n\
         }\n\
         fn super_path() {\n\
         mod reviewHolder { pub trait ReviewBase {} pub mod reviewInner { pub trait ReviewChild: super::ReviewBase {} } }\n\
         }\n\
         fn bare() {\n\
         mod reviewHolder { pub trait ReviewBase {} pub trait ReviewChild: ReviewBase {} }\n\
         }\n\
         fn crate_path() {\n\
         mod reviewHolder { pub trait ReviewBase {} pub trait ReviewChild: crate::ReviewBase {} }\n\
         }\n\
         fn outer_super() {\n\
         trait ReviewBase {}\n\
         mod reviewHolder { pub trait ReviewChild: super::ReviewBase {} }\n\
         }\n",
        &[bases("self_path::reviewHolder::ReviewChild", &[to("self::ReviewBase", "self_path::reviewHolder::ReviewBase")]),
          bases("super_path::reviewHolder::reviewInner::ReviewChild",
                &[to("super::ReviewBase", "super_path::reviewHolder::ReviewBase")]),
          bases("bare::reviewHolder::ReviewChild", &[to("ReviewBase", "bare::reviewHolder::ReviewBase")]),
          bases("crate_path::reviewHolder::ReviewChild", &[to("crate::ReviewBase", "ReviewBase")]),
          bases("outer_super::reviewHolder::ReviewChild", &[to("super::ReviewBase", "ReviewBase")])]),
    row("Python local owner paths keep the selected owner", L::Python, "py",
        concat!(
            "def one():\n",
            "    class ReviewHolder:\n",
            "        class ReviewBase:\n",
            "            pass\n",
            "    class ReviewChild(ReviewHolder.ReviewBase):\n",
            "        pass\n",
            "\n",
            "def lacks():\n",
            "    def other():\n",
            "        class ReviewHolder:\n",
            "            class ReviewBase:\n",
            "                pass\n",
            "    class ReviewHolder:\n",
            "        pass\n",
            "    class ReviewChild(ReviewHolder.ReviewBase):\n",
            "        pass\n",
            "\n",
            "def has():\n",
            "    def other():\n",
            "        class ReviewHolder:\n",
            "            class ReviewBase:\n",
            "                pass\n",
            "    class ReviewHolder:\n",
            "        class ReviewBase:\n",
            "            pass\n",
            "    class ReviewChild(ReviewHolder.ReviewBase):\n",
            "        pass\n",
        ),
        &[bases("one.ReviewChild", &[to("ReviewHolder.ReviewBase", "one.ReviewHolder.ReviewBase")]),
          bases("lacks.ReviewChild", &[text("ReviewHolder.ReviewBase")]),
          bases("has.ReviewChild", &[to("ReviewHolder.ReviewBase", "has.ReviewHolder.ReviewBase")])]),
    row("TypeScript local class fields holding class expressions stay written", L::TypeScript, "ts",
        "function one() {\n  class ReviewHolder { static ReviewBase = class {}; }\n  class ReviewChild extends ReviewHolder.ReviewBase {}\n}\n\
         function both() {\n  { class ReviewHolder { static ReviewBase = class {}; } }\n  \
         { class ReviewHolder { static ReviewBase = class {}; }\n    class ReviewChild extends ReviewHolder.ReviewBase {} }\n}\n",
        &[bases("one.ReviewChild", &[text("ReviewHolder.ReviewBase")]),
          bases("both.ReviewChild", &[text("ReviewHolder.ReviewBase")])]),
    row("JavaScript local class fields holding class expressions stay written", L::JavaScript, "js",
        "function one() {\n  class ReviewHolder { static ReviewBase = class {}; }\n  class ReviewChild extends ReviewHolder.ReviewBase {}\n}\n\
         function both() {\n  { class ReviewHolder { static ReviewBase = class {}; } }\n  \
         { class ReviewHolder { static ReviewBase = class {}; }\n    class ReviewChild extends ReviewHolder.ReviewBase {} }\n}\n",
        &[bases("one.ReviewChild", &[text("ReviewHolder.ReviewBase")]),
          bases("both.ReviewChild", &[text("ReviewHolder.ReviewBase")])]),

    // --- Rust impl consumers through a function's local module.
    PathMeaningRow {
        label: "Rust impl of a local module's self type",
        language: L::Rust,
        extension: "rs",
        source: "pub struct ReviewSelf;\npub trait ReviewTrait {}\n\
                 fn f() {\n    mod reviewHolder { pub struct ReviewSelf; }\n    \
                 impl ReviewTrait for reviewHolder::ReviewSelf { fn probe(&self) {} }\n}\n",
        owners: &[implementing("f::reviewHolder::ReviewSelf", &[to("ReviewTrait", "ReviewTrait")])],
        impl_members: &[ImplMember {
            method: "probe",
            container: "reviewHolder::ReviewSelf",
            owner: Some("f::reviewHolder::ReviewSelf"),
            implements: Some(to("ReviewTrait", "ReviewTrait")),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &["ReviewSelf"],
    },
    PathMeaningRow {
        label: "Rust impl of a local module's trait",
        language: L::Rust,
        extension: "rs",
        source: "pub struct ReviewSelf;\npub trait ReviewTrait {}\n\
                 fn f() {\n    mod reviewHolder { pub trait ReviewTrait {} }\n    struct ReviewSelf;\n    \
                 impl reviewHolder::ReviewTrait for ReviewSelf { fn probe(&self) {} }\n}\n",
        owners: &[implementing("f::ReviewSelf", &[to("reviewHolder::ReviewTrait", "f::reviewHolder::ReviewTrait")])],
        impl_members: &[ImplMember {
            method: "probe",
            container: "ReviewSelf",
            owner: Some("f::ReviewSelf"),
            implements: Some(to("reviewHolder::ReviewTrait", "f::reviewHolder::ReviewTrait")),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &["ReviewSelf"],
    },
    PathMeaningRow {
        label: "Rust impl of a local module's trait for its type",
        language: L::Rust,
        extension: "rs",
        source: "pub struct ReviewSelf;\npub trait ReviewTrait {}\n\
                 fn f() {\n    mod reviewHolder { pub struct ReviewSelf; pub trait ReviewTrait {} }\n    \
                 impl reviewHolder::ReviewTrait for reviewHolder::ReviewSelf { fn probe(&self) {} }\n}\n",
        owners: &[implementing("f::reviewHolder::ReviewSelf",
                               &[to("reviewHolder::ReviewTrait", "f::reviewHolder::ReviewTrait")])],
        impl_members: &[ImplMember {
            method: "probe",
            container: "reviewHolder::ReviewSelf",
            owner: Some("f::reviewHolder::ReviewSelf"),
            implements: Some(to("reviewHolder::ReviewTrait", "f::reviewHolder::ReviewTrait")),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &["ReviewSelf"],
    },
    PathMeaningRow {
        label: "Rust impl of a local module's missing self type beside a sibling block's",
        language: L::Rust,
        extension: "rs",
        source: "pub trait ReviewTrait {}\n\
                 fn f() {\n    { mod reviewHolder { pub struct ReviewSelf; } }\n    mod reviewHolder {}\n    \
                 impl ReviewTrait for reviewHolder::ReviewSelf { fn probe(&self) {} }\n}\n",
        owners: &[],
        impl_members: &[ImplMember {
            method: "probe",
            container: "reviewHolder::ReviewSelf",
            owner: None,
            implements: Some(to("ReviewTrait", "ReviewTrait")),
            container_kind: None,
        }],
        untouched: &["f::reviewHolder::ReviewSelf"],
    },
    // --- Bindings no record reports keep their scope and name groups
    // outside lexical scopes: an impl's parameters bind only inside it, and a
    // value (a `const` parameter, a destructured or pattern-bound value)
    // never hides a type.
    row("Rust impl type parameter before the trait's user at the root", L::Rust, "rs",
        "trait ReviewBase {}\nstruct ReviewHolder<T>(T);\n\
         impl<ReviewBase> ReviewHolder<ReviewBase> {}\ntrait ReviewChild: ReviewBase {}\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust impl type parameter after the trait's user at the root", L::Rust, "rs",
        "trait ReviewBase {}\nstruct ReviewHolder<T>(T);\n\
         trait ReviewChild: ReviewBase {}\nimpl<ReviewBase> ReviewHolder<ReviewBase> {}\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust impl type parameter before the trait's user in a module", L::Rust, "rs",
        "mod reviewScope {\n    pub trait ReviewBase {}\n    pub struct ReviewHolder<T>(T);\n    \
         impl<ReviewBase> ReviewHolder<ReviewBase> {}\n    pub trait ReviewChild: ReviewBase {}\n}\n",
        &[bases("reviewScope::ReviewChild", &[to("ReviewBase", "reviewScope::ReviewBase")])]),
    row("Rust impl type parameter after the trait's user in a module", L::Rust, "rs",
        "mod reviewScope {\n    pub trait ReviewBase {}\n    pub struct ReviewHolder<T>(T);\n    \
         pub trait ReviewChild: ReviewBase {}\n    impl<ReviewBase> ReviewHolder<ReviewBase> {}\n}\n",
        &[bases("reviewScope::ReviewChild", &[to("ReviewBase", "reviewScope::ReviewBase")])]),
    row("Rust impl const parameter before the trait's user at the root", L::Rust, "rs",
        "trait ReviewBase {}\nstruct ReviewHolder<const N: usize>;\n\
         impl<const ReviewBase: usize> ReviewHolder<{ ReviewBase }> {}\ntrait ReviewChild: ReviewBase {}\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust impl const parameter after the trait's user at the root", L::Rust, "rs",
        "trait ReviewBase {}\nstruct ReviewHolder<const N: usize>;\n\
         trait ReviewChild: ReviewBase {}\nimpl<const ReviewBase: usize> ReviewHolder<{ ReviewBase }> {}\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust impl const parameter before the trait's user in a module", L::Rust, "rs",
        "mod reviewScope {\n    pub trait ReviewBase {}\n    pub struct ReviewHolder<const N: usize>;\n    \
         impl<const ReviewBase: usize> ReviewHolder<{ ReviewBase }> {}\n    pub trait ReviewChild: ReviewBase {}\n}\n",
        &[bases("reviewScope::ReviewChild", &[to("ReviewBase", "reviewScope::ReviewBase")])]),
    row("Rust impl const parameter after the trait's user in a module", L::Rust, "rs",
        "mod reviewScope {\n    pub trait ReviewBase {}\n    pub struct ReviewHolder<const N: usize>;\n    \
         pub trait ReviewChild: ReviewBase {}\n    impl<const ReviewBase: usize> ReviewHolder<{ ReviewBase }> {}\n}\n",
        &[bases("reviewScope::ReviewChild", &[to("ReviewBase", "reviewScope::ReviewBase")])]),
    row("Rust impl parameter named like an unrelated module qualifier", L::Rust, "rs",
        "mod reviewHolder { pub trait ReviewBase {} }\nstruct ReviewWrap<T>(T);\n\
         impl<reviewHolder> ReviewWrap<reviewHolder> {}\ntrait ReviewChild: reviewHolder::ReviewBase {}\n",
        &[bases("ReviewChild", &[to("reviewHolder::ReviewBase", "reviewHolder::ReviewBase")])]),
    row("Rust impl parameters in a function bind only inside the impl", L::Rust, "rs",
        "fn types() {\n    trait ReviewBase {}\n    struct ReviewHolder<T>(T);\n    \
         trait ReviewBefore: ReviewBase {}\n    impl<ReviewBase> ReviewHolder<ReviewBase> {}\n    \
         trait ReviewAfter: ReviewBase {}\n}\n\
         fn consts() {\n    trait ReviewBase {}\n    struct ReviewHolder<const N: usize>;\n    \
         trait ReviewBefore: ReviewBase {}\n    impl<const ReviewBase: usize> ReviewHolder<{ ReviewBase }> {}\n    \
         trait ReviewAfter: ReviewBase {}\n}\n",
        &[bases("types::ReviewBefore", &[to("ReviewBase", "types::ReviewBase")]),
          bases("types::ReviewAfter", &[to("ReviewBase", "types::ReviewBase")]),
          bases("consts::ReviewBefore", &[to("ReviewBase", "consts::ReviewBase")]),
          bases("consts::ReviewAfter", &[to("ReviewBase", "consts::ReviewBase")])]),
    row("Rust named trait's const parameter never hides a trait in its members", L::Rust, "rs",
        "trait ReviewBase {}\n\
         trait ReviewOwner<const ReviewBase: usize> { fn member() { trait ReviewChild: ReviewBase {} } }\n",
        &[bases("ReviewOwner::member::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust named struct's const parameter never hides a trait in its fields", L::Rust, "rs",
        "trait ReviewBase {}\n\
         struct ReviewOwner<const ReviewBase: usize> { field: [u8; { trait ReviewChild: ReviewBase {} 0 }] }\n",
        &[bases("ReviewOwner::ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust const parameter of the declaration or its function", L::Rust, "rs",
        "trait ReviewBase {}\ntrait ReviewChild<const ReviewBase: usize>: ReviewBase {}\n\
         fn probe<const ReviewBase: usize>() { trait ReviewInner: ReviewBase {} }\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          bases("probe::ReviewInner", &[to("ReviewBase", "ReviewBase")])]),
    row("Rust named owner's type parameter hides a trait in its members", L::Rust, "rs",
        "trait ReviewBase {}\n\
         trait ReviewOwner<ReviewBase> { fn member() { trait ReviewChild: ReviewBase {} } }\n",
        &[bases("ReviewOwner::member::ReviewChild", &[text("ReviewBase")])]),
    row("Rust empty impl in a member of a named const-generic trait", L::Rust, "rs",
        "pub struct ReviewSelf;\npub trait ReviewTrait {}\n\
         trait ReviewOwner<const ReviewSelf: usize> { fn member() { impl ReviewTrait for ReviewSelf {} } }\n",
        &[implementing("ReviewSelf", &[to("ReviewTrait", "ReviewTrait")])]),
    PathMeaningRow {
        label: "Rust impl in a member of a named const-generic trait",
        language: L::Rust,
        extension: "rs",
        source: "pub struct ReviewSelf;\npub trait ReviewTrait {}\n\
                 trait ReviewOwner<const ReviewSelf: usize> {\n    \
                 fn member() { impl ReviewTrait for ReviewSelf { fn probe(&self) {} } }\n}\n",
        owners: &[implementing("ReviewSelf", &[to("ReviewTrait", "ReviewTrait")])],
        impl_members: &[ImplMember {
            method: "probe",
            container: "ReviewSelf",
            owner: Some("ReviewSelf"),
            implements: Some(to("ReviewTrait", "ReviewTrait")),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &[],
    },
    row("TypeScript module-level flat array binding is a value", L::TypeScript, "ts",
        "interface ReviewBase {}\nconst [ReviewBase] = external;\nclass ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript module-level nested array binding is a value", L::TypeScript, "ts",
        "interface ReviewBase {}\nconst [[ReviewBase]] = external;\nclass ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript module-level nested object binding is a value", L::TypeScript, "ts",
        "interface ReviewBase {}\nconst { x: { ReviewBase } } = external;\nclass ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript module-level flat shorthand binding", L::TypeScript, "ts",
        "interface ReviewBase {}\nexport const { ReviewBase } = external;\nclass ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript module-level nested binding of another name", L::TypeScript, "ts",
        "interface ReviewBase {}\nconst { x: { reviewOther } } = external;\nclass ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript nested bindings in functions are values", L::TypeScript, "ts",
        "interface ReviewBase {}\n\
         function object() { const { x: { ReviewBase } } = external; class ReviewChild implements ReviewBase {} }\n\
         function array() { const [[ReviewBase]] = external; class ReviewChild implements ReviewBase {} }\n",
        &[implementing("object.ReviewChild", &[to("ReviewBase", "ReviewBase")]),
          implementing("array.ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript ambient function's type parameter binds only in its signature", L::TypeScript, "ts",
        "interface ReviewBase {}\ndeclare function reviewDeclared<ReviewBase>(): void;\n\
         class ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript overload signature's type parameter binds only in its signature", L::TypeScript, "ts",
        "interface ReviewBase {}\nfunction reviewOverload<ReviewBase>(x: ReviewBase): void;\n\
         function reviewOverload(x: any) {}\nclass ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("TypeScript method signature's type parameter binds only in its signature", L::TypeScript, "ts",
        "interface ReviewBase {}\ninterface ReviewShape { m<ReviewBase>(): void; }\n\
         class ReviewChild implements ReviewBase {}\n",
        &[implementing("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("JavaScript module-level destructuring of another name", L::JavaScript, "js",
        "class ReviewBase {}\nconst { x: { reviewOther } } = external;\nclass ReviewChild extends ReviewBase {}\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("JavaScript module-level destructuring binds the base's value", L::JavaScript, "js",
        "class ReviewBase {}\nconst { x: { ReviewBase } } = external;\nclass ReviewChild extends ReviewBase {}\n",
        &[bases("ReviewChild", &[text("ReviewBase")])]),
    row("Python module-level tuple target binds the base's name", L::Python, "py",
        "class ReviewBase:\n    pass\n\nreviewOther, ReviewBase = external\n\nclass ReviewChild(ReviewBase):\n    pass\n",
        &[bases("ReviewChild", &[text("ReviewBase")])]),
    row("Python module-level tuple target of another name", L::Python, "py",
        "class ReviewBase:\n    pass\n\nreviewOther, reviewThird = external\n\nclass ReviewChild(ReviewBase):\n    pass\n",
        &[bases("ReviewChild", &[to("ReviewBase", "ReviewBase")])]),
    row("Scala package-level tuple binding is a term", L::Scala, "scala",
        "package reviewPkg\nclass reviewBase\nval (reviewBase, reviewOther) = (1, 2)\nclass ReviewChild extends reviewBase\n",
        &[bases("reviewPkg.ReviewChild", &[to("reviewBase", "reviewPkg.reviewBase")])]),
    row("Scala package-level nested tuple binding is a term", L::Scala, "scala",
        "package reviewPkg\nclass reviewBase\nval ((reviewBase, reviewOther), reviewThird) = ((1, 2), 3)\n\
         class ReviewChild extends reviewBase\n",
        &[bases("reviewPkg.ReviewChild", &[to("reviewBase", "reviewPkg.reviewBase")])]),
    row("Scala package-level extractor binding is a term", L::Scala, "scala",
        "package reviewPkg\nclass reviewBase\nval Some(reviewBase) = Option(1)\nclass ReviewChild extends reviewBase\n",
        &[bases("reviewPkg.ReviewChild", &[to("reviewBase", "reviewPkg.reviewBase")])]),
    row("Scala object tuple binding is a term", L::Scala, "scala",
        "object reviewHolder {\n  class reviewBase\n  val (reviewBase, reviewOther) = (1, 2)\n  \
         class ReviewChild extends reviewBase\n}\n",
        &[bases("reviewHolder.ReviewChild", &[to("reviewBase", "reviewHolder.reviewBase")])]),
    row("Scala object nested tuple binding is a term", L::Scala, "scala",
        "object reviewHolder {\n  class reviewBase\n  val ((reviewBase, reviewOther), reviewThird) = ((1, 2), 3)\n  \
         class ReviewChild extends reviewBase\n}\n",
        &[bases("reviewHolder.ReviewChild", &[to("reviewBase", "reviewHolder.reviewBase")])]),
    row("Scala object extractor binding is a term", L::Scala, "scala",
        "object reviewHolder {\n  class reviewBase\n  val Some(reviewBase) = Option(1)\n  \
         class ReviewChild extends reviewBase\n}\n",
        &[bases("reviewHolder.ReviewChild", &[to("reviewBase", "reviewHolder.reviewBase")])]),
    row("Scala simple vals beside a class", L::Scala, "scala",
        "package reviewPkg\nclass reviewBase\nval reviewBase = 1\nclass ReviewChild extends reviewBase\n\
         object reviewHolder {\n  class reviewBase\n  val reviewBase = 1\n  class ReviewChild extends reviewBase\n}\n",
        &[bases("reviewPkg.ReviewChild", &[to("reviewBase", "reviewPkg.reviewBase")]),
          bases("reviewPkg.reviewHolder.ReviewChild", &[to("reviewBase", "reviewPkg.reviewHolder.reviewBase")])]),
    row("Scala pattern bindings in a function are terms", L::Scala, "scala",
        "class reviewBase\nobject reviewProbe {\n  def tuple(): Unit = {\n    val (reviewBase, reviewOther) = (1, 2)\n    \
         class ReviewChild extends reviewBase\n  }\n  def nested(): Unit = {\n    \
         val ((reviewBase, reviewOther), reviewThird) = ((1, 2), 3)\n    class ReviewChild extends reviewBase\n  }\n  \
         def extractor(): Unit = {\n    val Some(reviewBase) = Option(1)\n    class ReviewChild extends reviewBase\n  }\n}\n",
        &[bases("reviewProbe.tuple.ReviewChild", &[to("reviewBase", "reviewBase")]),
          bases("reviewProbe.nested.ReviewChild", &[to("reviewBase", "reviewBase")]),
          bases("reviewProbe.extractor.ReviewChild", &[to("reviewBase", "reviewBase")])]),
];

/// Where a [`PythonMemberShape`] is written.
#[derive(Clone, Copy, PartialEq)]
enum PythonContexts {
    /// At module level and again inside `def reviewProbe(external):`.
    Both,
    /// Only inside the function (a `nonlocal` directive needs one).
    FunctionOnly,
}

/// A Python path through a class's members, written once at module level:
/// `source` ends with `class ReviewChild(<path>)`, and the member the path
/// names is the type declared on `line` (`target`), or nothing when the
/// shape leaves it unproved. [`python_member_rows`] writes every shape at
/// module level and inside a function, so each binding decision is checked
/// for a named owner and for a lexical (syntax) owner.
struct PythonMemberShape {
    label: &'static str,
    source: &'static str,
    path: &'static str,
    /// The named member's in-file qualified name and declaration line at
    /// module level.
    target: Option<(&'static str, u32)>,
    contexts: PythonContexts,
}

/// A two-segment shape: `ReviewHolder` declares `class ReviewBase` on line
/// 2, then `$body` (class-body lines, indented four spaces), then `$after`
/// (module-level lines) before the child.
macro_rules! py_holder {
    ($label:literal, $body:literal, $after:literal, $target:expr) => {
        PythonMemberShape {
            label: $label,
            source: concat!(
                "class ReviewHolder:\n    class ReviewBase:\n        pass\n", $body, $after,
                "class ReviewChild(ReviewHolder.ReviewBase):\n    pass\n"
            ),
            path: "ReviewHolder.ReviewBase",
            target: $target,
            contexts: PythonContexts::Both,
        }
    };
}

/// A three-segment shape: `ReviewHolder.ReviewMiddle` declares `class
/// ReviewBase` on line 3; `$middle` is written in the middle class's body
/// (eight spaces), `$body` in the holder's and `$after` at module level.
macro_rules! py_middle {
    ($label:literal, $middle:literal, $body:literal, $after:literal, $target:expr) => {
        PythonMemberShape {
            label: $label,
            source: concat!(
                "class ReviewHolder:\n    class ReviewMiddle:\n        class ReviewBase:\n            pass\n",
                $middle, $body, $after,
                "class ReviewChild(ReviewHolder.ReviewMiddle.ReviewBase):\n    pass\n"
            ),
            path: "ReviewHolder.ReviewMiddle.ReviewBase",
            target: $target,
            contexts: PythonContexts::Both,
        }
    };
}

/// A shape whose holder writes only `$body`, with no member of its own
/// before it.
macro_rules! py_directive {
    ($label:literal, $body:literal) => {
        PythonMemberShape {
            label: $label,
            source: concat!("class ReviewHolder:\n", $body, "class ReviewChild(ReviewHolder.ReviewBase):\n    pass\n"),
            path: "ReviewHolder.ReviewBase",
            target: None,
            contexts: PythonContexts::Both,
        }
    };
}

const PY_BASE: Option<(&str, u32)> = Some(("ReviewHolder.ReviewBase", 2));
const PY_MIDDLE_BASE: Option<(&str, u32)> = Some(("ReviewHolder.ReviewMiddle.ReviewBase", 3));

/// Every binding of a class member after the member's declaration that
/// leaves the member unproved, and the controls that keep it.
const PYTHON_MEMBER_SHAPES: &[PythonMemberShape] = &[
    // The member rebound in the class body by a binding no record reports.
    py_holder!("aliased `from` import", "    from builtins import object as ReviewBase\n", "", None),
    py_holder!("`from` import", "    from external import ReviewBase\n", "", None),
    py_holder!("aliased module import", "    import external as ReviewBase\n", "", None),
    py_holder!("star import", "    from external import *\n", "", None),
    py_holder!("deletion", "    del ReviewBase\n", "", None),
    py_holder!("augmented assignment", "    ReviewBase += external\n", "", None),
    py_holder!("nested tuple target", "    (reviewFirst, (ReviewBase, reviewSecond)) = external\n", "", None),
    py_holder!("nested list target", "    [reviewFirst, [ReviewBase, reviewSecond]] = external\n", "", None),
    py_holder!("starred target", "    reviewFirst, *ReviewBase = external\n", "", None),
    py_holder!("dictionary `match` capture",
        "    match external:\n        case {\"base\": ReviewBase}:\n            pass\n", "", None),
    // Records that conflict with the member (clean before the repair).
    py_holder!("simple assignment", "    ReviewBase = external\n", "", None),
    py_holder!("flat tuple target", "    reviewFirst, ReviewBase = external\n", "", None),
    py_holder!("flat list target", "    [reviewFirst, ReviewBase] = external\n", "", None),
    py_holder!("`for` target", "    for ReviewBase in external:\n        pass\n", "", None),
    py_holder!("`with` target", "    with external as ReviewBase:\n        pass\n", "", None),
    py_holder!("`except` alias",
        "    try:\n        pass\n    except external as ReviewBase:\n        pass\n", "", None),
    py_holder!("walrus", "    (ReviewBase := external)\n", "", None),
    py_holder!("same-named `def`", "    def ReviewBase(self):\n        pass\n", "", None),
    py_holder!("second same-named class", "    class ReviewBase:\n        pass\n", "", None),
    // Positive controls.
    py_holder!("unedited holder", "", "", PY_BASE),
    py_holder!("unrelated import", "    from external import ReviewOther\n", "", PY_BASE),
    py_holder!("unrelated deletion", "    del ReviewOther\n", "", PY_BASE),
    py_holder!("unrelated qualified write", "", "ReviewHolder.ReviewOther = external\n", PY_BASE),
    py_holder!("qualified write through another owner", "", "ReviewOtherHolder.ReviewBase = external\n", PY_BASE),
    // Explicit writes through the proved owner path.
    py_holder!("qualified assignment", "", "ReviewHolder.ReviewBase = external\n", None),
    py_holder!("qualified augmented assignment", "", "ReviewHolder.ReviewBase += external\n", None),
    py_holder!("qualified deletion", "", "del ReviewHolder.ReviewBase\n", None),
    py_holder!("qualified write in a function", "",
        "def reviewWriter(external):\n    ReviewHolder.ReviewBase = external\n", None),
    // Three segments: the final member and the owner passed through.
    py_middle!("three segments, unedited", "", "", "", PY_MIDDLE_BASE),
    py_middle!("three segments, final member imported", "        from external import ReviewBase\n", "", "", None),
    py_middle!("three segments, final member deleted", "        del ReviewBase\n", "", "", None),
    py_middle!("three segments, middle owner imported", "", "    from external import ReviewMiddle\n", "", None),
    py_middle!("three segments, middle owner deleted", "", "    del ReviewMiddle\n", "", None),
    py_middle!("three segments, final member written", "", "",
        "ReviewHolder.ReviewMiddle.ReviewBase = external\n", None),
    py_middle!("three segments, middle owner written", "", "", "del ReviewHolder.ReviewMiddle\n", None),
    // A directive places the class-body declaration at its destination.
    py_directive!("`global` before a class", "    global ReviewBase\n    class ReviewBase:\n        pass\n"),
    py_directive!("`global` before a `def`", "    global ReviewBase\n    def ReviewBase():\n        pass\n"),
    py_directive!("`global` before an import", "    global ReviewBase\n    from external import ReviewBase\n"),
    py_directive!("`global` before an assignment", "    global ReviewBase\n    ReviewBase = external\n"),
    py_directive!("`global` before a tuple assignment",
        "    global ReviewBase\n    reviewFirst, ReviewBase = external\n"),
    py_directive!("`global` before a deletion", "    global ReviewBase\n    del ReviewBase\n"),
    PythonMemberShape {
        label: "`global` class's nested class",
        source: "class ReviewHolder:\n    global ReviewBase\n    class ReviewBase:\n        class ReviewDeep:\n\
                 \x20           pass\nclass ReviewChild(ReviewHolder.ReviewBase.ReviewDeep):\n    pass\n",
        path: "ReviewHolder.ReviewBase.ReviewDeep",
        target: None,
        contexts: PythonContexts::Both,
    },
    PythonMemberShape {
        label: "`nonlocal` before a class",
        source: "ReviewBase = external\nclass ReviewHolder:\n    nonlocal ReviewBase\n    class ReviewBase:\n\
                 \x20       pass\nclass ReviewChild(ReviewHolder.ReviewBase):\n    pass\n",
        path: "ReviewHolder.ReviewBase",
        target: None,
        contexts: PythonContexts::FunctionOnly,
    },
    PythonMemberShape {
        label: "`nonlocal` before a `def`",
        source: "ReviewBase = external\nclass ReviewHolder:\n    nonlocal ReviewBase\n    def ReviewBase():\n\
                 \x20       pass\nclass ReviewChild(ReviewHolder.ReviewBase):\n    pass\n",
        path: "ReviewHolder.ReviewBase",
        target: None,
        contexts: PythonContexts::FunctionOnly,
    },
    PythonMemberShape {
        label: "`nonlocal` class's nested class",
        source: "ReviewBase = external\nclass ReviewHolder:\n    nonlocal ReviewBase\n    class ReviewBase:\n\
                 \x20       class ReviewDeep:\n            pass\n\
                 class ReviewChild(ReviewHolder.ReviewBase.ReviewDeep):\n    pass\n",
        path: "ReviewHolder.ReviewBase.ReviewDeep",
        target: None,
        contexts: PythonContexts::FunctionOnly,
    },
];

/// [`PYTHON_MEMBER_SHAPES`] as path-meaning rows: each shape at module
/// level (a named owner) and inside `def reviewProbe(external):` (a lexical
/// owner, one line down, every name under `reviewProbe`).
fn python_member_rows() -> Vec<PathMeaningRow> {
    fn leak(text: String) -> &'static str {
        Box::leak(text.into_boxed_str())
    }
    let mut rows = Vec::new();
    for shape in PYTHON_MEMBER_SHAPES {
        let contexts: &[bool] = match shape.contexts {
            PythonContexts::Both => &[false, true],
            PythonContexts::FunctionOnly => &[true],
        };
        for &in_function in contexts {
            let (source, prefix, offset, place) = if in_function {
                let body: String = shape
                    .source
                    .lines()
                    .map(|line| if line.is_empty() { "\n".to_string() } else { format!("    {line}\n") })
                    .collect();
                (leak(format!("def reviewProbe(external):\n{body}")), "reviewProbe.", 1, "function-local holder")
            } else {
                (shape.source, "", 0, "module-level holder")
            };
            let target = match shape.target {
                Some((qualified_name, line)) => {
                    to_at(shape.path, leak(format!("{prefix}{qualified_name}")), line + offset)
                }
                None => text(shape.path),
            };
            let owners: &'static [Owner] =
                Box::leak(vec![bases(leak(format!("{prefix}ReviewChild")), Box::leak(Box::new([target])))].into_boxed_slice());
            rows.push(row(leak(format!("Python member binding, {place}: {}", shape.label)), L::Python, "py", source, owners));
        }
    }
    rows
}

static PYTHON_MEMBER_ROWS: std::sync::LazyLock<Vec<PathMeaningRow>> = std::sync::LazyLock::new(python_member_rows);

/// Separately written nested Swift owners retain generic bindings from
/// every enclosing declaration, without importing ordinary members.
fn swift_nested_generic_rows() -> Vec<PathMeaningRow> {
    fn leak(value: String) -> &'static str { Box::leak(value.into_boxed_str()) }
    let mut rows = Vec::new();
    let mut add = |label: &str, source: String, owner: &str, target: Target, conformance: bool| {
        let targets = Box::leak(vec![target].into_boxed_slice());
        let owners = Box::leak(vec![Owner {
            qualified_name: leak(owner.to_string()), extends: &[],
            implements: if conformance { targets } else { &[] },
            unclassified: if conformance { &[] } else { targets },
        }].into_boxed_slice());
        rows.push(row(leak(format!("Swift nested generic extension: {label}")), L::Swift, "swift", leak(source), owners));
    };
    for (label, owner, declaration) in [
        ("outer parameter", "ReviewOuter.ReviewInner", "struct ReviewOuter<ReviewBase> { struct ReviewInner {} }"),
        ("three-segment outer parameter", "ReviewOuter.ReviewMiddle.ReviewInner", "struct ReviewOuter<ReviewBase> { struct ReviewMiddle { struct ReviewInner {} } }"),
        ("middle parameter", "ReviewOuter.ReviewMiddle.ReviewInner", "struct ReviewOuter { struct ReviewMiddle<ReviewBase> { struct ReviewInner {} } }"),
        ("own parameter control", "ReviewOuter.ReviewInner", "struct ReviewOuter { struct ReviewInner<ReviewBase> {} }"),
    ] {
        add(label, format!("class ReviewBase {{}}\n{declaration}\nextension {owner} {{ class ReviewChild: ReviewBase {{}} }}\n"),
            &format!("{owner}.ReviewChild"), text("ReviewBase"), false);
        add(&format!("{label} conformance"), format!("protocol ReviewBase {{}}\n{declaration}\nextension {owner}: ReviewBase {{}}\n"), owner, text("ReviewBase"), true);
    }
    add("qualified parameter", "struct ReviewHolder { class ReviewBase {} }\nstruct ReviewOuter<ReviewHolder> { struct ReviewInner {} }\nextension ReviewOuter.ReviewInner { class ReviewChild: ReviewHolder.ReviewBase {} }\n".into(),
        "ReviewOuter.ReviewInner.ReviewChild", text("ReviewHolder.ReviewBase"), false);
    for body in ["func probe() { class ReviewChild: ReviewBase {} }", "func probe() { let callback = { class ReviewChild: ReviewBase {} } }"] {
        add(body, format!("class ReviewBase {{}}\nstruct ReviewOuter<ReviewBase> {{ struct ReviewInner {{}} }}\nextension ReviewOuter.ReviewInner {{ {body} }}\n"),
            "ReviewOuter.ReviewInner.probe.ReviewChild", text("ReviewBase"), false);
    }
    add("ordinary owner member is not a header binding", "struct ReviewOuter { protocol ReviewBase {}\nstruct ReviewInner {} }\nextension ReviewOuter.ReviewInner: ReviewBase {}\n".into(),
        "ReviewOuter.ReviewInner", text("ReviewBase"), true);
    for name in ["Int", "Int8", "Int16", "Int32", "Int64", "UInt", "UInt8", "UInt16", "UInt32", "UInt64", "String", "Character", "Float", "Double"] {
        for qualified in [false, true] {
            let parameter = if qualified { "Swift" } else { name };
            let target = if qualified { leak(format!("Swift.{name}")) } else { name };
            add(target, format!("struct ReviewOuter<{parameter}> {{ struct ReviewInner {{}} }}\nextension ReviewOuter.ReviewInner {{ enum ReviewChild: {target} {{ case a }} }}\n"),
                "ReviewOuter.ReviewInner.ReviewChild", text(target), false);
        }
    }
    for (parameter, target) in [("Swift", "Swift.Unicode.Scalar"), ("Unicode", "Unicode.Scalar")] {
        add(target, format!("struct ReviewOuter<{parameter}> {{ struct ReviewInner {{}} }}\nextension ReviewOuter.ReviewInner {{ enum ReviewChild: {target} {{ case a }} }}\n"),
            "ReviewOuter.ReviewInner.ReviewChild", text(target), false);
    }
    for (label, source, owner, extends, implements) in [
        ("nongeneric nested body", "class ReviewBase {}\nstruct ReviewOuter { struct ReviewInner {} }\nextension ReviewOuter.ReviewInner { class ReviewChild: ReviewBase {} }\n", "ReviewOuter.ReviewInner.ReviewChild", &[to("ReviewBase", "ReviewBase")][..], &[][..]),
        ("nongeneric nested conformance", "protocol ReviewBase {}\nstruct ReviewOuter { struct ReviewInner {} }\nextension ReviewOuter.ReviewInner: ReviewBase {}\n", "ReviewOuter.ReviewInner", &[][..], &[to("ReviewBase", "ReviewBase")][..]),
        ("nearer method-local class", "class ReviewBase {}\nstruct ReviewOuter<ReviewBase> { struct ReviewInner {} }\nextension ReviewOuter.ReviewInner { func probe() { class ReviewBase {}\nclass ReviewChild: ReviewBase {} } }\n", "ReviewOuter.ReviewInner.probe.ReviewChild", &[to("ReviewBase", "ReviewOuter.ReviewInner.probe.ReviewBase")][..], &[][..]),
        ("unshadowed raw type", "struct ReviewOuter { struct ReviewInner {} }\nextension ReviewOuter.ReviewInner { enum ReviewChild: Int { case a } }\n", "ReviewOuter.ReviewInner.ReviewChild", &[][..], &[][..]),
    ] {
        let owners = Box::leak(vec![Owner { qualified_name: owner,
            extends: Box::leak(extends.to_vec().into_boxed_slice()),
            implements: Box::leak(implements.to_vec().into_boxed_slice()), unclassified: &[],
        }].into_boxed_slice());
        rows.push(row(leak(format!("Swift nested generic extension control: {label}")), L::Swift, "swift", source, owners));
    }
    rows
}

static SWIFT_NESTED_GENERIC_ROWS: std::sync::LazyLock<Vec<PathMeaningRow>> = std::sync::LazyLock::new(swift_nested_generic_rows);

/// Distinct owners that share one spelling, and the reopenings that do
/// not: a path continues through an intermediate owner only when the
/// language and the canonical owner path prove one owner (a reopened C++,
/// TypeScript, C# or PHP namespace, one canonical inline namespace), and
/// stops, text only, at competing owners (C++ namespaces reached through
/// different inline namespaces, a direct and an injected owner, two Rust
/// modules, two Scala objects), however unique the final name is. A lookup
/// made inside one of the competing owners reads that owner's own members.
/// The Rust impl rows check every impl consumer: self type, trait, members,
/// empty and inherent impls.
const OWNER_AMBIGUITY_MATRIX: &[PathMeaningRow] = &[
    // --- C++ inline namespaces: canonical identity decides merging.
    row(
        "C++ intermediate namespace injected from two inline namespaces",
        L::Cpp,
        "cpp",
        "namespace ReviewNs {\n\
         \x20   inline namespace a { namespace Shared { struct ReviewBase {}; } }\n\
         \x20   inline namespace b { namespace Shared {} }\n\
         }\n\
         struct ReviewChild : ReviewNs::Shared::ReviewBase {};\n\
         struct ReviewRooted : ::ReviewNs::Shared::ReviewBase {};\n\
         struct ReviewExplicit : ReviewNs::a::Shared::ReviewBase {};\n\
         namespace ReviewNs { struct ReviewBare : Shared::ReviewBase {}; }\n",
        &[
            bases("ReviewChild", &[text("ReviewNs::Shared::ReviewBase")]),
            bases("ReviewRooted", &[text("::ReviewNs::Shared::ReviewBase")]),
            bases("ReviewExplicit", &[to_at("ReviewNs::a::Shared::ReviewBase", "ReviewNs::a::Shared::ReviewBase", 2)]),
            bases("ReviewNs::ReviewBare", &[text("Shared::ReviewBase")]),
        ],
    ),
    row(
        "C++ direct and injected owners of one spelling",
        L::Cpp,
        "cpp",
        "namespace ReviewNs {\n\
         \x20   namespace Shared { struct ReviewBase {}; }\n\
         \x20   inline namespace a { namespace Shared {} }\n\
         }\n\
         struct ReviewChild : ReviewNs::Shared::ReviewBase {};\n",
        &[bases("ReviewChild", &[text("ReviewNs::Shared::ReviewBase")])],
    ),
    row(
        "C++ ambiguity under a nested inline namespace stops every later segment",
        L::Cpp,
        "cpp",
        "namespace ReviewNs {\n\
         \x20   inline namespace a { inline namespace c { namespace Shared {\n\
         \x20       struct ReviewBase {};\n\
         \x20       namespace Inner { struct ReviewBase {}; }\n\
         \x20   } } }\n\
         \x20   inline namespace b { namespace Shared {} }\n\
         }\n\
         struct ReviewChild : ReviewNs::Shared::ReviewBase {};\n\
         struct ReviewDeeper : ReviewNs::Shared::Inner::ReviewBase {};\n",
        &[
            bases("ReviewChild", &[text("ReviewNs::Shared::ReviewBase")]),
            bases("ReviewDeeper", &[text("ReviewNs::Shared::Inner::ReviewBase")]),
        ],
    ),
    row(
        "C++ ordinary namespace reopened (control)",
        L::Cpp,
        "cpp",
        "namespace ReviewNs { namespace Shared { struct ReviewBase {}; } }\n\
         namespace ReviewNs { namespace Shared {} }\n\
         struct ReviewChild : ReviewNs::Shared::ReviewBase {};\n",
        &[bases("ReviewChild", &[to_at("ReviewNs::Shared::ReviewBase", "ReviewNs::Shared::ReviewBase", 1)])],
    ),
    row(
        "C++ one canonical inline namespace defined twice (control)",
        L::Cpp,
        "cpp",
        "namespace ReviewNs { inline namespace a { namespace Shared { struct ReviewBase {}; } } }\n\
         namespace ReviewNs { inline namespace a { namespace Shared {} } }\n\
         struct ReviewChild : ReviewNs::Shared::ReviewBase {};\n",
        &[bases("ReviewChild", &[to_at("ReviewNs::Shared::ReviewBase", "ReviewNs::a::Shared::ReviewBase", 1)])],
    ),
    row(
        "C++ differently named injected namespace (control)",
        L::Cpp,
        "cpp",
        "namespace ReviewNs {\n\
         \x20   inline namespace a { namespace Shared { struct ReviewBase {}; } }\n\
         \x20   inline namespace b { namespace Other {} }\n\
         }\n\
         struct ReviewChild : ReviewNs::Shared::ReviewBase {};\n",
        &[bases("ReviewChild", &[to_at("ReviewNs::Shared::ReviewBase", "ReviewNs::a::Shared::ReviewBase", 2)])],
    ),
    row(
        "C++ two injected final names and two injected struct owners (control)",
        L::Cpp,
        "cpp",
        "namespace ReviewNs {\n\
         \x20   inline namespace a { struct ReviewBase {}; struct Shared { struct ReviewInner {}; }; }\n\
         \x20   inline namespace b { struct ReviewBase {}; struct Shared {}; }\n\
         }\n\
         struct ReviewChild : ReviewNs::ReviewBase {};\n\
         struct ReviewOwned : ReviewNs::Shared::ReviewInner {};\n",
        &[
            bases("ReviewChild", &[text("ReviewNs::ReviewBase")]),
            bases("ReviewOwned", &[text("ReviewNs::Shared::ReviewInner")]),
        ],
    ),
    row(
        "C++ duplicate type owners with a member in one (control)",
        L::Cpp,
        "cpp",
        "struct ReviewHolder { struct ReviewBase {}; };\n\
         struct ReviewHolder {};\n\
         struct ReviewChild : ReviewHolder::ReviewBase {};\n",
        &[bases("ReviewChild", &[text("ReviewHolder::ReviewBase")])],
    ),
    // --- Rust modules never merge.
    row(
        "Rust two modules of one name, the base in only one",
        L::Rust,
        "rs",
        "mod ReviewHolder { pub trait ReviewBase {} pub mod Inner { pub trait ReviewBase {} } }\n\
         mod ReviewHolder {}\n\
         trait ReviewChild: ReviewHolder::ReviewBase {}\n\
         trait ReviewRooted: crate::ReviewHolder::ReviewBase {}\n\
         trait ReviewDeeper: ReviewHolder::Inner::ReviewBase {}\n",
        &[
            bases("ReviewChild", &[text("ReviewHolder::ReviewBase")]),
            bases("ReviewRooted", &[text("crate::ReviewHolder::ReviewBase")]),
            bases("ReviewDeeper", &[text("ReviewHolder::Inner::ReviewBase")]),
        ],
    ),
    row(
        "Rust modules of one name under opposite cfg attributes",
        L::Rust,
        "rs",
        "#[cfg(unix)]\n\
         mod ReviewHolder { pub trait ReviewBase {} }\n\
         #[cfg(not(unix))]\n\
         mod ReviewHolder {}\n\
         trait ReviewChild: ReviewHolder::ReviewBase {}\n",
        &[bases("ReviewChild", &[text("ReviewHolder::ReviewBase")])],
    ),
    row(
        "Rust lookup inside one of two same-named modules reads its own members",
        L::Rust,
        "rs",
        "mod ReviewHolder {\n\
         \x20   pub trait ReviewBase {}\n\
         \x20   pub trait ReviewChild: ReviewBase {}\n\
         \x20   pub trait ReviewSelf: self::ReviewBase {}\n\
         \x20   mod inner { pub trait ReviewUp: super::ReviewBase {} }\n\
         }\n\
         mod ReviewHolder { pub trait ReviewBase {} }\n",
        &[
            bases("ReviewHolder::ReviewChild", &[to_at("ReviewBase", "ReviewHolder::ReviewBase", 2)]),
            bases("ReviewHolder::ReviewSelf", &[to_at("self::ReviewBase", "ReviewHolder::ReviewBase", 2)]),
            bases("ReviewHolder::inner::ReviewUp", &[to_at("super::ReviewBase", "ReviewHolder::ReviewBase", 2)]),
        ],
    ),
    row(
        "Rust duplicate local modules (control)",
        L::Rust,
        "rs",
        "fn review_scope() {\n\
         \x20   mod ReviewHolder { pub trait ReviewBase {} pub trait ReviewSelf: self::ReviewBase {} }\n\
         \x20   mod ReviewHolder { pub trait ReviewBase {} }\n\
         \x20   trait ReviewChild: ReviewHolder::ReviewBase {}\n\
         }\n",
        &[
            bases("review_scope::ReviewChild", &[text("ReviewHolder::ReviewBase")]),
            bases(
                "review_scope::ReviewHolder::ReviewSelf",
                &[to_at("self::ReviewBase", "review_scope::ReviewHolder::ReviewBase", 2)],
            ),
        ],
    ),
    PathMeaningRow {
        label: "Rust impl of a self type through two same-named modules",
        language: L::Rust,
        extension: "rs",
        source: "mod ReviewHolder { pub struct ReviewBase; }\n\
                 mod ReviewHolder {}\n\
                 pub trait ReviewTrait {}\n\
                 impl ReviewTrait for ReviewHolder::ReviewBase { fn review_probe(&self) {} }\n",
        owners: &[implementing("ReviewHolder::ReviewBase", &[])],
        impl_members: &[ImplMember {
            method: "review_probe",
            container: "ReviewHolder::ReviewBase",
            owner: None,
            implements: Some(to_at("ReviewTrait", "ReviewTrait", 3)),
            container_kind: None,
        }],
        untouched: &["ReviewHolder::ReviewBase"],
    },
    PathMeaningRow {
        label: "Rust impl of a trait through two same-named modules",
        language: L::Rust,
        extension: "rs",
        source: "mod ReviewHolder { pub trait ReviewTrait {} }\n\
                 mod ReviewHolder {}\n\
                 pub struct ReviewBase;\n\
                 impl ReviewHolder::ReviewTrait for ReviewBase { fn review_probe(&self) {} }\n",
        owners: &[implementing("ReviewBase", &[text("ReviewHolder::ReviewTrait")])],
        impl_members: &[ImplMember {
            method: "review_probe",
            container: "ReviewBase",
            owner: Some("ReviewBase"),
            implements: Some(text("ReviewHolder::ReviewTrait")),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &[],
    },
    PathMeaningRow {
        label: "Rust impl of a self type and a trait through two same-named modules",
        language: L::Rust,
        extension: "rs",
        source: "mod ReviewHolder { pub struct ReviewBase; pub trait ReviewTrait {} }\n\
                 mod ReviewHolder {}\n\
                 impl ReviewHolder::ReviewTrait for ReviewHolder::ReviewBase { fn review_probe(&self) {} }\n",
        owners: &[implementing("ReviewHolder::ReviewBase", &[])],
        impl_members: &[ImplMember {
            method: "review_probe",
            container: "ReviewHolder::ReviewBase",
            owner: None,
            implements: Some(text("ReviewHolder::ReviewTrait")),
            container_kind: None,
        }],
        untouched: &["ReviewHolder::ReviewBase"],
    },
    PathMeaningRow {
        label: "Rust empty impl of a self type through two same-named modules",
        language: L::Rust,
        extension: "rs",
        source: "mod ReviewHolder { pub struct ReviewBase; }\n\
                 mod ReviewHolder {}\n\
                 pub trait ReviewTrait {}\n\
                 impl ReviewTrait for ReviewHolder::ReviewBase {}\n",
        owners: &[implementing("ReviewHolder::ReviewBase", &[])],
        impl_members: &[],
        untouched: &["ReviewHolder::ReviewBase"],
    },
    PathMeaningRow {
        label: "Rust inherent impl of a self type through two same-named modules",
        language: L::Rust,
        extension: "rs",
        source: "mod ReviewHolder { pub struct ReviewBase; }\n\
                 mod ReviewHolder {}\n\
                 impl ReviewHolder::ReviewBase { fn review_probe(&self) {} }\n",
        owners: &[implementing("ReviewHolder::ReviewBase", &[])],
        impl_members: &[ImplMember {
            method: "review_probe",
            container: "ReviewHolder::ReviewBase",
            owner: None,
            implements: None,
            container_kind: None,
        }],
        untouched: &["ReviewHolder::ReviewBase"],
    },
    PathMeaningRow {
        label: "Rust impl of a self type through one module (control)",
        language: L::Rust,
        extension: "rs",
        source: "mod ReviewHolder { pub struct ReviewBase; }\n\
                 pub trait ReviewTrait {}\n\
                 impl ReviewTrait for ReviewHolder::ReviewBase { fn review_probe(&self) {} }\n",
        owners: &[implementing("ReviewHolder::ReviewBase", &[to_at("ReviewTrait", "ReviewTrait", 2)])],
        impl_members: &[ImplMember {
            method: "review_probe",
            container: "ReviewHolder::ReviewBase",
            owner: Some("ReviewHolder::ReviewBase"),
            implements: Some(to_at("ReviewTrait", "ReviewTrait", 2)),
            container_kind: Some(SymbolKindV2::Type),
        }],
        untouched: &[],
    },
    // --- Scala objects never merge.
    row(
        "Scala two objects of one name, the base in only one",
        L::Scala,
        "scala",
        "object ReviewHolder { class ReviewBase; object Inner { class ReviewBase } }\n\
         object ReviewHolder\n\
         class ReviewChild extends ReviewHolder.ReviewBase\n\
         class ReviewDeeper extends ReviewHolder.Inner.ReviewBase\n",
        &[
            unclassified("ReviewChild", &[text("ReviewHolder.ReviewBase")]),
            unclassified("ReviewDeeper", &[text("ReviewHolder.Inner.ReviewBase")]),
        ],
    ),
    row(
        "Scala lookup inside one of two same-named objects reads its own members",
        L::Scala,
        "scala",
        "object ReviewHolder {\n\
         \x20 class ReviewBase\n\
         \x20 class ReviewChild extends ReviewBase\n\
         }\n\
         object ReviewHolder { class ReviewBase }\n",
        &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])],
    ),
    row(
        "Scala duplicate local objects (control)",
        L::Scala,
        "scala",
        "object ReviewScope {\n\
         \x20 def review(): Unit = {\n\
         \x20   object ReviewHolder { class ReviewBase }\n\
         \x20   object ReviewHolder { class ReviewOther }\n\
         \x20   class ReviewChild extends ReviewHolder.ReviewBase\n\
         \x20 }\n\
         }\n",
        &[unclassified("ReviewScope.review.ReviewChild", &[text("ReviewHolder.ReviewBase")])],
    ),
    row(
        "Scala companion class and object, the base in the object (control)",
        L::Scala,
        "scala",
        "class ReviewHolder\n\
         object ReviewHolder {\n\
         \x20 class ReviewBase\n\
         \x20 class ReviewInside extends ReviewBase\n\
         }\n\
         class ReviewChild extends ReviewHolder.ReviewBase\n",
        &[
            bases("ReviewHolder.ReviewInside", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 3)]),
            bases("ReviewChild", &[to_at("ReviewHolder.ReviewBase", "ReviewHolder.ReviewBase", 3)]),
        ],
    ),
    // --- Namespace reopenings that merge (controls).
    row(
        "TypeScript namespace reopened (control)",
        L::TypeScript,
        "ts",
        "namespace ReviewNs { export class ReviewBase {} }\n\
         namespace ReviewNs { export class ReviewOther {} }\n\
         class ReviewChild extends ReviewNs.ReviewBase {}\n",
        &[bases("ReviewChild", &[to_at("ReviewNs.ReviewBase", "ReviewNs.ReviewBase", 1)])],
    ),
    row(
        "C# namespace reopened (control)",
        L::CSharp,
        "cs",
        "namespace ReviewNs { class ReviewBase {} }\n\
         namespace ReviewNs { class ReviewOther {} }\n\
         class ReviewChild : ReviewNs.ReviewBase {}\n",
        &[bases("ReviewChild", &[to_at("ReviewNs.ReviewBase", "ReviewNs.ReviewBase", 1)])],
    ),
    row(
        "PHP block namespace reopened, rooted target (control)",
        L::Php,
        "php",
        "<?php\n\
         namespace ReviewNs { class ReviewBase {} }\n\
         namespace ReviewNs { class ReviewOther {} }\n\
         namespace ReviewUse { class ReviewChild extends \\ReviewNs\\ReviewBase {} }\n",
        &[bases("ReviewUse\\ReviewChild", &[to_at("\\ReviewNs\\ReviewBase", "ReviewNs\\ReviewBase", 2)])],
    ),
    row(
        "PHP statement namespace reopened, rooted target (control)",
        L::Php,
        "php",
        "<?php\n\
         namespace ReviewNs;\n\
         class ReviewBase {}\n\
         namespace ReviewUse;\n\
         class ReviewChild extends \\ReviewNs\\ReviewBase {}\n\
         namespace ReviewNs;\n\
         class ReviewSibling extends ReviewBase {}\n",
        &[
            bases("ReviewUse\\ReviewChild", &[to_at("\\ReviewNs\\ReviewBase", "ReviewNs\\ReviewBase", 3)]),
            bases("ReviewNs\\ReviewSibling", &[to_at("ReviewBase", "ReviewNs\\ReviewBase", 3)]),
        ],
    ),
    // --- Duplicate ordinary owners (controls).
    row(
        "Java duplicate classes with a member in one (control)",
        L::Java,
        "java",
        "class ReviewHolder { static class ReviewBase {} }\n\
         class ReviewHolder {}\n\
         class ReviewChild extends ReviewHolder.ReviewBase {}\n",
        &[bases("ReviewChild", &[text("ReviewHolder.ReviewBase")])],
    ),
    row(
        "Swift duplicate types with a member in one (control)",
        L::Swift,
        "swift",
        "struct ReviewHolder { class ReviewBase {} }\n\
         struct ReviewHolder {}\n\
         class ReviewChild: ReviewHolder.ReviewBase {}\n",
        &[unclassified("ReviewChild", &[text("ReviewHolder.ReviewBase")])],
    ),
    row(
        "TypeScript duplicate classes before a qualified base (control)",
        L::TypeScript,
        "ts",
        "class ReviewHolder {}\n\
         class ReviewHolder {}\n\
         class ReviewChild extends ReviewHolder.ReviewBase {}\n",
        &[bases("ReviewChild", &[text("ReviewHolder.ReviewBase")])],
    ),
    row(
        "JavaScript duplicate classes before a qualified base (control)",
        L::JavaScript,
        "js",
        "class ReviewHolder {}\n\
         class ReviewHolder {}\n\
         class ReviewChild extends ReviewHolder.ReviewBase {}\n",
        &[bases("ReviewChild", &[text("ReviewHolder.ReviewBase")])],
    ),
    row(
        "Python duplicate classes before a qualified base (control)",
        L::Python,
        "py",
        "class ReviewHolder:\n    class ReviewBase:\n        pass\n\
         class ReviewHolder:\n    pass\n\
         class ReviewChild(ReviewHolder.ReviewBase):\n    pass\n",
        &[bases("ReviewChild", &[text("ReviewHolder.ReviewBase")])],
    ),
    row(
        "Go duplicate interfaces before an embedding (control)",
        L::Go,
        "go",
        "package shapes\n\ntype ReviewBase interface{ M() }\ntype ReviewBase interface{ N() }\n\
         type ReviewChild interface{ ReviewBase }\n",
        &[bases("shapes.ReviewChild", &[text("ReviewBase")])],
    ),
];

/// Swift declarations inside an extension whose owner is not proved
/// (ambiguous, aliased, imported, external): the owner's generic
/// parameters and member types are unknown, so a base the lexical scopes
/// leave unbound keeps its text and its uncertain category, a raw-type
/// spelling included; a nearer lexical declaration still decides.
fn swift_unproved_owner_rows() -> Vec<PathMeaningRow> {
    fn leak(value: String) -> &'static str { Box::leak(value.into_boxed_str()) }
    let mut rows = Vec::new();
    let mut add = |label: &str, source: String, owner: &str, target: Target, resolved: bool| {
        let targets: &'static [Target] = Box::leak(vec![target].into_boxed_slice());
        let owners = Box::leak(vec![Owner {
            qualified_name: leak(owner.to_string()),
            extends: if resolved { targets } else { &[] },
            implements: &[],
            unclassified: if resolved { &[] } else { targets },
        }].into_boxed_slice());
        rows.push(row(leak(format!("Swift unproved extension owner: {label}")), L::Swift, "swift", leak(source), owners));
    };
    const AMBIGUOUS: &str = "struct ReviewHolder<ReviewBase> { struct ReviewInner {} }\nstruct ReviewHolder {}\n";
    add("ambiguous nested owner with a possible parameter",
        format!("class ReviewBase {{}}\n{AMBIGUOUS}extension ReviewHolder.ReviewInner {{ class ReviewChild: ReviewBase {{}} }}\n"),
        "ReviewHolder.ReviewInner.ReviewChild", text("ReviewBase"), false);
    add("alias-blocked owner",
        "class ReviewBase {}\ntypealias ReviewHolder = External\nextension ReviewHolder.ReviewInner { class ReviewChild: ReviewBase {} }\n".into(),
        "ReviewHolder.ReviewInner.ReviewChild", text("ReviewBase"), false);
    add("selectively imported owner",
        "import struct Lib.ReviewHolder\nclass ReviewBase {}\nextension ReviewHolder { class ReviewChild: ReviewBase {} }\n".into(),
        "ReviewHolder.ReviewChild", text("ReviewBase"), false);
    add("external owner",
        "class ReviewBase {}\nextension External { class ReviewChild: ReviewBase {} }\n".into(),
        "External.ReviewChild", text("ReviewBase"), false);
    add("nearer method-local class under an ambiguous owner",
        format!("class ReviewBase {{}}\n{AMBIGUOUS}extension ReviewHolder.ReviewInner {{ func probe() {{ class ReviewBase {{}}\nclass ReviewChild: ReviewBase {{}} }} }}\n"),
        "ReviewHolder.ReviewInner.probe.ReviewChild", to_at("ReviewBase", "ReviewHolder.ReviewInner.probe.ReviewBase", 4), true);
    add("nearer method-local class under an external owner",
        "class ReviewBase {}\nextension External { func probe() { class ReviewBase {}\nclass ReviewChild: ReviewBase {} } }\n".into(),
        "External.probe.ReviewChild", to_at("ReviewBase", "External.probe.ReviewBase", 2), true);
    let simple = ["Int", "Int8", "Int16", "Int32", "Int64", "UInt", "UInt8", "UInt16", "UInt32", "UInt64", "String", "Character", "Float", "Double"];
    let spellings = simple
        .iter()
        .flat_map(|name| [(*name, *name), ("Swift", leak(format!("Swift.{name}")))])
        .chain([("Unicode", "Unicode.Scalar"), ("Swift", "Swift.Unicode.Scalar")]);
    for (parameter, target) in spellings {
        add(&format!("raw type {target}"),
            format!("struct ReviewHolder<{parameter}> {{ struct ReviewInner {{}} }}\nstruct ReviewHolder {{}}\n\
                extension ReviewHolder.ReviewInner {{ enum ReviewChild: {target} {{ case a }} }}\n"),
            "ReviewHolder.ReviewInner.ReviewChild", text(target), false);
    }
    rows
}

static SWIFT_UNPROVED_OWNER_ROWS: std::sync::LazyLock<Vec<PathMeaningRow>> =
    std::sync::LazyLock::new(swift_unproved_owner_rows);

macro_rules! rust_holder {
    ($second:literal) => {
        concat!("mod ReviewHolder {\n",
            "    pub trait ReviewBase {}\n",
            "    pub trait ReviewChild: ReviewBase {}\n",
            "    pub trait ReviewSelf: self::ReviewBase {}\n",
            "    mod inner { pub trait ReviewUp: super::ReviewBase {} }\n",
            "}\n",
            "mod ReviewHolder { ", $second, " }\n")
    };
}

const fn rust_own(written: &'static str) -> Target {
    to_at(written, "ReviewHolder::ReviewBase", 2)
}

macro_rules! rust_impl_row {
    ($label:literal, $body:literal, $import:literal, $members:expr, $implements:expr) => {
        PathMeaningRow {
            label: concat!("Rust ", $label, ", sibling module imports `", $import, "`"),
            language: L::Rust,
            extension: "rs",
            source: concat!("mod ReviewHolder {\n",
                "    pub struct ReviewBase;\n",
                "    pub trait ReviewTrait {}\n",
                "    ", $body, "\n",
                "}\n",
                "mod ReviewHolder { use external::", $import, "; }\n"),
            owners: &[implementing("ReviewHolder::ReviewBase", $implements)],
            impl_members: $members,
            untouched: &[],
        }
    };
}

const OWN_TRAIT: Target = to_at("ReviewTrait", "ReviewHolder::ReviewTrait", 3);
const TRAIT_MEMBER: &[ImplMember] = &[ImplMember {
    method: "review_probe",
    container: "ReviewBase",
    owner: Some("ReviewHolder::ReviewBase"),
    implements: Some(OWN_TRAIT),
    container_kind: Some(SymbolKindV2::Type),
}];
const INHERENT_MEMBER: &[ImplMember] = &[ImplMember {
    method: "review_probe",
    container: "ReviewBase",
    owner: Some("ReviewHolder::ReviewBase"),
    implements: None,
    container_kind: Some(SymbolKindV2::Type),
}];

/// Lookups made inside one of several same-named owners read that
/// owner's own imports, wildcard and unrecorded bindings, inherited-name
/// barrier and possible built-in shadowing, never another same-named
/// owner's. Reopened namespaces and C# `partial` fragments still share one
/// lookup, and the same facts inside the actual owner still leave the
/// target unproved. The Rust impl rows check every
/// impl consumer: self type, trait, empty and inherent impls.
const SOURCE_OWNER_MATRIX: &[PathMeaningRow] = &[
        // --- Rust: imports and globs of a sibling module.
        row("Rust sibling module import: bare, self:: and inner super::", L::Rust, "rs",
            rust_holder!("use external::ReviewBase;"),
            &[bases("ReviewHolder::ReviewChild", &[rust_own("ReviewBase")]),
              bases("ReviewHolder::ReviewSelf", &[rust_own("self::ReviewBase")]),
              bases("ReviewHolder::inner::ReviewUp", &[rust_own("super::ReviewBase")])]),
        row("Rust sibling module glob: bare, and self:: as a control", L::Rust, "rs",
            rust_holder!("use external::*;"),
            &[bases("ReviewHolder::ReviewChild", &[rust_own("ReviewBase")]),
              bases("ReviewHolder::ReviewSelf", &[rust_own("self::ReviewBase")])]),
        row("Rust function-local modules, sibling import (control)", L::Rust, "rs",
            "fn review_scope() {\n\
             \x20   mod ReviewHolder { pub trait ReviewBase {} pub trait ReviewChild: ReviewBase {} pub trait ReviewSelf: self::ReviewBase {} }\n\
             \x20   mod ReviewHolder { use external::ReviewBase; }\n\
             }\n",
            &[bases("review_scope::ReviewHolder::ReviewChild", &[to_at("ReviewBase", "review_scope::ReviewHolder::ReviewBase", 2)]),
              bases("review_scope::ReviewHolder::ReviewSelf", &[to_at("self::ReviewBase", "review_scope::ReviewHolder::ReviewBase", 2)])]),
        row("Rust import inside the actual module (negative control)", L::Rust, "rs",
            "mod ReviewHolder {\n\
             \x20   pub trait ReviewBase {}\n\
             \x20   use external::ReviewBase;\n\
             \x20   pub trait ReviewChild: ReviewBase {}\n\
             \x20   pub trait ReviewSelf: self::ReviewBase {}\n\
             }\n\
             mod ReviewHolder {}\n",
            &[bases("ReviewHolder::ReviewChild", &[text("ReviewBase")]),
              bases("ReviewHolder::ReviewSelf", &[text("self::ReviewBase")])]),
        // --- Python: class-body imports and bindings of a sibling class.
        row("Python sibling class imports the base through builtins", L::Python, "py",
            "class ReviewHolder:\n    class ReviewBase:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\
             class ReviewHolder:\n    from builtins import object as ReviewBase\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Python sibling class imports the base", L::Python, "py",
            "class ReviewHolder:\n    class ReviewBase:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\
             class ReviewHolder:\n    from external import ReviewBase\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Python sibling class binds the base by a tuple target (control)", L::Python, "py",
            "class ReviewHolder:\n    class ReviewBase:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\
             class ReviewHolder:\n    ReviewBase, other = value\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Python sibling class with only an external base (control)", L::Python, "py",
            "class ReviewHolder:\n    class ReviewBase:\n        pass\n    class ReviewChild(ReviewBase):\n        pass\n\
             class ReviewHolder(External):\n    pass\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Python import inside the actual class (negative control)", L::Python, "py",
            "class ReviewHolder:\n    class ReviewBase:\n        pass\n    from external import ReviewBase\n    class ReviewChild(ReviewBase):\n        pass\n\
             class ReviewHolder:\n    pass\n",
            &[bases("ReviewHolder.ReviewChild", &[text("ReviewBase")])]),
        // --- Scala: imports, wildcards and patterns of a sibling object.
        row("Scala sibling object imports the base", L::Scala, "scala",
            "object ReviewHolder {\n  class ReviewBase\n  class ReviewChild extends ReviewBase\n}\n\
             object ReviewHolder { import external.ReviewBase }\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Scala sibling object wildcard import", L::Scala, "scala",
            "class ReviewBase\n\
             object ReviewHolder {\n  class ReviewChild extends ReviewBase\n}\n\
             object ReviewHolder { import external._ }\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewBase", 1)])]),
        row("Scala sibling object pattern bindings (control)", L::Scala, "scala",
            "object ReviewHolder {\n  class ReviewBase\n  class ReviewChild extends ReviewBase\n}\n\
             object ReviewHolder {\n  val (reviewBase, x) = (1, 2)\n  val (ReviewBase, y) = (3, 4)\n}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Scala import inside the actual object (negative control)", L::Scala, "scala",
            "object ReviewHolder {\n  class ReviewBase\n  import external.ReviewBase\n  class ReviewChild extends ReviewBase\n}\n\
             object ReviewHolder\n",
            &[unclassified("ReviewHolder.ReviewChild", &[text("ReviewBase")])]),
        // --- Inherited-name barriers of a sibling owner.
        row("Java sibling class with a base", L::Java, "java",
            "class ReviewBase {}\nclass ReviewHolder { static class ReviewChild extends ReviewBase {} }\nclass ReviewHolder extends External {}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewBase", 1)])]),
        row("Java actual class with a base (negative control)", L::Java, "java",
            "class ReviewBase {}\nclass ReviewHolder extends External { static class ReviewChild extends ReviewBase {} }\nclass ReviewHolder {}\n",
            &[bases("ReviewHolder.ReviewChild", &[text("ReviewBase")])]),
        row("Java own member beside a sibling's (control)", L::Java, "java",
            "class ReviewHolder { static class ReviewBase {} static class ReviewChild extends ReviewBase {} }\nclass ReviewHolder { static class ReviewBase {} }\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 1)])]),
        row("Java sibling generic parameter (control)", L::Java, "java",
            "class ReviewHolder { static class ReviewBase {} static class ReviewChild extends ReviewBase {} }\nclass ReviewHolder<ReviewBase> {}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 1)])]),
        row("Swift sibling type with a base", L::Swift, "swift",
            "class ReviewBase {}\nstruct ReviewHolder {\nclass ReviewChild: ReviewBase {}\n}\nstruct ReviewHolder: External {}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewBase", 1)])]),
        row("Swift actual type with a base (negative control)", L::Swift, "swift",
            "class ReviewBase {}\nstruct ReviewHolder: External {\nclass ReviewChild: ReviewBase {}\n}\nstruct ReviewHolder {}\n",
            &[unclassified("ReviewHolder.ReviewChild", &[text("ReviewBase")])]),
        row("Swift own member beside a sibling's (control)", L::Swift, "swift",
            "struct ReviewHolder {\nclass ReviewBase {}\nclass ReviewChild: ReviewBase {}\n}\nstruct ReviewHolder {\nclass ReviewBase {}\n}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Swift sibling generic parameter (control)", L::Swift, "swift",
            "struct ReviewHolder {\nclass ReviewBase {}\nclass ReviewChild: ReviewBase {}\n}\nstruct ReviewHolder<ReviewBase> {}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("Swift extension of an unproved nested owner, sibling with a base (control)", L::Swift, "swift",
            "class ReviewBase {}\nstruct ReviewHolder {\nstruct ReviewInner {}\n}\nstruct ReviewHolder: External {}\n\
             extension ReviewHolder.ReviewInner {\nclass ReviewChild: ReviewBase {}\n}\n",
            &[unclassified("ReviewHolder.ReviewInner.ReviewChild", &[text("ReviewBase")])]),
        row("Scala sibling object with a base", L::Scala, "scala",
            "class ReviewBase\nobject ReviewHolder {\n  class ReviewChild extends ReviewBase\n}\nobject ReviewHolder extends External\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewBase", 1)])]),
        row("Scala sibling generic class (control)", L::Scala, "scala",
            "object ReviewHolder {\n  class ReviewBase\n  class ReviewChild extends ReviewBase\n}\nclass ReviewHolder[ReviewBase]\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("C# sibling non-partial class with a base", L::CSharp, "cs",
            "class ReviewBase {}\nclass ReviewHolder { class ReviewChild : ReviewBase {} }\nclass ReviewHolder : External {}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewBase", 1)])]),
        row("C# own member beside a non-partial sibling's", L::CSharp, "cs",
            "class ReviewHolder { class ReviewBase {} class ReviewChild : ReviewBase {} }\nclass ReviewHolder { class ReviewBase {} }\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 1)])]),
        row("C# partial fragments share members (control)", L::CSharp, "cs",
            "partial class ReviewHolder { class ReviewChild : ReviewBase {} }\npartial class ReviewHolder { class ReviewBase {} }\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 2)])]),
        row("C# partial fragments, qualified path from outside (control)", L::CSharp, "cs",
            "partial class ReviewHolder { class ReviewBase {} }\npartial class ReviewHolder {}\nclass ReviewChild : ReviewHolder.ReviewBase {}\n",
            &[bases("ReviewChild", &[to_at("ReviewHolder.ReviewBase", "ReviewHolder.ReviewBase", 1)])]),
        row("C# non-partial types, qualified path from outside (control)", L::CSharp, "cs",
            "class ReviewHolder { class ReviewBase {} }\nclass ReviewHolder {}\nclass ReviewChild : ReviewHolder.ReviewBase {}\n",
            &[unclassified("ReviewChild", &[text("ReviewHolder.ReviewBase")])]),
        row("C# partial fragment with a base is the whole type's barrier (control)", L::CSharp, "cs",
            "class ReviewBase {}\npartial class ReviewHolder { class ReviewChild : ReviewBase {} }\npartial class ReviewHolder : External {}\n",
            &[unclassified("ReviewHolder.ReviewChild", &[text("ReviewBase")])]),
        row("C# sibling generic parameter (control)", L::CSharp, "cs",
            "class ReviewHolder { class ReviewBase {} class ReviewChild : ReviewBase {} }\nclass ReviewHolder<ReviewBase> {}\n",
            &[bases("ReviewHolder.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.ReviewBase", 1)])]),
        // --- C++.
        row("C++ struct owner beside a same-named namespace's using-directive", L::Cpp, "cpp",
            "struct ReviewHolder { struct ReviewBase {}; struct ReviewChild : ReviewBase {}; };\n\
             namespace ReviewHolder { using namespace external; }\n",
            &[bases("ReviewHolder::ReviewChild", &[to_at("ReviewBase", "ReviewHolder::ReviewBase", 1)])]),
        row("C++ using-directive in the actual namespace (negative control)", L::Cpp, "cpp",
            "struct ReviewBase {};\nnamespace ReviewHolder { using namespace external; struct ReviewChild : ReviewBase {}; }\n",
            &[bases("ReviewHolder::ReviewChild", &[text("ReviewBase")])]),
        row("C++ own member beside a sibling's (control)", L::Cpp, "cpp",
            "struct ReviewHolder { struct ReviewBase {}; struct ReviewChild : ReviewBase {}; };\nstruct ReviewHolder { struct ReviewBase {}; };\n",
            &[bases("ReviewHolder::ReviewChild", &[to_at("ReviewBase", "ReviewHolder::ReviewBase", 1)])]),
        row("C++ sibling struct with a base", L::Cpp, "cpp",
            "struct ReviewBase {};\nstruct ReviewHolder { struct ReviewChild : ReviewBase {}; };\nstruct ReviewHolder : External {};\n",
            &[bases("ReviewHolder::ReviewChild", &[to_at("ReviewBase", "ReviewBase", 1)])]),
        row("C++ sibling template parameter (control)", L::Cpp, "cpp",
            "struct ReviewHolder { struct ReviewBase {}; struct ReviewChild : ReviewBase {}; };\ntemplate <typename ReviewBase> struct ReviewHolder {};\n",
            &[bases("ReviewHolder::ReviewChild", &[to_at("ReviewBase", "ReviewHolder::ReviewBase", 1)])]),
        // --- Intentional sharing and other languages' controls.
        row("TypeScript reopened namespace, unqualified base in a sibling fragment (control)", L::TypeScript, "ts",
            "namespace ReviewNs { export class ReviewBase {} }\nnamespace ReviewNs { class ReviewChild extends ReviewBase {} }\n",
            &[bases("ReviewNs.ReviewChild", &[to_at("ReviewBase", "ReviewNs.ReviewBase", 1)])]),
        row("PHP reopened block namespace, unqualified base (control)", L::Php, "php",
            "<?php\nnamespace ReviewNs { class ReviewBase {} }\nnamespace ReviewNs { class ReviewChild extends ReviewBase {} }\n",
            &[bases("ReviewNs\\ReviewChild", &[to_at("ReviewBase", "ReviewNs\\ReviewBase", 2)])]),
        row("JavaScript method-local base, sibling class with a base (control)", L::JavaScript, "js",
            "class ReviewHolder { review() { class ReviewBase {}\nclass ReviewChild extends ReviewBase {} } }\nclass ReviewHolder extends External {}\n",
            &[bases("ReviewHolder.review.ReviewChild", &[to_at("ReviewBase", "ReviewHolder.review.ReviewBase", 1)])]),
        row("Go same-named interfaces, one embedding External (control)", L::Go, "go",
            "package shapes\n\ntype ReviewBase interface{ M() }\ntype ReviewHolder interface{ N() }\ntype ReviewHolder interface{ External }\n\
             type ReviewChild interface{ ReviewBase }\n",
            &[bases("shapes.ReviewChild", &[to_at("ReviewBase", "shapes.ReviewBase", 3)])]),
    // --- Rust impls inside the first module; the sibling imports one name.
    rust_impl_row!("trait impl's self type", "impl ReviewTrait for ReviewBase { fn review_probe(&self) {} }", "ReviewBase", TRAIT_MEMBER, &[OWN_TRAIT]),
    rust_impl_row!("trait impl's trait", "impl ReviewTrait for ReviewBase { fn review_probe(&self) {} }", "ReviewTrait", TRAIT_MEMBER, &[OWN_TRAIT]),
    rust_impl_row!("empty trait impl", "impl ReviewTrait for ReviewBase {}", "ReviewBase", &[], &[OWN_TRAIT]),
    rust_impl_row!("inherent impl", "impl ReviewBase { fn review_probe(&self) {} }", "ReviewBase", INHERENT_MEMBER, &[]),
];

/// Every Swift standard raw-type spelling whose first segment only the
/// second of two same-named owners declares: no base, as without it.
fn swift_sibling_shadow_rows() -> Vec<PathMeaningRow> {
    fn leak(value: String) -> &'static str { Box::leak(value.into_boxed_str()) }
    let mut rows = Vec::new();
    let simple = ["Int", "Int8", "Int16", "Int32", "Int64", "UInt", "UInt8", "UInt16", "UInt32", "UInt64", "String", "Character", "Float", "Double"];
    let spellings = simple
        .iter()
        .flat_map(|name| [(*name, *name), ("Swift", leak(format!("Swift.{name}")))])
        .chain([("Unicode", "Unicode.Scalar"), ("Swift", "Swift.Unicode.Scalar")]);
    for (shadow, target) in spellings {
        rows.push(row(leak(format!("Swift raw type {target} shadowed only in a sibling owner")), L::Swift, "swift",
            leak(format!("struct ReviewHolder {{\nenum ReviewChild: {target} {{ case value }}\n}}\nstruct ReviewHolder {{\nclass {shadow} {{}}\n}}\n")),
            Box::leak(vec![bases("ReviewHolder.ReviewChild", &[])].into_boxed_slice())));
    }
    rows
}

static SWIFT_SIBLING_SHADOW_ROWS: std::sync::LazyLock<Vec<PathMeaningRow>> = std::sync::LazyLock::new(swift_sibling_shadow_rows);

/// The six written C# forms a `partial` type may take; `record` and
/// `record class` are one category.
const CSHARP_PARTIAL_FORMS: [&str; 6] = ["class", "struct", "interface", "record", "record struct", "record class"];

/// Whether two written C# forms declare one category of type.
fn csharp_forms_compatible(first: &str, second: &str) -> bool {
    fn category(form: &str) -> &str { if form == "record class" { "record" } else { form } }
    category(first) == category(second)
}

/// The C# `partial` identity rows: declarations of one bound name are
/// fragments of one type only when every fragment is `partial` and they
/// agree on category, generic arity and enclosing owner. `ReviewChild`
/// sits in the first owner and writes `ReviewBase`, which (unless the row
/// says otherwise) only the second owner declares: a genuine fragment's
/// member resolves (`extends`, line 2), any other owner's stays text with
/// no ID in `unclassified_bases`.
fn csharp_partial_identity_rows() -> Vec<PathMeaningRow> {
    fn leak(value: String) -> &'static str { Box::leak(value.into_boxed_str()) }
    fn owners(owner: &str, target: Target) -> &'static [Owner] {
        let targets: &'static [Target] = Box::leak(vec![target].into_boxed_slice());
        let resolved = target.resolves_to.is_some();
        Box::leak(vec![Owner {
            qualified_name: leak(owner.to_string()),
            extends: if resolved { targets } else { &[] },
            implements: &[],
            unclassified: if resolved { &[] } else { targets },
        }].into_boxed_slice())
    }
    const CHILD: &str = "ReviewHolder.ReviewChild";
    const SECOND: Target = to_at("ReviewBase", "ReviewHolder.ReviewBase", 2);
    let parameters = |arity: usize| ["", "<T>", "<T, U>"][arity];
    let pair = |first: &str, second: &str| {
        format!("partial {first} {{ class ReviewChild : ReviewBase {{}} }}\npartial {second} {{ class ReviewBase {{}} }}\n")
    };
    let mut rows = Vec::new();
    let mut add = |label: String, source: String, owners: &'static [Owner]| {
        rows.push(row(leak(format!("C# partial identity: {label}")), L::CSharp, "cs", leak(source), owners));
    };
    for form in CSHARP_PARTIAL_FORMS {
        let arities: &[(usize, usize)] = if form == "record class" { &[(1, 2)] } else { &[(0, 1), (0, 2), (1, 2)] };
        for &(first, second) in arities {
            add(format!("{form} arities {first}/{second}"),
                pair(&format!("{form} ReviewHolder{}", parameters(first)), &format!("{form} ReviewHolder{}", parameters(second))),
                owners(CHILD, text("ReviewBase")));
        }
    }
    for form in CSHARP_PARTIAL_FORMS {
        let arities: &[usize] = if form == "record class" { &[1] } else { &[0, 1] };
        for &arity in arities {
            let written = format!("{form} ReviewHolder{}", parameters(arity));
            add(format!("matching {form} fragments at arity {arity} (control)"), pair(&written, &written), owners(CHILD, SECOND));
        }
    }
    for first in CSHARP_PARTIAL_FORMS {
        for second in CSHARP_PARTIAL_FORMS.into_iter().filter(|second| *second != first) {
            let compatible = csharp_forms_compatible(first, second);
            add(format!("{first} then {second}{}", if compatible { " (compatible control)" } else { "" }),
                pair(&format!("{first} ReviewHolder"), &format!("{second} ReviewHolder")),
                owners(CHILD, if compatible { SECOND } else { text("ReviewBase") }));
        }
    }
    add("own member beside a different arity's".into(),
        "partial class ReviewHolder { class ReviewBase {} class ReviewChild : ReviewBase {} }\n\
         partial class ReviewHolder<T> { class ReviewBase {} }\n".into(),
        owners(CHILD, to_at("ReviewBase", "ReviewHolder.ReviewBase", 1)));
    add("inherited-name barrier only on a different arity".into(),
        "class ReviewBase {}\npartial class ReviewHolder { class ReviewChild : ReviewBase {} }\n\
         partial class ReviewHolder<T> : External {}\n".into(),
        owners(CHILD, to_at("ReviewBase", "ReviewBase", 1)));
    add("outside path, member only in the generic type".into(),
        "partial class ReviewHolder {}\npartial class ReviewHolder<T> { public class ReviewBase {} }\n\
         class ReviewChild : ReviewHolder.ReviewBase {}\n".into(),
        owners("ReviewChild", text("ReviewHolder.ReviewBase")));
    add("outside path, bare name denotes the arity-0 type (control)".into(),
        "partial class ReviewHolder { public class ReviewBase {} }\npartial class ReviewHolder<T> { public class ReviewBase {} }\n\
         class ReviewChild : ReviewHolder.ReviewBase {}\n".into(),
        owners("ReviewChild", to_at("ReviewHolder.ReviewBase", "ReviewHolder.ReviewBase", 1)));
    add("outside path, written type argument denotes the generic type (control)".into(),
        "partial class ReviewHolder { public class ReviewBase {} }\npartial class ReviewHolder<T> { public class ReviewBase {} }\n\
         class ReviewChild : ReviewHolder<int>.ReviewBase {}\n".into(),
        owners("ReviewChild", to_at("ReviewHolder<int>.ReviewBase", "ReviewHolder.ReviewBase", 2)));
    add("nested owner, partial enclosing types of different arity".into(),
        "partial class ReviewOuter { partial class ReviewHolder { class ReviewChild : ReviewBase {} } }\n\
         partial class ReviewOuter<T> { partial class ReviewHolder { class ReviewBase {} } }\n".into(),
        owners("ReviewOuter.ReviewHolder.ReviewChild", text("ReviewBase")));
    add("nested owner, non-partial enclosing types of different arity".into(),
        "class ReviewOuter { partial class ReviewHolder { class ReviewChild : ReviewBase {} } }\n\
         class ReviewOuter<T> { partial class ReviewHolder { class ReviewBase {} } }\n".into(),
        owners("ReviewOuter.ReviewHolder.ReviewChild", text("ReviewBase")));
    add("nested owner, matching partial enclosing fragments (control)".into(),
        "partial class ReviewOuter { partial class ReviewHolder { class ReviewChild : ReviewBase {} } }\n\
         partial class ReviewOuter { partial class ReviewHolder { class ReviewBase {} } }\n".into(),
        owners("ReviewOuter.ReviewHolder.ReviewChild", to_at("ReviewBase", "ReviewOuter.ReviewHolder.ReviewBase", 2)));
    add("namespace beside a partial type".into(),
        "partial class ReviewHolder { class ReviewChild : ReviewBase {} }\nnamespace ReviewHolder { class ReviewBase {} }\n".into(),
        owners(CHILD, text("ReviewBase")));
    add("matching fragments beside a different arity (control)".into(),
        "partial class ReviewHolder { class ReviewChild : ReviewBase {} }\npartial class ReviewHolder<T> { class ReviewBase {} }\n\
         partial class ReviewHolder { class ReviewBase {} }\n".into(),
        owners(CHILD, to_at("ReviewBase", "ReviewHolder.ReviewBase", 3)));
    add("generic parameter of a different arity (control)".into(),
        "class ReviewBase {}\npartial class ReviewHolder { class ReviewChild : ReviewBase {} }\n\
         partial class ReviewHolder<ReviewBase> {}\n".into(),
        owners(CHILD, to_at("ReviewBase", "ReviewBase", 1)));
    add("non-partial types of different arity (control)".into(),
        "class ReviewHolder { class ReviewChild : ReviewBase {} }\nclass ReviewHolder<T> { class ReviewBase {} }\n".into(),
        owners(CHILD, text("ReviewBase")));
    rows.push(PathMeaningRow {
        label: "C# partial identity: containment, Probe in the first owner's Child (control)",
        language: L::CSharp,
        extension: "cs",
        source: CSHARP_PARTIAL_CONTAINMENT,
        owners: owners(CHILD, text("ReviewBase")),
        impl_members: &[ImplMember {
            method: "Probe",
            container: "ReviewHolder.ReviewChild",
            owner: Some("ReviewHolder.ReviewChild"),
            implements: None,
            container_kind: Some(SymbolKindV2::Class),
        }],
        untouched: &[],
    });
    rows
}

const CSHARP_PARTIAL_CONTAINMENT: &str = "partial class ReviewHolder { class ReviewChild : ReviewBase { void Probe() {} } }\n\
    partial class ReviewHolder<T> { class ReviewBase {} }\n";

static CSHARP_PARTIAL_IDENTITY_ROWS: std::sync::LazyLock<Vec<PathMeaningRow>> =
    std::sync::LazyLock::new(csharp_partial_identity_rows);

/// The C# identity rows hold every swept shape: 16 arity rows, 11
/// matching-fragment controls, the 30 directed category pairs (two of them
/// the `record`/`record class` controls) and 13 owner, path, enclosing,
/// namespace, generic, non-partial and containment rows.
#[test]
fn csharp_partial_identity_rows_cover_every_swept_shape() {
    let rows = &*CSHARP_PARTIAL_IDENTITY_ROWS;
    assert_eq!(rows.len(), 70);
    assert_eq!(rows.iter().filter(|row| row.label.contains(" arities ")).count(), 16);
    assert_eq!(rows.iter().filter(|row| row.label.contains(": matching ") && row.label.contains(" at arity ")).count(), 11);
    assert_eq!(rows.iter().filter(|row| row.label.contains(" then ")).count(), 30);
    assert_eq!(rows.iter().filter(|row| row.label.contains(" then ") && row.label.ends_with("(compatible control)")).count(), 2);
}

/// The containment control keeps source-contained memberships: `ReviewChild`
/// belongs to the first (non-generic) `ReviewHolder` and `Probe` to
/// `ReviewChild`, on the complete index and on a Containment query, while
/// its base names no declaration.
#[test]
fn csharp_partial_identity_keeps_source_contained_memberships() {
    let file = write_temp(CSHARP_PARTIAL_CONTAINMENT, "cs");
    let tree_file = TreeFile::new(file.path()).expect("parse");
    assert!(tree_file.syntax_diagnostics().is_empty(), "{:?}", tree_file.syntax_diagnostics());
    let complete = tree_file.symbol_index_v2().expect("complete index");
    let contained = tree_file
        .query(&SymbolQuery::new().kinds([SymbolKindV2::Class, SymbolKindV2::Method]).capabilities([tree_hugger::Capability::Containment]))
        .expect("containment query")
        .index;
    for (label, index) in [("complete", &complete), ("containment", &contained)] {
        let holder = type_record_at(index, label, "ReviewHolder", Some(1));
        let child = type_record(index, label, "ReviewHolder.ReviewChild");
        let probe = method(index, "Probe");
        let parent = |record: &SymbolRecord| record.relations.parent.as_ref().and_then(|parent| parent.id.clone());
        assert_eq!(parent(child), Some(holder.id.clone()), "{label}: ReviewChild belongs to the first ReviewHolder");
        assert_eq!(parent(probe), Some(child.id.clone()), "{label}: Probe belongs to ReviewChild");
        assert!(holder.relations.members.iter().any(|member| member.id.as_ref() == Some(&child.id)), "{label}: listed");
        let [base] = &child.relations.unclassified_bases[..] else { panic!("{label}: one uncertain base") };
        assert_eq!((base.name.as_str(), base.id.as_ref(), base.qualified_name.as_deref()), ("ReviewBase", None, None), "{label}");
        assert!(child.relations.extends.is_empty(), "{label}: no proved superclass");
    }
}

/// Generic parameter lists of arity 0, 1 and 2, as declared and as written.
const CSHARP_DECLARED_ARITY: [&str; 3] = ["", "<T>", "<T, U>"];
const CSHARP_WRITTEN_ARITY: [&str; 3] = ["", "<int>", "<int, string>"];

/// The lookup sites a C# written generic argument count must reach, each
/// built for every target form, `partial` or not.
#[derive(Clone, Copy, Debug)]
enum ArityLookup {
    /// `ReviewBase<…>` beside every arity of `ReviewBase`.
    Bare,
    /// `ReviewSpace.ReviewBase<…>` into a namespace holding every arity.
    NamespaceQualified,
    /// `ReviewHolder.ReviewBase<…>` into the one `ReviewHolder`.
    Nested,
    /// `ReviewBase<…>` from inside `ReviewHolder`, which holds every arity
    /// and competes with an outside `ReviewHolder<T>`.
    AnchoredMember,
    /// `ReviewHolder<…>.ReviewBase` from inside `ReviewOuter`, which holds
    /// `ReviewHolder` of every arity and competes with `ReviewOuter<T>`; the
    /// form is the intermediate owner's.
    AnchoredIntermediate,
    /// `ReviewBase<…>` from inside the one `ReviewHolder`, which holds only
    /// one arity, beside a file-level `ReviewBase` of every arity.
    OutwardFromType,
    /// The same inside a `ReviewHolder` competing with `ReviewHolder<T>`.
    OutwardFromAnchoredType,
    /// The same inside `namespace ReviewSpace`.
    OutwardFromNamespace,
    /// `global::ReviewBase<…>` from inside a `ReviewHolder` holding one
    /// arity of its own.
    Rooted,
}

const ARITY_LOOKUPS: [ArityLookup; 9] = [
    ArityLookup::Bare,
    ArityLookup::NamespaceQualified,
    ArityLookup::Nested,
    ArityLookup::AnchoredMember,
    ArityLookup::AnchoredIntermediate,
    ArityLookup::OutwardFromType,
    ArityLookup::OutwardFromAnchoredType,
    ArityLookup::OutwardFromNamespace,
    ArityLookup::Rooted,
];

/// A generated C# source: its lines, and for each `ReviewChild{arity}` the
/// target its one written base must name.
#[derive(Default)]
struct ArityRowBuilder {
    lines: Vec<String>,
    children: Vec<(String, Target)>,
}

impl ArityRowBuilder {
    fn leak(value: String) -> &'static str { Box::leak(value.into_boxed_str()) }

    /// Appends `line`, returning its one-based number.
    fn line(&mut self, line: String) -> u32 {
        self.lines.push(line);
        u32::try_from(self.lines.len()).expect("small source")
    }

    /// A `ReviewChild{arity}` (in-file name `owner`) writing `written`,
    /// which names the declaration `target` on `line` or (`None`) nothing.
    fn child(&mut self, indent: &str, owner: String, written: String, target: Option<(String, u32)>) {
        let name = owner.rsplit('.').next().expect("name").to_string();
        self.line(format!("{indent}class {name} : {written} {{}}"));
        let written = Self::leak(written);
        let target = match target {
            Some((qualified_name, line)) => to_at(written, Self::leak(qualified_name), line),
            None => text(written),
        };
        self.children.push((owner, target));
    }

    /// The row: a class child's base is `extends` for a class, `record` or
    /// `record class` target, `implements` for an interface, uncertain for
    /// a struct or `record struct` (the C# classification rules); an
    /// unresolved base is uncertain.
    fn row(self, label: String, form: &str) -> PathMeaningRow {
        let owners: Vec<Owner> = self
            .children
            .into_iter()
            .map(|(owner, target)| {
                let targets: &'static [Target] = Box::leak(vec![target].into_boxed_slice());
                let qualified_name = Self::leak(owner);
                match (target.resolves_to, form) {
                    (Some(_), "class" | "record" | "record class") => bases(qualified_name, targets),
                    (Some(_), "interface") => implementing(qualified_name, targets),
                    _ => unclassified(qualified_name, targets),
                }
            })
            .collect();
        let source = Self::leak(self.lines.join("\n") + "\n");
        row(Self::leak(format!("C# written arity: {label}")), L::CSharp, "cs", source, Box::leak(owners.into_boxed_slice()))
    }
}

/// One row per [`ArityLookup`] site, target form and `partial` state:
/// every arity of the target family is declared, each on its own line, and
/// `ReviewChild0..2` write the counts 0, 1 and 2, so each must name the one
/// declaration of its arity (a same-named declaration of another arity
/// neither competes nor hides it). Outward sites put one arity, `near`,
/// in the nearer scope (two different `near` per form across the
/// `partial` states, so every outer arity is reached outward): the child
/// writing `near` takes the nearer one, the others the file's.
fn csharp_written_arity_rows() -> Vec<PathMeaningRow> {
    let mut rows = Vec::new();
    for lookup in ARITY_LOOKUPS {
        for (form_index, form) in CSHARP_PARTIAL_FORMS.into_iter().enumerate() {
            for partial in [false, true] {
                let near = (form_index + usize::from(partial)) % 3;
                let keyword = if partial { format!("partial {form}") } else { form.to_string() };
                let declare = |name: &str, arity: usize| format!("{keyword} {name}{} {{}}", CSHARP_DECLARED_ARITY[arity]);
                let mut source = ArityRowBuilder::default();
                let family = |source: &mut ArityRowBuilder, indent: &str| -> [u32; 3] {
                    [0, 1, 2].map(|arity| source.line(format!("{indent}{}", declare("ReviewBase", arity))))
                };
                match lookup {
                    ArityLookup::Bare => {
                        let lines = family(&mut source, "");
                        for arity in 0..3 {
                            source.child("", format!("ReviewChild{arity}"),
                                format!("ReviewBase{}", CSHARP_WRITTEN_ARITY[arity]), Some(("ReviewBase".into(), lines[arity])));
                        }
                    }
                    ArityLookup::NamespaceQualified | ArityLookup::Nested => {
                        let owner = if matches!(lookup, ArityLookup::Nested) { "class ReviewHolder" } else { "namespace ReviewSpace" };
                        let prefix = if matches!(lookup, ArityLookup::Nested) { "ReviewHolder" } else { "ReviewSpace" };
                        source.line(format!("{owner} {{"));
                        let lines = family(&mut source, "    ");
                        source.line("}".into());
                        for arity in 0..3 {
                            source.child("", format!("ReviewChild{arity}"),
                                format!("{prefix}.ReviewBase{}", CSHARP_WRITTEN_ARITY[arity]),
                                Some((format!("{prefix}.ReviewBase"), lines[arity])));
                        }
                    }
                    ArityLookup::AnchoredMember => {
                        source.line("class ReviewHolder {".into());
                        let lines = family(&mut source, "    ");
                        for arity in 0..3 {
                            source.child("    ", format!("ReviewHolder.ReviewChild{arity}"),
                                format!("ReviewBase{}", CSHARP_WRITTEN_ARITY[arity]),
                                Some(("ReviewHolder.ReviewBase".into(), lines[arity])));
                        }
                        source.line("}".into());
                        source.line("class ReviewHolder<T> {}".into());
                    }
                    ArityLookup::AnchoredIntermediate => {
                        source.line("class ReviewOuter {".into());
                        let lines = [0, 1, 2].map(|arity| {
                            let line = source.line(format!("    {keyword} ReviewHolder{} {{", CSHARP_DECLARED_ARITY[arity]));
                            source.line("        public class ReviewBase {}".into());
                            source.line("    }".into());
                            line + 1
                        });
                        for arity in 0..3 {
                            source.child("    ", format!("ReviewOuter.ReviewChild{arity}"),
                                format!("ReviewHolder{}.ReviewBase", CSHARP_WRITTEN_ARITY[arity]),
                                Some(("ReviewOuter.ReviewHolder.ReviewBase".into(), lines[arity])));
                        }
                        source.line("}".into());
                        source.line("class ReviewOuter<T> {}".into());
                    }
                    ArityLookup::OutwardFromType | ArityLookup::OutwardFromAnchoredType | ArityLookup::OutwardFromNamespace
                    | ArityLookup::Rooted => {
                        let outer = family(&mut source, "");
                        let (open, scope) = match lookup {
                            ArityLookup::OutwardFromNamespace => ("namespace ReviewSpace {", "ReviewSpace"),
                            _ => ("class ReviewHolder {", "ReviewHolder"),
                        };
                        source.line(open.into());
                        let nearer = source.line(format!("    {}", declare("ReviewBase", near)));
                        for arity in 0..3 {
                            let rooted = matches!(lookup, ArityLookup::Rooted);
                            let target = if arity == near && !rooted {
                                (format!("{scope}.ReviewBase"), nearer)
                            } else {
                                ("ReviewBase".to_string(), outer[arity])
                            };
                            source.child("    ", format!("{scope}.ReviewChild{arity}"),
                                format!("{}ReviewBase{}", if rooted { "global::" } else { "" }, CSHARP_WRITTEN_ARITY[arity]),
                                Some(target));
                        }
                        source.line("}".into());
                        if matches!(lookup, ArityLookup::OutwardFromAnchoredType) {
                            source.line("class ReviewHolder<T> {}".into());
                        }
                    }
                }
                // The intermediate site's targets are its owners' classes.
                let target_form = if matches!(lookup, ArityLookup::AnchoredIntermediate) { "class" } else { form };
                rows.push(source.row(format!("{lookup:?} {keyword}"), target_form));
            }
        }
    }
    rows.extend(csharp_written_arity_controls());
    rows
}

/// The barriers and competitions the written count does not lift: two
/// declarations of the written arity (non-`partial` duplicates or repeated
/// `partial` fragments, bare, nested and in an anchored owner) name
/// nothing, a count no declaration has names nothing, a using alias or a
/// type parameter of the name ends the lookup; and the clean named
/// intermediate paths.
fn csharp_written_arity_controls() -> Vec<PathMeaningRow> {
    let written = |arity: usize| format!("ReviewBase{}", CSHARP_WRITTEN_ARITY[arity]);
    let mut rows = Vec::new();
    for (label, partial) in [("two non-partial ReviewBase<T>", ""), ("repeated partial ReviewBase<T> fragments", "partial ")] {
        let mut source = ArityRowBuilder::default();
        let lines = [
            source.line(format!("{partial}class ReviewBase {{}}")),
            source.line(format!("{partial}class ReviewBase<T> {{}}")),
            source.line(format!("{partial}class ReviewBase<T, U> {{}}")),
        ];
        source.line(format!("{partial}class ReviewBase<T> {{}}"));
        for (arity, line) in lines.into_iter().enumerate() {
            let target = (arity != 1).then(|| ("ReviewBase".to_string(), line));
            source.child("", format!("ReviewChild{arity}"), written(arity), target);
        }
        rows.push(source.row(format!("same-arity competition, {label}"), "class"));
    }
    let mut source = ArityRowBuilder::default();
    source.line("class ReviewHolder {".into());
    let zero = source.line("    class ReviewBase {}".into());
    source.line("    class ReviewBase<T> {}".into());
    source.line("    class ReviewBase<T> {}".into());
    source.child("    ", "ReviewHolder.ReviewChild0".into(), written(0), Some(("ReviewHolder.ReviewBase".into(), zero)));
    source.child("    ", "ReviewHolder.ReviewChild1".into(), written(1), None);
    source.line("}".into());
    source.line("class ReviewHolder<T> {}".into());
    rows.push(source.row("same-arity competition in an anchored owner".into(), "class"));
    let mut source = ArityRowBuilder::default();
    source.line("class ReviewHolder {".into());
    let zero = source.line("    public class ReviewBase {}".into());
    source.line("    public class ReviewBase<T> {}".into());
    source.line("    public class ReviewBase<T> {}".into());
    source.line("}".into());
    source.child("", "ReviewChild0".into(), "ReviewHolder.ReviewBase".into(), Some(("ReviewHolder.ReviewBase".into(), zero)));
    source.child("", "ReviewChild1".into(), "ReviewHolder.ReviewBase<int>".into(), None);
    rows.push(source.row("same-arity competition through a nested path".into(), "class"));
    let mut source = ArityRowBuilder::default();
    let one = source.line("class ReviewBase<T> {}".into());
    source.child("", "ReviewChild0".into(), written(0), None);
    source.child("", "ReviewChild1".into(), written(1), Some(("ReviewBase".into(), one)));
    source.child("", "ReviewChild2".into(), written(2), None);
    rows.push(source.row("wrong-arity negatives beside the only declaration".into(), "class"));
    let mut source = ArityRowBuilder::default();
    source.line("class ReviewBase<T, U> {}".into());
    source.line("class ReviewHolder {".into());
    let near = source.line("    class ReviewBase<T> {}".into());
    source.child("    ", "ReviewHolder.ReviewChild0".into(), written(0), None);
    source.child("    ", "ReviewHolder.ReviewChild1".into(), written(1), Some(("ReviewHolder.ReviewBase".into(), near)));
    source.line("}".into());
    rows.push(source.row("outward lookup finds no declaration of the written arity".into(), "class"));
    let mut source = ArityRowBuilder::default();
    source.line("using ReviewBase = System.Object;".into());
    source.line("class ReviewBase<T> {}".into());
    source.child("", "ReviewChild1".into(), written(1), None);
    rows.push(source.row("a using alias of the name stays a barrier".into(), "class"));
    let mut source = ArityRowBuilder::default();
    source.line("class ReviewBase<T> {}".into());
    source.line("class ReviewHolder<ReviewBase> {".into());
    source.child("    ", "ReviewHolder.ReviewChild1".into(), written(1), None);
    source.line("}".into());
    rows.push(source.row("a type parameter of the name stays a barrier".into(), "class"));
    let mut source = ArityRowBuilder::default();
    source.line("class ReviewOuter {".into());
    let lines = [0, 1, 2].map(|arity| {
        let line = source.line(format!("    public class ReviewHolder{} {{", CSHARP_DECLARED_ARITY[arity]));
        source.line("        public class ReviewBase {}".into());
        source.line("    }".into());
        line + 1
    });
    source.line("}".into());
    for arity in 0..3 {
        source.child("", format!("ReviewChild{arity}"),
            format!("ReviewOuter.ReviewHolder{}.ReviewBase", CSHARP_WRITTEN_ARITY[arity]),
            Some(("ReviewOuter.ReviewHolder.ReviewBase".into(), lines[arity])));
    }
    for (arity, arguments) in CSHARP_WRITTEN_ARITY.into_iter().enumerate() {
        source.child("", format!("ReviewDirect{arity}"), format!("ReviewHolder{arguments}.ReviewBase"), None);
    }
    rows.push(source.row("named intermediate owners of every arity (control)".into(), "class"));
    rows
}

static CSHARP_WRITTEN_ARITY_ROWS: std::sync::LazyLock<Vec<PathMeaningRow>> =
    std::sync::LazyLock::new(csharp_written_arity_rows);

/// Every [`ArityLookup`] site has a row for each of the six forms in both
/// `partial` states, plus the nine barrier, competition and intermediate
/// controls; every source parses (checked by the path-meaning consumers).
#[test]
fn csharp_written_arity_rows_cover_every_swept_site() {
    let rows = &*CSHARP_WRITTEN_ARITY_ROWS;
    for lookup in ARITY_LOOKUPS {
        let prefix = format!("C# written arity: {lookup:?} ");
        assert_eq!(rows.iter().filter(|row| row.label.starts_with(&prefix)).count(), 12, "{lookup:?}");
    }
    assert_eq!(rows.len(), 9 * 12 + 9);
    let children: usize = rows.iter().map(|row| row.owners.len()).sum();
    assert!(children >= 9 * 12 * 3, "every written arity at every site ({children})");
}

/// The source-owner rows cover all four Rust impl consumers and all 30
/// Swift raw-type spellings.
#[test]
fn source_owner_rows_cover_every_swept_site() {
    assert_eq!(SWIFT_SIBLING_SHADOW_ROWS.len(), 30);
    assert_eq!(SOURCE_OWNER_MATRIX.iter().filter(|row| row.label.contains(", sibling module imports")).count(), 4);
}

/// Every path-meaning row: [`PATH_MEANING_MATRIX`], the generated Python
/// member-binding rows ([`PYTHON_MEMBER_SHAPES`]), the Swift nested generic
/// rows, [`OWNER_AMBIGUITY_MATRIX`], the Swift unproved-owner rows and the
/// source-owner rows ([`SOURCE_OWNER_MATRIX`], the Swift sibling-shadow
/// rows), the C# `partial` identity rows and the C# written-arity rows.
pub fn path_meaning_rows() -> impl Iterator<Item = &'static PathMeaningRow> {
    PATH_MEANING_MATRIX
        .iter()
        .chain(PYTHON_MEMBER_ROWS.iter())
        .chain(SWIFT_NESTED_GENERIC_ROWS.iter())
        .chain(OWNER_AMBIGUITY_MATRIX)
        .chain(SWIFT_UNPROVED_OWNER_ROWS.iter())
        .chain(SOURCE_OWNER_MATRIX)
        .chain(SWIFT_SIBLING_SHADOW_ROWS.iter())
        .chain(CSHARP_PARTIAL_IDENTITY_ROWS.iter())
        .chain(CSHARP_WRITTEN_ARITY_ROWS.iter())
}

declaration_fact_group_tests!(shared path_meaning_matrix_holds_on_complete_and_declarations_only_paths => owners_hold_on_complete_and_declarations_only_paths);

/// Every path-meaning row's ([`path_meaning_group`]) owners hold on the complete index and
/// on a declarations-only query of the owner's kind.
fn owners_hold_on_complete_and_declarations_only_paths(group: usize) {
    let failed = failing_rows(path_meaning_group(group), |row| {
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
    let impl_rows = || path_meaning_rows().filter(|row| !row.impl_members.is_empty());
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
pub struct RelationShape {
    pub label: &'static str,
    extension: &'static str,
    header: &'static str,
    body: fn(usize) -> String,
}

pub const RELATION_SHAPES: &[RelationShape] = &[
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

crate::symbols_shared_context::shape_tests!(relation_resolution_grows_linearly_with_declarations => relation_shape_grows_linearly {
    rust_modules => "Rust modules with a supertrait, a trait impl and an external impl each",
    typescript_classes => "TypeScript classes extending and implementing",
    java_nested_classes => "Java nested classes extending an outer type",
    cpp_namespaces => "C++ namespaces with a local and an external base",
    python_classes => "Python classes with positional bases",
});

/// Resolving written bases and Rust impls stays linear in the declarations:
/// the declaration index is built once per file and each target is looked
/// up in it, never by rescanning the file per declaration. Results are
/// checked at both sizes so the shapes are known to produce relations.
fn relation_shape_grows_linearly(label: &str) {
    use crate::symbols_shared_context::{LARGE, SMALL, grows_linearly_within};

    // Relation resolution counts every lookup segment, candidate visit and
    // written-target insertion: a record writing three bases, each looked
    // up through a module path, takes more than the shared-context budget
    // of 16 steps. The doubling bound is what proves linear growth.
    const STEPS_PER_RECORD: u64 = 32;

    let dir = tempfile::TempDir::new().expect("temp");
    let mut failures = Vec::new();
    {
        let shape = crate::symbols_shared_context::shape_labeled(RELATION_SHAPES, label, |shape| shape.label);
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
                .flat_map(|record| record.relations.extends.iter().chain(&record.relations.implements).chain(&record.relations.unclassified_bases))
                .filter(|target| target.id.is_some())
                .count();
            assert!(resolved >= declarations, "{}: {resolved} resolved targets for {declarations}", shape.label);
            measured.push((work.shared_context_steps, index.symbols.len()));
        }
        let [(small_steps, _), (large_steps, large_records)] = measured[..] else {
            unreachable!("two sizes")
        };
        if !grows_linearly_within(small_steps, large_steps, large_records, STEPS_PER_RECORD) {
            failures.push(format!(
                "{}: {small_steps} steps for {SMALL} declarations, {large_steps} for {LARGE} ({large_records} records)",
                shape.label
            ));
        }
    }
    assert!(failures.is_empty(), "relation resolution grew faster than the declarations:\n{}", failures.join("\n"));
}

/// One generic reader at the `u16` position boundary: an enclosing owner
/// whose list starts with `ReviewBase` and holds `count` parameters in all,
/// around a type whose base is written `ReviewBase` while a file-level
/// `ReviewBase` also exists. The parameter shadows it whatever the list's
/// length, so the base never gets the file-level declaration's ID.
pub struct GenericBoundaryRow {
    pub language: ProgrammingLanguage,
    pub extension: &'static str,
    /// The source, `{params}` standing for the comma-separated list.
    pub template: &'static str,
    /// The oversized owner's name, as the analysis error reports it.
    pub owner: &'static str,
    /// The kinds a selective query asks for: the inner type's, never the
    /// owner's.
    pub kinds: &'static [SymbolKindV2],
    pub child: Owner,
}

const TYPE_KINDS: &[SymbolKindV2] = &[
    SymbolKindV2::Type,
    SymbolKindV2::Class,
    SymbolKindV2::Interface,
    SymbolKindV2::Enum,
    SymbolKindV2::Trait,
];

const fn boundary(
    language: ProgrammingLanguage,
    extension: &'static str,
    template: &'static str,
    owner: &'static str,
    child: Owner,
) -> GenericBoundaryRow {
    GenericBoundaryRow { language, extension, template, owner, kinds: TYPE_KINDS, child }
}

/// One row per language with an explicit generic-parameter reader.
pub const GENERIC_BOUNDARY_ROWS: &[GenericBoundaryRow] = &[
    boundary(L::Rust, "rs",
        "pub trait ReviewBase {}\nfn factory<{params}>() { trait ReviewChild: ReviewBase {} }\n",
        "factory", bases("factory::ReviewChild", &[text("ReviewBase")])),
    boundary(L::TypeScript, "ts",
        "class ReviewBase {}\nfunction factory<{params}>() { class ReviewChild extends ReviewBase {} }\n",
        "factory", bases("factory.ReviewChild", &[text("ReviewBase")])),
    boundary(L::Go, "go",
        "package shapes\n\ntype ReviewBase interface{ M() }\n\n\
         func factory[{params} any]() {\n\ttype ReviewChild interface{ ReviewBase }\n}\n",
        "factory", bases("shapes.factory.ReviewChild", &[text("ReviewBase")])),
    boundary(L::Python, "py",
        "class ReviewBase:\n    pass\n\ndef factory[{params}]():\n    class ReviewChild(ReviewBase):\n        pass\n",
        "factory", bases("factory.ReviewChild", &[text("ReviewBase")])),
    boundary(L::Java, "java",
        "class ReviewBase {}\nclass Holder { <{params}> void factory() { class ReviewChild extends ReviewBase {} } }\n",
        "factory", bases("Holder.factory.ReviewChild", &[text("ReviewBase")])),
    boundary(L::Swift, "swift",
        "class ReviewBase {}\nfunc factory<{params}>() { class ReviewChild: ReviewBase {} }\n",
        "factory",
        Owner { qualified_name: "factory.ReviewChild", extends: &[], implements: &[], unclassified: &[text("ReviewBase")] }),
    boundary(L::Scala, "scala",
        "class ReviewBase\nobject Holder {\n  def factory[{params}](): Unit = {\n    class ReviewChild extends ReviewBase\n  }\n}\n",
        "factory",
        Owner { qualified_name: "Holder.factory.ReviewChild", extends: &[], implements: &[], unclassified: &[text("ReviewBase")] }),
    // A C# method cannot declare a type, so the owner is a generic class and
    // the query selects interfaces only.
    GenericBoundaryRow {
        language: L::CSharp,
        extension: "cs",
        template: "interface ReviewBase {}\nclass Holder<{params}> { interface IReviewChild : ReviewBase {} }\n",
        owner: "Holder",
        kinds: &[SymbolKindV2::Interface],
        child: bases("Holder.IReviewChild", &[text("ReviewBase")]),
    },
    boundary(L::Cpp, "cpp",
        "struct ReviewBase {};\ntemplate <{params}> void factory() { struct ReviewChild : ReviewBase {}; }\n",
        "factory", bases("factory::ReviewChild", &[text("ReviewBase")])),
];

/// The parameter list of `count` entries, `ReviewBase` first, spelled for
/// `language`; C++ uses unnamed middle entries, which still occupy slots.
/// Rust and C# reuse a filler name: only `ReviewBase` participates in the
/// ownership assertion, while every entry still consumes a position.
pub fn boundary_params(language: ProgrammingLanguage, count: usize) -> String {
    if language == L::Cpp {
        // Unnamed template parameters still consume positions. Keep the
        // shadowing name first and a named parameter at the boundary.
        return std::iter::once("typename ReviewBase")
            .chain(std::iter::repeat_n("typename", count - 2))
            .chain(std::iter::once("typename Last"))
            .collect::<Vec<_>>()
            .join(", ");
    }
    if matches!(language, L::Rust | L::CSharp) {
        return std::iter::once("ReviewBase")
            .chain(std::iter::repeat_n("T", count - 2))
            .chain(std::iter::once("Last"))
            .collect::<Vec<_>>()
            .join(",");
    }
    std::iter::once("ReviewBase".to_string())
        .chain((0..count - 1).map(|index| {
            // Identifier length is irrelevant to the position boundary.
            let head = char::from(b'A' + ((index >> 12) & 15) as u8);
            format!("{head}{:03X}", index & 0xfff)
        }))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The largest list `GenericParam::position` numbers, and one more.
pub const FITS: usize = u16::MAX as usize + 1;
pub const OVERFLOWS: usize = FITS + 1;

pub fn boundary_source(row: &GenericBoundaryRow, count: usize) -> String {
    row.template.replace("{params}", &boundary_params(row.language, count))
}

/// The selective query of a boundary row: its inner type's kinds only.
pub fn boundary_query(row: &GenericBoundaryRow) -> SymbolQuery {
    SymbolQuery::new().kinds(row.kinds.iter().copied())
}

/// Checks the row's inner type in `index`: its base keeps its text and no
/// target ID, so the file-level `ReviewBase` is never claimed.
pub fn check_boundary_child(label: &str, row: &GenericBoundaryRow, index: &FileSymbolIndex) {
    check_owner_targets(label, index, index, std::slice::from_ref(&row.child));
}

/// The [`GENERIC_BOUNDARY_ROWS`] row of `language`.
pub fn boundary_row(language: ProgrammingLanguage) -> &'static GenericBoundaryRow {
    GENERIC_BOUNDARY_ROWS.iter().find(|row| row.language == language).expect("a boundary row")
}

/// With 65,536 parameters (last position `u16::MAX`) the parameter
/// shadows the file-level declaration with its owner selected or on a
/// selective query that does not select the owner. Unrelated complete-
/// index passes are checked on a small source in `check_complete_boundary_owner`.
fn check_boundary_fits(language: ProgrammingLanguage, owner_selected: bool) {
    let row = boundary_row(language);
    let label = format!("{} {FITS} {}", language.name(), if owner_selected { "owner selected" } else { "selective" });
    let fits = write_temp(&boundary_source(row, FITS), row.extension);
    let tree_file = TreeFile::new(fits.path()).expect("parse");
    assert!(tree_file.syntax_diagnostics().is_empty(), "{label}: {:?}", tree_file.syntax_diagnostics());
    let index = if owner_selected {
        let query = if language == L::CSharp {
            // C# also captures every parameter as a Type declaration. The
            // boundary assertion needs the owner's list, not 65,536 extra records.
            SymbolQuery::new().kinds([SymbolKindV2::Class, SymbolKindV2::Interface])
        } else {
            SymbolQuery::new()
        };
        tree_file.query(&query).expect("a full u16 list is representable").index
    } else {
        tree_file.query(&boundary_query(row)).expect("selective").index
    };
    check_boundary_child(&label, row, &index);
    if owner_selected {
        let owner = index.symbols.iter().find(|record| record.identity.name == row.owner).expect("generic owner");
        let named = if language == L::Cpp { 2 } else { FITS };
        assert_eq!(owner.type_info.generic_params.len(), named, "{label}: named entries retain their positions");
        assert_eq!(owner.type_info.generic_params.last().expect("last parameter").position, u16::MAX, "{label}");
    }
}

/// Complete-index wiring of the same ownership rule uses a small list;
/// the large cases independently prove position boundaries and selection.
fn check_complete_boundary_owner(language: ProgrammingLanguage) {
    let row = boundary_row(language);
    let file = write_temp(&boundary_source(row, 3), row.extension);
    let index = TreeFile::new(file.path()).expect("parse").symbol_index_v2().expect("complete");
    check_boundary_child(language.name(), row, &index);
}

/// With 65,537 parameters a selective query, which does not select the
/// owner, still sees the binding (`complete` false), and the complete
/// index, which does, reports the overflow instead of a truncated list.
fn check_boundary_overflows(language: ProgrammingLanguage, complete: bool) {
    let row = boundary_row(language);
    let label = language.name();
    let overflows = write_temp(&boundary_source(row, OVERFLOWS), row.extension);
    let tree_file = TreeFile::new(overflows.path()).expect("parse");
    if !complete {
        let selective = tree_file.query(&boundary_query(row)).expect("the owner is not selected").index;
        check_boundary_child(&format!("{label} {OVERFLOWS} selective"), row, &selective);
        return;
    }
    match tree_file.symbol_index_v2() {
        Err(tree_hugger::TreeHuggerError::TooManyGenericParameters { symbol, count, .. }) => {
            assert_eq!((symbol.as_str(), count), (row.owner, OVERFLOWS), "{label}: overflow");
        }
        other => panic!("{label}: expected TooManyGenericParameters, got {:?}", other.map(|_| ())),
    }
}

/// One test per reader and check, retaining the full u16 position boundary.
mod generic_boundary {
    use super::*;

    macro_rules! sides {
        ($($reader:ident => $language:expr),* $(,)?) => {
            $(mod $reader {
                use super::*;
                #[test]
                fn fits_selected_owner() { check_boundary_fits($language, true) }
                #[test]
                fn complete_owner_shadowing() { check_complete_boundary_owner($language) }
                #[test]
                fn fits_selective() { check_boundary_fits($language, false) }
                #[test]
                fn overflows_complete() { check_boundary_overflows($language, true) }
                #[test]
                fn overflows_selective() { check_boundary_overflows($language, false) }
            })*
        };
    }

    sides! {
        rust => L::Rust,
        typescript => L::TypeScript,
        go => L::Go,
        python => L::Python,
        java => L::Java,
        swift => L::Swift,
        scala => L::Scala,
        csharp => L::CSharp,
        cpp => L::Cpp,
    }

    #[test]
    fn every_generic_reader_has_a_row() {
        let languages: Vec<_> = GENERIC_BOUNDARY_ROWS.iter().map(|row| row.language).collect();
        assert_eq!(languages, [L::Rust, L::TypeScript, L::Go, L::Python, L::Java, L::Swift, L::Scala, L::CSharp, L::Cpp]);
    }
}

/// A Rust impl inside a function whose oversized generic list declares the
/// impl's self type name: the impl attaches to no file-level declaration on
/// a type-only query, a method-only query, and one with Containment.
pub fn oversized_impl_row() -> PathMeaningRow {
    let source = format!(
        "pub trait Base {{}}\npub struct ReviewBase;\n\
         fn factory<{}>() {{ impl Base for ReviewBase {{ fn probe(&self) {{}} }} }}\n",
        boundary_params(L::Rust, OVERFLOWS)
    );
    PathMeaningRow {
        label: "Rust impl of an oversized enclosing function's generic parameter",
        language: L::Rust,
        extension: "rs",
        source: Box::leak(source.into_boxed_str()),
        owners: &[],
        impl_members: OVERSIZED_IMPL_MEMBERS,
        untouched: &["ReviewBase"],
    }
}

const OVERSIZED_IMPL_MEMBERS: &[ImplMember] = &[ImplMember {
    method: "probe",
    container: "ReviewBase",
    owner: None,
    implements: Some(to("Base", "Base")),
    container_kind: None,
}];

fn check_oversized_enclosing_impl(path: &str) {
    let row = oversized_impl_row();
    let file = write_temp(row.source, row.extension);
    let tree_file = TreeFile::new(file.path()).expect("parse");
    if path == "complete" {
        assert!(
            matches!(tree_file.symbol_index_v2(), Err(tree_hugger::TreeHuggerError::TooManyGenericParameters { .. })),
            "{}: the selected owner overflows", row.label
        );
        return;
    }
    if path == "types" {
        let types = tree_file.query(&SymbolQuery::new().kinds(TYPE_KINDS.iter().copied())).expect("types").index;
        let base = types.symbols.iter().find(|record| record.identity.name == "ReviewBase").expect("the struct");
        assert!(base.relations.implements.is_empty(), "{} (type-only): `ReviewBase` implements nothing", row.label);
        return;
    }
    let containment = path == "contained_methods";
    let mut query = SymbolQuery::new().kinds([SymbolKindV2::Method]);
    if containment { query = query.capabilities([tree_hugger::Capability::Containment]); }
    let index = tree_file.query(&query).expect("query").index;
    check_impl_facts(&format!("{} ({path})", row.label), &row, &index, &index, containment);
}

mod an_oversized_enclosing_generic_list_still_owns_its_impls {
    use super::*;
    #[test]
    fn types() { check_oversized_enclosing_impl("types"); }
    #[test]
    fn methods() { check_oversized_enclosing_impl("methods"); }
    #[test]
    fn contained_methods() { check_oversized_enclosing_impl("contained_methods"); }
    #[test]
    fn complete() { check_oversized_enclosing_impl("complete"); }
}

/// Lookup reads every enclosing parameter name even when its position
/// cannot be serialized and the owner is outside the selected kinds.
#[test]
fn swift_nested_extension_sees_unnumbered_enclosing_parameters() {
    let mut names = (0..=u16::MAX as usize).map(|i| format!("T{i}")).collect::<Vec<_>>();
    names.push("ReviewBase".to_string());
    let source = format!("class ReviewBase {{}}\nstruct ReviewOuter<{}> {{ struct ReviewInner {{}} }}\nextension ReviewOuter.ReviewInner {{ protocol ReviewChild: ReviewBase {{}} }}\n", names.join(","));
    let file = write_temp(&source, "swift");
    let tree = TreeFile::new(file.path()).expect("parse");
    let result = tree.query(&SymbolQuery::new().kinds([SymbolKindV2::Interface])).expect("unselected oversized struct owner");
    let child = type_record(&result.index, "oversized Swift owner", "ReviewOuter.ReviewInner.ReviewChild");
    assert!(child.relations.unclassified_bases.is_empty());
    assert_eq!(child.relations.extends.len(), 1);
    assert_eq!(child.relations.extends[0].name, "ReviewBase");
    assert!(child.relations.extends[0].id.is_none());
    assert!(child.relations.extends[0].qualified_name.is_none());
    assert!(child.type_info.generic_params.is_empty(), "enclosing parameters are lookup context only");
}


#[test]
fn type_only_imports_preserve_relation_type_owner_barriers() {
    const F: &[Target] = &[text("F")];
    const ALIAS: &[Target] = &[text("Alias")];
    const DEF: &[Target] = &[text("Def")];
    const Q: &[Target] = &[text("Q")];
    for &(_, import, name, _) in crate::symbols_occurrences::TYPE_ONLY_IMPORT_MATRIX {
        let expected = match name { "F" => F, "Alias" => ALIAS, "Def" => DEF, _ => unreachable!() };
        let source = format!("interface {name} {{}}\n{{ {import} class Child implements {name} {{}} }}\n");
        check_owners("type-only import barrier", "ts", &source, &[
            Owner { qualified_name: "Child", extends: &[], implements: expected, unclassified: &[] },
        ]);
    }
    check_owners("entity alias barrier", "ts", "namespace N { export class F {} }\nimport Q = N.F;\nclass Child extends Q {}\n", &[
        Owner { qualified_name: "Child", extends: Q, implements: &[], unclassified: &[] },
    ]);
}
