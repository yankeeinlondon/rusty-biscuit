//! Tests 2 and 6 of `2026-10-03-visibility-and-kind`: the kind matrix, asserted
//! through `TreeFile::symbols()` and `symbol_index_v2()`, and the rule that no
//! fixture symbol reports `SymbolKind::Unknown`.

use std::collections::HashMap;

use tree_hugger::{ProgrammingLanguage, SymbolKind, SymbolKindV2, TreeFile};

use ProgrammingLanguage as L;
use SymbolKind as K;

use crate::symbols_function_locals::assert_named_kind;
use crate::symbols_fixtures::{
    all_fixtures, display_name, extract_inline, extract_path, extract_primary,
};

/// One committed cell of the kind matrix: `(language, name, container, kind,
/// container kind)`, asserted in the language's primary fixture.
pub type KindRow = (
    ProgrammingLanguage,
    &'static str,
    Option<&'static str>,
    SymbolKind,
    Option<SymbolKind>,
);

/// The kind matrix: every committed `ok`, `FIX` and `LIM` cell, one row per
/// fixture declaration. Deferred (`FEAT`) cells have no rows.
pub const KIND_MATRIX: &[KindRow] = &[
    // Rust
    (L::Rust, "public_function", None, K::Function, None),
    (L::Rust, "PUBLIC_CONSTANT", None, K::Constant, None),
    (L::Rust, "PUBLIC_STATIC", None, K::Variable, None),
    (L::Rust, "PRIVATE_STATIC_MUT", None, K::Variable, None),
    (L::Rust, "LOCAL_STATIC", None, K::Variable, None),
    (L::Rust, "PublicAlias", None, K::TypeAlias, None),
    (L::Rust, "PrivateAlias", None, K::TypeAlias, None),
    (L::Rust, "outer_module", None, K::Module, None),
    (L::Rust, "crate_inner_module", Some("outer_module"), K::Module, Some(K::Module)),
    (L::Rust, "private_module", None, K::Module, None),
    (L::Rust, "geo", None, K::Module, None),
    (L::Rust, "PublicStruct", None, K::Type, None),
    (L::Rust, "PublicUnion", None, K::Type, None),
    (L::Rust, "PublicEnum", None, K::Enum, None),
    (L::Rust, "UnitVariant", Some("PublicEnum"), K::EnumVariant, Some(K::Enum)),
    (L::Rust, "TupleVariant", Some("PublicEnum"), K::EnumVariant, Some(K::Enum)),
    (L::Rust, "StructVariant", Some("PublicEnum"), K::EnumVariant, Some(K::Enum)),
    (L::Rust, "PrivateEnumVariant", Some("PrivateEnum"), K::EnumVariant, Some(K::Enum)),
    (L::Rust, "PublicTrait", None, K::Trait, None),
    (L::Rust, "public_field", Some("PublicStruct"), K::Field, Some(K::Type)),
    (L::Rust, "union_int_field", Some("PublicUnion"), K::Field, Some(K::Type)),
    (L::Rust, "union_float_field", Some("PublicUnion"), K::Field, Some(K::Type)),
    // trait declaration: bodyless and provided methods, associated items
    (L::Rust, "required_trait_method", Some("PublicTrait"), K::Method, Some(K::Trait)),
    (L::Rust, "provided_trait_method", Some("PublicTrait"), K::Method, Some(K::Trait)),
    (L::Rust, "TRAIT_ASSOCIATED_CONSTANT", Some("PublicTrait"), K::Constant, Some(K::Trait)),
    (L::Rust, "TraitAssociatedType", Some("PublicTrait"), K::TypeAlias, Some(K::Trait)),
    // inherent impl: `self` method, associated functions and constant
    (L::Rust, "instance_method", Some("PublicStruct"), K::Method, Some(K::Type)),
    (L::Rust, "new_associated_function", Some("PublicStruct"), K::Method, Some(K::Type)),
    (L::Rust, "crate_associated_function", Some("PublicStruct"), K::Method, Some(K::Type)),
    (L::Rust, "INHERENT_ASSOCIATED_CONSTANT", Some("PublicStruct"), K::Constant, Some(K::Type)),
    // trait impls, same-file and standard-library traits
    (L::Rust, "required_trait_method", Some("PublicStruct"), K::Method, Some(K::Type)),
    (L::Rust, "TRAIT_ASSOCIATED_CONSTANT", Some("PublicStruct"), K::Constant, Some(K::Type)),
    (L::Rust, "TraitAssociatedType", Some("PublicStruct"), K::TypeAlias, Some(K::Type)),
    (L::Rust, "fmt", Some("PublicStruct"), K::Method, Some(K::Type)),
    (L::Rust, "private_trait_method", Some("PrivateStruct"), K::Method, Some(K::Type)),
    // `impl crate::geo::Coord`, `impl<T> Wrapper<T>`, a foreign type
    (L::Rust, "coord_method", Some("Coord"), K::Method, Some(K::Type)),
    (L::Rust, "wrapper_method", Some("Wrapper"), K::Method, Some(K::Type)),
    (L::Rust, "private_trait_method", Some("String"), K::Method, None),
    // TypeScript
    (L::TypeScript, "exportedFunction", None, K::Function, None),
    (L::TypeScript, "ExportedWidget", None, K::Class, None),
    (L::TypeScript, "ExportedAbstractBase", None, K::Class, None),
    (L::TypeScript, "ModuleAbstractBase", None, K::Class, None),
    (L::TypeScript, "ExportedInterface", None, K::Interface, None),
    (L::TypeScript, "ExportedEnum", None, K::Enum, None),
    (L::TypeScript, "ExportedAlias", None, K::TypeAlias, None),
    (L::TypeScript, "ModuleAlias", None, K::TypeAlias, None),
    (L::TypeScript, "constructor", Some("ExportedWidget"), K::Constructor, Some(K::Class)),
    (L::TypeScript, "publicMethod", Some("ExportedWidget"), K::Method, Some(K::Class)),
    (L::TypeScript, "#hashPrivateMethod", Some("ExportedWidget"), K::Method, Some(K::Class)),
    (L::TypeScript, "abstractMethod", Some("ExportedAbstractBase"), K::Method, Some(K::Class)),
    (L::TypeScript, "interfaceMethod", Some("ExportedInterface"), K::Method, Some(K::Interface)),
    (L::TypeScript, "interfaceProperty", Some("ExportedInterface"), K::Property, Some(K::Interface)),
    (L::TypeScript, "publicProperty", Some("ExportedWidget"), K::Property, Some(K::Class)),
    (L::TypeScript, "staticProperty", Some("ExportedWidget"), K::Property, Some(K::Class)),
    (L::TypeScript, "#hashPrivateProperty", Some("ExportedWidget"), K::Property, Some(K::Class)),
    // JavaScript
    (L::JavaScript, "exportedFunction", None, K::Function, None),
    (L::JavaScript, "ExportedWidget", None, K::Class, None),
    (L::JavaScript, "constructor", Some("ExportedWidget"), K::Constructor, Some(K::Class)),
    (L::JavaScript, "unmarkedMethod", Some("ExportedWidget"), K::Method, Some(K::Class)),
    (L::JavaScript, "unmarkedProperty", Some("ExportedWidget"), K::Property, Some(K::Class)),
    (L::JavaScript, "uninitializedStaticProperty", Some("ExportedWidget"), K::Property, Some(K::Class)),
    (L::JavaScript, "#hashPrivateProperty", Some("ExportedWidget"), K::Property, Some(K::Class)),
    // Go: structs and interfaces are `Type` (LIM)
    (L::Go, "ExportedFunction", None, K::Function, None),
    (L::Go, "ExportedMethod", None, K::Method, None),
    (L::Go, "ExportedStruct", None, K::Type, None),
    (L::Go, "ExportedInterface", None, K::Type, None),
    (L::Go, "symbols", None, K::Namespace, None),
    // Python
    (L::Python, "module_function", None, K::Function, None),
    (L::Python, "PlainWidget", None, K::Class, None),
    (L::Python, "instance_method", Some("PlainWidget"), K::Method, Some(K::Class)),
    (L::Python, "static_method", Some("PlainWidget"), K::Method, Some(K::Class)),
    (L::Python, "class_method", Some("PlainWidget"), K::Method, Some(K::Class)),
    (L::Python, "annotated_instance_field", Some("PlainWidget"), K::Field, Some(K::Class)),
    (L::Python, "unannotated_static_field", Some("PlainWidget"), K::Field, Some(K::Class)),
    // Java
    (L::Java, "com.example.symbols", None, K::Namespace, None),
    (L::Java, "PublicWidget", None, K::Class, None),
    (L::Java, "AnnotatedPackageWidget", None, K::Class, None),
    (L::Java, "PublicShape", None, K::Interface, None),
    (L::Java, "PackageShape", None, K::Interface, None),
    (L::Java, "PublicColor", None, K::Enum, None),
    (L::Java, "PublicPoint", None, K::Type, None),
    (L::Java, "publicMethod", Some("PublicWidget"), K::Method, Some(K::Class)),
    (L::Java, "area", Some("PublicShape"), K::Method, Some(K::Interface)),
    (L::Java, "publicField", Some("PublicWidget"), K::Field, Some(K::Class)),
    (L::Java, "SIDES", Some("PublicShape"), K::Field, Some(K::Interface)),
    // C#
    (L::CSharp, "Example.Symbols", None, K::Namespace, None),
    (L::CSharp, "PublicWidget", Some("Example.Symbols"), K::Class, Some(K::Namespace)),
    (L::CSharp, "IShape", Some("Example.Symbols"), K::Interface, Some(K::Namespace)),
    (L::CSharp, "PublicColor", Some("Example.Symbols"), K::Enum, Some(K::Namespace)),
    (L::CSharp, "PublicStruct", Some("Example.Symbols"), K::Type, Some(K::Namespace)),
    (L::CSharp, "PublicMethod", Some("PublicWidget"), K::Method, Some(K::Class)),
    (L::CSharp, "PublicField", Some("PublicWidget"), K::Field, Some(K::Class)),
    (L::CSharp, "Count", Some("PublicWidget"), K::Property, Some(K::Class)),
    (L::CSharp, "Total", Some("PublicWidget"), K::Property, Some(K::Class)),
    // C
    (L::C, "public_function", None, K::Function, None),
    (L::C, "NamedColor", None, K::Enum, None),
    (L::C, "NamedTag", None, K::Type, None),
    (L::C, "AliasedTag", None, K::Type, None),
    (L::C, "NamedUnion", None, K::Type, None),
    (L::C, "TagAlias", None, K::TypeAlias, None),
    (L::C, "AnonymousStructAlias", None, K::TypeAlias, None),
    (L::C, "CountAlias", None, K::TypeAlias, None),
    (L::C, "tag_field", Some("NamedTag"), K::Field, Some(K::Type)),
    (L::C, "tag_name_field", Some("NamedTag"), K::Field, Some(K::Type)),
    (L::C, "union_int_field", Some("NamedUnion"), K::Field, Some(K::Type)),
    // C++
    (L::Cpp, "namespaceFunction", Some("fixture_space"), K::Function, Some(K::Namespace)),
    (L::Cpp, "AccessWidget", Some("fixture_space"), K::Class, Some(K::Namespace)),
    (L::Cpp, "ScopedColor", Some("fixture_space"), K::Enum, Some(K::Namespace)),
    (L::Cpp, "PlainStruct", Some("fixture_space"), K::Type, Some(K::Namespace)),
    (L::Cpp, "PlainUnion", Some("fixture_space"), K::Type, Some(K::Namespace)),
    // declared in the class body
    (L::Cpp, "firstPublicMethod", Some("AccessWidget"), K::Method, Some(K::Class)),
    (L::Cpp, "staticFactory", Some("AccessWidget"), K::Method, Some(K::Class)),
    // defined in the class body
    (L::Cpp, "secondPublicMethod", Some("AccessWidget"), K::Method, Some(K::Class)),
    (L::Cpp, "instanceMethod", Some("AccessWidget"), K::Method, Some(K::Class)),
    (L::Cpp, "countCalls", Some("AccessWidget"), K::Method, Some(K::Class)),
    (L::Cpp, "protectedMethod", Some("AccessWidget"), K::Method, Some(K::Class)),
    (L::Cpp, "structPublicMethod", Some("PlainStruct"), K::Method, Some(K::Type)),
    (L::Cpp, "firstPublicField", Some("AccessWidget"), K::Field, Some(K::Class)),
    (L::Cpp, "structPublicField", Some("PlainStruct"), K::Field, Some(K::Type)),
    (L::Cpp, "unionIntField", Some("PlainUnion"), K::Field, Some(K::Type)),
    // Swift: classes, structs and enums are `Type` (LIM)
    (L::Swift, "publicFunction", None, K::Function, None),
    (L::Swift, "localValue", None, K::Variable, None),
    (L::Swift, "OpenWidget", None, K::Type, None),
    (L::Swift, "PublicApi", None, K::Type, None),
    (L::Swift, "PublicDirection", None, K::Type, None),
    (L::Swift, "PublicDrawable", None, K::Interface, None),
    (L::Swift, "instanceMethod", Some("OpenWidget"), K::Method, Some(K::Type)),
    (L::Swift, "staticMethod", Some("OpenWidget"), K::Method, Some(K::Type)),
    (L::Swift, "unmarkedMethodInPublicStruct", Some("PublicApi"), K::Method, Some(K::Type)),
    (L::Swift, "unmarkedMethodInPublicExtension", Some("OpenWidget"), K::Method, Some(K::Type)),
    (L::Swift, "north", Some("PublicDirection"), K::EnumVariant, Some(K::Type)),
    (L::Swift, "south", Some("PublicDirection"), K::EnumVariant, Some(K::Type)),
    (L::Swift, "east", Some("PlainDirection"), K::EnumVariant, Some(K::Type)),
    (L::Swift, "west", Some("PlainDirection"), K::EnumVariant, Some(K::Type)),
    (L::Swift, "draw", Some("PublicDrawable"), K::Method, Some(K::Interface)),
    (L::Swift, "drawableName", Some("PublicDrawable"), K::Field, Some(K::Interface)),
    (L::Swift, "publicProperty", Some("OpenWidget"), K::Field, Some(K::Type)),
    // Scala
    (L::Scala, "topLevelFunction", None, K::Function, None),
    (L::Scala, "Shape", None, K::Class, None),
    (L::Scala, "CasePoint", None, K::Class, None),
    (L::Scala, "PlainTrait", None, K::Trait, None),
    (L::Scala, "PlainColor", None, K::Enum, None),
    (L::Scala, "StandaloneObject", None, K::Module, None),
    (L::Scala, "publicMethod", Some("PlainWidget"), K::Method, Some(K::Class)),
    (L::Scala, "sum", Some("CasePoint"), K::Method, Some(K::Class)),
    (L::Scala, "abstractTraitMethod", Some("PlainTrait"), K::Method, Some(K::Trait)),
    (L::Scala, "concreteTraitMethod", Some("PlainTrait"), K::Method, Some(K::Trait)),
    (L::Scala, "objectMethod", Some("StandaloneObject"), K::Method, Some(K::Module)),
    // PHP
    (L::Php, "topLevelFunction", None, K::Function, None),
    (L::Php, "PublicWidget", None, K::Class, None),
    (L::Php, "Shape", None, K::Interface, None),
    (L::Php, "Greets", None, K::Trait, None),
    (L::Php, "Suit", None, K::Enum, None),
    (L::Php, "area", Some("PublicWidget"), K::Method, Some(K::Class)),
    (L::Php, "greet", Some("Greets"), K::Method, Some(K::Trait)),
    (L::Php, "traitStaticMethod", Some("Greets"), K::Method, Some(K::Trait)),
    (L::Php, "publicField", Some("PublicWidget"), K::Field, Some(K::Class)),
    // Lua
    (L::Lua, "globalFunction", None, K::Function, None),
    (L::Lua, "chunkLocalFunction", None, K::Function, None),
    // Perl: subs, both package forms, and `my` / `our` / `state` names
    (L::Perl, "Fixture::Greeter", None, K::Module, None),
    (L::Perl, "Fixture::Block", None, K::Module, None),
    (L::Perl, "greet", Some("Fixture::Greeter"), K::Function, Some(K::Module)),
    (L::Perl, "other_package_sub", Some("Fixture::Other"), K::Function, Some(K::Module)),
    (L::Perl, "block_package_sub", Some("Fixture::Block"), K::Function, Some(K::Module)),
    (L::Perl, "after_block_sub", Some("Fixture::Other"), K::Function, Some(K::Module)),
    (L::Perl, "$package_level_our", Some("Fixture::Greeter"), K::Variable, Some(K::Module)),
    (L::Perl, "$block_package_our", Some("Fixture::Block"), K::Variable, Some(K::Module)),
    (L::Perl, "$joined_name", Some("greet"), K::Variable, Some(K::Function)),
    (L::Perl, "$call_count", Some("greet"), K::Variable, Some(K::Function)),
    (L::Perl, "$first_name", Some("greet"), K::Variable, Some(K::Function)),
    (L::Perl, "$last_name", Some("greet"), K::Variable, Some(K::Function)),
    (L::Perl, "$block_local", Some("block_package_sub"), K::Variable, Some(K::Function)),
    // Bash and Zsh
    (L::Bash, "keyword_function", None, K::Function, None),
    (L::Bash, "function_keyword_function", None, K::Function, None),
    (L::Zsh, "paren_function", None, K::Function, None),
    (L::Zsh, "keyword_function", None, K::Function, None),
];

/// Tests 2 and 6: every kind-matrix row holds through `TreeFile::symbols()`
/// and `symbol_index_v2()`.
#[test]
fn kind_matrix_holds_through_both_paths() {
    for (language, rows) in rows_by_language(KIND_MATRIX, |row| row.0) {
        let extracted = extract_primary(language);
        for (_, name, container, kind, container_kind) in rows {
            extracted.assert_kind(name, *container, *kind, *container_kind);
        }
    }
}

/// Test 6: no symbol of any fixture reports `SymbolKind::Unknown`. An entry
/// here needs a reason; the list starts empty.
#[test]
fn no_fixture_symbol_reports_an_unknown_kind() {
    const ALLOWED_UNKNOWN: &[(&str, &str, &str)] = &[];
    let mut checked = 0;
    for path in all_fixtures() {
        let extracted = extract_path(&path);
        for symbol in &extracted.symbols {
            checked += 1;
            if symbol.kind != SymbolKind::Unknown {
                continue;
            }
            assert!(
                ALLOWED_UNKNOWN
                    .iter()
                    .any(|(file, name, _reason)| *file == extracted.label && *name == symbol.name),
                "{}: `{}` reports SymbolKind::Unknown",
                extracted.label,
                symbol.name
            );
        }
        for record in &extracted.records {
            assert_ne!(
                record.kind,
                SymbolKindV2::Unknown,
                "{}: record `{}` reports Unknown",
                extracted.label,
                record.identity.name
            );
        }
    }
    assert!(checked > 400, "only {checked} symbols checked");
}

/// Groups matrix rows by language, keeping the table's language order.
pub fn rows_by_language<T>(
    rows: &[T],
    language_of: impl Fn(&T) -> ProgrammingLanguage,
) -> Vec<(ProgrammingLanguage, Vec<&T>)> {
    let mut groups: Vec<(ProgrammingLanguage, Vec<&T>)> = Vec::new();
    for row in rows {
        let language = language_of(row);
        match groups.iter_mut().find(|(existing, _)| *existing == language) {
            Some((_, group)) => group.push(row),
            None => groups.push((language, vec![row])),
        }
    }
    groups
}

/// A C `typedef struct Tag {…} Alias;` reports the tag and the alias once
/// each; an anonymous struct invents no tag; `typedef struct S {…} S;` keeps
/// both same-named declarations.
#[test]
fn c_typedefs_report_tag_and_alias_once_each() {
    let c = extract_primary(ProgrammingLanguage::C);
    let kinds_of = |name: &str| {
        let mut kinds: Vec<SymbolKind> = c
            .symbols
            .iter()
            .filter(|symbol| symbol.name == name)
            .map(|symbol| symbol.kind)
            .collect();
        kinds.sort();
        kinds
    };
    assert_eq!(kinds_of("AliasedTag"), [SymbolKind::Type]);
    assert_eq!(kinds_of("TagAlias"), [SymbolKind::TypeAlias]);
    assert_eq!(kinds_of("AnonymousStructAlias"), [SymbolKind::TypeAlias]);
    assert_eq!(kinds_of("SameName"), [SymbolKind::Type, SymbolKind::TypeAlias]);
    assert!(
        c.symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Type)
            .all(|symbol| !symbol.name.is_empty()),
        "an anonymous struct must not invent a tag"
    );
}

/// Removed catch-all patterns: JavaScript `this.x = …` and Python attribute
/// assignments declare nothing, and Perl `local` overrides rather than declares.
#[test]
fn assignments_and_local_overrides_declare_no_symbol() {
    for (language, names) in [
        (ProgrammingLanguage::JavaScript, &["assignedOnlyThroughThis"][..]),
        (
            ProgrammingLanguage::Python,
            &["assigned_through_self", "assigned_from_outside"][..],
        ),
        (ProgrammingLanguage::Perl, &["$_"][..]),
    ] {
        let extracted = extract_primary(language);
        for name in names {
            assert!(
                extracted.symbols.iter().all(|symbol| symbol.name != *name),
                "{}: `{name}` must produce no symbol",
                extracted.label
            );
        }
    }

    // Python reports a class attribute once, from the class body, even when
    // a method assigns it through `self`.
    let python = extract_primary(ProgrammingLanguage::Python);
    let declarations = python
        .symbols
        .iter()
        .filter(|symbol| symbol.name == "unannotated_static_field")
        .count();
    assert_eq!(declarations, 1);

    // The same holds for a JavaScript class field the constructor reassigns.
    let javascript = extract_primary(ProgrammingLanguage::JavaScript);
    javascript.assert_kind(
        "unmarkedProperty",
        Some("ExportedWidget"),
        SymbolKind::Property,
        Some(SymbolKind::Class),
    );

    // No Perl record, by any name, comes from a `local` override.
    let perl = extract_primary(ProgrammingLanguage::Perl);
    for record in &perl.records {
        let text = record.source.declaration_text.as_deref().unwrap_or_default();
        assert!(
            !text.trim_start().starts_with("local"),
            "{}: `{}` was declared by `{text}`",
            perl.label,
            record.identity.name
        );
    }
}

/// Perl variables record their declaring keyword in the declaration text, and
/// each name of a list form shares it.
#[test]
fn perl_declaration_text_keeps_the_declaring_keyword() {
    let perl = extract_primary(ProgrammingLanguage::Perl);
    for (name, container, keyword) in [
        ("$package_level_our", "Fixture::Greeter", "our"),
        ("$joined_name", "greet", "my"),
        ("$call_count", "greet", "state"),
        ("$first_name", "greet", "my"),
        ("$last_name", "greet", "my"),
    ] {
        let text = perl
            .record(name, Some(container))
            .source
            .declaration_text
            .clone()
            .unwrap_or_default();
        assert!(
            text.starts_with(keyword),
            "{name}: declaration text {text:?} lacks `{keyword}`"
        );
    }
}

#[test]
fn rust_trait_impl_members_record_the_implemented_trait() {
    let rust = extract_primary(ProgrammingLanguage::Rust);
    let public_trait = rust.record("PublicTrait", None);
    let private_trait = rust.record("PrivateTrait", None);

    let implements = |name: &str, container: &str| {
        rust.record(name, Some(container))
            .relations
            .implements
            .iter()
            .map(|reference| (reference.name.clone(), reference.id.clone()))
            .collect::<Vec<_>>()
    };

    assert_eq!(implements("instance_method", "PublicStruct"), vec![]);
    assert_eq!(
        implements("required_trait_method", "PublicStruct"),
        vec![("PublicTrait".to_string(), Some(public_trait.id.clone()))]
    );
    assert_eq!(
        implements("fmt", "PublicStruct"),
        vec![("Display".to_string(), None)],
        "a standard-library trait has a name but no same-file id"
    );
    assert_eq!(
        implements("private_trait_method", "String"),
        vec![("PrivateTrait".to_string(), Some(private_trait.id.clone()))]
    );

    // Parameters are not members of the impl.
    let self_in_fmt = rust
        .records
        .iter()
        .find(|record| {
            record.identity.name == "formatter"
                && record.identity.module_path.as_deref() == Some("PublicStruct")
        })
        .expect("formatter parameter");
    assert!(self_in_fmt.relations.implements.is_empty());
}

#[test]
fn rust_impl_members_are_members_of_the_implemented_type() {
    let rust = extract_primary(ProgrammingLanguage::Rust);
    let public_struct = rust.record("PublicStruct", None);
    let member_names: Vec<&str> = public_struct
        .relations
        .members
        .iter()
        .map(|member| member.name.as_str())
        .collect();
    for expected in [
        "new_associated_function",
        "instance_method",
        "private_instance_method",
        "crate_associated_function",
        "required_trait_method",
        "fmt",
        "public_field",
    ] {
        assert!(
            member_names.contains(&expected),
            "PublicStruct members {member_names:?} lack {expected}"
        );
    }

    let coord_method = rust.record("coord_method", Some("Coord"));
    assert_eq!(
        coord_method.relations.container.as_ref().map(|c| c.name.as_str()),
        Some("Coord")
    );
    let foreign = rust.record("private_trait_method", Some("String"));
    assert!(
        foreign.relations.container.is_none(),
        "a type declared elsewhere has no record to link to"
    );
}

/// Kinds and containers are decided once: both public paths agree on every
/// symbol of every fixture.
#[test]
fn symbols_and_records_agree_for_every_fixture_symbol() {
    let mut compared = 0;
    for path in all_fixtures() {
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let symbols = tree_file.symbols().expect("symbols");
        let records = tree_file.symbol_records().expect("records");
        let label = display_name(&path);
        assert_eq!(symbols.len(), records.len(), "{label}: symbol count");
        for (symbol, record) in symbols.iter().zip(&records) {
            assert_eq!(record.identity.name, symbol.name, "{label}: order");
            assert_eq!(
                record.kind,
                SymbolKindV2::from(symbol.kind),
                "{label}: `{}` kind",
                symbol.name
            );
            assert_eq!(
                record.identity.module_path, symbol.container_name,
                "{label}: `{}` container",
                symbol.name
            );
            assert_eq!(
                record.identity.container_kind,
                symbol.container_kind.map(SymbolKindV2::from),
                "{label}: `{}` container kind",
                symbol.name
            );
            compared += 1;
        }
    }
    assert!(compared > 400, "only {compared} symbols compared");
}

/// A container's kind is the kind its own declaration reports, so a type is
/// never `Class` as a container and `Type` standalone.
#[test]
fn container_kind_matches_the_containers_standalone_kind() {
    let mut checked = 0;
    for path in all_fixtures() {
        let extracted = extract_path(&path);
        let mut standalone: HashMap<&str, Vec<SymbolKind>> = HashMap::new();
        for symbol in &extracted.symbols {
            if symbol.kind.is_type() || matches!(symbol.kind, SymbolKind::Module | SymbolKind::Namespace)
            {
                standalone.entry(symbol.name.as_str()).or_default().push(symbol.kind);
            }
        }

        for symbol in &extracted.symbols {
            let (Some(container), Some(container_kind)) =
                (symbol.container_name.as_deref(), symbol.container_kind)
            else {
                continue;
            };
            let Some(kinds) = standalone.get(container) else {
                continue;
            };
            // Same-named declarations of different kinds (a Scala class and its
            // companion object) cannot be matched by name alone.
            if kinds.iter().any(|kind| *kind != kinds[0]) {
                continue;
            }
            assert_eq!(
                container_kind, kinds[0],
                "{}: `{}` names {container} as a {container_kind:?}, but {container} reports {:?}",
                extracted.label, symbol.name, kinds[0]
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} containers checked");
}

#[test]
fn kinds_decided_at_extraction_reach_the_v1_symbols() {
    use SymbolKind::*;
    let typescript = extract_primary(ProgrammingLanguage::TypeScript);
    typescript.assert_kind("constructor", Some("ExportedWidget"), Constructor, Some(Class));
    typescript.assert_kind("constructor", Some("ModuleWidget"), Constructor, Some(Class));
    typescript.assert_kind("ExportedAlias", None, TypeAlias, None);
    typescript.assert_kind("publicProperty", Some("ExportedWidget"), Property, Some(Class));
    typescript.assert_kind("staticProperty", Some("ExportedWidget"), Property, Some(Class));

    let alias = typescript.record("ExportedAlias", None);
    match &alias.kind_data {
        tree_hugger::SymbolKindData::TypeAlias(data) => {
            assert_eq!(data.aliased_type_text.as_deref(), Some("string"));
        }
        other => panic!("ExportedAlias kind data: {other:?}"),
    }
    // The constructor carries function data; a property does not.
    assert!(
        typescript
            .symbol("constructor", Some("ExportedWidget"))
            .signature
            .is_some()
    );
    assert!(
        typescript
            .symbol("publicProperty", Some("ExportedWidget"))
            .signature
            .is_none()
    );
}

#[test]
fn enum_variants_never_carry_a_signature() {
    let rust = extract_primary(ProgrammingLanguage::Rust);
    let variant = rust.symbol("TupleVariant", Some("PublicEnum"));
    assert!(variant.signature.is_none(), "a tuple variant is not a function");
    assert!(matches!(
        rust.record("TupleVariant", Some("PublicEnum")).kind_data,
        tree_hugger::SymbolKindData::Unknown
    ));
    // The enum keeps its variants in its own metadata.
    let public_enum = rust.symbol("PublicEnum", None);
    let variants: Vec<&str> = public_enum
        .type_metadata
        .as_ref()
        .expect("enum metadata")
        .variants
        .iter()
        .map(|variant| variant.name.as_str())
        .collect();
    assert_eq!(variants, ["UnitVariant", "TupleVariant", "StructVariant"]);
}

#[test]
fn swift_function_local_binding_is_a_variable_not_a_field() {
    let swift = extract_primary(ProgrammingLanguage::Swift);
    swift.assert_kind("localValue", None, SymbolKind::Variable, None);
    swift.assert_kind(
        "publicProperty",
        Some("OpenWidget"),
        SymbolKind::Field,
        Some(SymbolKind::Type),
    );
}

#[test]
fn functions_declared_directly_in_a_type_body_are_methods() {
    use SymbolKind::*;
    let python = extract_primary(ProgrammingLanguage::Python);
    let python_class = python.symbol("PlainWidget", None).kind;
    python.assert_kind("static_method", Some("PlainWidget"), Method, Some(python_class));
    python.assert_kind("class_method", Some("PlainWidget"), Method, Some(python_class));
    python.assert_kind("module_function", None, Function, None);

    let scala = extract_primary(ProgrammingLanguage::Scala);
    scala.assert_kind("abstractTraitMethod", Some("PlainTrait"), Method, Some(Trait));
    scala.assert_kind("concreteTraitMethod", Some("PlainTrait"), Method, Some(Trait));
    scala.assert_kind("objectMethod", Some("StandaloneObject"), Method, Some(Module));
    scala.assert_kind("publicMethod", Some("PlainWidget"), Method, Some(Class));
    scala.assert_kind("topLevelFunction", None, Function, None);

    let swift = extract_primary(ProgrammingLanguage::Swift);
    swift.assert_kind("instanceMethod", Some("OpenWidget"), Method, Some(Type));
    swift.assert_kind("unmarkedMethodInPrivateExtension", Some("OpenWidget"), Method, Some(Type));
    swift.assert_kind("unmarkedMethodInPlainStruct", Some("PlainStruct"), Method, Some(Type));
    swift.assert_kind("publicFunction", None, Function, None);

    let cpp = extract_primary(ProgrammingLanguage::Cpp);
    let cpp_class = cpp.symbol("AccessWidget", Some("fixture_space")).kind;
    cpp.assert_kind("AccessWidget", Some("AccessWidget"), Method, Some(cpp_class));
}

#[test]
fn function_nested_in_a_function_body_stays_a_function() {
    let python = extract_inline(
        "class Outer:\n    def method(self):\n        def inner():\n            pass\n        return inner\n",
        "py",
    );
    assert_eq!(python.symbol("method", Some("Outer")).kind, SymbolKind::Method);
    let inner = python
        .symbols
        .iter()
        .find(|symbol| symbol.name == "inner")
        .expect("inner");
    assert_eq!(inner.kind, SymbolKind::Function);

    let rust = extract_inline(
        "struct S;\nimpl S {\n    fn method() {\n        fn helper() {}\n        helper();\n    }\n}\n",
        "rs",
    );
    assert_eq!(rust.symbol("method", Some("S")).kind, SymbolKind::Method);
    let helper = rust
        .symbols
        .iter()
        .find(|symbol| symbol.name == "helper")
        .expect("helper");
    assert_eq!(helper.kind, SymbolKind::Function);
}

#[test]
fn members_name_their_nearest_type_container() {
    use SymbolKind::*;
    let cpp = extract_primary(ProgrammingLanguage::Cpp);
    let cpp_class = cpp.symbol("AccessWidget", Some("fixture_space")).kind;
    cpp.assert_kind("firstPublicField", Some("AccessWidget"), Field, Some(cpp_class));
    cpp.assert_kind("firstPublicMethod", Some("AccessWidget"), Method, Some(cpp_class));
    cpp.assert_kind("unionIntField", Some("PlainUnion"), Field, Some(Type));

    let c = extract_primary(ProgrammingLanguage::C);
    c.assert_kind("union_int_field", Some("NamedUnion"), Field, Some(Type));

    let php = extract_primary(ProgrammingLanguage::Php);
    php.assert_kind("greet", Some("Greets"), Method, Some(Trait));
    php.assert_kind("traitStaticMethod", Some("Greets"), Method, Some(Trait));

    let typescript = extract_primary(ProgrammingLanguage::TypeScript);
    typescript.assert_kind(
        "concreteProtectedMethod",
        Some("ExportedAbstractBase"),
        Method,
        Some(Class),
    );

    // A C# file-scoped namespace scopes the declarations after it, which are
    // its siblings rather than its children.
    let csharp = extract_primary(ProgrammingLanguage::CSharp);
    csharp.assert_kind("PublicWidget", Some("Example.Symbols"), Class, Some(Namespace));
    csharp.assert_kind("UnmarkedEnum", Some("Example.Symbols"), Enum, Some(Namespace));
}

/// Extraction supplies function data for every function-like declaration,
/// including Lua and shell functions and C# methods returning a user type.
#[test]
fn function_like_symbols_carry_extracted_signatures() {
    let lua = extract_primary(ProgrammingLanguage::Lua);
    let parameters: Vec<&str> = lua
        .symbol("globalFunction", None)
        .signature
        .as_ref()
        .expect("Lua signature")
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
    assert_eq!(parameters, ["parameterValue"]);
    assert!(lua.symbol("chunkLocalFunction", None).signature.is_some());

    let variadic = extract_inline("function logAll(prefix, ...)\nend\n", "lua");
    let signature = variadic
        .symbol("logAll", None)
        .signature
        .clone()
        .expect("variadic signature");
    assert_eq!(signature.parameters.len(), 2);
    assert!(signature.parameters[1].is_variadic);

    for language in [ProgrammingLanguage::Bash, ProgrammingLanguage::Zsh] {
        let shell = extract_primary(language);
        for symbol in shell.symbols.iter().filter(|symbol| symbol.kind.is_function()) {
            assert!(
                symbol.signature.is_some(),
                "{}: `{}` has no signature",
                shell.label,
                symbol.name
            );
        }
    }

    let csharp = extract_primary(ProgrammingLanguage::CSharp);
    let factory = csharp.symbol("StaticFactory", Some("PublicWidget"));
    let signature = factory.signature.as_ref().expect("C# signature");
    assert_eq!(signature.return_type.as_deref(), Some("PublicWidget"));
    assert!(signature.parameters.is_empty());
    assert_eq!(
        csharp
            .symbol("Unit", Some("IShape"))
            .signature
            .as_ref()
            .and_then(|signature| signature.return_type.as_deref()),
        Some("IShape")
    );
}

/// A function declarator nested in a parameter list declares a parameter of
/// function type, never a function: C and C++ function-typed, pointer and
/// reference parameters, with or without names inside them, are Parameters
/// like `int a`. The other languages with function-typed parameters already
/// report them as Parameters.
#[test]
fn function_typed_parameters_are_parameters() {
    let c_source = "int f(void cbProbe(int), void (*ptrProbe)(int), int (*outerProbe)(int (*innerProbe)(long)));\n\
                    int g(void namedProbe(int valueProbe)) { return 0; }\n";
    let cpp_source = format!(
        "{c_source}int r(void (&refProbe)(int), void defaultProbe(int) = nullptr);\n"
    );
    let c = extract_inline(c_source, "c");
    let cpp = extract_inline(&cpp_source, "cpp");
    let names = ["cbProbe", "ptrProbe", "outerProbe", "innerProbe", "namedProbe", "valueProbe"];
    for name in names {
        assert_named_kind(&c, name, K::Parameter);
        assert_named_kind(&cpp, name, K::Parameter);
    }
    for name in ["refProbe", "defaultProbe"] {
        assert_named_kind(&cpp, name, K::Parameter);
    }
    for (extracted, name) in [(&c, "f"), (&c, "g"), (&cpp, "f"), (&cpp, "g"), (&cpp, "r")] {
        assert_named_kind(extracted, name, K::Function);
    }

    let others = [
        ("package p\nfunc F(cbProbe func(x int) int) {}\n", "go"),
        ("export function f(cbProbe: (x: number) => void) {}\n", "ts"),
        ("func f(cbProbe: (Int) -> Void) {}\n", "swift"),
        ("pub fn f(cbProbe: fn(i32) -> i32) {}\n", "rs"),
        ("object O { def f(cbProbe: Int => Int): Unit = () }\n", "scala"),
        ("class C { void M(System.Func<int, int> cbProbe) {} }\n", "cs"),
    ];
    for (source, extension) in others {
        assert_named_kind(&extract_inline(source, extension), "cbProbe", K::Parameter);
    }
}
