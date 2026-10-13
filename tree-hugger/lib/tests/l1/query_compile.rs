//! Query compilation checks for every supported language and query kind.

use tree_hugger::ProgrammingLanguage;
use tree_hugger::queries::{GrammarRef, QueryKind, query_for};
use tree_sitter::Query;

/// Every supported language must have queries that compile for every query
/// kind. A broken `references.scm` previously slipped through because only
/// `lint.scm` and `comments.scm` were covered, which let semantic analysis
/// silently fail for PHP, Perl, and Zsh.
fn assert_query_compiles(language: ProgrammingLanguage, kind: QueryKind) {
    let grammar = language.tree_sitter_language();
    let grammar_ref = GrammarRef {
        language,
        grammar: &grammar,
        id: language.query_name(),
    };
    assert!(query_for(grammar_ref, kind).is_ok(), "{language} {kind} query should compile");
}
fn test_query_compiles(language: ProgrammingLanguage, query_text: &str) {
    let ts_language = language.tree_sitter_language();
    let result = Query::new(&ts_language, query_text);

    match &result {
        Ok(query) => {
            println!(
                "{}: {} patterns, captures: {:?}",
                language,
                query.pattern_count(),
                query.capture_names()
            );
        }
        Err(e) => {
            panic!(
                "{} lint query failed to compile: {:?}\n\nQuery:\n{}",
                language, e, query_text
            );
        }
    }

    assert!(result.is_ok(), "{} lint query should compile", language);
}

#[test]
fn test_rust_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Rust,
        include_str!("../../queries/rust/lint.scm"),
    );
}

#[test]
fn test_javascript_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::JavaScript,
        include_str!("../../queries/javascript/lint.scm"),
    );
}

#[test]
fn test_typescript_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::TypeScript,
        include_str!("../../queries/typescript/lint.scm"),
    );
}

#[test]
fn test_go_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Go,
        include_str!("../../queries/go/lint.scm"),
    );
}

#[test]
fn test_python_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Python,
        include_str!("../../queries/python/lint.scm"),
    );
}

#[test]
fn test_java_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Java,
        include_str!("../../queries/java/lint.scm"),
    );
}

#[test]
fn test_php_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Php,
        include_str!("../../queries/php/lint.scm"),
    );
}

#[test]
fn test_perl_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Perl,
        include_str!("../../queries/perl/lint.scm"),
    );
}

#[test]
fn test_bash_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Bash,
        include_str!("../../queries/bash/lint.scm"),
    );
}

#[test]
fn test_zsh_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Zsh,
        include_str!("../../queries/zsh/lint.scm"),
    );
}

#[test]
fn test_c_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::C,
        include_str!("../../queries/c/lint.scm"),
    );
}

#[test]
fn test_cpp_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Cpp,
        include_str!("../../queries/cpp/lint.scm"),
    );
}

#[test]
fn test_csharp_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::CSharp,
        include_str!("../../queries/c_sharp/lint.scm"),
    );
}

#[test]
fn test_swift_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Swift,
        include_str!("../../queries/swift/lint.scm"),
    );
}

#[test]
fn test_scala_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Scala,
        include_str!("../../queries/scala/lint.scm"),
    );
}

#[test]
fn test_lua_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Lua,
        include_str!("../../queries/lua/lint.scm"),
    );
}

// =============================================================================
// Comments Query Compilation Tests
// =============================================================================

#[test]
fn test_rust_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Rust,
        include_str!("../../queries/rust/comments.scm"),
    );
}

#[test]
fn test_javascript_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::JavaScript,
        include_str!("../../queries/javascript/comments.scm"),
    );
}

#[test]
fn test_typescript_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::TypeScript,
        include_str!("../../queries/typescript/comments.scm"),
    );
}

#[test]
fn test_go_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Go,
        include_str!("../../queries/go/comments.scm"),
    );
}

#[test]
fn test_python_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Python,
        include_str!("../../queries/python/comments.scm"),
    );
}

#[test]
fn test_java_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Java,
        include_str!("../../queries/java/comments.scm"),
    );
}

#[test]
fn test_php_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Php,
        include_str!("../../queries/php/comments.scm"),
    );
}

#[test]
fn test_perl_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Perl,
        include_str!("../../queries/perl/comments.scm"),
    );
}

#[test]
fn test_bash_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Bash,
        include_str!("../../queries/bash/comments.scm"),
    );
}

#[test]
fn test_zsh_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Zsh,
        include_str!("../../queries/zsh/comments.scm"),
    );
}

#[test]
fn test_c_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::C,
        include_str!("../../queries/c/comments.scm"),
    );
}

#[test]
fn test_cpp_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Cpp,
        include_str!("../../queries/cpp/comments.scm"),
    );
}

#[test]
fn test_csharp_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::CSharp,
        include_str!("../../queries/c_sharp/comments.scm"),
    );
}

#[test]
fn test_swift_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Swift,
        include_str!("../../queries/swift/comments.scm"),
    );
}

#[test]
fn test_scala_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Scala,
        include_str!("../../queries/scala/comments.scm"),
    );
}

#[test]
fn test_lua_comments_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Lua,
        include_str!("../../queries/lua/comments.scm"),
    );
}

mod test_all_queries_compile_for_all_languages {
    use super::*;
    macro_rules! language_query_tests {
        ($($module:ident => $language:ident),* $(,)?) => {
            const LANGUAGES: &[ProgrammingLanguage] = &[$(ProgrammingLanguage::$language),*];
            $(mod $module {
                use super::*;
                query_kind_tests!(ProgrammingLanguage::$language;
                    locals => Locals, imports => Imports, exports => Exports,
                    references => References, lint => Lint, comments => Comments);
            })*
        };
    }
    macro_rules! query_kind_tests {
        ($language:expr; $($test:ident => $kind:ident),* $(,)?) => {
            $(#[test] fn $test() { assert_query_compiles($language, QueryKind::$kind); })*
        };
    }
    language_query_tests! {
        rust => Rust,
        javascript => JavaScript,
        typescript => TypeScript,
        go => Go,
        python => Python,
        java => Java,
        php => Php,
        perl => Perl,
        bash => Bash,
        zsh => Zsh,
        c => C,
        cpp => Cpp,
        csharp => CSharp,
        swift => Swift,
        scala => Scala,
        lua => Lua,
    }
    #[test]
    fn every_language_is_scheduled() {
        assert_eq!(LANGUAGES, ProgrammingLanguage::all());
    }
}
