//! The containment matrix of `2026-10-03-symbol-filters-and-reports`: with
//! only the containment capability requested, each member's
//! `relations.parent` is its container's record, and the container lists it
//! in `relations.members`. Covers Rust `impl` blocks, TypeScript classes, Go
//! receiver methods, Python classes and Lua tables, among the rest.

use tree_hugger::{Capability, ProgrammingLanguage, SymbolKindV2, SymbolQuery, SymbolRecord, TreeFile};

use crate::symbols_fixtures::{display_name, primary_fixture};

use ProgrammingLanguage as L;
use SymbolKindV2 as K;

/// `(member name, container name, container kind)`.
pub type ContainmentCase = (&'static str, &'static str, SymbolKindV2);

pub struct ContainmentRow {
    pub language: ProgrammingLanguage,
    /// The cases, or why the language has no members to attach.
    pub cases: Result<&'static [ContainmentCase], &'static str>,
}

pub const CONTAINMENT_MATRIX: &[ContainmentRow] = &[
    ContainmentRow {
        language: L::Rust,
        cases: Ok(&[
            // Inherent and trait `impl` members attach to the implemented type.
            ("instance_method", "PublicStruct", K::Type),
            ("fmt", "PublicStruct", K::Type),
            ("public_field", "PublicStruct", K::Type),
            ("required_trait_method", "PublicTrait", K::Trait),
            ("UnitVariant", "PublicEnum", K::Enum),
            ("load", "Record", K::Type),
            ("module_public_function", "outer_module", K::Module),
        ]),
    },
    ContainmentRow {
        language: L::TypeScript,
        cases: Ok(&[
            ("publicMethod", "ExportedWidget", K::Class),
            ("publicProperty", "ExportedWidget", K::Class),
            ("interfaceMethod", "ExportedInterface", K::Interface),
        ]),
    },
    ContainmentRow {
        language: L::JavaScript,
        cases: Ok(&[
            ("unmarkedMethod", "ExportedWidget", K::Class),
            ("staticProperty", "ExportedWidget", K::Class),
            ("instanceMethod", "ModuleWidget", K::Class),
        ]),
    },
    ContainmentRow {
        language: L::Go,
        cases: Ok(&[
            // Receiver methods attach to the receiver's base type, pointer
            // receivers included.
            ("ExportedMethod", "ExportedStruct", K::Type),
            ("packageMethod", "ExportedStruct", K::Type),
            ("OtherExportedMethod", "packageStruct", K::Type),
        ]),
    },
    ContainmentRow {
        language: L::Python,
        cases: Ok(&[
            ("instance_method", "PlainWidget", K::Class),
            ("static_method", "PlainWidget", K::Class),
            ("annotated_instance_field", "PlainWidget", K::Class),
        ]),
    },
    ContainmentRow {
        language: L::Java,
        cases: Ok(&[
            ("publicMethod", "PublicWidget", K::Class),
            ("publicMethodInPrivateClass", "PrivateNestedClass", K::Class),
            ("area", "PublicShape", K::Interface),
        ]),
    },
    ContainmentRow {
        language: L::CSharp,
        cases: Ok(&[
            ("PublicMethod", "PublicWidget", K::Class),
            ("StructPublicField", "PublicStruct", K::Type),
            ("PublicWidget", "Example.Symbols", K::Namespace),
        ]),
    },
    ContainmentRow {
        language: L::C,
        cases: Ok(&[
            ("tag_field", "NamedTag", K::Type),
            ("union_int_field", "NamedUnion", K::Type),
            ("COLOR_RED", "NamedColor", K::Enum),
        ]),
    },
    ContainmentRow {
        language: L::Cpp,
        cases: Ok(&[
            ("firstPublicMethod", "AccessWidget", K::Class),
            ("structPublicField", "PlainStruct", K::Type),
            ("namespaceFunction", "fixture_space", K::Namespace),
        ]),
    },
    ContainmentRow {
        language: L::Swift,
        cases: Ok(&[
            ("openMethod", "OpenWidget", K::Type),
            ("publicApiField", "PublicApi", K::Type),
            ("draw", "PublicDrawable", K::Interface),
        ]),
    },
    ContainmentRow {
        language: L::Scala,
        cases: Ok(&[
            ("publicMethod", "PlainWidget", K::Class),
            ("companionStaticMethod", "PlainWidget", K::Module),
            ("abstractTraitMethod", "PlainTrait", K::Trait),
        ]),
    },
    ContainmentRow {
        language: L::Php,
        cases: Ok(&[
            ("staticFactory", "PublicWidget", K::Class),
            ("greet", "Greets", K::Trait),
            ("CLASS_CONSTANT", "ConstantForms", K::Class),
        ]),
    },
    ContainmentRow {
        language: L::Perl,
        cases: Ok(&[
            // Package contents attach to the package.
            ("greet", "Fixture::Greeter", K::Module),
            ("block_package_sub", "Fixture::Block", K::Module),
        ]),
    },
    ContainmentRow {
        language: L::Lua,
        cases: Ok(&[
            // `M.f`, `M:m` and `M.x = …` attach to the table's declaration.
            ("tableFunction", "Widget", K::Variable),
            ("methodWithSelf", "Widget", K::Variable),
            ("tableField", "Widget", K::Variable),
        ]),
    },
    ContainmentRow {
        language: L::Bash,
        cases: Err("shell declarations have no owning type"),
    },
    ContainmentRow {
        language: L::Zsh,
        cases: Err("shell declarations have no owning type"),
    },
];

/// The languages with a containment row, for the coverage guard.
pub fn containment_matrix_languages() -> Vec<ProgrammingLanguage> {
    CONTAINMENT_MATRIX.iter().map(|row| row.language).collect()
}

fn one<'a>(records: &'a [SymbolRecord], label: &str, what: &str, matches: impl Fn(&SymbolRecord) -> bool) -> &'a SymbolRecord {
    let found: Vec<&SymbolRecord> = records.iter().filter(|record| matches(record)).collect();
    assert_eq!(found.len(), 1, "{label}: expected one {what}, found {}", found.len());
    found[0]
}

#[test]
fn containment_matrix_links_members_to_their_containers() {
    let query = SymbolQuery::new().capabilities([Capability::Containment]);
    for row in CONTAINMENT_MATRIX {
        let path = primary_fixture(row.language).expect("primary fixture");
        let label = display_name(&path);
        let records = TreeFile::new(&path)
            .expect("parse fixture")
            .query(&query)
            .expect("containment query")
            .index
            .symbols;

        let cases = match row.cases {
            Ok(cases) => cases,
            Err(reason) => {
                let linked: Vec<&str> = records
                    .iter()
                    .filter(|record| record.relations.parent.is_some())
                    .map(|record| record.identity.qualified_name.as_str())
                    .collect();
                // Locals of a shell function have no container either.
                assert!(linked.is_empty(), "{label}: members linked despite `{reason}`: {linked:?}");
                continue;
            }
        };
        assert!(!cases.is_empty(), "{label}: empty containment row");
        for (member, container, container_kind) in cases {
            let owner = one(&records, &label, &format!("{container_kind:?} `{container}`"), |record| {
                record.identity.name == *container && record.kind == *container_kind
            });
            let child = one(&records, &label, &format!("`{member}` in `{container}`"), |record| {
                record.identity.name == *member && record.identity.container_name.as_deref() == Some(*container)
            });
            let parent = child
                .relations
                .parent
                .as_ref()
                .unwrap_or_else(|| panic!("{label}: `{member}` has no parent"));
            assert_eq!(parent.id.as_ref(), Some(&owner.id), "{label}: `{member}` parent is not `{container}`");
            assert_eq!(child.identity.container_kind, Some(*container_kind), "{label}: `{member}` container kind");
            assert!(
                owner.relations.members.iter().any(|reference| reference.id.as_ref() == Some(&child.id)),
                "{label}: `{container}` does not list `{member}`"
            );
        }
    }
}
