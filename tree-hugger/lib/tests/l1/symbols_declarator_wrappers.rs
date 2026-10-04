//! A C or C++ name keeps its symbol, kind and owner whatever declarators
//! wrap it: an attribute after the name (`int x [[maybe_unused]];`), extra
//! parentheses (`int (f)(int);`), and pointer, reference, array or callback
//! layers around a member function's declarator (`int (*m(int))(long);`).
//! A function-pointer field stays a Field, never a Method.
//!
//! Every row names the symbol it expects and is looked up on both public
//! paths (`symbols()` and the `symbol_index_v2()` records), so a declaration
//! that produces no symbol fails, not only a reported symbol with the wrong
//! facts.

use tree_hugger::{SymbolKind, SymbolKindV2, Visibility, VisibilityLevel};

use crate::symbols_field_lists::{Standalone::Yes, TypeRow, assert_field_lists};
use crate::symbols_fixtures::{Extracted, extract_inline};

use SymbolKind as K;

const C_AND_CPP: &[&str] = &["c", "cpp"];
const CPP_ONLY: &[&str] = &["cpp"];
const C_ONLY: &[&str] = &["c"];

const PUBLIC: Option<Visibility> = Some(Visibility::Public);
const MODULE_PRIVATE: Option<Visibility> = Some(Visibility::ModulePrivate);
const UNKNOWN: Option<Visibility> = None;

/// `(languages, source, name, container, kind, visibility, is_static,
/// is_static_storage)`. A container is a `struct` (kind Type) or, in C++, a
/// `namespace`.
type Row = (
    &'static [&'static str],
    &'static str,
    &'static str,
    Option<&'static str>,
    SymbolKind,
    Option<Visibility>,
    bool,
    bool,
);

const METHOD_OWNER: &str = "struct MethodOwner {\n\
    int *declared_pointer(int value);\n\
    int &declared_reference(int value);\n\
    int **declared_double(int value);\n\
    int (*declared_callback(int value))(long);\n\
    int (*declared_array(int value))[2];\n\
    int **defined_double(int value) { return 0; }\n\
    int (*defined_callback(int value))(long) { return 0; }\n\
    int (*(*defined_chain(int first))(long second))(char third) { return 0; }\n\
    int (*defined_array(int value))[2] { return 0; }\n\
    int attributed_method [[nodiscard]] (int value);\n\
    int (parenthesized_method)(int value);\n\
    static int **static_double(int value);\n\
    template<class T> T **template_double(T value);\n\
#if 1\n\
    int (*conditional_callback(int value))(long);\n\
#endif\n\
    int scalar_declared(int value);\n\
    int scalar_defined(int value) { return value; }\n\
    int *pointer_defined(int value) { return 0; }\n\
    int (*callback_field)(int);\n\
    int (*attributed_callback_field [[maybe_unused]])(int);\n\
};\n";

const ROWS: &[Row] = &[
    // Attributed variables.
    (C_AND_CPP, "int scalar_var [[maybe_unused]] = 1;\n", "scalar_var", None, K::Variable, PUBLIC, false, false),
    (C_AND_CPP, "int *pointer_var [[maybe_unused]];\n", "pointer_var", None, K::Variable, PUBLIC, false, false),
    (C_AND_CPP, "int array_var [[maybe_unused]] [2];\n", "array_var", None, K::Variable, PUBLIC, false, false),
    (C_AND_CPP, "int (*callback_var [[maybe_unused]])(int);\n", "callback_var", None, K::Variable, PUBLIC, false, false),
    (C_AND_CPP, "static int static_var [[maybe_unused]];\n", "static_var", None, K::Variable, MODULE_PRIVATE, false, true),
    (C_AND_CPP, "void host(void) { int local_var [[maybe_unused]] = 0; }\n",
        "local_var", None, K::Variable, UNKNOWN, false, false),
    (C_AND_CPP, "void host(void) { static int local_static [[maybe_unused]] = 0; }\n",
        "local_static", None, K::Variable, UNKNOWN, false, true),
    (CPP_ONLY, "int target;\nint &reference_var [[maybe_unused]] = target;\n",
        "reference_var", None, K::Variable, PUBLIC, false, false),
    (CPP_ONLY, "namespace ns { int namespaced_var [[maybe_unused]] = 1; }\n",
        "namespaced_var", Some("ns"), K::Variable, PUBLIC, false, false),
    (CPP_ONLY, "extern \"C\" { int *linkage_var [[maybe_unused]]; }\n",
        "linkage_var", None, K::Variable, PUBLIC, false, false),
    // Attributed and parenthesized function names.
    (C_AND_CPP, "int attributed_fn [[deprecated]] (int value);\n",
        "attributed_fn", None, K::Function, PUBLIC, false, false),
    (C_AND_CPP, "static int static_fn [[deprecated]] (int value) { return value; }\n",
        "static_fn", None, K::Function, MODULE_PRIVATE, false, true),
    (C_AND_CPP, "int (parenthesized_fn)(int value);\n", "parenthesized_fn", None, K::Function, PUBLIC, false, false),
    (C_AND_CPP, "int ((twice_parenthesized_fn))(int value) { return value; }\n",
        "twice_parenthesized_fn", None, K::Function, PUBLIC, false, false),
    (C_AND_CPP, "int *(parenthesized_pointer_fn)(int value);\n",
        "parenthesized_pointer_fn", None, K::Function, PUBLIC, false, false),
    (CPP_ONLY, "template<class T> T *attributed_template [[nodiscard]] (T value);\n",
        "attributed_template", None, K::Function, PUBLIC, false, false),
    (CPP_ONLY, "struct S { int attributed_out(int value); };\n\
                int S::attributed_out [[deprecated]] (int value) { return value; }\n",
        "attributed_out", None, K::Function, PUBLIC, false, false),
    (CPP_ONLY, "struct S { int parenthesized_out(int value); };\n\
                int (S::parenthesized_out)(int value) { return value; }\n",
        "parenthesized_out", None, K::Function, PUBLIC, false, false),
    // Methods behind return declarators, attributes and parentheses.
    (CPP_ONLY, METHOD_OWNER, "declared_pointer", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "declared_reference", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "declared_double", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "declared_callback", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "declared_array", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "defined_double", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "defined_callback", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "defined_chain", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "defined_array", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "attributed_method", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "parenthesized_method", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "static_double", Some("MethodOwner"), K::Method, PUBLIC, true, false),
    (CPP_ONLY, METHOD_OWNER, "template_double", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "conditional_callback", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    // Typedef names behind a declarator.
    (C_AND_CPP, "typedef int *PointerAlias;\n", "PointerAlias", None, K::TypeAlias, UNKNOWN, false, false),
    (C_AND_CPP, "typedef int AttributedAlias [[maybe_unused]];\n",
        "AttributedAlias", None, K::TypeAlias, UNKNOWN, false, false),
    (C_AND_CPP, "typedef int (*CallbackAlias)(int);\n", "CallbackAlias", None, K::TypeAlias, UNKNOWN, false, false),
    (C_AND_CPP, "typedef int ArrayAlias[3];\n", "ArrayAlias", None, K::TypeAlias, UNKNOWN, false, false),
    (C_AND_CPP, "typedef int FunctionAlias(int);\n", "FunctionAlias", None, K::TypeAlias, UNKNOWN, false, false),
    (CPP_ONLY, "typedef int &ReferenceAlias;\n", "ReferenceAlias", None, K::TypeAlias, UNKNOWN, false, false),
    // Controls.
    (C_ONLY, "[[maybe_unused]] const int leading_attribute = 1;\n",
        "leading_attribute", None, K::Variable, PUBLIC, false, false),
    (CPP_ONLY, "[[maybe_unused]] const int leading_attribute = 1;\n",
        "leading_attribute", None, K::Variable, MODULE_PRIVATE, false, false),
    (C_AND_CPP, "int (*plain_callback_var)(int);\n", "plain_callback_var", None, K::Variable, PUBLIC, false, false),
    (C_AND_CPP, "int plain_fn(int value);\n", "plain_fn", None, K::Function, PUBLIC, false, false),
    (C_AND_CPP, "typedef int PlainAlias;\n", "PlainAlias", None, K::TypeAlias, UNKNOWN, false, false),
    (CPP_ONLY, "struct S { int plain_out(int value); };\nint S::plain_out(int value) { return value; }\n",
        "plain_out", None, K::Function, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "scalar_declared", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "scalar_defined", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "pointer_defined", Some("MethodOwner"), K::Method, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "callback_field", Some("MethodOwner"), K::Field, PUBLIC, false, false),
    (CPP_ONLY, METHOD_OWNER, "attributed_callback_field", Some("MethodOwner"), K::Field, PUBLIC, false, false),
];

fn container_kind(container: Option<&str>) -> Option<SymbolKind> {
    container.map(|name| if name == "ns" { K::Namespace } else { K::Type })
}

/// Every mismatch of `row` on both public paths, empty when the row holds.
fn row_failures(extracted: &Extracted, extension: &str, row: &Row) -> Vec<String> {
    let &(_, _, name, container, kind, visibility, is_static, is_static_storage) = row;
    let label = format!(".{extension} `{name}` in {container:?}");
    let mut failures = Vec::new();

    let symbols: Vec<_> = extracted
        .symbols
        .iter()
        .filter(|symbol| symbol.name == name && symbol.container_name.as_deref() == container)
        .collect();
    match symbols.as_slice() {
        [symbol] => {
            let actual = (
                symbol.kind,
                symbol.container_kind,
                symbol.visibility,
                symbol.is_static,
                symbol.is_static_storage,
            );
            let expected = (kind, container_kind(container), visibility, is_static, is_static_storage);
            if actual != expected {
                failures.push(format!("{label} symbols(): {actual:?}, expected {expected:?}"));
            }
        }
        found => failures.push(format!("{label} symbols(): {} symbols, expected 1", found.len())),
    }

    let records: Vec<_> = extracted
        .records
        .iter()
        .filter(|record| record.identity.name == name && record.identity.module_path.as_deref() == container)
        .collect();
    match records.as_slice() {
        [record] => {
            let actual = (record.kind, record.identity.container_kind, record.visibility.level);
            let expected = (
                SymbolKindV2::from(kind),
                container_kind(container).map(SymbolKindV2::from),
                VisibilityLevel::from(visibility),
            );
            if actual != expected {
                failures.push(format!("{label} symbol_index_v2(): {actual:?}, expected {expected:?}"));
            }
        }
        found => failures.push(format!("{label} symbol_index_v2(): {} records, expected 1", found.len())),
    }
    failures
}

#[test]
fn c_and_cpp_names_behind_declarator_wrappers_keep_their_symbols() {
    let mut failures = Vec::new();
    for row in ROWS {
        for extension in row.0 {
            let extracted = extract_inline(row.1, extension);
            failures.extend(row_failures(&extracted, extension, row));
        }
    }
    assert!(failures.is_empty(), "declarator rows failed: {failures:#?}");
}

/// `AttributeOwner` and `Control`'s complete field lists at member access
/// `$access` (C has none, a C++ `struct` member is Public).
macro_rules! attributed_owner_rows {
    ($access:expr) => {
        &[
            (
                "AttributeOwner",
                &[
                    ("scalar", false, $access, Yes),
                    ("pointer", false, $access, Yes),
                    ("values", false, $access, Yes),
                    ("callback", false, $access, Yes),
                ],
            ),
            ("Control", &[("plain", false, $access, Yes), ("plain_callback", false, $access, Yes)]),
        ]
    };
}

const C_OWNER_ROWS: &[TypeRow] = attributed_owner_rows!(UNKNOWN);
const CPP_OWNER_ROWS: &[TypeRow] = attributed_owner_rows!(PUBLIC);

/// An attributed field is a standalone Field and a type-metadata entry; a
/// field of function-pointer type is a Field next to methods that share its
/// declarator shapes.
#[test]
fn c_and_cpp_attributed_fields_are_listed() {
    let owner = "struct AttributeOwner {\n\
        int scalar [[maybe_unused]];\n\
        int *pointer [[maybe_unused]];\n\
        int values [[maybe_unused]] [2];\n\
        int (*callback [[maybe_unused]])(int);\n\
    };\n\
    struct Control { int plain; int (*plain_callback)(int); };\n";
    for (extension, rows) in [("c", C_OWNER_ROWS), ("cpp", CPP_OWNER_ROWS)] {
        assert_field_lists(&extract_inline(owner, extension), rows);
    }

    let access = extract_inline(
        "class Access {\n\
            int hidden [[maybe_unused]];\n\
         public:\n\
            static int shared [[maybe_unused]];\n\
         #if 1\n\
            int guarded [[maybe_unused]];\n\
         #endif\n\
         };\n\
         template<class T> struct Templated { T *member [[maybe_unused]]; };\n",
        "cpp",
    );
    assert_field_lists(
        &access,
        &[
            (
                "Access",
                &[
                    ("hidden", false, Some(Visibility::Private), Yes),
                    ("shared", true, PUBLIC, Yes),
                    ("guarded", false, PUBLIC, Yes),
                ],
            ),
            ("Templated", &[("member", false, PUBLIC, Yes)]),
        ],
    );

    let methods = extract_inline(METHOD_OWNER, "cpp");
    assert_field_lists(
        &methods,
        &[(
            "MethodOwner",
            &[
                ("callback_field", false, PUBLIC, Yes),
                ("attributed_callback_field", false, PUBLIC, Yes),
            ],
        )],
    );
}

/// An attributed enumerator keeps the kind of a plain one.
#[test]
fn c_and_cpp_attributed_enumerators_match_plain_ones() {
    for extension in C_AND_CPP {
        let extracted = extract_inline("enum Color { RED [[deprecated]], GREEN };\n", extension);
        let kind_of = |name: &str| {
            let found: Vec<_> = extracted.symbols.iter().filter(|symbol| symbol.name == name).collect();
            assert_eq!(found.len(), 1, ".{extension}: one `{name}`: {found:?}");
            found[0].kind
        };
        assert_eq!(kind_of("RED"), kind_of("GREEN"), ".{extension}: enumerator kinds");
    }
}
