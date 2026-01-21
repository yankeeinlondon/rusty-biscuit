//! Integration tests for lint_diagnostics functionality.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tree_hugger_lib::{DiagnosticSeverity, TreeFile};

fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_rust_lint_todo_comment() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"// TODO: fix this
fn main() {}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    assert!(!diagnostics.is_empty(), "Expected TODO comment diagnostic");
    assert_eq!(diagnostics[0].rule, Some("todo-comment".to_string()));
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Info);
}

#[test]
fn test_rust_lint_unwrap_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x: Option<i32> = Some(42);
    let _ = x.unwrap();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    assert!(!diagnostics.is_empty(), "Expected unwrap call diagnostic");
    let unwrap_diagnostic = diagnostics.iter().find(|d| d.rule.as_deref() == Some("unwrap-call"));
    assert!(unwrap_diagnostic.is_some(), "Expected unwrap-call rule");
}

#[test]
fn test_javascript_lint_todo_comment() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"// TODO: implement this
function foo() {}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    assert!(!diagnostics.is_empty(), "Expected TODO comment diagnostic for JavaScript");
}

#[test]
fn test_python_lint_todo_comment() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.py",
        r#"# TODO: implement this
def foo():
    pass
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    assert!(!diagnostics.is_empty(), "Expected TODO comment diagnostic for Python");
}

#[test]
fn test_lint_source_context() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"// TODO: fix this bug
fn main() {}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    assert!(!diagnostics.is_empty());
    let diagnostic = &diagnostics[0];

    // Should have source context
    assert!(diagnostic.context.is_some());
    let context = diagnostic.context.as_ref().unwrap();

    // Context should contain the comment text
    assert!(context.line_text.contains("TODO"));
}
