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

// ============================================================================
// Rust Pattern Rules
// ============================================================================

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

    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));
    assert!(unwrap_diagnostic.is_some(), "Expected unwrap-call rule");
    assert_eq!(
        unwrap_diagnostic.unwrap().severity,
        DiagnosticSeverity::Warning
    );
}

#[test]
fn test_rust_lint_expect_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x: Option<i32> = Some(42);
    let _ = x.expect("should work");
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let expect_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("expect-call"));
    assert!(expect_diagnostic.is_some(), "Expected expect-call rule");
}

#[test]
fn test_rust_lint_dbg_macro() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = 42;
    dbg!(x);
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dbg_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dbg-macro"));
    assert!(dbg_diagnostic.is_some(), "Expected dbg-macro rule");
}

// ============================================================================
// JavaScript/TypeScript Pattern Rules
// ============================================================================

#[test]
fn test_javascript_lint_debugger_statement() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"function foo() {
    debugger;
    return 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let debugger_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("debugger-statement"));
    assert!(
        debugger_diagnostic.is_some(),
        "Expected debugger-statement rule"
    );
}

#[test]
fn test_javascript_lint_eval_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"function foo() {
    eval("console.log('hello')");
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let eval_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("eval-call"));
    assert!(eval_diagnostic.is_some(), "Expected eval-call rule");
}

#[test]
fn test_typescript_lint_debugger_statement() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.ts",
        r#"function foo(): void {
    debugger;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let debugger_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("debugger-statement"));
    assert!(
        debugger_diagnostic.is_some(),
        "Expected debugger-statement rule for TypeScript"
    );
}

// ============================================================================
// Python Pattern Rules
// ============================================================================

#[test]
fn test_python_lint_eval_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.py",
        r#"def foo():
    eval("print('hello')")
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let eval_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("eval-call"));
    assert!(
        eval_diagnostic.is_some(),
        "Expected eval-call rule for Python"
    );
}

#[test]
fn test_python_lint_exec_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.py",
        r#"def foo():
    exec("x = 1")
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let exec_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("exec-call"));
    assert!(
        exec_diagnostic.is_some(),
        "Expected exec-call rule for Python"
    );
}

#[test]
fn test_python_lint_breakpoint_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.py",
        r#"def foo():
    breakpoint()
    return 1
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let breakpoint_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("breakpoint-call"));
    assert!(
        breakpoint_diagnostic.is_some(),
        "Expected breakpoint-call rule for Python"
    );
}

// ============================================================================
// Ignore Directives
// ============================================================================

#[test]
fn test_ignore_directive_line() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x: Option<i32> = Some(42);
    // tree-hugger-ignore: unwrap-call
    let _ = x.unwrap();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));
    assert!(
        unwrap_diagnostic.is_none(),
        "unwrap-call should be ignored by directive"
    );
}

#[test]
fn test_ignore_directive_file() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"// tree-hugger-ignore-file: unwrap-call
fn main() {
    let x: Option<i32> = Some(42);
    let _ = x.unwrap();
    let _ = x.unwrap();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unwrap_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.as_deref() == Some("unwrap-call"))
        .collect();
    assert!(
        unwrap_diagnostics.is_empty(),
        "All unwrap-call diagnostics should be ignored by file directive"
    );
}

#[test]
fn test_ignore_directive_all() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    // tree-hugger-ignore
    let _ = value.unwrap();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    // Line 3 should be completely ignored
    let line3_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.range.start_line == 3)
        .collect();
    assert!(
        line3_diagnostics.is_empty(),
        "All diagnostics on line 3 should be ignored"
    );
}

// ============================================================================
// Semantic Lint Rules
// ============================================================================

#[test]
fn test_semantic_undefined_symbol_rust() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let _value = missing_value;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let undefined_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("undefined-symbol"));
    assert!(
        undefined_diagnostic.is_some(),
        "Expected undefined-symbol rule"
    );
}

#[test]
fn test_semantic_unused_symbol_rust() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"struct Holder {
    unused_value: i32,
}

fn main() {
    let _ = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unused_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unused-symbol"));
    assert!(unused_diagnostic.is_some(), "Expected unused-symbol rule");
}

#[test]
fn test_semantic_unused_import_go() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.go",
        r#"package main

import "fmt"

func main() {
    _ = 1
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unused_import = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unused-import"));
    assert!(unused_import.is_some(), "Expected unused-import rule");
}

#[test]
fn test_semantic_dead_code_rust() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"function demo() {
  return;
  const neverReached = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(dead_code.is_some(), "Expected dead-code rule");
}

// ============================================================================
// Source Context
// ============================================================================

#[test]
fn test_lint_source_context() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = Some(1);
    x.unwrap();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));
    assert!(unwrap_diagnostic.is_some());

    let diagnostic = unwrap_diagnostic.unwrap();
    assert!(diagnostic.context.is_some());
    let context = diagnostic.context.as_ref().unwrap();

    // Context should contain the unwrap call
    assert!(context.line_text.contains("unwrap"));
}

// ============================================================================
// PHP Pattern Rules
// ============================================================================

#[test]
fn test_php_lint_eval_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.php",
        r#"<?php
function foo() {
    eval("$x = 1;");
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let eval_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("eval-call"));
    assert!(eval_diagnostic.is_some(), "Expected eval-call rule for PHP");
}

// ============================================================================
// No False Positives Tests
// ============================================================================

#[test]
fn test_no_false_positive_unwrap_or() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x: Option<i32> = Some(42);
    let _ = x.unwrap_or(0);
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));
    assert!(
        unwrap_diagnostic.is_none(),
        "unwrap_or should not trigger unwrap-call"
    );
}

#[test]
fn test_no_false_positive_eval_identifier() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"function foo() {
    const eval_result = 1;
    return eval_result;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let eval_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("eval-call"));
    assert!(
        eval_diagnostic.is_none(),
        "eval as identifier should not trigger eval-call"
    );
}
