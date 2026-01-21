//! Tests to verify lint queries compile correctly for all languages.

use tree_hugger_lib::ProgrammingLanguage;
use tree_sitter::Query;

fn test_query_compiles(language: ProgrammingLanguage, query_text: &str) {
    let ts_language = language.tree_sitter_language();
    let result = Query::new(&ts_language, query_text);

    match &result {
        Ok(query) => {
            println!("{}: {} patterns, captures: {:?}",
                language,
                query.pattern_count(),
                query.capture_names()
            );
        }
        Err(e) => {
            panic!("{} lint query failed to compile: {:?}\n\nQuery:\n{}",
                language, e, query_text);
        }
    }

    assert!(result.is_ok(), "{} lint query should compile", language);
}

#[test]
fn test_rust_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Rust,
        include_str!("../queries/rust/lint.scm")
    );
}

#[test]
fn test_javascript_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::JavaScript,
        include_str!("../queries/javascript/lint.scm")
    );
}

#[test]
fn test_typescript_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::TypeScript,
        include_str!("../queries/typescript/lint.scm")
    );
}

#[test]
fn test_go_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Go,
        include_str!("../queries/go/lint.scm")
    );
}

#[test]
fn test_python_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Python,
        include_str!("../queries/python/lint.scm")
    );
}

#[test]
fn test_java_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Java,
        include_str!("../queries/java/lint.scm")
    );
}

#[test]
fn test_php_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Php,
        include_str!("../queries/php/lint.scm")
    );
}

#[test]
fn test_perl_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Perl,
        include_str!("../queries/perl/lint.scm")
    );
}

#[test]
fn test_bash_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Bash,
        include_str!("../queries/bash/lint.scm")
    );
}

#[test]
fn test_zsh_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Zsh,
        include_str!("../queries/zsh/lint.scm")
    );
}

#[test]
fn test_c_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::C,
        include_str!("../queries/c/lint.scm")
    );
}

#[test]
fn test_cpp_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Cpp,
        include_str!("../queries/cpp/lint.scm")
    );
}

#[test]
fn test_csharp_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::CSharp,
        include_str!("../queries/c_sharp/lint.scm")
    );
}

#[test]
fn test_swift_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Swift,
        include_str!("../queries/swift/lint.scm")
    );
}

#[test]
fn test_scala_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Scala,
        include_str!("../queries/scala/lint.scm")
    );
}

#[test]
fn test_lua_lint_query_compiles() {
    test_query_compiles(
        ProgrammingLanguage::Lua,
        include_str!("../queries/lua/lint.scm")
    );
}
