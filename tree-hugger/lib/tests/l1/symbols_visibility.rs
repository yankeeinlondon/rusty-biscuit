//! Test 3 of `2026-10-03-visibility-and-kind`: the visibility matrix for all
//! sixteen languages, asserted on `SymbolInfo::visibility` and
//! `SymbolRecord.visibility.level`.

use tree_hugger::{ProgrammingLanguage, SymbolKind, Visibility, VisibilityLevel};

use crate::symbols_fixtures::{
    Extracted, extract_inline, extract_path, extract_primary, symbols_fixture_dir,
};
use crate::symbols_kinds::rows_by_language;

use ProgrammingLanguage as L;

pub fn assert_level(extracted: &Extracted, name: &str, container: Option<&str>, expected: Option<Visibility>) {
    let symbol = extracted.symbol(name, container);
    assert_eq!(
        symbol.visibility, expected,
        "{}: `{name}` SymbolInfo::visibility",
        extracted.label
    );
    assert_eq!(
        extracted.record(name, container).visibility.level,
        VisibilityLevel::from(expected),
        "{}: `{name}` SymbolRecord.visibility.level",
        extracted.label
    );
}

/// One committed cell of the visibility matrix: `(language, name, container,
/// level)` in the language's primary fixture; `None` is Unknown.
pub type VisibilityRow = (
    ProgrammingLanguage,
    &'static str,
    Option<&'static str>,
    Option<Visibility>,
);

const PUBLIC: Option<Visibility> = Some(Visibility::Public);
const PROTECTED: Option<Visibility> = Some(Visibility::Protected);
const PRIVATE: Option<Visibility> = Some(Visibility::Private);
const INTERNAL: Option<Visibility> = Some(Visibility::Internal);
const PACKAGE: Option<Visibility> = Some(Visibility::Package);
const CRATE: Option<Visibility> = Some(Visibility::Crate);
const MODULE_PRIVATE: Option<Visibility> = Some(Visibility::ModulePrivate);
const PROTECTED_INTERNAL: Option<Visibility> = Some(Visibility::ProtectedInternal);
const PRIVATE_PROTECTED: Option<Visibility> = Some(Visibility::PrivateProtected);
const UNKNOWN: Option<Visibility> = None;

/// The visibility matrix: every mapped level of every language, unmarked
/// declarations, private members of public types and public members of
/// private types, and locals/parameters that stay Unknown.
pub const VISIBILITY_MATRIX: &[VisibilityRow] = &[
    // Rust
    (L::Rust, "public_function", None, PUBLIC),
    (L::Rust, "crate_function", None, CRATE),
    (L::Rust, "self_function", None, PRIVATE),
    (L::Rust, "private_function", None, PRIVATE),
    (L::Rust, "super_function", Some("crate_inner_module"), CRATE),
    (L::Rust, "in_path_function", Some("crate_inner_module"), CRATE),
    (L::Rust, "outer_module", None, PUBLIC),
    (L::Rust, "crate_inner_module", Some("outer_module"), CRATE),
    (L::Rust, "private_module", None, PRIVATE),
    (L::Rust, "PUBLIC_CONSTANT", None, PUBLIC),
    (L::Rust, "PRIVATE_CONSTANT", None, PRIVATE),
    (L::Rust, "PUBLIC_STATIC", None, PUBLIC),
    (L::Rust, "PRIVATE_STATIC_MUT", None, PRIVATE),
    (L::Rust, "PublicAlias", None, PUBLIC),
    (L::Rust, "PrivateAlias", None, PRIVATE),
    (L::Rust, "PublicStruct", None, PUBLIC),
    (L::Rust, "PrivateStruct", None, PRIVATE),
    (L::Rust, "public_field", Some("PublicStruct"), PUBLIC),
    (L::Rust, "crate_field", Some("PublicStruct"), CRATE),
    (L::Rust, "private_field", Some("PublicStruct"), PRIVATE),
    (L::Rust, "public_field_in_private_struct", Some("PrivateStruct"), PUBLIC),
    (L::Rust, "union_float_field", Some("PublicUnion"), PRIVATE),
    // enum variants and their fields inherit the enum's level
    (L::Rust, "UnitVariant", Some("PublicEnum"), PUBLIC),
    (L::Rust, "variant_field", Some("PublicEnum"), PUBLIC),
    (L::Rust, "PrivateEnumVariant", Some("PrivateEnum"), PRIVATE),
    // trait items inherit the trait's level
    (L::Rust, "TRAIT_ASSOCIATED_CONSTANT", Some("PublicTrait"), PUBLIC),
    (L::Rust, "TraitAssociatedType", Some("PublicTrait"), PUBLIC),
    (L::Rust, "required_trait_method", Some("PublicTrait"), PUBLIC),
    (L::Rust, "private_trait_method", Some("PrivateTrait"), PRIVATE),
    // trait-impl items: the same-file trait's level, else Unknown
    (L::Rust, "required_trait_method", Some("PublicStruct"), PUBLIC),
    (L::Rust, "TRAIT_ASSOCIATED_CONSTANT", Some("PublicStruct"), PUBLIC),
    (L::Rust, "private_trait_method", Some("PrivateStruct"), PRIVATE),
    (L::Rust, "private_trait_method", Some("String"), PRIVATE),
    (L::Rust, "fmt", Some("PublicStruct"), UNKNOWN),
    // inherent impl items follow the ordinary rule
    (L::Rust, "private_instance_method", Some("PublicStruct"), PRIVATE),
    (L::Rust, "crate_associated_function", Some("PublicStruct"), CRATE),
    (L::Rust, "INHERENT_ASSOCIATED_CONSTANT", Some("PublicStruct"), PUBLIC),
    (L::Rust, "local_value", None, UNKNOWN),
    (L::Rust, "parameter_value", None, UNKNOWN),
    (L::Rust, "LOCAL_STATIC", None, UNKNOWN),
    // TypeScript (a module file)
    (L::TypeScript, "exportedFunction", None, PUBLIC),
    (L::TypeScript, "moduleFunction", None, MODULE_PRIVATE),
    (L::TypeScript, "EXPORTED_CONSTANT", None, PUBLIC),
    (L::TypeScript, "MODULE_CONSTANT", None, MODULE_PRIVATE),
    (L::TypeScript, "ExportedAlias", None, PUBLIC),
    (L::TypeScript, "ModuleAlias", None, MODULE_PRIVATE),
    (L::TypeScript, "ExportedInterface", None, PUBLIC),
    (L::TypeScript, "ModuleInterface", None, MODULE_PRIVATE),
    (L::TypeScript, "ExportedEnum", None, PUBLIC),
    (L::TypeScript, "ModuleEnum", None, MODULE_PRIVATE),
    (L::TypeScript, "ExportedAbstractBase", None, PUBLIC),
    (L::TypeScript, "ModuleAbstractBase", None, MODULE_PRIVATE),
    (L::TypeScript, "ModuleWidget", None, MODULE_PRIVATE),
    (L::TypeScript, "interfaceProperty", Some("ExportedInterface"), PUBLIC),
    (L::TypeScript, "interfaceMethod", Some("ExportedInterface"), PUBLIC),
    (L::TypeScript, "moduleInterfaceMethod", Some("ModuleInterface"), PUBLIC),
    (L::TypeScript, "publicProperty", Some("ExportedWidget"), PUBLIC),
    (L::TypeScript, "protectedProperty", Some("ExportedWidget"), PROTECTED),
    (L::TypeScript, "privateProperty", Some("ExportedWidget"), PRIVATE),
    (L::TypeScript, "#hashPrivateProperty", Some("ExportedWidget"), PRIVATE),
    (L::TypeScript, "unmarkedProperty", Some("ExportedWidget"), PUBLIC),
    (L::TypeScript, "publicMethod", Some("ExportedWidget"), PUBLIC),
    (L::TypeScript, "protectedMethod", Some("ExportedWidget"), PROTECTED),
    (L::TypeScript, "privateMethod", Some("ExportedWidget"), PRIVATE),
    (L::TypeScript, "#hashPrivateMethod", Some("ExportedWidget"), PRIVATE),
    (L::TypeScript, "unmarkedMethod", Some("ExportedWidget"), PUBLIC),
    (L::TypeScript, "constructor", Some("ExportedWidget"), PUBLIC),
    (L::TypeScript, "concreteProtectedMethod", Some("ExportedAbstractBase"), PROTECTED),
    (L::TypeScript, "instanceMethod", Some("ModuleWidget"), PUBLIC),
    (L::TypeScript, "localValue", None, UNKNOWN),
    (L::TypeScript, "parameterValue", None, UNKNOWN),
    // A function bound to a top-level `const` has the declaration's access.
    (L::TypeScript, "asyncArrow", None, PUBLIC),
    // JavaScript (a module file)
    (L::JavaScript, "exportedFunction", None, PUBLIC),
    (L::JavaScript, "moduleFunction", None, MODULE_PRIVATE),
    (L::JavaScript, "EXPORTED_CONSTANT", None, PUBLIC),
    (L::JavaScript, "MODULE_CONSTANT", None, MODULE_PRIVATE),
    (L::JavaScript, "ExportedWidget", None, PUBLIC),
    (L::JavaScript, "ModuleWidget", None, MODULE_PRIVATE),
    (L::JavaScript, "unmarkedProperty", Some("ExportedWidget"), PUBLIC),
    (L::JavaScript, "#hashPrivateProperty", Some("ExportedWidget"), PRIVATE),
    (L::JavaScript, "unmarkedMethod", Some("ExportedWidget"), PUBLIC),
    (L::JavaScript, "#hashPrivateMethod", Some("ExportedWidget"), PRIVATE),
    (L::JavaScript, "localValue", None, UNKNOWN),
    (L::JavaScript, "asyncArrow", None, PUBLIC),
    (L::JavaScript, "documentedArrow", None, MODULE_PRIVATE),
    // Go
    (L::Go, "ExportedFunction", None, PUBLIC),
    (L::Go, "packageFunction", None, PACKAGE),
    (L::Go, "ÉtéFunction", None, PUBLIC),
    (L::Go, "ExportedConstant", None, PUBLIC),
    (L::Go, "packageConstant", None, PACKAGE),
    (L::Go, "ExportedVariable", None, PUBLIC),
    (L::Go, "packageVariable", None, PACKAGE),
    (L::Go, "ExportedStruct", None, PUBLIC),
    (L::Go, "packageStruct", None, PACKAGE),
    (L::Go, "ExportedMethod", Some("ExportedStruct"), PUBLIC),
    (L::Go, "packageMethod", Some("ExportedStruct"), PACKAGE),
    (L::Go, "OtherExportedMethod", Some("packageStruct"), PUBLIC),
    (L::Go, "symbols", None, UNKNOWN),
    (L::Go, "localValue", None, UNKNOWN),
    (L::Go, "parameterValue", None, UNKNOWN),
    // Java
    (L::Java, "PublicWidget", None, PUBLIC),
    (L::Java, "PackageWidget", None, PACKAGE),
    (L::Java, "publicField", Some("PublicWidget"), PUBLIC),
    (L::Java, "protectedField", Some("PublicWidget"), PROTECTED),
    (L::Java, "privateField", Some("PublicWidget"), PRIVATE),
    (L::Java, "packageField", Some("PublicWidget"), PACKAGE),
    (L::Java, "publicMethod", Some("PublicWidget"), PUBLIC),
    (L::Java, "protectedMethod", Some("PublicWidget"), PROTECTED),
    (L::Java, "privateMethod", Some("PublicWidget"), PRIVATE),
    (L::Java, "packageMethod", Some("PublicWidget"), PACKAGE),
    (L::Java, "PrivateNestedClass", Some("PublicWidget"), PRIVATE),
    (L::Java, "publicMethodInPrivateClass", Some("PrivateNestedClass"), PUBLIC),
    (L::Java, "PublicShape", None, PUBLIC),
    (L::Java, "PackageShape", None, PACKAGE),
    // interface members default to Public, even in a package interface
    (L::Java, "area", Some("PublicShape"), PUBLIC),
    (L::Java, "describe", Some("PublicShape"), PUBLIC),
    (L::Java, "unit", Some("PublicShape"), PUBLIC),
    (L::Java, "SIDES", Some("PublicShape"), PUBLIC),
    (L::Java, "packageShapeMethod", Some("PackageShape"), PUBLIC),
    (L::Java, "com.example.symbols", None, UNKNOWN),
    (L::Java, "localValue", Some("PublicWidget"), UNKNOWN),
    // C#
    (L::CSharp, "PublicWidget", Some("Example.Symbols"), PUBLIC),
    (L::CSharp, "UnmarkedTopLevelClass", Some("Example.Symbols"), INTERNAL),
    (L::CSharp, "InternalClass", Some("Example.Symbols"), INTERNAL),
    (L::CSharp, "FileLocalClass", Some("Example.Symbols"), MODULE_PRIVATE),
    (L::CSharp, "UnmarkedEnum", Some("Example.Symbols"), INTERNAL),
    (L::CSharp, "PublicField", Some("PublicWidget"), PUBLIC),
    (L::CSharp, "ProtectedField", Some("PublicWidget"), PROTECTED),
    (L::CSharp, "PrivateField", Some("PublicWidget"), PRIVATE),
    (L::CSharp, "InternalField", Some("PublicWidget"), INTERNAL),
    (L::CSharp, "ProtectedInternalField", Some("PublicWidget"), PROTECTED_INTERNAL),
    (L::CSharp, "PrivateProtectedField", Some("PublicWidget"), PRIVATE_PROTECTED),
    (L::CSharp, "UnmarkedField", Some("PublicWidget"), PRIVATE),
    // setter-only accessor modifiers leave the property's level alone
    (L::CSharp, "Count", Some("PublicWidget"), PUBLIC),
    (L::CSharp, "Total", Some("PublicWidget"), INTERNAL),
    (L::CSharp, "ProtectedInternalMethod", Some("PublicWidget"), PROTECTED_INTERNAL),
    (L::CSharp, "PrivateProtectedMethod", Some("PublicWidget"), PRIVATE_PROTECTED),
    (L::CSharp, "UnmarkedMethod", Some("PublicWidget"), PRIVATE),
    (L::CSharp, "PrivateNestedClass", Some("PublicWidget"), PRIVATE),
    (L::CSharp, "PublicMethodInPrivateClass", Some("PrivateNestedClass"), PUBLIC),
    (L::CSharp, "StructUnmarkedField", Some("PublicStruct"), PRIVATE),
    // interface members: abstract, default-implemented, static, nested type
    (L::CSharp, "Area", Some("IShape"), PUBLIC),
    (L::CSharp, "Draw", Some("IShape"), PUBLIC),
    (L::CSharp, "Unit", Some("IShape"), PUBLIC),
    (L::CSharp, "NestedHelper", Some("IShape"), PUBLIC),
    (L::CSharp, "Log", Some("IShape"), PRIVATE),
    // explicit interface implementation
    (L::CSharp, "Area", Some("Square"), PRIVATE),
    (L::CSharp, "Example.Symbols", None, UNKNOWN),
    (L::CSharp, "localValue", Some("PublicWidget"), UNKNOWN),
    // C: the file-scope linkage rule
    (L::C, "public_global_variable", None, PUBLIC),
    (L::C, "file_static_variable", None, MODULE_PRIVATE),
    (L::C, "extern_declared_variable", None, PUBLIC),
    (L::C, "public_function", None, PUBLIC),
    (L::C, "file_static_function", None, MODULE_PRIVATE),
    (L::C, "NamedTag", None, UNKNOWN),
    (L::C, "TagAlias", None, UNKNOWN),
    (L::C, "tag_field", Some("NamedTag"), UNKNOWN),
    (L::C, "COLOR_RED", Some("NamedColor"), UNKNOWN),
    (L::C, "FIXTURE_MACRO", None, UNKNOWN),
    (L::C, "local_value", None, UNKNOWN),
    (L::C, "local_static_counter", None, UNKNOWN),
    // C++: access sections, class-key defaults, and the linkage rule
    (L::Cpp, "implicitPrivateField", Some("AccessWidget"), PRIVATE),
    (L::Cpp, "implicitPrivateMethod", Some("AccessWidget"), PRIVATE),
    (L::Cpp, "firstPublicField", Some("AccessWidget"), PUBLIC),
    (L::Cpp, "firstPublicMethod", Some("AccessWidget"), PUBLIC),
    (L::Cpp, "secondPublicMethod", Some("AccessWidget"), PUBLIC),
    (L::Cpp, "protectedField", Some("AccessWidget"), PROTECTED),
    (L::Cpp, "protectedMethod", Some("AccessWidget"), PROTECTED),
    (L::Cpp, "explicitPrivateField", Some("AccessWidget"), PRIVATE),
    (L::Cpp, "explicitPrivateMethod", Some("AccessWidget"), PRIVATE),
    (L::Cpp, "structPublicField", Some("PlainStruct"), PUBLIC),
    (L::Cpp, "structPublicMethod", Some("PlainStruct"), PUBLIC),
    (L::Cpp, "structPrivateField", Some("PlainStruct"), PRIVATE),
    (L::Cpp, "unionIntField", Some("PlainUnion"), PUBLIC),
    (L::Cpp, "namespaceFunction", Some("fixture_space"), PUBLIC),
    (L::Cpp, "hiddenVariable", None, MODULE_PRIVATE),
    (L::Cpp, "hiddenFunction", None, MODULE_PRIVATE),
    (L::Cpp, "LIMIT", None, MODULE_PRIVATE),
    (L::Cpp, "SHARED", None, PUBLIC),
    (L::Cpp, "INLINE_LIMIT", None, PUBLIC),
    (L::Cpp, "CONSTEXPR_LIMIT", None, MODULE_PRIVATE),
    (L::Cpp, "constCharPointer", None, PUBLIC),
    (L::Cpp, "constPointerToChar", None, MODULE_PRIVATE),
    (L::Cpp, "publicGlobal", None, PUBLIC),
    (L::Cpp, "fileStaticVariable", None, MODULE_PRIVATE),
    (L::Cpp, "fileStaticFunction", None, MODULE_PRIVATE),
    (L::Cpp, "api", None, PUBLIC),
    (L::Cpp, "AccessWidget", Some("fixture_space"), UNKNOWN),
    (L::Cpp, "SizeAlias", Some("fixture_space"), UNKNOWN),
    (L::Cpp, "fixture_space", None, UNKNOWN),
    (L::Cpp, "memberLocalStatic", Some("AccessWidget"), UNKNOWN),
    (L::Cpp, "localValue", Some("fixture_space"), UNKNOWN),
    // Swift
    (L::Swift, "OpenWidget", None, PUBLIC),
    (L::Swift, "openMethod", Some("OpenWidget"), PUBLIC),
    (L::Swift, "publicMethod", Some("OpenWidget"), PUBLIC),
    (L::Swift, "packageMethod", Some("OpenWidget"), PACKAGE),
    (L::Swift, "internalMethod", Some("OpenWidget"), INTERNAL),
    (L::Swift, "fileprivateMethod", Some("OpenWidget"), MODULE_PRIVATE),
    (L::Swift, "privateMethod", Some("OpenWidget"), PRIVATE),
    // an unmarked member of a public or open type is Internal
    (L::Swift, "unmarkedMethodInOpenClass", Some("OpenWidget"), INTERNAL),
    (L::Swift, "unmarkedMethodInPublicStruct", Some("PublicApi"), INTERNAL),
    (L::Swift, "publicFunction", None, PUBLIC),
    (L::Swift, "unmarkedFunction", None, INTERNAL),
    (L::Swift, "fileprivateFunction", None, MODULE_PRIVATE),
    (L::Swift, "privateFunction", None, PRIVATE),
    // setter-only modifiers
    (L::Swift, "count", Some("PublicApi"), PUBLIC),
    (L::Swift, "total", Some("PublicApi"), INTERNAL),
    (L::Swift, "fileprivateTotal", Some("PublicApi"), INTERNAL),
    // the nearest container with an explicit keyword decides
    (L::Swift, "InnerPrivate", Some("PublicApi"), PRIVATE),
    (L::Swift, "unmarkedMethodInInnerPrivate", Some("InnerPrivate"), MODULE_PRIVATE),
    (L::Swift, "privateHiddenField", Some("PrivateHidden"), MODULE_PRIVATE),
    (L::Swift, "unmarkedMethodInPrivateStruct", Some("PrivateHidden"), MODULE_PRIVATE),
    (L::Swift, "unmarkedMethodInFileprivateStruct", Some("FileprivateHidden"), MODULE_PRIVATE),
    (L::Swift, "unmarkedMethodInPlainStruct", Some("PlainStruct"), INTERNAL),
    (L::Swift, "unmarkedMethodInPrivateExtension", Some("OpenWidget"), MODULE_PRIVATE),
    (L::Swift, "unmarkedMethodInPublicExtension", Some("OpenWidget"), PUBLIC),
    // enum cases and protocol requirements inherit their container
    (L::Swift, "north", Some("PublicDirection"), PUBLIC),
    (L::Swift, "east", Some("PlainDirection"), INTERNAL),
    (L::Swift, "draw", Some("PublicDrawable"), PUBLIC),
    (L::Swift, "drawableName", Some("PublicDrawable"), PUBLIC),
    (L::Swift, "plainDraw", Some("PlainDrawable"), INTERNAL),
    (L::Swift, "localValue", None, UNKNOWN),
    (L::Swift, "parameterValue", None, UNKNOWN),
    // Scala
    (L::Scala, "topLevelFunction", None, PUBLIC),
    (L::Scala, "privateTopLevelFunction", None, PRIVATE),
    (L::Scala, "PlainTrait", None, PUBLIC),
    (L::Scala, "PrivateClass", None, PRIVATE),
    (L::Scala, "methodInPrivateClass", Some("PrivateClass"), PUBLIC),
    (L::Scala, "publicField", Some("PlainWidget"), PUBLIC),
    (L::Scala, "thisQualifiedField", Some("PlainWidget"), PRIVATE),
    (L::Scala, "publicMethod", Some("PlainWidget"), PUBLIC),
    (L::Scala, "privateMethod", Some("PlainWidget"), PRIVATE),
    (L::Scala, "protectedMethod", Some("PlainWidget"), PROTECTED),
    (L::Scala, "packageQualifiedMethod", Some("PlainWidget"), PACKAGE),
    (L::Scala, "classQualifiedMethod", Some("PlainWidget"), UNKNOWN),
    (L::Scala, "tick", Some("Shape"), PROTECTED),
    (L::Scala, "scale", Some("Shape"), PACKAGE),
    (L::Scala, "secret", Some("Shape"), UNKNOWN),
    (L::Scala, "objectMethod", Some("StandaloneObject"), PUBLIC),
    (L::Scala, "privateObjectMethod", Some("StandaloneObject"), PRIVATE),
    (L::Scala, "localValue", None, UNKNOWN),
    // PHP
    (L::Php, "topLevelFunction", None, PUBLIC),
    (L::Php, "PublicWidget", None, PUBLIC),
    (L::Php, "publicField", Some("PublicWidget"), PUBLIC),
    (L::Php, "protectedField", Some("PublicWidget"), PROTECTED),
    (L::Php, "privateField", Some("PublicWidget"), PRIVATE),
    (L::Php, "legacyUnmarkedField", Some("PublicWidget"), PUBLIC),
    (L::Php, "protectedMethod", Some("PublicWidget"), PROTECTED),
    (L::Php, "privateMethod", Some("PublicWidget"), PRIVATE),
    (L::Php, "unmarkedMethod", Some("PublicWidget"), PUBLIC),
    (L::Php, "traitHelper", Some("Greets"), PRIVATE),
    (L::Php, "area", Some("Shape"), PUBLIC),
    // Lua
    (L::Lua, "globalFunction", None, PUBLIC),
    (L::Lua, "globalVariable", None, PUBLIC),
    (L::Lua, "chunkLocalFunction", None, MODULE_PRIVATE),
    (L::Lua, "chunkLocalVariable", None, MODULE_PRIVATE),
    (L::Lua, "Widget", None, MODULE_PRIVATE),
    (L::Lua, "nestedBlockLocal", None, UNKNOWN),
    (L::Lua, "nestedBlockLocalFunction", None, UNKNOWN),
    (L::Lua, "blockLocalValue", None, UNKNOWN),
    (L::Lua, "tableFunction", Some("Widget"), UNKNOWN),
    (L::Lua, "methodWithSelf", Some("Widget"), UNKNOWN),
    (L::Lua, "tableField", Some("Widget"), UNKNOWN),
    // Python, Perl, Bash and Zsh: access is a convention or does not exist
    (L::Python, "module_function", None, UNKNOWN),
    (L::Python, "_conventionally_private_function", None, UNKNOWN),
    (L::Python, "PlainWidget", None, UNKNOWN),
    (L::Python, "instance_method", Some("PlainWidget"), UNKNOWN),
    (L::Python, "annotated_instance_field", Some("PlainWidget"), UNKNOWN),
    (L::Python, "MODULE_CONSTANT", None, UNKNOWN),
    (L::Perl, "greet", Some("Fixture::Greeter"), UNKNOWN),
    (L::Perl, "_conventionally_private_sub", Some("Fixture::Greeter"), UNKNOWN),
    (L::Perl, "Fixture::Greeter", None, UNKNOWN),
    (L::Perl, "$package_level_our", Some("Fixture::Greeter"), UNKNOWN),
    (L::Perl, "$joined_name", Some("greet"), UNKNOWN),
    (L::Perl, "$call_count", Some("greet"), UNKNOWN),
    (L::Bash, "keyword_function", None, UNKNOWN),
    (L::Bash, "GLOBAL_VARIABLE", None, UNKNOWN),
    (L::Zsh, "paren_function", None, UNKNOWN),
    (L::Zsh, "GLOBAL_VARIABLE", None, UNKNOWN),
];

crate::language_cases::language_tests!(visibility_matrix_holds_through_both_paths => check_visibility_matrix_holds_through_both_paths);

/// Test 3: every visibility-matrix row holds on `SymbolInfo::visibility` and
/// `SymbolRecord.visibility.level`.
fn check_visibility_matrix_holds_through_both_paths(language: ProgrammingLanguage) {
    for (language, rows) in rows_by_language(VISIBILITY_MATRIX, |row| row.0).into_iter().filter(|(row_language, _)| *row_language == language) {
        let extracted = extract_primary(language);
        for (_, name, container, expected) in rows {
            assert_level(&extracted, name, *container, *expected);
        }
    }
}

/// Same-named declarations the matrix cannot address by name: a C/C++
/// function an earlier `static` prototype made ModulePrivate, and a C++
/// overload with different parameters, which keeps its own level.
#[test]
fn earlier_static_prototypes_reach_later_declarations() {
    let levels = |language: ProgrammingLanguage, name: &str| {
        let extracted = extract_primary(language);
        extracted
            .symbols
            .iter()
            .filter(|symbol| symbol.name == name)
            .map(|symbol| (symbol.range.start_line, symbol.visibility))
            .collect::<Vec<_>>()
    };

    let c = levels(ProgrammingLanguage::C, "prototyped_helper");
    assert_eq!(c.len(), 2, "{c:?}");
    assert!(c.iter().all(|(_, level)| *level == MODULE_PRIVATE), "{c:?}");

    // `static int helper(int);`, `int helper(int helperValue) {…}`,
    // `int helper(long helperValue);`
    let cpp = levels(ProgrammingLanguage::Cpp, "helper");
    let cpp_levels: Vec<Option<Visibility>> = cpp.iter().map(|(_, level)| *level).collect();
    assert_eq!(cpp_levels, [MODULE_PRIVATE, MODULE_PRIVATE, PUBLIC], "{cpp:?}");
}

/// Every same-named declaration of `name` in source order, asserted on
/// `SymbolInfo::visibility` and `SymbolRecord.visibility.level` alike.
fn assert_levels_in_order(extracted: &Extracted, name: &str, expected: &[Option<Visibility>]) {
    let mut symbols: Vec<_> = extracted
        .symbols
        .iter()
        .filter(|symbol| symbol.name == name)
        .collect();
    symbols.sort_by_key(|symbol| (symbol.range.start_line, symbol.range.start_column));
    let symbol_levels: Vec<Option<Visibility>> =
        symbols.iter().map(|symbol| symbol.visibility).collect();
    assert_eq!(symbol_levels, expected, "{}: `{name}` SymbolInfo::visibility", extracted.label);

    let mut records: Vec<_> = extracted
        .records
        .iter()
        .filter(|record| record.identity.name == name)
        .collect();
    records.sort_by_key(|record| record.source.declaration_span.start_byte);
    let record_levels: Vec<VisibilityLevel> =
        records.iter().map(|record| record.visibility.level).collect();
    let expected_levels: Vec<VisibilityLevel> =
        expected.iter().copied().map(VisibilityLevel::from).collect();
    assert_eq!(
        record_levels, expected_levels,
        "{}: `{name}` SymbolRecord.visibility.level",
        extracted.label
    );
}

/// An earlier `static` declaration reaches a later C declaration through
/// every declarator of a multi-declarator prototype.
#[test]
fn c_earlier_static_declaration_reaches_every_declarator() {
    let c = extract_inline(
        "static int multiFirst(int), multiSecond(int);\n\
         int multiFirst(int x) { return x; }\n\
         int multiSecond(int x) { return x; }\n\
         static int unnamedProbe(int);\n\
         int unnamedProbe(int value) { return value; }\n",
        "c",
    );
    for name in ["multiFirst", "multiSecond", "unnamedProbe"] {
        assert_levels_in_order(&c, name, &[MODULE_PRIVATE, MODULE_PRIVATE]);
    }
}

/// C++ function identity for the earlier-`static` scan: every declarator of
/// both declarations, template wrappers, and parameter types without names or
/// default arguments. Overloads and other namespaces keep their own level.
#[test]
fn cpp_earlier_static_declaration_matches_function_identity() {
    let cpp = extract_inline(
        "static int multiFirst(int), multiSecond(int);\n\
         int multiSecond(int x) { return x; }\n\
         static int ownSecond(int);\n\
         int ownFirst(long), ownSecond(int);\n\
         static int defaultProbe(int x = 0);\n\
         int defaultProbe(int x) { return x; }\n\
         static int unnamedDefault(int = 0);\n\
         int unnamedDefault(int x) { return x; }\n\
         template<class T> static T templateLinkProbe(T x);\n\
         template<class T> T templateLinkProbe(T x) { return x; }\n\
         static int unnamedProbe(int);\n\
         int unnamedProbe(int value) { return value; }\n\
         static char* pointerProbe(int);\n\
         char* pointerProbe(int x) { return nullptr; }\n\
         static int overloadProbe(int);\n\
         int overloadProbe(long x) { return 0; }\n\
         namespace first { static int namespaceProbe(int); }\n\
         namespace second { int namespaceProbe(int x) { return x; } }\n",
        "cpp",
    );
    for name in [
        "multiSecond",
        "ownSecond",
        "defaultProbe",
        "unnamedDefault",
        "templateLinkProbe",
        "unnamedProbe",
        "pointerProbe",
    ] {
        assert_levels_in_order(&cpp, name, &[MODULE_PRIVATE, MODULE_PRIVATE]);
    }
    assert_levels_in_order(&cpp, "multiFirst", &[MODULE_PRIVATE]);
    assert_levels_in_order(&cpp, "ownFirst", &[PUBLIC]);
    assert_levels_in_order(&cpp, "overloadProbe", &[MODULE_PRIVATE, PUBLIC]);
    assert_levels_in_order(&cpp, "namespaceProbe", &[MODULE_PRIVATE, PUBLIC]);
}

/// C++ function identity ignores spelling that leaves the namespace and the
/// parameter types unchanged: parameter names at every nesting level,
/// comments, nested versus `a::b` namespace blocks, a namespace-qualified
/// definition, and the parameter adjustments the language applies (top-level
/// `const`, array and function parameters as pointers, `(void)`, qualifier
/// order). A different callback, pointee, variadic tail or namespace still
/// keeps the later declaration's own level.
#[test]
fn cpp_earlier_static_declaration_ignores_names_comments_and_namespace_spelling() {
    let cpp = extract_inline(
        "static int callback(void (*)(int));\n\
         int callback(void (*cb)(int value)) { return 0; }\n\
         static int refCallback(void (&)(int));\n\
         int refCallback(void (&cb)(int value)) { return 0; }\n\
         static int nestedCallback(int (*)(int (*)(long)));\n\
         int nestedCallback(int (*outer)(int (*inner)(long x))) { return 0; }\n\
         static int commentProbe(int);\n\
         int commentProbe(int /* explanation */ value) { return value; }\n\
         namespace a { namespace b { static int helper(int); } }\n\
         namespace a::b { int helper(int value) { return value; } }\n\
         namespace a::b { static int flatFirst(int); }\n\
         namespace a { namespace b { int flatFirst(int x) { return x; } } }\n\
         namespace a { static int qualifiedProbe(int); }\n\
         int a::qualifiedProbe(int x) { return x; }\n\
         static int constProbe(const int);\n\
         int constProbe(int x) { return x; }\n\
         static int decayProbe(int[4]);\n\
         int decayProbe(int* values) { return 0; }\n\
         static int functionDecay(void cb(int));\n\
         int functionDecay(void (*cb)(int)) { return 0; }\n\
         static int voidProbe(void);\n\
         int voidProbe() { return 0; }\n\
         static int orderProbe(const int*);\n\
         int orderProbe(int const* p) { return 0; }\n\
         static int sameInner(void (*cb)(int value));\n\
         int sameInner(void (*cb)(int value)) { return 0; }\n\
         static int absentInner(void (*)(int));\n\
         int absentInner(void (*)(int)) { return 0; }\n\
         static int arrayProbe(int first[4]);\n\
         int arrayProbe(int second[4]) { return 0; }\n\
         static int callbackType(void (*)(int));\n\
         int callbackType(void (*cb)(long value)) { return 0; }\n\
         static int pointeeConst(const int*);\n\
         int pointeeConst(int* p) { return 0; }\n\
         static int variadicProbe(int, ...);\n\
         int variadicProbe(int x) { return x; }\n\
         namespace a { namespace c { static int otherSpace(int); } }\n\
         namespace a::d { int otherSpace(int x) { return x; } }\n",
        "cpp",
    );
    for name in [
        "callback",
        "refCallback",
        "nestedCallback",
        "commentProbe",
        "helper",
        "flatFirst",
        "qualifiedProbe",
        "constProbe",
        "decayProbe",
        "functionDecay",
        "voidProbe",
        "orderProbe",
        "sameInner",
        "absentInner",
        "arrayProbe",
    ] {
        assert_levels_in_order(&cpp, name, &[MODULE_PRIVATE, MODULE_PRIVATE]);
    }
    for name in ["callbackType", "pointeeConst", "variadicProbe", "otherSpace"] {
        assert_levels_in_order(&cpp, name, &[MODULE_PRIVATE, PUBLIC]);
    }
}

/// C identifies a function by its name alone, so inner callback names and
/// parameter comments never matter.
#[test]
fn c_earlier_static_declaration_ignores_parameter_spelling() {
    let c = extract_inline(
        "static int callback(void (*)(int));\n\
         int callback(void (*cb)(int value)) { return 0; }\n\
         static int commentProbe(int);\n\
         int commentProbe(int /* explanation */ value) { return value; }\n",
        "c",
    );
    for name in ["callback", "commentProbe"] {
        assert_levels_in_order(&c, name, &[MODULE_PRIVATE, MODULE_PRIVATE]);
    }
}

/// JavaScript and TypeScript module status comes from the file alone: a
/// classic or global script leaves unexported top-level declarations Unknown;
/// CommonJS evidence or a module extension makes them ModulePrivate.
#[test]
fn script_and_module_files_decide_top_level_access() {
    let fixture = |name: &str| extract_path(&symbols_fixture_dir().join(name));

    let javascript_script = fixture("javascript_script.js");
    for name in ["scriptFunction", "scriptVariable", "ScriptClass"] {
        assert_level(&javascript_script, name, None, UNKNOWN);
    }
    assert_level(&javascript_script, "scriptMethod", Some("ScriptClass"), PUBLIC);

    let typescript_script = fixture("typescript_script.ts");
    for name in ["scriptFunction", "SCRIPT_CONSTANT", "ScriptClass", "ScriptAlias"] {
        assert_level(&typescript_script, name, None, UNKNOWN);
    }

    let commonjs = fixture("javascript_commonjs.js");
    for name in ["path", "commonjsHelper", "commonjsExported", "CommonjsClass"] {
        assert_level(&commonjs, name, None, MODULE_PRIVATE);
    }

    let mjs = fixture("javascript_extension_module.mjs");
    for name in ["extensionModuleFunction", "extensionModuleConstant", "ExtensionModuleClass"] {
        assert_level(&mjs, name, None, MODULE_PRIVATE);
    }
    let cts = fixture("typescript_extension_module.cts");
    for name in ["extensionModuleFunction", "ExtensionModuleAlias", "ExtensionModuleClass"] {
        assert_level(&cts, name, None, MODULE_PRIVATE);
    }
}

/// CommonJS evidence counts anywhere in the file: a `require` inside a
/// function and an `exports.x` assignment inside a conditional.
#[test]
fn commonjs_evidence_counts_anywhere_in_the_file() {
    let lazy_require = extract_inline(
        "function load() {\n  return require(\"fs\");\n}\nconst helper = 1;\n",
        "js",
    );
    assert_level(&lazy_require, "helper", None, MODULE_PRIVATE);

    let wrapped_require = extract_inline("const fs = ((require))(\"fs\");\nconst helper = 1;\n", "js");
    assert_level(&wrapped_require, "helper", None, MODULE_PRIVATE);

    let conditional_export = extract_inline(
        "const helper = 1;\nif (helper) {\n  exports.helper = helper;\n}\n",
        "js",
    );
    assert_level(&conditional_export, "helper", None, MODULE_PRIVATE);

    let module_exports = extract_inline("const helper = 1;\nmodule.exports = helper;\n", "js");
    assert_level(&module_exports, "helper", None, MODULE_PRIVATE);

    // Control: no module evidence, so the declaration is a global.
    let script = extract_inline("const helper = 1;\nconsole.log(helper.exports);\n", "js");
    assert_level(&script, "helper", None, UNKNOWN);
}

/// Restricted Rust access and unresolved Scala qualifiers keep their source
/// text, since no level can carry it.
#[test]
fn unmappable_access_keeps_its_source_text() {
    let rust = extract_primary(ProgrammingLanguage::Rust);
    for (name, container, text) in [
        ("crate_function", None, "pub(crate) fn crate_function()"),
        ("super_function", Some("crate_inner_module"), "pub(super) fn super_function()"),
        ("crate_field", Some("PublicStruct"), "pub(crate) crate_field: u32"),
    ] {
        assert_eq!(
            rust.record(name, container).source.declaration_text.as_deref(),
            Some(text),
            "{name}"
        );
    }

    let scala = extract_primary(ProgrammingLanguage::Scala);
    assert_eq!(
        scala
            .record("classQualifiedMethod", Some("PlainWidget"))
            .source
            .declaration_text
            .as_deref(),
        Some("private[PlainWidget] def classQualifiedMethod(): Unit =")
    );
}

crate::language_cases::language_tests!(field_metadata_agrees_with_the_standalone_fields => check_field_metadata_agrees_with_the_standalone_fields);

/// Field metadata (`hug classes`) carries the same level as the standalone
/// field, including C#'s compound levels and defaults no keyword states.
fn check_field_metadata_agrees_with_the_standalone_fields(language: ProgrammingLanguage) {
    let mut compared = 0;
    for language in [language] {
        let extracted = extract_primary(language);
        for owner in &extracted.symbols {
            let Some(metadata) = &owner.type_metadata else {
                continue;
            };
            for field in &metadata.fields {
                let Some(standalone) = extracted.symbols.iter().find(|symbol| {
                    symbol.name == field.name
                        && symbol.container_name.as_deref() == Some(owner.name.as_str())
                        && matches!(symbol.kind, SymbolKind::Field | SymbolKind::Property)
                }) else {
                    continue;
                };
                assert_eq!(
                    field.visibility, standalone.visibility,
                    "{}: {}.{} FieldInfo visibility",
                    extracted.label, owner.name, field.name
                );
                assert_eq!(
                    field.is_static, standalone.is_static,
                    "{}: {}.{} FieldInfo is_static",
                    extracted.label, owner.name, field.name
                );
                compared += 1;
            }
        }
    }
    assert!(compared >= crate::language_cases::corpus_minimums(language).metadata_fields, "{language}: corpus coverage shrank ({compared})");

}

/// Parameters and locals have no member or module access, even inside a
/// type whose members default to a level.
#[test]
fn parameters_and_locals_stay_unknown() {
    let java = extract_inline(
        "interface Shape {\n    default int area(int side) {\n        int squared = side * side;\n        return squared;\n    }\n}\n",
        "java",
    );
    assert_level(&java, "area", Some("Shape"), Some(Visibility::Public));
    for name in ["side", "squared"] {
        let symbol = java
            .symbols
            .iter()
            .find(|symbol| symbol.name == name)
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(symbol.visibility, None, "`{name}` inside an interface method");
        assert!(matches!(symbol.kind, SymbolKind::Parameter | SymbolKind::Variable));
    }

    let rust = extract_inline("pub fn outer() {\n    pub fn nested() {}\n}\n", "rs");
    assert_level(&rust, "outer", None, Some(Visibility::Public));
    assert_level(&rust, "nested", None, None);

    let rust_fixture = extract_primary(ProgrammingLanguage::Rust);
    assert_level(&rust_fixture, "local_value", None, None);
    assert_level(&rust_fixture, "parameter_value", None, None);
}

/// One cell of the earlier-`static` linkage matrix: `(languages, source,
/// name, levels of every declaration of name in source order)`. Each row is
/// its own file, so no row can match another row's prototype.
type LinkageRow = (&'static [&'static str], &'static str, &'static str, &'static [Option<Visibility>]);

const C_AND_CPP: &[&str] = &["c", "cpp"];
const CPP_ONLY: &[&str] = &["cpp"];
const MP: Option<Visibility> = MODULE_PRIVATE;

/// The earlier-`static` scan identifies the function declarator that directly
/// declares the name (through function-pointer, function-reference and
/// array-pointer return types), the exact namespace of an unqualified or
/// qualified C++ declaration and, for a function template, its header,
/// leading and trailing return type and both `requires` clauses. A row whose later declaration is Public is a
/// declaration the scan must not attach to an unrelated `static`; a row whose
/// later declaration is ModulePrivate is one it must not miss.
const LINKAGE_MATRIX: &[LinkageRow] = &[
    // A static in an enclosing namespace never reaches a nested namespace.
    (CPP_ONLY, "static int scope_leak(int);\n\
                namespace unrelated { int scope_leak(int x) { return x; } }\n",
        "scope_leak", &[MP, PUBLIC]),
    (CPP_ONLY, "namespace parent { static int nested_leak(int);\n\
                namespace child { int nested_leak(int x) { return x; } } }\n",
        "nested_leak", &[MP, PUBLIC]),
    (CPP_ONLY, "namespace a { static int flat_leak(int); }\n\
                namespace a::b { int flat_leak(int x) { return x; } }\n",
        "flat_leak", &[MP, PUBLIC]),
    (CPP_ONLY, "namespace parent { static int if_leak(int);\n\
                namespace child {\n#if FEATURE\nint if_leak(int x) { return x; }\n#endif\n} }\n",
        "if_leak", &[MP, PUBLIC]),
    (CPP_ONLY, "namespace parent { template<class T> static int template_leak(T);\n\
                namespace child { template<class T> int template_leak(T x) { return 0; } } }\n",
        "template_leak", &[MP, PUBLIC]),
    (CPP_ONLY, "namespace parent { static int c_leak(int);\n\
                namespace child { extern \"C\" int c_leak(int x) { return x; } } }\n",
        "c_leak", &[MP, PUBLIC]),
    // A nearer namespace that the qualifier names wins over a farther one.
    (CPP_ONLY, "namespace shadow { static int shadowed(int); }\n\
                namespace host { namespace shadow { int shadowed(int); }\n\
                int shadow::shadowed(int x) { return x; } }\n",
        "shadowed", &[MP, PUBLIC, PUBLIC]),
    // Return declarators: the function's own parameters, not the returned type's.
    (C_AND_CPP, "static int (*pointer_result(int))(int);\n\
                 int (*pointer_result(int value))(int) { return 0; }\n",
        "pointer_result", &[MP, MP]),
    (C_AND_CPP, "static int (*array_result(int))[2];\n\
                 int (*array_result(int value))[2] { return 0; }\n",
        "array_result", &[MP, MP]),
    (C_AND_CPP, "static void (*nested_result(int))(void (*)(int));\n\
                 void (*nested_result(int value))(void (*cb)(int inner)) { return 0; }\n",
        "nested_result", &[MP, MP]),
    (C_AND_CPP, "static int (*(*chained_result(int))(int))(long);\n\
                 int (*(*chained_result(int value))(int inner))(long deep) { return 0; }\n",
        "chained_result", &[MP, MP]),
    (CPP_ONLY, "static int (&reference_result(int))(int);\n\
                int (&reference_result(int value))(int) { return reference_result(value); }\n",
        "reference_result", &[MP, MP]),
    (CPP_ONLY, "namespace same { static int (*namespaced_result(int))(int);\n\
                int (*namespaced_result(int value))(int) { return 0; } }\n",
        "namespaced_result", &[MP, MP]),
    (C_AND_CPP, "static int first_declarator(int), (*second_result(int))(int);\n\
                 int (*second_result(int value))(int) { return 0; }\n",
        "second_result", &[MP, MP]),
    (C_AND_CPP, "#if FEATURE\nstatic int (*conditional_result(int))(int);\n#endif\n\
                 int (*conditional_result(int value))(int) { return 0; }\n",
        "conditional_result", &[MP, MP]),
    (CPP_ONLY, "extern \"C\" {\nstatic int (*linkage_result(int))(int);\n\
                int (*linkage_result(int value))(int) { return 0; }\n}\n",
        "linkage_result", &[MP, MP]),
    // An attribute on a parameter's name is not part of its type.
    (CPP_ONLY, "static int attributed_parameter(int);\n\
                int attributed_parameter(int value [[maybe_unused]]) { return 0; }\n",
        "attributed_parameter", &[MP, MP]),
    // Clean controls.
    (C_AND_CPP, "static int (*same_result(int value))(int);\n\
                 int (*same_result(int value))(int) { return 0; }\n",
        "same_result", &[MP, MP]),
    (C_AND_CPP, "static char *plain_result(int);\n\
                 char *plain_result(int value) { return 0; }\n",
        "plain_result", &[MP, MP]),
    (C_AND_CPP, "static int (*static_result(int value))(int) { return 0; }\n",
        "static_result", &[MP]),
    (C_AND_CPP, "int (*public_result(int value))(int) { return 0; }\n",
        "public_result", &[PUBLIC]),
    (CPP_ONLY, "namespace same_space { static int same_ns(int);\n\
                int same_ns(int x) { return x; } }\n",
        "same_ns", &[MP, MP]),
    (CPP_ONLY, "namespace left { static int sibling(int); }\n\
                namespace right { int sibling(int x) { return x; } }\n",
        "sibling", &[MP, PUBLIC]),
    (CPP_ONLY, "static int overload(int);\nint overload(long x) { return 0; }\n",
        "overload", &[MP, PUBLIC]),
    (CPP_ONLY, "namespace q { static int qualified(int); }\n\
                int q::qualified(int x) { return x; }\n",
        "qualified", &[MP, MP]),
    (CPP_ONLY, "namespace host { namespace inner { static int nearer(int); }\n\
                int inner::nearer(int x) { return x; } }\n",
        "nearer", &[MP, MP]),
    (CPP_ONLY, "namespace top { namespace mid { static int outward(int); } }\n\
                namespace top { int mid::outward(int x) { return x; } }\n",
        "outward", &[MP, MP]),
    (CPP_ONLY, "namespace parent { static int child_first(int); }\n\
                namespace parent { namespace child { } int child_first(int x) { return x; } }\n",
        "child_first", &[MP, MP]),
    (CPP_ONLY, "static int unknown_space(int);\n\
                int elsewhere::unknown_space(int x) { return x; }\n",
        "unknown_space", &[MP, PUBLIC]),
    // A function template and a non-template function are different
    // functions, and so are templates with different return types.
    (CPP_ONLY, "template<class T> static int template_only(int);\n\
                int template_only(int x) { return x; }\n",
        "template_only", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> static T template_return(int);\n\
                template<class T> int template_return(int x) { return x; }\n",
        "template_return", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> static T template_same(T);\n\
                template<class T> T template_same(T value) { return value; }\n",
        "template_same", &[MP, MP]),
    // A template's trailing return type and both constraint positions are
    // part of its signature, wherever the declarations sit.
    (CPP_ONLY, "template<class T> static auto trailing_differs(T) -> int;\n\
                template<class T> auto trailing_differs(T value) -> long { return 0; }\n",
        "trailing_differs", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> concept Big = sizeof(T) >= 4;\n\
                template<class T> requires Small<T> static int leading_constraint(T);\n\
                template<class T> requires Big<T> int leading_constraint(T value) { return 0; }\n",
        "leading_constraint", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> concept Big = sizeof(T) >= 4;\n\
                template<class T> static int trailing_constraint(T) requires Small<T>;\n\
                template<class T> int trailing_constraint(T value) requires Big<T> { return 0; }\n",
        "trailing_constraint", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> requires Small<T> static int constraint_dropped(T);\n\
                template<class T> int constraint_dropped(T value) { return 0; }\n",
        "constraint_dropped", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> static int constraint_added(T);\n\
                template<class T> int constraint_added(T value) requires Small<T> { return 0; }\n",
        "constraint_added", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> requires Small<T> static int constraint_moved(T);\n\
                template<class T> int constraint_moved(T value) requires Small<T> { return 0; }\n",
        "constraint_moved", &[MP, PUBLIC]),
    (CPP_ONLY, "namespace reopened { template<class T> static auto reopened_trailing(T) -> int; }\n\
                namespace reopened { template<class T> auto reopened_trailing(T value) -> long { return 0; } }\n",
        "reopened_trailing", &[MP, PUBLIC]),
    (CPP_ONLY, "#if FEATURE\ntemplate<class T> static auto if_trailing(T) -> int;\n#endif\n\
                template<class T> auto if_trailing(T value) -> long { return 0; }\n",
        "if_trailing", &[MP, PUBLIC]),
    (CPP_ONLY, "extern \"C++\" { template<class T> static auto linkage_trailing(T) -> int; }\n\
                template<class T> auto linkage_trailing(T value) -> long { return 0; }\n",
        "linkage_trailing", &[MP, PUBLIC]),
    // An abbreviated function template (`auto` parameter) is a template too.
    (CPP_ONLY, "static auto abbreviated_trailing(auto) -> int;\n\
                auto abbreviated_trailing(auto value) -> long { return 0; }\n",
        "abbreviated_trailing", &[MP, PUBLIC]),
    (CPP_ONLY, "static int abbreviated_return(const auto &);\n\
                long abbreviated_return(const auto &value) { return 0; }\n",
        "abbreviated_return", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> concept Big = sizeof(T) >= 4;\n\
                static int abbreviated_constraint(auto x) requires Small<decltype(x)>;\n\
                int abbreviated_constraint(auto x) requires Big<decltype(x)> { return 0; }\n",
        "abbreviated_constraint", &[MP, PUBLIC]),
    // Clean template controls.
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> concept Big = sizeof(T) >= 4;\n\
                template<Small T> static int constrained_parameter(T);\n\
                template<Big T> int constrained_parameter(T value) { return 0; }\n",
        "constrained_parameter", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> static int leading_return(T);\n\
                template<class T> long leading_return(T value) { return 0; }\n",
        "leading_return", &[MP, PUBLIC]),
    (CPP_ONLY, "template<class T> static auto trailing_same(T) -> int;\n\
                template<class T> auto trailing_same(T value) -> int { return 0; }\n",
        "trailing_same", &[MP, MP]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> static int constraint_same(T) requires Small<T>;\n\
                template<class T> int constraint_same(T value) requires Small<T> { return 0; }\n",
        "constraint_same", &[MP, MP]),
    (CPP_ONLY, "template<class T> concept Small = sizeof(T) < 4;\n\
                template<class T> requires Small<T> static int leading_same(T);\n\
                template<class T> requires Small<T> int leading_same(T value) { return 0; }\n",
        "leading_same", &[MP, MP]),
    (CPP_ONLY, "static auto abbreviated_same(auto) -> int;\n\
                auto abbreviated_same(auto value) -> int { return 0; }\n",
        "abbreviated_same", &[MP, MP]),
    // An exception specification is not part of a function's identity.
    (CPP_ONLY, "template<class T> static auto noexcept_spelling(T) noexcept -> int;\n\
                template<class T> auto noexcept_spelling(T value) noexcept(true) -> int { return 0; }\n",
        "noexcept_spelling", &[MP, MP]),
    // An ordinary function's return type is not part of its identity.
    (CPP_ONLY, "static int plain_trailing(int);\n\
                auto plain_trailing(int value) -> int { return value; }\n",
        "plain_trailing", &[MP, MP]),
];

#[test]
fn earlier_static_linkage_matrix_holds_for_c_and_cpp() {
    let mut failures = Vec::new();
    for (languages, source, name, expected) in LINKAGE_MATRIX {
        for extension in *languages {
            let extracted = extract_inline(source, extension);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                assert_levels_in_order(&extracted, name, expected);
            }));
            if outcome.is_err() {
                failures.push(format!(".{extension} `{name}`"));
            }
        }
    }
    assert!(failures.is_empty(), "linkage matrix rows failed: {failures:#?}");
}

/// An explicitly `static` function-pointer-return definition is storage-static.
#[test]
fn static_function_pointer_return_is_storage_static() {
    for extension in C_AND_CPP {
        let extracted = extract_inline(
            "static int (*static_result(int value))(int) { return 0; }\n",
            extension,
        );
        assert!(
            extracted.symbol("static_result", None).is_static_storage,
            ".{extension}: `static_result` must be storage-static"
        );
    }
}

/// One cell group of the const-object linkage matrix: `(languages, source,
/// (name, level of its one declaration) for every name under test)`.
type ConstObjectRow = (&'static [&'static str], &'static str, &'static [(&'static str, Option<Visibility>)]);

const C_ONLY: &[&str] = &["c"];

/// A C++ namespace-scope object is ModulePrivate when the object itself is
/// `const`: its own outermost type, read by walking the declarator from the
/// outside toward the name (an array takes its element's constness), not the
/// qualifier of a nested pointer or of the pointee. A reference is never a
/// const object, `constexpr` included. C has no implicit const-private rule.
const CONST_OBJECT_MATRIX: &[ConstObjectRow] = &[
    // Nested pointers: only the qualifier on the outermost pointer counts.
    (CPP_ONLY, "char *const *mutable_to_const;\nconst int *const *mutable_to_const_int;\n",
        &[("mutable_to_const", PUBLIC), ("mutable_to_const_int", PUBLIC)]),
    (CPP_ONLY, "char **const const_double = 0;\nconst int **const const_double_int = 0;\n",
        &[("const_double", MP), ("const_double_int", MP)]),
    // Arrays take their element's constness.
    (CPP_ONLY, "char **const const_double_array[2] = {};\nchar *const *mutable_pointer_array[2];\n",
        &[("const_double_array", MP), ("mutable_pointer_array", PUBLIC)]),
    // Parenthesized declarators are walked through.
    (CPP_ONLY, "int (*const const_function_pointer)(int) = 0;\n\
                const int (*mutable_array_pointer)[2];\n\
                int (*const const_array_pointer)[2] = 0;\n",
        &[("const_function_pointer", MP), ("mutable_array_pointer", PUBLIC), ("const_array_pointer", MP)]),
    // References are never const objects, even under `constexpr`.
    (CPP_ONLY, "const int backing[2] = {1, 2};\n\
                const int (&array_reference)[2] = backing;\n\
                constexpr const int &constexpr_reference = backing[0];\n",
        &[("backing", MP), ("array_reference", PUBLIC), ("constexpr_reference", PUBLIC)]),
    // Every name of a multi-declarator declaration is decided on its own.
    (CPP_ONLY, "int multi_a, **const multi_b = 0, *const *multi_c = 0;\n",
        &[("multi_a", PUBLIC), ("multi_b", MP), ("multi_c", PUBLIC)]),
    // Scope wrappers.
    (CPP_ONLY, "namespace wrapped { char **const ns_const = 0; char *const *ns_mutable; }\n",
        &[("ns_const", MP), ("ns_mutable", PUBLIC)]),
    (CPP_ONLY, "#if FEATURE\nchar **const if_const = 0;\nchar *const *if_mutable;\n#endif\n",
        &[("if_const", MP), ("if_mutable", PUBLIC)]),
    (CPP_ONLY, "extern \"C\" {\nchar **const c_block_const = 0;\nchar *const *c_block_mutable;\n}\n",
        &[("c_block_const", MP), ("c_block_mutable", PUBLIC)]),
    // Clean controls: scalars, single and double pointers.
    (CPP_ONLY, "const int const_scalar = 1;\nint *mutable_pointer;\nint *const const_pointer = 0;\n\
                const int *pointer_to_const;\nint **mutable_double;\n",
        &[("const_scalar", MP), ("mutable_pointer", PUBLIC), ("const_pointer", MP),
          ("pointer_to_const", PUBLIC), ("mutable_double", PUBLIC)]),
    // Clean controls: arrays.
    (CPP_ONLY, "const int const_array[2] = {1, 2};\nconst int const_grid[2][3] = {};\n\
                int mutable_array[2];\nconst int *array_of_pointers_to_const[2];\n\
                int *const array_of_const_pointers[2] = {};\n",
        &[("const_array", MP), ("const_grid", MP), ("mutable_array", PUBLIC),
          ("array_of_pointers_to_const", PUBLIC), ("array_of_const_pointers", MP)]),
    // Clean controls: mutable function pointer, reference to a const scalar.
    (CPP_ONLY, "const int referenced = 1;\nint (*mutable_function_pointer)(int);\n\
                const int &scalar_reference = referenced;\n",
        &[("mutable_function_pointer", PUBLIC), ("scalar_reference", PUBLIC)]),
    // Overrides: `extern` and `inline` keep a const object Public;
    // `constexpr` makes a pointer const.
    (CPP_ONLY, "extern char **const extern_pointer;\ninline char **const inline_pointer = 0;\n\
                extern const int extern_array[2];\ninline const int inline_array[2] = {};\n\
                constexpr int *constexpr_pointer = nullptr;\n",
        &[("extern_pointer", PUBLIC), ("inline_pointer", PUBLIC), ("extern_array", PUBLIC),
          ("inline_array", PUBLIC), ("constexpr_pointer", MP)]),
    // The unnamed-namespace rule applies independently of constness.
    (CPP_ONLY, "namespace { char *const *anonymous_mutable; }\n",
        &[("anonymous_mutable", MP)]),
    // Locals have no linkage; members take their access, not object linkage.
    (CPP_ONLY, "void host() { char **const local_const = 0; char *const *local_mutable = 0; }\n\
                struct Holder { char **const member_const; char *const *member_mutable; };\n",
        &[("local_const", UNKNOWN), ("local_mutable", UNKNOWN),
          ("member_const", PUBLIC), ("member_mutable", PUBLIC)]),
    // C: no implicit const-private rule.
    (C_ONLY, "const int c_const_scalar = 1;\nchar *const *c_mutable_to_const;\n\
              char **const c_const_double = 0;\nint (*const c_const_function_pointer)(int) = 0;\n",
        &[("c_const_scalar", PUBLIC), ("c_mutable_to_const", PUBLIC),
          ("c_const_double", PUBLIC), ("c_const_function_pointer", PUBLIC)]),
];

#[test]
fn const_object_linkage_matrix_reads_the_outermost_declarator() {
    let mut failures = Vec::new();
    for (languages, source, cells) in CONST_OBJECT_MATRIX {
        for extension in *languages {
            let extracted = extract_inline(source, extension);
            for (name, expected) in *cells {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    assert_levels_in_order(&extracted, name, &[*expected]);
                }));
                if outcome.is_err() {
                    failures.push(format!(".{extension} `{name}`"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "const-object linkage rows failed: {failures:#?}");
}

#[test]
fn compound_csharp_field_metadata_levels() {
    let csharp = extract_primary(ProgrammingLanguage::CSharp);
    let fields = &csharp
        .symbol("PublicWidget", Some("Example.Symbols"))
        .type_metadata
        .as_ref()
        .expect("PublicWidget metadata")
        .fields;
    let level = |name: &str| fields.iter().find(|field| field.name == name).map(|field| field.visibility);
    assert_eq!(level("ProtectedInternalField"), Some(PROTECTED_INTERNAL));
    assert_eq!(level("PrivateProtectedField"), Some(PRIVATE_PROTECTED));
    assert_eq!(level("UnmarkedField"), Some(PRIVATE));

}
