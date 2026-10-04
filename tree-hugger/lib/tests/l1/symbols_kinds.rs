//! Tests 2 and 6 of `2026-10-03-visibility-and-kind`: the kind matrix, asserted
//! through `TreeFile::symbols()` and `symbol_index_v2()`, and the rule that no
//! fixture symbol reports `SymbolKind::Unknown`. Rows land with the extraction
//! changes that make them pass.

use std::collections::HashMap;

use tree_hugger::{ProgrammingLanguage, SymbolKind, SymbolKindV2, TreeFile};

use crate::symbols_fixtures::{
    all_fixtures, display_name, extract_inline, extract_path, extract_primary,
};

/// Rust smoke sample of the kind matrix: enum variants, modules that are not
/// their own container, and `impl` members contained by the implemented type.
#[test]
fn rust_kind_rows_hold_through_both_paths() {
    use SymbolKind::*;
    let rust = extract_primary(ProgrammingLanguage::Rust);

    let rows: &[(&str, Option<&str>, SymbolKind, Option<SymbolKind>)] = &[
        ("UnitVariant", Some("PublicEnum"), EnumVariant, Some(Enum)),
        ("TupleVariant", Some("PublicEnum"), EnumVariant, Some(Enum)),
        ("StructVariant", Some("PublicEnum"), EnumVariant, Some(Enum)),
        ("PrivateEnumVariant", Some("PrivateEnum"), EnumVariant, Some(Enum)),
        ("outer_module", None, Module, None),
        ("crate_inner_module", Some("outer_module"), Module, Some(Module)),
        ("private_module", None, Module, None),
        ("geo", None, Module, None),
        // inherent impl, `self` method and associated functions
        ("instance_method", Some("PublicStruct"), Method, Some(Type)),
        ("new_associated_function", Some("PublicStruct"), Method, Some(Type)),
        ("crate_associated_function", Some("PublicStruct"), Method, Some(Type)),
        // trait impls, same-file and standard-library traits
        ("required_trait_method", Some("PublicStruct"), Method, Some(Type)),
        ("fmt", Some("PublicStruct"), Method, Some(Type)),
        ("private_trait_method", Some("PrivateStruct"), Method, Some(Type)),
        // `impl crate::geo::Coord`, `impl<T> Wrapper<T>`, a foreign type
        ("coord_method", Some("Coord"), Method, Some(Type)),
        ("wrapper_method", Some("Wrapper"), Method, Some(Type)),
        ("private_trait_method", Some("String"), Method, None),
        // union fields
        ("union_int_field", Some("PublicUnion"), Field, Some(Type)),
        ("union_float_field", Some("PublicUnion"), Field, Some(Type)),
    ];
    for (name, container, kind, container_kind) in rows {
        rust.assert_kind(name, *container, *kind, *container_kind);
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
