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
