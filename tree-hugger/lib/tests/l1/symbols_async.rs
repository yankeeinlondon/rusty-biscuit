//! The `async` part of the flags matrix of
//! `2026-10-03-symbol-filters-and-reports`: the `Modifier::Async` entry (and
//! `SymbolInfo::is_async`) is set exactly on each fixture's async
//! declarations, and never in a language without async syntax.

use std::collections::BTreeSet;

use tree_hugger::{Modifier, ProgrammingLanguage, SymbolRecord};

use crate::symbols_fixtures::{Extracted, extract_inline, extract_primary};

use ProgrammingLanguage as L;

/// `(name, container)` of an async declaration.
pub type AsyncCase = (&'static str, Option<&'static str>);

pub struct AsyncRow {
    pub language: ProgrammingLanguage,
    /// Every async declaration in the primary fixture, or why the language
    /// has none.
    pub declarations: Result<&'static [AsyncCase], &'static str>,
}

const NO_ASYNC: &str = "the language has no async declarations";

pub const ASYNC_MATRIX: &[AsyncRow] = &[
    AsyncRow {
        language: L::Rust,
        declarations: Ok(&[("async_function", None), ("load", Some("Record"))]),
    },
    AsyncRow {
        language: L::TypeScript,
        declarations: Ok(&[("asyncFunction", None), ("asyncArrow", None), ("area", Some("FirstShape"))]),
    },
    AsyncRow {
        language: L::JavaScript,
        declarations: Ok(&[("asyncFunction", None), ("asyncArrow", None), ("area", Some("FirstShape"))]),
    },
    AsyncRow { language: L::Go, declarations: Err(NO_ASYNC) },
    AsyncRow {
        language: L::Python,
        declarations: Ok(&[("async_function", None), ("area", Some("FirstShape"))]),
    },
    AsyncRow { language: L::Java, declarations: Err(NO_ASYNC) },
    AsyncRow {
        language: L::CSharp,
        declarations: Ok(&[("RunAsync", Some("AsyncWork"))]),
    },
    AsyncRow { language: L::C, declarations: Err(NO_ASYNC) },
    AsyncRow {
        language: L::Cpp,
        declarations: Err("coroutines are marked in the body (`co_await`), not the declaration"),
    },
    AsyncRow {
        language: L::Swift,
        declarations: Ok(&[("asyncFunction", None), ("area", Some("FirstShape"))]),
    },
    AsyncRow { language: L::Scala, declarations: Err(NO_ASYNC) },
    AsyncRow { language: L::Php, declarations: Err(NO_ASYNC) },
    AsyncRow { language: L::Perl, declarations: Err(NO_ASYNC) },
    AsyncRow { language: L::Lua, declarations: Err(NO_ASYNC) },
    AsyncRow { language: L::Bash, declarations: Err(NO_ASYNC) },
    AsyncRow { language: L::Zsh, declarations: Err(NO_ASYNC) },
];

/// The languages with an async row, for the coverage guard.
pub fn async_matrix_languages() -> Vec<ProgrammingLanguage> {
    ASYNC_MATRIX.iter().map(|row| row.language).collect()
}

fn has_async(record: &SymbolRecord) -> bool {
    record.modifiers.iter().any(|modifier| matches!(modifier, Modifier::Async))
}

/// `(name, container)` pairs.
type Declarations = BTreeSet<(String, Option<String>)>;

/// The `(name, container)` of every async declaration, on both paths.
fn async_declarations(extracted: &Extracted) -> (Declarations, Declarations) {
    let symbols = extracted
        .symbols
        .iter()
        .filter(|symbol| symbol.is_async)
        .map(|symbol| (symbol.name.clone(), symbol.container_name.clone()))
        .collect();
    let records = extracted
        .records
        .iter()
        .filter(|record| has_async(record))
        .map(|record| (record.identity.name.clone(), record.identity.container_name.clone()))
        .collect();
    (symbols, records)
}

#[test]
fn async_is_set_exactly_on_each_fixtures_async_declarations() {
    for row in ASYNC_MATRIX {
        let extracted = extract_primary(row.language);
        let expected: Declarations = match row.declarations {
            Ok(cases) => {
                assert!(!cases.is_empty(), "{}: empty async row", row.language.name());
                cases
                    .iter()
                    .map(|(name, container)| (name.to_string(), container.map(str::to_string)))
                    .collect()
            }
            Err(_) => BTreeSet::new(),
        };
        let (symbols, records) = async_declarations(&extracted);
        assert_eq!(symbols, expected, "{}: async in symbols()", extracted.label);
        assert_eq!(records, expected, "{}: async in symbol_index_v2()", extracted.label);
    }
}

/// Async forms the fixtures do not hold, each with the declarations that are
/// and are not async.
#[test]
fn async_forms_outside_the_fixtures() {
    let cases: &[(&str, &str, &[&str], &[&str])] = &[
        (
            "trait T { async fn required(&self); }\nfn sync_with_block() { let _ = async {}; }\n",
            "rs",
            &["required"],
            &["sync_with_block"],
        ),
        (
            "const expression = async function () {};\nclass C { handler = async () => {}; plain = () => {}; }\nconst o = { async method() {} };\n",
            "js",
            &["expression", "handler", "method"],
            &["plain", "o", "C"],
        ),
        (
            "class C:\n    @staticmethod\n    async def decorated():\n        pass\n\n    def plain(self):\n        pass\n",
            "py",
            &["decorated"],
            &["plain"],
        ),
        (
            "class C { void Outer() { async Task Local() { } } }\n",
            "cs",
            &["Local"],
            &["Outer"],
        ),
        (
            "protocol P { func required() async }\nclass C { init() async {} func plain() {} }\n",
            "swift",
            &["required", "init"],
            &["plain"],
        ),
    ];
    for (source, extension, async_names, sync_names) in cases {
        let extracted = extract_inline(source, extension);
        for (names, expected) in [(*async_names, true), (*sync_names, false)] {
            for name in names {
                let records: Vec<&SymbolRecord> =
                    extracted.records.iter().filter(|record| record.identity.name == *name).collect();
                assert_eq!(records.len(), 1, ".{extension}: one `{name}` record");
                assert_eq!(has_async(records[0]), expected, ".{extension}: `{name}` async");
            }
        }
    }
}
