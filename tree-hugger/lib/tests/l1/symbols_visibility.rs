//! Test 3 of `2026-10-03-visibility-and-kind`: the visibility matrix for all
//! sixteen languages, asserted on `SymbolInfo::visibility` and
//! `SymbolRecord.visibility.level`.

use tree_hugger::{ProgrammingLanguage, SymbolKind, Visibility, VisibilityLevel};

use crate::symbols_fixtures::{
    Extracted, extract_inline, extract_path, extract_primary, symbols_fixture_dir,
};
use crate::symbols_kinds::rows_by_language;

use ProgrammingLanguage as L;

fn assert_level(extracted: &Extracted, name: &str, container: Option<&str>, expected: Option<Visibility>) {
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
    (L::Go, "ExportedMethod", None, PUBLIC),
    (L::Go, "packageMethod", None, PACKAGE),
    (L::Go, "OtherExportedMethod", None, PUBLIC),
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
    (L::Lua, "tableFunction", None, UNKNOWN),
    (L::Lua, "methodWithSelf", None, UNKNOWN),
    (L::Lua, "tableField", None, UNKNOWN),
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

/// Test 3: every visibility-matrix row holds on `SymbolInfo::visibility` and
/// `SymbolRecord.visibility.level`.
#[test]
fn visibility_matrix_holds_through_both_paths() {
    for (language, rows) in rows_by_language(VISIBILITY_MATRIX, |row| row.0) {
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

/// Field metadata (`hug classes`) carries the same level as the standalone
/// field, including C#'s compound levels and defaults no keyword states.
#[test]
fn field_metadata_agrees_with_the_standalone_fields() {
    let mut compared = 0;
    for language in ProgrammingLanguage::all() {
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
    assert!(compared > 50, "only {compared} field entries compared");

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
