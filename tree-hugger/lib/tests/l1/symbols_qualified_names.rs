//! The qualified-name matrix of `2026-10-03-symbol-filters-and-reports`:
//! every language's in-file qualified names use the language's own
//! separators, start at an explicit namespace or package where the source
//! declares one, and derive the symbol ID, which therefore does not depend on
//! how the file's path was spelled. A query's result shows each name not
//! rooted at a namespace behind its file prefix (by default the file name
//! plus `#`).

use std::collections::HashSet;
use std::path::Path;

use tree_hugger::{ProgrammingLanguage, SourceFile, SymbolQuery, SymbolRecord, file_identity};

use crate::symbols_fixtures::{Extracted, extract_inline, extract_primary, in_file_name, primary_fixture};

use ProgrammingLanguage as L;

/// `(name, container, in-file qualified name, is_namespace_rooted)`.
pub type QualifiedCase = (&'static str, Option<&'static str>, &'static str, bool);

pub struct QualifiedNameRow {
    pub language: ProgrammingLanguage,
    /// Must hold an item, a member (unless the language has none) and a
    /// local or nested declaration.
    pub cases: &'static [QualifiedCase],
    /// A method name declared on two types in the fixture, or why the
    /// language has no methods.
    pub same_named_method: Result<&'static str, &'static str>,
    /// Whether the language has explicit namespace or package syntax. With
    /// it the row holds a rooted case; without it no record is rooted.
    pub has_namespace_syntax: bool,
}

pub const QUALIFIED_NAME_MATRIX: &[QualifiedNameRow] = &[
    QualifiedNameRow {
        language: L::Rust,
        cases: &[
            ("public_function", None, "public_function", false),
            // Inline modules qualify from source but do not root the name.
            ("module_public_function", Some("outer_module"), "outer_module::module_public_function", false),
            ("super_function", Some("crate_inner_module"), "outer_module::crate_inner_module::super_function", false),
            ("Record", Some("schema"), "schema::Record", false),
            // An `impl` member is qualified by the implemented type, inside
            // the module that holds the `impl`.
            ("load", Some("Record"), "schema::Record::load", false),
            ("instance_method", Some("PublicStruct"), "PublicStruct::instance_method", false),
            ("private_trait_method", Some("String"), "String::private_trait_method", false),
            ("required_trait_method", Some("PublicTrait"), "PublicTrait::required_trait_method", false),
            ("variant_field", Some("PublicEnum"), "PublicEnum::StructVariant::variant_field", false),
            ("local_value", None, "private_function::local_value", false),
            ("nested_module_function", Some("nested_local_module"), "function_with_nested_items::nested_local_module::nested_module_function", false),
        ],
        same_named_method: Ok("required_trait_method"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::TypeScript,
        cases: &[
            ("exportedFunction", None, "exportedFunction", false),
            ("publicMethod", Some("ExportedWidget"), "ExportedWidget.publicMethod", false),
            ("initialValue", Some("ExportedWidget"), "ExportedWidget.constructor.initialValue", false),
            // A TypeScript namespace qualifies from source and contains its
            // members, but does not root the name: the file fallback still
            // applies.
            ("namespaceFunction", Some("ExportedNamespace"), "ExportedNamespace.namespaceFunction", false),
            ("localValue", None, "moduleFunction.localValue", false),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::JavaScript,
        cases: &[
            ("exportedFunction", None, "exportedFunction", false),
            ("unmarkedMethod", Some("ExportedWidget"), "ExportedWidget.unmarkedMethod", false),
            ("#hashPrivateMethod", Some("ExportedWidget"), "ExportedWidget.#hashPrivateMethod", false),
            ("localValue", None, "moduleFunction.localValue", false),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::Go,
        cases: &[
            ("symbols", None, "symbols", true),
            ("ExportedFunction", None, "symbols.ExportedFunction", true),
            // A receiver method is qualified by its receiver's base type.
            ("ExportedMethod", Some("ExportedStruct"), "symbols.ExportedStruct.ExportedMethod", true),
            ("localValue", None, "symbols.packageFunction.localValue", true),
        ],
        same_named_method: Ok("Area"),
        has_namespace_syntax: true,
    },
    QualifiedNameRow {
        language: L::Python,
        cases: &[
            ("module_function", None, "module_function", false),
            ("instance_method", Some("PlainWidget"), "PlainWidget.instance_method", false),
            ("annotated_instance_field", Some("PlainWidget"), "PlainWidget.annotated_instance_field", false),
            ("nested_function", None, "outer_function.nested_function", false),
            ("local_value", None, "module_function.local_value", false),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::Java,
        cases: &[
            ("com.example.symbols", None, "com.example.symbols", true),
            ("PublicWidget", None, "com.example.symbols.PublicWidget", true),
            ("publicMethodInPrivateClass", Some("PrivateNestedClass"), "com.example.symbols.PublicWidget.PrivateNestedClass.publicMethodInPrivateClass", true),
            ("localValue", Some("PublicWidget"), "com.example.symbols.PublicWidget.methodWithLocals.localValue", true),
            // A constructor is not extracted, but still names its parameters.
            ("initialValue", Some("PublicWidget"), "com.example.symbols.PublicWidget.PublicWidget.initialValue", true),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: true,
    },
    QualifiedNameRow {
        language: L::CSharp,
        cases: &[
            ("Example.Symbols", None, "Example.Symbols", true),
            ("PublicWidget", Some("Example.Symbols"), "Example.Symbols.PublicWidget", true),
            ("PublicMethod", Some("PublicWidget"), "Example.Symbols.PublicWidget.PublicMethod", true),
            ("localValue", Some("PublicWidget"), "Example.Symbols.PublicWidget.MethodWithLocals.localValue", true),
        ],
        same_named_method: Ok("Area"),
        has_namespace_syntax: true,
    },
    QualifiedNameRow {
        language: L::C,
        cases: &[
            ("public_function", None, "public_function", false),
            ("tag_field", Some("NamedTag"), "NamedTag.tag_field", false),
            ("COLOR_RED", Some("NamedColor"), "NamedColor.COLOR_RED", false),
            ("local_value", None, "public_function.local_value", false),
        ],
        same_named_method: Err("C has no methods"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::Cpp,
        cases: &[
            ("fixture_space", None, "fixture_space", true),
            ("namespaceFunction", Some("fixture_space"), "fixture_space::namespaceFunction", true),
            ("firstPublicMethod", Some("AccessWidget"), "fixture_space::AccessWidget::firstPublicMethod", true),
            ("area", Some("FirstShape"), "outer_space::inner_space::FirstShape::area", true),
            // Outside a named namespace (here also in an anonymous one) a
            // name is not rooted.
            ("area", Some("SecondShape"), "SecondShape::area", false),
            ("hiddenFunction", None, "hiddenFunction", false),
            ("localStaticCounter", None, "functionWithLocalStatic::localStaticCounter", false),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: true,
    },
    QualifiedNameRow {
        language: L::Swift,
        cases: &[
            ("publicFunction", None, "publicFunction", false),
            ("openMethod", Some("OpenWidget"), "OpenWidget.openMethod", false),
            ("unmarkedMethodInPublicExtension", Some("OpenWidget"), "OpenWidget.unmarkedMethodInPublicExtension", false),
            ("unmarkedMethodInInnerPrivate", Some("InnerPrivate"), "PublicApi.InnerPrivate.unmarkedMethodInInnerPrivate", false),
            ("localValue", None, "unmarkedFunction.localValue", false),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::Scala,
        cases: &[
            ("topLevelFunction", None, "example.symbols.topLevelFunction", true),
            ("objectMethod", Some("StandaloneObject"), "example.symbols.StandaloneObject.objectMethod", true),
            ("publicMethod", Some("PlainWidget"), "example.symbols.PlainWidget.publicMethod", true),
            ("localValue", None, "example.symbols.topLevelFunction.localValue", true),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: true,
    },
    QualifiedNameRow {
        language: L::Php,
        cases: &[
            // A namespace segment is followed by `\`, a type by `::`.
            ("topLevelFunction", None, "Example\\Symbols\\topLevelFunction", true),
            ("PublicWidget", None, "Example\\Symbols\\PublicWidget", true),
            ("staticFactory", Some("PublicWidget"), "Example\\Symbols\\PublicWidget::staticFactory", true),
            ("initialValue", Some("PublicWidget"), "Example\\Symbols\\PublicWidget::__construct::initialValue", true),
        ],
        same_named_method: Ok("area"),
        has_namespace_syntax: true,
    },
    QualifiedNameRow {
        language: L::Perl,
        cases: &[
            ("Fixture::Greeter", None, "Fixture::Greeter", true),
            ("greet", Some("Fixture::Greeter"), "Fixture::Greeter::greet", true),
            // A package name is absolute, block form included.
            ("Fixture::Block", None, "Fixture::Block", true),
            ("block_package_sub", Some("Fixture::Block"), "Fixture::Block::block_package_sub", true),
            ("after_block_sub", Some("Fixture::Other"), "Fixture::Other::after_block_sub", true),
            ("$first_name", Some("greet"), "Fixture::Greeter::greet::$first_name", true),
        ],
        same_named_method: Err("Perl subs belong to packages, not types"),
        has_namespace_syntax: true,
    },
    QualifiedNameRow {
        language: L::Lua,
        cases: &[
            ("globalFunction", None, "globalFunction", false),
            // `M.f` and `M:m` keep their own separator.
            ("tableFunction", Some("Widget"), "Widget.tableFunction", false),
            ("methodWithSelf", Some("Widget"), "Widget:methodWithSelf", false),
            ("tableField", Some("Widget"), "Widget.tableField", false),
            ("blockLocalValue", None, "globalFunction.blockLocalValue", false),
        ],
        same_named_method: Ok("methodWithSelf"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::Bash,
        cases: &[
            ("keyword_function", None, "keyword_function", false),
            ("nested_function", None, "outer_function.nested_function", false),
            ("local_value", None, "keyword_function.local_value", false),
        ],
        same_named_method: Err("shell functions have no owning type"),
        has_namespace_syntax: false,
    },
    QualifiedNameRow {
        language: L::Zsh,
        cases: &[
            ("paren_function", None, "paren_function", false),
            ("nested_function", None, "outer_function.nested_function", false),
            ("local_value", None, "paren_function.local_value", false),
        ],
        same_named_method: Err("shell functions have no owning type"),
        has_namespace_syntax: false,
    },
];

/// The languages with a qualified-name row, for the coverage guard.
pub fn qualified_name_matrix_languages() -> Vec<ProgrammingLanguage> {
    QUALIFIED_NAME_MATRIX.iter().map(|row| row.language).collect()
}

fn assert_qualified(extracted: &Extracted, case: &QualifiedCase) {
    let (name, container, qualified, rooted) = *case;
    let label = &extracted.label;
    let symbol = extracted.symbol(name, container);
    assert_eq!(symbol.qualified_name, qualified, "{label}: `{name}` in symbols()");
    assert_eq!(symbol.is_namespace_rooted, rooted, "{label}: `{name}` rooted in symbols()");
    let record = extracted.record(name, container);
    let displayed = if rooted { qualified.to_string() } else { format!("{label}#{qualified}") };
    assert_eq!(record.identity.qualified_name, displayed, "{label}: `{name}` in symbol_index_v2()");
    assert_eq!(record.identity.is_namespace_rooted, rooted, "{label}: `{name}` rooted in symbol_index_v2()");
}

crate::language_cases::language_tests!(qualified_name_matrix_holds_through_both_paths => check_qualified_name_matrix_holds_through_both_paths);

fn check_qualified_name_matrix_holds_through_both_paths(language: ProgrammingLanguage) {
    for row in QUALIFIED_NAME_MATRIX.iter().filter(|row| row.language == language) {
        let extracted = extract_primary(row.language);
        let language = row.language.name();
        for case in row.cases {
            assert_qualified(&extracted, case);
        }

        if row.has_namespace_syntax {
            assert!(row.cases.iter().any(|case| case.3), "{language}: no rooted case");
        } else {
            let rooted: Vec<&str> = extracted
                .records
                .iter()
                .filter(|record| record.identity.is_namespace_rooted)
                .map(|record| record.identity.qualified_name.as_str())
                .collect();
            assert!(rooted.is_empty(), "{language}: rooted names without namespace syntax: {rooted:?}");
        }

        match row.same_named_method {
            Ok(method) => {
                let same: Vec<&SymbolRecord> = extracted
                    .records
                    .iter()
                    .filter(|record| record.identity.name == method && record.identity.container_name.is_some())
                    .collect();
                assert!(same.len() >= 2, "{language}: `{method}` is declared on fewer than two types");
                let names: HashSet<&str> = same.iter().map(|record| record.identity.qualified_name.as_str()).collect();
                let ids: HashSet<&str> = same.iter().map(|record| record.id.0.as_str()).collect();
                assert_eq!(names.len(), same.len(), "{language}: `{method}` qualified names collide: {names:?}");
                assert_eq!(ids.len(), same.len(), "{language}: `{method}` IDs collide");
            }
            Err(reason) => assert!(
                !extracted.records.iter().any(|record| record.kind == tree_hugger::SymbolKindV2::Method),
                "{language}: methods extracted despite `{reason}`"
            ),
        }
    }
}

crate::language_cases::language_tests!(every_qualified_name_ends_in_its_name_and_ids_are_unique => check_every_qualified_name_ends_in_its_name_and_ids_are_unique);

/// Every qualified name ends in the symbol's own name and is never empty;
/// IDs are unique within a file.
fn check_every_qualified_name_ends_in_its_name_and_ids_are_unique(language: ProgrammingLanguage) {
    for language in [language] {
        let extracted = extract_primary(language);
        let mut ids = HashSet::new();
        for record in &extracted.records {
            let identity = &record.identity;
            assert!(
                identity.qualified_name.ends_with(&identity.name),
                "{}: `{}` qualified as `{}`",
                extracted.label,
                identity.name,
                identity.qualified_name
            );
            assert!(ids.insert(record.id.0.clone()), "{}: duplicate ID for `{}`", extracted.label, identity.qualified_name);
        }
    }
}

/// Repeated locals of one name in one function share a qualified name; the
/// declaration order tells their IDs apart.
#[test]
fn same_qualified_names_get_distinct_ids() {
    let extracted = extract_inline("fn f() {\n    let x = 1;\n    let x = 2;\n}\n", "rs");
    let xs: Vec<&SymbolRecord> = extracted.records.iter().filter(|record| record.identity.name == "x").collect();
    assert_eq!(xs.len(), 2);
    assert!(xs.iter().all(|record| in_file_name(record) == "f::x"));
    assert_ne!(xs[0].id, xs[1].id);
}

/// The ID derives from the file's identity and the in-file name, so two
/// spellings of one file (relative with `..`, absolute) give one ID per
/// declaration and identical qualified names.
#[test]
fn two_spellings_of_one_file_give_the_same_ids() {
    let fixture = primary_fixture(L::Java).expect("Java fixture");
    let symbols_dir = fixture.parent().expect("fixture dir");
    let file_name = fixture.file_name().expect("file name");
    let detour = symbols_dir.join("multi").join("..").join(file_name);

    let records = |path: &Path| -> Vec<(String, String)> {
        SourceFile::read(path)
            .expect("read source")
            .query(&SymbolQuery::new())
            .expect("query")
            .index
            .symbols
            .into_iter()
            .map(|record| (record.id.0, record.identity.qualified_name))
            .collect()
    };
    assert_eq!(file_identity(&detour), file_identity(&fixture), "the spellings name one file");
    let direct = records(&fixture);
    assert!(!direct.is_empty());
    assert_eq!(records(&detour), direct);
}
