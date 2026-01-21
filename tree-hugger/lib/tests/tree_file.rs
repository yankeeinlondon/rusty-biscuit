use std::path::PathBuf;

use tree_hugger_lib::{ProgrammingLanguage, TreeFile, TreeHuggerError};

#[test]
fn parses_all_fixtures() -> Result<(), TreeHuggerError> {
    let fixtures = vec![
        ("sample.rs", ProgrammingLanguage::Rust, true),
        ("sample.js", ProgrammingLanguage::JavaScript, true),
        ("sample.ts", ProgrammingLanguage::TypeScript, true),
        ("sample.go", ProgrammingLanguage::Go, true),
        ("sample.py", ProgrammingLanguage::Python, true),
        ("Sample.java", ProgrammingLanguage::Java, true),
        ("sample.php", ProgrammingLanguage::Php, true),
        ("sample.pl", ProgrammingLanguage::Perl, false),
        ("sample.sh", ProgrammingLanguage::Bash, true),
        ("sample.zsh", ProgrammingLanguage::Zsh, true),
        ("sample.c", ProgrammingLanguage::C, true),
        ("sample.cpp", ProgrammingLanguage::Cpp, true),
        ("sample.cs", ProgrammingLanguage::CSharp, true),
        ("sample.swift", ProgrammingLanguage::Swift, true),
        ("sample.scala", ProgrammingLanguage::Scala, true),
        ("sample.lua", ProgrammingLanguage::Lua, true),
    ];

    for (file, language, expect_symbols) in fixtures {
        let path = fixture_path(file);
        let tree_file = TreeFile::new(&path)?;
        assert_eq!(tree_file.language, language, "language mismatch for {file}");
        assert!(tree_file.syntax_diagnostics().is_empty());

        let symbols = tree_file.symbols()?;
        if expect_symbols {
            assert!(!symbols.is_empty(), "expected symbols for {file}");
        }
    }

    Ok(())
}

#[test]
fn captures_imports_for_javascript() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("sample.js"))?;
    let imports = tree_file.imported_symbols()?;
    assert!(!imports.is_empty());
    Ok(())
}

#[test]
fn captures_exports_for_javascript() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("sample.js"))?;
    let exports = tree_file.exported_symbols()?;
    assert!(!exports.is_empty(), "should find exported 'greet' function");
    assert_eq!(exports[0].name, "greet");
    Ok(())
}

fn fixture_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(file)
}

// ============================================================================
// Regression tests for function signature and doc comment extraction
// These tests ensure return types, parameters, and doc comments are captured.
// ============================================================================

#[test]
fn extracts_rust_function_return_type() -> Result<(), TreeHuggerError> {
    // Regression test: Rust return types were not being extracted because the
    // code looked for a "return_type" node, but Rust's AST has the type as a
    // direct child after the "->" token.
    let tree_file = TreeFile::new(fixture_path("sample.rs"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    // Check return type
    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("String"),
        "Rust function return type should be extracted"
    );

    // Check parameters
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("&str"),
        "parameter type should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_rust_doc_comments() -> Result<(), TreeHuggerError> {
    // Regression test: Doc comments should be extracted from preceding /// comments
    let tree_file = TreeFile::new(fixture_path("sample.rs"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let doc = greet.doc_comment.as_ref().expect("should have doc comment");
    assert!(
        doc.contains("Greets a person by name"),
        "doc comment should be extracted: {doc}"
    );

    Ok(())
}

#[test]
fn extracts_typescript_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: TypeScript return types should be extracted from type_annotation
    let tree_file = TreeFile::new(fixture_path("sample.ts"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("string"),
        "TypeScript function return type should be extracted"
    );

    // Check that variadic parameters are captured
    let greet_many = symbols
        .iter()
        .find(|s| s.name == "greetMany")
        .expect("should find greetMany function");

    let sig = greet_many.signature.as_ref().expect("should have signature");
    assert!(!sig.parameters.is_empty(), "variadic parameter should be captured");
    assert!(
        sig.parameters[0].is_variadic,
        "parameter should be marked as variadic"
    );
    assert_eq!(sig.parameters[0].name, "names");

    Ok(())
}

#[test]
fn extracts_typescript_doc_comments_through_export() -> Result<(), TreeHuggerError> {
    // Regression test: Doc comments for exported functions should be found by
    // looking at the parent export_statement's siblings
    let tree_file = TreeFile::new(fixture_path("sample.ts"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let doc = greet.doc_comment.as_ref().expect("should have doc comment for exported function");
    assert!(
        doc.contains("Greets a person by name"),
        "doc comment should be extracted through export: {doc}"
    );

    Ok(())
}

#[test]
fn extracts_python_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: Python return types should be extracted from type nodes
    let tree_file = TreeFile::new(fixture_path("sample.py"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("str"),
        "Python function return type should be extracted"
    );

    // Check default value
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert!(
        sig.parameters[0].default_value.is_some(),
        "default value should be captured"
    );

    // Check variadic parameter
    let greet_many = symbols
        .iter()
        .find(|s| s.name == "greet_many")
        .expect("should find greet_many function");

    let sig = greet_many.signature.as_ref().expect("should have signature");
    assert!(!sig.parameters.is_empty(), "variadic parameter should be captured");
    assert!(
        sig.parameters[0].is_variadic,
        "parameter should be marked as variadic"
    );

    Ok(())
}

#[test]
fn extracts_go_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: Go return types and method parameters should be correct
    let tree_file = TreeFile::new(fixture_path("sample.go"))?;
    let symbols = tree_file.symbols()?;

    // Check function return type
    let greet = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find Greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("string"),
        "Go function return type should be extracted"
    );

    // Check method parameters (should NOT include receiver)
    let method_greet = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Method)
        .expect("should find Greet method");

    let sig = method_greet.signature.as_ref().expect("should have signature");
    assert!(!sig.parameters.is_empty(), "method should have parameters");
    // The first parameter should be "name", not the receiver "g"
    assert_eq!(
        sig.parameters[0].name, "name",
        "Go method parameters should not include receiver"
    );

    Ok(())
}

#[test]
fn extracts_go_doc_comments() -> Result<(), TreeHuggerError> {
    // Regression test: Go doc comments should be extracted from // comments
    let tree_file = TreeFile::new(fixture_path("sample.go"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find Greet function");

    let doc = greet.doc_comment.as_ref().expect("should have doc comment");
    assert!(
        doc.contains("greets a person by name"),
        "Go doc comment should be extracted: {doc}"
    );

    Ok(())
}
