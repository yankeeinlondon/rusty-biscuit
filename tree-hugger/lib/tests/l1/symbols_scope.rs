//! The scope matrix of `2026-10-03-symbol-filters-and-reports`: every
//! language's primary fixture classifies declarations as `Item`, `Member` or
//! `Local` by the nearest semantic declaration, on both public paths
//! (`TreeFile::symbols()` and the records of `symbol_index_v2()`).

use tree_hugger::{ProgrammingLanguage, SymbolKind, SymbolScope};

use crate::symbols_fixtures::{Extracted, extract_inline, extract_path, extract_primary};

use ProgrammingLanguage as L;
use SymbolScope::{Item, Local, Member};

/// One expected scope: `(name, container, scope)` in the primary fixture.
pub type ScopeCase = (&'static str, Option<&'static str>, SymbolScope);

/// One language's row.
pub struct ScopeRow {
    pub language: ProgrammingLanguage,
    /// Must hold an `Item` and a `Local` case, a `Member` case unless
    /// `no_members` says why the language has none, and a `Parameter` case
    /// unless `no_parameters` says why none is extracted.
    pub cases: &'static [ScopeCase],
    /// Why no declaration in the language is member-scoped.
    pub no_members: Option<&'static str>,
    /// Why the language produces no `Parameter` records.
    pub no_parameters: Option<&'static str>,
}

const SHELL_NO_MEMBERS: &str = "shell functions and variables have no owning type";
const SHELL_NO_PARAMETERS: &str = "shell functions declare no parameter list";

pub const SCOPE_MATRIX: &[ScopeRow] = &[
    ScopeRow {
        language: L::Rust,
        cases: &[
            ("public_function", None, Item),
            // Inline-module contents are items, however deep.
            ("module_public_function", Some("outer_module"), Item),
            ("super_function", Some("crate_inner_module"), Item),
            ("Record", Some("schema"), Item),
            // `impl` and `trait` contents are members.
            ("instance_method", Some("PublicStruct"), Member),
            ("load", Some("Record"), Member),
            ("required_trait_method", Some("PublicTrait"), Member),
            ("public_field", Some("PublicStruct"), Member),
            ("UnitVariant", Some("PublicEnum"), Member),
            ("parameter_value", None, Local),
            ("local_value", None, Local),
            ("LOCAL_STATIC", None, Local),
            // A type and a module declared in a function are local, and so
            // is everything in that module; the type's own fields are members.
            ("NestedLocalType", None, Local),
            ("nested_field", Some("NestedLocalType"), Member),
            ("nested_local_module", None, Local),
            ("nested_module_function", Some("nested_local_module"), Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::TypeScript,
        cases: &[
            ("exportedFunction", None, Item),
            ("ExportedWidget", None, Item),
            // Namespace contents are items.
            ("namespaceFunction", Some("ExportedNamespace"), Item),
            ("publicMethod", Some("ExportedWidget"), Member),
            ("staticProperty", Some("ExportedWidget"), Member),
            ("interfaceProperty", Some("ExportedInterface"), Member),
            ("parameterValue", None, Local),
            ("initialValue", Some("ExportedWidget"), Local),
            ("localValue", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::JavaScript,
        cases: &[
            ("exportedFunction", None, Item),
            ("MODULE_CONSTANT", None, Item),
            ("unmarkedMethod", Some("ExportedWidget"), Member),
            ("constructor", Some("ExportedWidget"), Member),
            ("unmarkedProperty", Some("ExportedWidget"), Member),
            ("parameterValue", None, Local),
            ("localValue", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Go,
        cases: &[
            ("ExportedFunction", None, Item),
            ("ExportedStruct", None, Item),
            // A receiver method is a member of its receiver's type.
            ("ExportedMethod", Some("ExportedStruct"), Member),
            ("Area", Some("packageStruct"), Member),
            ("parameterValue", None, Local),
            ("localValue", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Python,
        cases: &[
            ("module_function", None, Item),
            ("MODULE_CONSTANT", None, Item),
            ("instance_method", Some("PlainWidget"), Member),
            // Decorated methods keep their membership.
            ("static_method", Some("PlainWidget"), Member),
            ("annotated_instance_field", Some("PlainWidget"), Member),
            ("parameter_value", None, Local),
            ("local_value", None, Local),
            ("nested_function", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Java,
        cases: &[
            ("PublicWidget", None, Item),
            ("publicMethod", Some("PublicWidget"), Member),
            ("PrivateNestedClass", Some("PublicWidget"), Member),
            ("publicMethodInPrivateClass", Some("PrivateNestedClass"), Member),
            ("SIDES", Some("PublicShape"), Member),
            ("parameterValue", Some("PublicWidget"), Local),
            ("localValue", Some("PublicWidget"), Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::CSharp,
        cases: &[
            // Namespace contents are items.
            ("PublicWidget", Some("Example.Symbols"), Item),
            ("IShape", Some("Example.Symbols"), Item),
            ("PublicMethod", Some("PublicWidget"), Member),
            ("Count", Some("PublicWidget"), Member),
            ("RunAsync", Some("AsyncWork"), Member),
            ("parameterValue", Some("PublicWidget"), Local),
            ("localValue", Some("PublicWidget"), Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::C,
        cases: &[
            ("public_function", None, Item),
            ("public_global_variable", None, Item),
            ("tag_field", Some("NamedTag"), Member),
            ("COLOR_RED", Some("NamedColor"), Member),
            ("parameter_value", None, Local),
            ("local_value", None, Local),
            ("local_static_counter", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Cpp,
        cases: &[
            // Named and anonymous namespace contents are items.
            ("namespaceFunction", Some("fixture_space"), Item),
            ("AccessWidget", Some("fixture_space"), Item),
            ("FirstShape", Some("inner_space"), Item),
            ("hiddenFunction", None, Item),
            ("firstPublicMethod", Some("AccessWidget"), Member),
            ("staticMemberField", Some("AccessWidget"), Member),
            ("Red", Some("ScopedColor"), Member),
            ("parameterValue", Some("fixture_space"), Local),
            ("localValue", Some("fixture_space"), Local),
            ("memberLocalStatic", Some("AccessWidget"), Local),
            // A template type parameter names nothing outside its template.
            ("T", Some("DeclaratorFieldForms"), Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Swift,
        cases: &[
            ("publicFunction", None, Item),
            ("OpenWidget", None, Item),
            ("openMethod", Some("OpenWidget"), Member),
            // Extension members belong to the extended type.
            ("unmarkedMethodInPublicExtension", Some("OpenWidget"), Member),
            ("north", Some("PublicDirection"), Member),
            ("parameterValue", None, Local),
            ("localValue", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Scala,
        cases: &[
            ("topLevelFunction", None, Item),
            ("StandaloneObject", None, Item),
            ("objectMethod", Some("StandaloneObject"), Member),
            ("publicMethod", Some("PlainWidget"), Member),
            ("publicField", Some("PlainWidget"), Member),
            ("parameterValue", None, Local),
            ("constructorField", Some("PlainWidget"), Local),
            ("localValue", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Php,
        cases: &[
            ("topLevelFunction", None, Item),
            ("TOP_CONSTANT", None, Item),
            ("greet", Some("Greets"), Member),
            ("publicField", Some("PublicWidget"), Member),
            ("CLASS_CONSTANT", Some("ConstantForms"), Member),
            ("parameterValue", None, Local),
            ("initialValue", Some("PublicWidget"), Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Perl,
        cases: &[
            // A package is a namespace: its subs and `our` variables are items.
            ("greet", Some("Fixture::Greeter"), Item),
            ("$package_level_our", Some("Fixture::Greeter"), Item),
            ("block_package_sub", Some("Fixture::Block"), Item),
            ("$first_name", Some("greet"), Local),
            ("$block_local", Some("block_package_sub"), Local),
        ],
        no_members: Some("Perl packages are namespaces; no type members are extracted"),
        no_parameters: Some("Perl subs declare no parameter list"),
    },
    ScopeRow {
        language: L::Lua,
        cases: &[
            ("globalFunction", None, Item),
            ("chunkLocalVariable", None, Item),
            // A chunk-level block is not a function body.
            ("nestedBlockLocal", None, Item),
            // `M.f`, `M:m` and `M.x = …` name their table wherever written.
            ("tableFunction", Some("Widget"), Member),
            ("methodWithSelf", Some("Widget"), Member),
            ("tableField", Some("Widget"), Member),
            ("parameterValue", None, Local),
            ("blockLocalValue", None, Local),
        ],
        no_members: None,
        no_parameters: None,
    },
    ScopeRow {
        language: L::Bash,
        cases: &[
            ("keyword_function", None, Item),
            ("outer_function", None, Item),
            ("GLOBAL_VARIABLE", None, Item),
            ("local_value", None, Local),
            // Global once the outer function runs, but declared in its body.
            ("nested_function", None, Local),
        ],
        no_members: Some(SHELL_NO_MEMBERS),
        no_parameters: Some(SHELL_NO_PARAMETERS),
    },
    ScopeRow {
        language: L::Zsh,
        cases: &[
            ("paren_function", None, Item),
            ("outer_function", None, Item),
            ("GLOBAL_VARIABLE", None, Item),
            ("local_value", None, Local),
            ("nested_function", None, Local),
        ],
        no_members: Some(SHELL_NO_MEMBERS),
        no_parameters: Some(SHELL_NO_PARAMETERS),
    },
];

/// The languages with a scope-matrix row, for the coverage guard.
pub fn scope_matrix_languages() -> Vec<ProgrammingLanguage> {
    SCOPE_MATRIX.iter().map(|row| row.language).collect()
}

fn assert_scope(extracted: &Extracted, name: &str, container: Option<&str>, scope: SymbolScope) {
    let label = &extracted.label;
    assert_eq!(
        extracted.symbol(name, container).scope,
        scope,
        "{label}: `{name}` in {container:?} scope in symbols()"
    );
    assert_eq!(
        extracted.record(name, container).scope,
        scope,
        "{label}: `{name}` in {container:?} scope in symbol_index_v2()"
    );
}

crate::language_cases::language_tests!(scope_matrix_holds_through_both_paths => check_scope_matrix_holds_through_both_paths);

fn check_scope_matrix_holds_through_both_paths(language: ProgrammingLanguage) {
    for row in SCOPE_MATRIX.iter().filter(|row| row.language == language) {
        let extracted = extract_primary(row.language);
        let language = row.language.name();
        for (name, container, scope) in row.cases {
            assert_scope(&extracted, name, *container, *scope);
        }

        // The row holds each required case, and an n/a is really n/a.
        let has = |scope: SymbolScope| row.cases.iter().any(|case| case.2 == scope);
        assert!(has(Item), "{language}: no Item case");
        assert!(has(Local), "{language}: no Local case");
        let members = extracted.records.iter().filter(|record| record.scope == Member).count();
        match row.no_members {
            None => assert!(has(Member), "{language}: no Member case"),
            Some(reason) => assert_eq!(members, 0, "{language}: member-scoped records despite `{reason}`"),
        }
        let parameters: Vec<_> = extracted
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Parameter)
            .collect();
        match row.no_parameters {
            None => assert!(
                row.cases.iter().any(|(name, container, scope)| *scope == Local
                    && extracted.symbol(name, *container).kind == SymbolKind::Parameter),
                "{language}: no Local parameter case"
            ),
            Some(reason) => assert!(parameters.is_empty(), "{language}: parameters despite `{reason}`"),
        }
    }
}

crate::language_cases::language_tests!(parameters_are_local_and_member_kinds_are_never_items => check_parameters_are_local_and_member_kinds_are_never_items);

/// Over every fixture: a parameter is always `Local`, and a member kind is
/// never at file level or in a namespace (it is `Member`, or `Local` when a
/// language captures a local function as a method).
fn check_parameters_are_local_and_member_kinds_are_never_items(language: ProgrammingLanguage) {
    let mut checked = 0usize;
    for path in crate::symbols_fixtures::fixtures_for(language, false) {
        let extracted = extract_path(&path);
        for symbol in &extracted.symbols {
            let label = format!("{}: {:?} `{}` line {}", extracted.label, symbol.kind, symbol.name, symbol.range.start_line);
            if symbol.kind == SymbolKind::Parameter {
                assert_eq!(symbol.scope, Local, "{label}");
            }
            if matches!(
                symbol.kind,
                SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Field | SymbolKind::Property | SymbolKind::EnumVariant
            ) {
                assert_ne!(symbol.scope, Item, "{label}");
            }
            checked += 1;
        }
    }
    assert!(checked >= crate::language_cases::corpus_minimums(language).symbols, "{language}: only {checked} fixture symbols checked");
}

/// Spec cases whose constructs no shared fixture holds: `(source, extension,
/// cases)`, each named declaration with its expected scope.
type Snippet = (&'static str, &'static str, &'static [(&'static str, SymbolScope)]);

const SNIPPETS: &[Snippet] = &[
    (
        "const api = { get() { return 1; } };\nfunction f() {\n  const Klass = class { m() {} };\n  const o = { n() {} };\n}\n",
        "js",
        &[
            ("api", Item),
            // Object-literal and class-expression members are members, even
            // inside a function.
            ("get", Member),
            ("f", Item),
            ("Klass", Local),
            ("m", Member),
            ("o", Local),
            ("n", Member),
        ],
    ),
    (
        "object Holder {\n  given intOrdering: Ordering[Int] with {\n    def compare(a: Int, b: Int): Int = 0\n  }\n}\n",
        "scala",
        &[("Holder", Item), ("compare", Member)],
    ),
    (
        "class A {\n  void m() {\n    class LocalType { int v; }\n    Runnable r = new Runnable() { public void run() {} };\n  }\n}\n",
        "java",
        &[
            ("A", Item),
            ("m", Member),
            ("LocalType", Local),
            ("v", Member),
            // An anonymous class's method is its member.
            ("run", Member),
            ("r", Local),
        ],
    ),
    (
        "<?php\nfunction make() {\n  return new class { public function inner() {} };\n}\n",
        "php",
        &[("make", Item), ("inner", Member)],
    ),
    (
        "def outer():\n    class Inner:\n        def m(self):\n            pass\n",
        "py",
        &[("outer", Item), ("Inner", Local), ("m", Member)],
    ),
    (
        "void g() {\n  struct L { int v; };\n}\n",
        "cpp",
        &[("g", Item), ("L", Local), ("v", Member)],
    ),
];

#[test]
fn constructs_outside_the_fixtures_get_their_scope() {
    for (source, extension, cases) in SNIPPETS {
        let extracted = extract_inline(source, extension);
        for (name, scope) in *cases {
            let found: Vec<SymbolScope> = extracted
                .symbols
                .iter()
                .filter(|symbol| symbol.name == *name)
                .map(|symbol| symbol.scope)
                .collect();
            assert_eq!(found, [*scope], ".{extension}: `{name}`");
            let records: Vec<SymbolScope> = extracted
                .records
                .iter()
                .filter(|record| record.identity.name == *name)
                .map(|record| record.scope)
                .collect();
            assert_eq!(records, [*scope], ".{extension}: `{name}` record");
        }
    }
}
