//! C and C++ signature parameters read from the named function's own
//! declarator: declarator wrappers around the function (returned pointers,
//! references, arrays and callbacks) neither erase its parameters nor
//! substitute a returned function's, and each parameter's name is found under
//! any declarator chain. Parameters bound by a pattern rather than an
//! identifier keep the pattern and its declared type. A parameter's name,
//! type and default come from their own grammar fields, so a comment beside
//! a default's `=` neither erases the default nor becomes it.

use tree_hugger::ProgrammingLanguage;
use tree_hugger::{ParameterInfo, SymbolInfo, SymbolKindData};

use crate::symbols_fixtures::{Extracted, extract_inline};

const C_AND_CPP: &[&str] = &["c", "cpp"];
const CPP_ONLY: &[&str] = &["cpp"];
const C_ONLY: &[&str] = &["c"];

/// `(languages, source, function name, expected parameter names)`. Every
/// function-like symbol of that name must carry exactly these parameters.
type SignatureRow = (&'static [&'static str], &'static str, &'static str, &'static [&'static str]);

const SIGNATURE_MATRIX: &[SignatureRow] = &[
    // Declarators around the function itself.
    (C_AND_CPP, "int *pointer_return(int value) { return 0; }\n", "pointer_return", &["value"]),
    (C_AND_CPP, "int **double_pointer_return(int value) { return 0; }\n", "double_pointer_return", &["value"]),
    (C_AND_CPP, "int (*callback_return(int actual))(long returned) { return 0; }\n",
        "callback_return", &["actual"]),
    (C_AND_CPP, "int (*unnamed_callback_return(int actual))(long) { return 0; }\n",
        "unnamed_callback_return", &["actual"]),
    (C_AND_CPP, "int (*(*chained_return(int first))(long second))(char third) { return 0; }\n",
        "chained_return", &["first"]),
    (C_AND_CPP, "int (*array_return(int actual))[2] { return 0; }\n", "array_return", &["actual"]),
    (CPP_ONLY, "int (&reference_return(int actual))(long) { return 0; }\n", "reference_return", &["actual"]),
    // Declaration and wrapper sites.
    (C_AND_CPP, "int *pointer_prototype(int value);\n", "pointer_prototype", &["value"]),
    (C_AND_CPP, "static int *static_pointer(int value) { return 0; }\n", "static_pointer", &["value"]),
    (C_AND_CPP, "#if 1\nint *conditional_pointer(int value) { return 0; }\n#endif\n",
        "conditional_pointer", &["value"]),
    (CPP_ONLY, "template<class T> T *template_pointer(T value) { return 0; }\n", "template_pointer", &["value"]),
    (CPP_ONLY, "extern \"C\" { int *linkage_pointer(int value) { return 0; } }\n", "linkage_pointer", &["value"]),
    (CPP_ONLY, "extern \"C\" int *linkage_single(int value) { return 0; }\n", "linkage_single", &["value"]),
    // C++ members: in-class definitions and an out-of-class qualified one.
    (CPP_ONLY, "struct Owner { int *member_pointer(int value) { return 0; } };\n", "member_pointer", &["value"]),
    (CPP_ONLY, "struct Owner { int field; int &member_reference(int value) { return field; } };\n",
        "member_reference", &["value"]),
    (CPP_ONLY, "struct Owner { template<class T> static T *static_template(T value) { return 0; } };\n",
        "static_template", &["value"]),
    (CPP_ONLY, "struct Owner { int *qualified_pointer(int value); };\n\
                int *Owner::qualified_pointer(int value) { return 0; }\n",
        "qualified_pointer", &["value"]),
    // Attributed and parenthesized function names, and member functions
    // behind return declarators.
    (C_AND_CPP, "int attributed_name [[deprecated]] (int value);\n", "attributed_name", &["value"]),
    (C_AND_CPP, "int (parenthesized_name)(int value) { return value; }\n", "parenthesized_name", &["value"]),
    (CPP_ONLY, "struct Owner { int attributed_member [[nodiscard]] (int value); };\n",
        "attributed_member", &["value"]),
    (CPP_ONLY, "struct Owner { int (*member_callback(int actual))(long returned); };\n",
        "member_callback", &["actual"]),
    (CPP_ONLY, "struct Owner { int (*(*member_chain(int first))(long second))(char third) { return 0; } };\n",
        "member_chain", &["first"]),
    // Declarators around a parameter's name.
    (C_AND_CPP, "int double_pointer_param(char **argv) { return 0; }\n", "double_pointer_param", &["argv"]),
    (C_AND_CPP, "int array_param(int values[2]) { return 0; }\n", "array_param", &["values"]),
    (C_AND_CPP, "int callback_param(int (*callback)(long ignored)) { return 0; }\n",
        "callback_param", &["callback"]),
    // `int (value)` would be an unnamed parameter of function type: the
    // grammar applies C's typedef-name reading, so parentheses need a layer.
    (C_AND_CPP, "int parenthesized_param(int (*value)) { return 0; }\n", "parenthesized_param", &["value"]),
    (C_AND_CPP, "int nested_param(int (*(*value)[2])(long ignored)) { return 0; }\n", "nested_param", &["value"]),
    (C_AND_CPP, "int attributed_param(int value [[maybe_unused]]) { return 0; }\n",
        "attributed_param", &["value"]),
    // C++ parameter node kinds.
    (CPP_ONLY, "int defaulted_param(int defaulted = 1) { return 0; }\n", "defaulted_param", &["defaulted"]),
    (CPP_ONLY, "template<class... Ts> int pack_param(Ts... values) { return 0; }\n", "pack_param", &["values"]),
    (CPP_ONLY, "template<class... Ts> int forwarding_pack(Ts&&... values) { return 0; }\n",
        "forwarding_pack", &["values"]),
    (CPP_ONLY, "int cpp_variadic(int first, ...) { return 0; }\n", "cpp_variadic", &["first", "..."]),
    // Controls.
    (C_AND_CPP, "int scalar_control(int value) { return value; }\n", "scalar_control", &["value"]),
    (C_AND_CPP, "int pointer_param(char *text) { return 0; }\n", "pointer_param", &["text"]),
    (CPP_ONLY, "int reference_params(int &left, int &&right) { return 0; }\n",
        "reference_params", &["left", "right"]),
    (C_ONLY, "int c_variadic(int first, ...) { return 0; }\n", "c_variadic", &["first", "..."]),
];

fn names(parameters: &[ParameterInfo]) -> Vec<&str> {
    parameters.iter().map(|parameter| parameter.name.as_str()).collect()
}

/// Every function-like symbol named `name`, on both public paths: the v1
/// symbols and the v2 records converted back to `SymbolInfo`.
fn functions_named(extracted: &Extracted, name: &str) -> Vec<(&'static str, SymbolInfo)> {
    let mut found: Vec<(&'static str, SymbolInfo)> = extracted
        .symbols
        .iter()
        .filter(|symbol| symbol.name == name && symbol.kind.is_function())
        .map(|symbol| ("symbols()", symbol.clone()))
        .collect();
    found.extend(
        extracted
            .records
            .iter()
            .filter(|record| record.identity.name == name)
            .filter_map(|record| SymbolInfo::try_from(record.clone()).ok())
            .filter(|symbol| symbol.kind.is_function())
            .map(|symbol| ("symbol_index_v2()", symbol)),
    );
    found
}

#[test]
fn c_and_cpp_signatures_read_the_named_functions_own_parameters() {
    let mut failures = Vec::new();
    for (languages, source, name, expected) in SIGNATURE_MATRIX {
        for extension in *languages {
            let extracted = extract_inline(source, extension);
            let functions = functions_named(&extracted, name);
            if functions.is_empty() {
                failures.push(format!(".{extension} `{name}`: no function symbol"));
            }
            for (path, symbol) in functions {
                let actual = symbol.signature.as_ref().map(|signature| names(&signature.parameters));
                if actual.as_deref() != Some(*expected) {
                    failures.push(format!(".{extension} `{name}` ({path}): {actual:?}, expected {expected:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "signature rows failed: {failures:#?}");
}

/// The parameter kinds beyond a name: a default value, a pack and a
/// variadic tail are reported as such.
#[test]
fn cpp_default_pack_and_variadic_parameters_keep_their_details() {
    let extracted = extract_inline(
        "int defaulted(int value = 1 + 2) { return 0; }\n\
         template<class... Ts> int packed(int first, Ts... rest) { return 0; }\n\
         int tail(int first, ...) { return 0; }\n",
        "cpp",
    );
    let parameters = |name: &str| {
        extracted
            .symbol(name, None)
            .signature
            .clone()
            .unwrap_or_else(|| panic!("`{name}`: no signature"))
            .parameters
    };

    let defaulted = parameters("defaulted");
    assert_eq!(defaulted[0].default_value.as_deref(), Some("1 + 2"), "`defaulted`: default value");
    assert!(!defaulted[0].is_variadic, "`defaulted`: not variadic");

    let packed = parameters("packed");
    assert_eq!(names(&packed), ["first", "rest"], "`packed`: names");
    assert!(!packed[0].is_variadic && packed[1].is_variadic, "`packed`: only the pack is variadic");

    let tail = parameters("tail");
    assert_eq!(names(&tail), ["first", "..."], "`tail`: names");
    assert!(tail[1].is_variadic, "`tail`: the variadic tail is variadic");
}

/// `(extension, source, function name, expected (name, type text, variadic))`.
type PatternRow = (&'static str, &'static str, &'static str, &'static [(&'static str, Option<&'static str>, bool)]);

/// Parameters whose binding is not a plain identifier keep their pattern as
/// written for a name and their declared type: destructuring, wildcards,
/// rest patterns, and every list of a curried Scala `def`.
const PATTERN_MATRIX: &[PatternRow] = &[
    ("rs", "pub fn patterned((a, b): (u32, u32), _: String) {}\n", "patterned",
        &[("(a, b)", Some("(u32, u32)"), false), ("_", Some("String"), false)]),
    ("rs", "fn more(&r: &u8, S { x }: S, mut m: u8) {}\n", "more",
        &[("&r", Some("&u8"), false), ("S { x }", Some("S"), false), ("m", Some("u8"), false)]),
    // A typed `self` pattern is the receiver.
    ("rs", "struct T;\nimpl T { fn boxed(self: Box<Self>, n: u8) {} }\n", "boxed",
        &[("self", Some("Box<Self>"), false), ("n", Some("u8"), false)]),
    ("ts", "export function patterned({x}: {x:number}, [a,b]: number[]) {}\n", "patterned",
        &[("{x}", Some("{x:number}"), false), ("[a,b]", Some("number[]"), false)]),
    ("ts", "function restPattern(d = 1, ...[r, s]: number[]) {}\n", "restPattern",
        &[("d", None, false), ("[r, s]", Some("number[]"), true)]),
    // A TypeScript `this` parameter types the receiver; it is no argument.
    ("ts", "function withThis(this: Window, a: number) {}\n", "withThis", &[("a", Some("number"), false)]),
    ("js", "function patterned({x}, [a,b], {y} = {}, ...[r, s]) {}\n", "patterned",
        &[("{x}", None, false), ("[a,b]", None, false), ("{y}", None, false), ("[r, s]", None, true)]),
    ("scala", "object O {\n  def patterned(a: Int)(b: String): Int = a\n}\n", "patterned",
        &[("a", Some("Int"), false), ("b", Some("String"), false)]),
    ("scala", "object O {\n  def contextual(a: Int)(using c: Ctx): Int = a\n}\n", "contextual",
        &[("a", Some("Int"), false), ("c", Some("Ctx"), false)]),
];

#[test]
fn patterned_parameters_keep_their_pattern_and_type_text() {
    let mut failures = Vec::new();
    for (extension, source, name, expected) in PATTERN_MATRIX {
        let extracted = extract_inline(source, extension);
        let functions = functions_named(&extracted, name);
        if functions.is_empty() {
            failures.push(format!(".{extension} `{name}`: no function symbol"));
        }
        for (path, symbol) in functions {
            let actual: Option<Vec<(String, Option<String>, bool)>> = symbol.signature.as_ref().map(|signature| {
                signature
                    .parameters
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.type_annotation.clone(), parameter.is_variadic))
                    .collect()
            });
            let wanted: Vec<(String, Option<String>, bool)> = expected
                .iter()
                .map(|(name, type_text, variadic)| (name.to_string(), type_text.map(str::to_string), *variadic))
                .collect();
            if actual.as_ref() != Some(&wanted) {
                failures.push(format!(".{extension} `{name}` ({path}): {actual:?}, expected {wanted:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "pattern rows failed: {failures:#?}");
}

/// `(extension, source, function name, expected (name, type text, default))`.
type ParameterFieldRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, Option<&'static str>, Option<&'static str>)],
);

/// A parameter's type and default value are read from their own fields,
/// never mistaken for its name: a C# named type (`W x`) and a bare default
/// (`int y = external`) are identifiers like the name, as is a Python
/// default (`x=external`).
const PARAMETER_FIELD_MATRIX: &[ParameterFieldRow] = &[
    (
        "cs",
        "class W {}\nclass G {\n    void g(W x, int y = external, ref W r, params W[] rest) {}\n}\n",
        "g",
        &[("x", Some("W"), None), ("y", Some("int"), Some("external")), ("r", Some("W"), None), ("rest", Some("W[]"), None)],
    ),
    ("cs", "class W {}\nclass G {\n    G(W x) {}\n}\n", "G", &[("x", Some("W"), None)]),
    (
        "py",
        "def external(): pass\nclass W: pass\ndef g(x=external, y: W=external): pass\n",
        "g",
        &[("x", None, Some("external")), ("y", Some("W"), Some("external"))],
    ),
];

/// A parameter as `(name, type text, default)`.
type ParameterFields = (String, Option<String>, Option<String>);

#[test]
fn parameter_names_types_and_defaults_come_from_their_own_fields() {
    let mut failures = Vec::new();
    for (extension, source, name, expected) in PARAMETER_FIELD_MATRIX {
        let extracted = extract_inline(source, extension);
        let functions = functions_named(&extracted, name);
        if functions.is_empty() {
            failures.push(format!(".{extension} `{name}`: no function symbol"));
        }
        for (path, symbol) in functions {
            let actual: Option<Vec<ParameterFields>> = symbol.signature.as_ref().map(|signature| {
                signature
                    .parameters
                    .iter()
                    .map(|parameter| {
                        (parameter.name.clone(), parameter.type_annotation.clone(), parameter.default_value.clone())
                    })
                    .collect()
            });
            let wanted: Vec<ParameterFields> = expected
                .iter()
                .map(|(name, type_text, default)| {
                    (name.to_string(), type_text.map(str::to_string), default.map(str::to_string))
                })
                .collect();
            if actual.as_ref() != Some(&wanted) {
                failures.push(format!(".{extension} `{name}` ({path}): {actual:?}, expected {wanted:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "parameter field rows failed: {failures:#?}");
}

/// Where the comment-position matrix writes comments around a default's
/// `=`: nowhere (the control), before it, after it, or on both sides.
const COMMENT_POSITIONS: &[(&str, bool, bool)] =
    &[("control", false, false), ("before", true, false), ("after", false, true), ("both", true, true)];

/// One default-capable language of the comment-position matrix.
struct DefaultLanguage {
    extension: &'static str,
    /// A parameter written as `name`, its comment-or-empty `before` and
    /// `after` texts around `=`, and its default.
    parameter: fn(name: &str, before: &str, after: &str, default: &str) -> String,
    /// The source declaring every callable in `callables` with `parameters`.
    source: fn(parameters: &str) -> String,
    /// Every callable the source declares, each with the parameters.
    callables: &'static [&'static str],
    /// The type text every parameter declares.
    type_text: Option<&'static str>,
    /// The comment written before and after `=` when the position asks.
    comments: (&'static str, &'static str),
}

const BLOCK_COMMENTS: (&str, &str) = (" /* before */ ", " /* after */ ");

/// Every language whose parameters take a default, with each callable form
/// the language's readers serve: a C# method, constructor and local
/// function, a PHP function, `__construct` and method, a Scala object and
/// class method.
const DEFAULT_LANGUAGES: &[DefaultLanguage] = &[
    DefaultLanguage {
        extension: "cs",
        parameter: |name, before, after, default| format!("int {name}{before}={after}{default}"),
        source: |parameters| {
            format!(
                "class K {{\n    void g({parameters}) {{}}\n    K({parameters}) {{}}\n    \
                 void Outer() {{ void Local({parameters}) {{}} }}\n}}\n"
            )
        },
        callables: &["g", "K", "Local"],
        type_text: Some("int"),
        comments: BLOCK_COMMENTS,
    },
    DefaultLanguage {
        extension: "php",
        parameter: |name, before, after, default| format!("int ${name}{before}={after}{default}"),
        source: |parameters| {
            format!(
                "<?php\nfunction g({parameters}) {{}}\nclass K {{\n    \
                 function __construct({parameters}) {{}}\n    function method({parameters}) {{}}\n}}\n"
            )
        },
        callables: &["g", "__construct", "method"],
        type_text: Some("int"),
        comments: BLOCK_COMMENTS,
    },
    DefaultLanguage {
        extension: "scala",
        parameter: |name, before, after, default| format!("{name}: Int{before}={after}{default}"),
        source: |parameters| {
            format!(
                "object O {{\n  def g({parameters}): Int = 1\n}}\n\
                 class K {{\n  def method({parameters}): Int = 1\n}}\n"
            )
        },
        callables: &["g", "method"],
        type_text: Some("Int"),
        comments: BLOCK_COMMENTS,
    },
    DefaultLanguage {
        extension: "swift",
        parameter: |name, before, after, default| format!("{name}: Int{before}={after}{default}"),
        source: |parameters| {
            format!("func g({parameters}) {{}}\nclass K {{\n    init({parameters}) {{}}\n    func method({parameters}) {{}}\n}}\n")
        },
        callables: &["g", "init", "method"],
        type_text: Some("Int"),
        comments: BLOCK_COMMENTS,
    },
    // Python comments run to the end of the line; the parameter list's
    // parentheses join the lines around them.
    DefaultLanguage {
        extension: "py",
        parameter: |name, before, after, default| format!("\n    {name}: int{before}={after}{default}"),
        source: |parameters| format!("def g({parameters},\n):\n    pass\n"),
        callables: &["g"],
        type_text: Some("int"),
        comments: ("  # before\n    ", "  # after\n    "),
    },
    DefaultLanguage {
        extension: "js",
        parameter: |name, before, after, default| format!("{name}{before}={after}{default}"),
        source: |parameters| format!("function g({parameters}) {{}}\nclass K {{\n    method({parameters}) {{}}\n}}\n"),
        callables: &["g", "method"],
        type_text: None,
        comments: BLOCK_COMMENTS,
    },
    DefaultLanguage {
        extension: "ts",
        parameter: |name, before, after, default| format!("{name}: number{before}={after}{default}"),
        source: |parameters| format!("function g({parameters}) {{}}\nclass K {{\n    method({parameters}) {{}}\n}}\n"),
        callables: &["g", "method"],
        type_text: Some("number"),
        comments: BLOCK_COMMENTS,
    },
    DefaultLanguage {
        extension: "cpp",
        parameter: |name, before, after, default| format!("int {name}{before}={after}{default}"),
        source: |parameters| {
            format!("void g({parameters}) {{}}\nstruct K {{\n    K({parameters}) {{}}\n    void method({parameters}) {{}}\n}};\n")
        },
        callables: &["g", "K", "method"],
        type_text: Some("int"),
        comments: BLOCK_COMMENTS,
    },
];

/// A record's parameter as `(name, type text, default, optional, variadic)`.
type RecordedParameter = (String, Option<String>, Option<String>, bool, bool);

/// The parameters `x = 1, y = 2, z = 3`, each with a comment before and/or
/// after its `=` as `before` and `after` ask.
fn commented_parameters(language: &DefaultLanguage, before: bool, after: bool) -> String {
    let (before_text, after_text) = language.comments;
    [("x", "1"), ("y", "2"), ("z", "3")]
        .iter()
        .map(|(name, default)| {
            (language.parameter)(
                name,
                if before { before_text } else { "" },
                if after { after_text } else { "" },
                default,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `(kind, in-file name)` of every record, in order: comments add no
/// declaration and remove none.
fn record_shape(extracted: &Extracted) -> Vec<String> {
    extracted
        .records
        .iter()
        .map(|record| format!("{:?} {}", record.kind, crate::symbols_fixtures::in_file_name(record)))
        .collect()
}

crate::language_cases::language_tests!(comments_around_a_default_never_erase_or_supply_it => check_comments_around_a_default_never_erase_or_supply_it);

/// A comment before, after or on both sides of a default's `=` changes
/// nothing a parameter reports: in every default-capable language and every
/// callable form, the record keeps the names, types, defaults `1`/`2`/`3`,
/// optionality, arity and declaration list of the uncommented control, and
/// the v1 signature the same defaults.
fn check_comments_around_a_default_never_erase_or_supply_it(language: ProgrammingLanguage) {
    let mut failures = Vec::new();
    for language in DEFAULT_LANGUAGES.iter().filter(|row| ProgrammingLanguage::from_extension(row.extension) == Some(language)) {
        let extension = language.extension;
        let control = extract_inline(&(language.source)(&commented_parameters(language, false, false)), extension);
        for (position, before, after) in COMMENT_POSITIONS {
            let source = (language.source)(&commented_parameters(language, *before, *after));
            let extracted = extract_inline(&source, extension);
            let label = |name: &str| format!(".{extension} {position} `{name}`");
            if record_shape(&extracted) != record_shape(&control) {
                failures.push(format!(
                    ".{extension} {position}: records {:?}, control {:?}",
                    record_shape(&extracted),
                    record_shape(&control)
                ));
            }
            for name in language.callables {
                // A constructor shares its name with its class.
                let callables: Vec<_> = extracted
                    .records
                    .iter()
                    .filter(|record| record.identity.name == *name)
                    .filter_map(|record| match &record.kind_data {
                        SymbolKindData::Function(function) => Some(function),
                        _ => None,
                    })
                    .collect();
                let [function] = callables.as_slice() else {
                    failures.push(format!("{}: {} callable records", label(name), callables.len()));
                    continue;
                };
                let actual: Vec<RecordedParameter> = function
                    .parameters
                    .iter()
                    .map(|parameter| {
                        (
                            parameter.name.clone(),
                            parameter.type_text.clone(),
                            parameter.default_value.clone(),
                            parameter.is_optional,
                            parameter.is_variadic,
                        )
                    })
                    .collect();
                let expected: Vec<RecordedParameter> = [("x", "1"), ("y", "2"), ("z", "3")]
                    .iter()
                    .map(|(name, default)| {
                        (name.to_string(), language.type_text.map(str::to_string), Some(default.to_string()), true, false)
                    })
                    .collect();
                if actual != expected || function.arity != 3 || function.is_variadic {
                    failures.push(format!(
                        "{}: {actual:?} (arity {}, variadic {}), expected {expected:?}",
                        label(name),
                        function.arity,
                        function.is_variadic
                    ));
                }
                for (path, symbol) in functions_named(&extracted, name) {
                    let defaults: Option<Vec<Option<&str>>> = symbol.signature.as_ref().map(|signature| {
                        signature.parameters.iter().map(|parameter| parameter.default_value.as_deref()).collect()
                    });
                    if defaults != Some(vec![Some("1"), Some("2"), Some("3")]) {
                        failures.push(format!("{} ({path}): defaults {defaults:?}", label(name)));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "comment-position rows failed: {failures:#?}");
}

/// The siblings of the default readers that took the node beside a token:
/// a C# `params` array keeps its type and name with comments between them,
/// and a Swift alias's target is never the comment after its `=`.
#[test]
fn comments_beside_params_arrays_and_alias_targets_are_skipped() {
    for source in [
        "class K {\n    void p(params int[] xs) {}\n}\n",
        "class K {\n    void p(params /* c */ int[] /* d */ xs) {}\n}\n",
    ] {
        let extracted = extract_inline(source, "cs");
        let signature = extracted.symbol("p", Some("K")).signature.clone().expect("`p`: signature");
        let parameters: Vec<(&str, Option<&str>, bool)> = signature
            .parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.type_annotation.as_deref(), parameter.is_variadic))
            .collect();
        assert_eq!(parameters, [("xs", Some("int[]"), true)], "{source:?}");
    }

    for source in ["typealias A = Int\n", "typealias A = /* c */ Int\n", "typealias A /* b */ = /* c */ Int\n"] {
        let extracted = extract_inline(source, "swift");
        let record = extracted.records.iter().find(|record| record.identity.name == "A").expect("alias record");
        let SymbolKindData::TypeAlias(alias) = &record.kind_data else {
            panic!("{source:?}: {:?} is not alias data", record.kind_data);
        };
        assert_eq!(alias.aliased_type_text.as_deref(), Some("Int"), "{source:?}");
    }
}
