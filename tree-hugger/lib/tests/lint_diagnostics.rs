//! Integration tests for lint_diagnostics functionality.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tree_hugger_lib::{DiagnosticKind, DiagnosticSeverity, TreeFile};

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

// ============================================================================
// Undefined Module Tests
// ============================================================================

#[test]
fn test_undefined_module_rust_reports_warning() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let _ = unknown_module::function();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let undefined_module = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("undefined-module"));
    assert!(
        undefined_module.is_some(),
        "Expected undefined-module warning for unknown_module::function()"
    );
    assert_eq!(
        undefined_module.unwrap().severity,
        DiagnosticSeverity::Warning,
        "undefined-module should be a Warning, not Error"
    );
}

#[test]
fn test_undefined_module_self_not_reported() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"struct Foo;
impl Foo {
    fn bar(&self) {
        self.baz();
    }
    fn baz(&self) {}
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let self_diagnostic = diagnostics.iter().find(|d| {
        d.rule.as_deref() == Some("undefined-module")
            && d.message.contains("'self'")
    });
    assert!(
        self_diagnostic.is_none(),
        "self.method() should NOT report undefined-module"
    );
}

#[test]
fn test_undefined_module_capital_self_not_reported() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"struct Foo;
impl Foo {
    fn new() -> Self {
        Self
    }
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let self_diagnostic = diagnostics.iter().find(|d| {
        d.rule.as_deref() == Some("undefined-module")
            && d.message.contains("'Self'")
    });
    assert!(
        self_diagnostic.is_none(),
        "Self::new() should NOT report undefined-module"
    );
}

#[test]
fn test_undefined_module_super_not_reported() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"mod outer {
    fn helper() {}
    mod inner {
        fn use_helper() {
            super::helper();
        }
    }
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let super_diagnostic = diagnostics.iter().find(|d| {
        d.rule.as_deref() == Some("undefined-module")
            && d.message.contains("'super'")
    });
    assert!(
        super_diagnostic.is_none(),
        "super::something should NOT report undefined-module"
    );
}

#[test]
fn test_undefined_module_single_letter_not_reported() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn generic<T: Clone>(value: T) -> T {
    T::clone(&value)
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let single_letter_diagnostic = diagnostics.iter().find(|d| {
        d.rule.as_deref() == Some("undefined-module")
            && d.message.contains("'T'")
    });
    assert!(
        single_letter_diagnostic.is_none(),
        "Single-letter qualifiers like T should NOT report undefined-module"
    );
}

#[test]
fn test_undefined_module_javascript_this_not_reported() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"class Foo {
    bar() {
        this.baz();
    }
    baz() {}
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let this_diagnostic = diagnostics.iter().find(|d| {
        d.rule.as_deref() == Some("undefined-module")
            && d.message.contains("'this'")
    });
    assert!(
        this_diagnostic.is_none(),
        "this.method() should NOT report undefined-module"
    );
}

#[test]
fn test_undefined_module_defined_import_not_reported() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"use std::io;

fn main() {
    let _ = io::stdout();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let io_diagnostic = diagnostics.iter().find(|d| {
        d.rule.as_deref() == Some("undefined-module")
            && d.message.contains("'io'")
    });
    assert!(
        io_diagnostic.is_none(),
        "Imported modules like 'io' should NOT report undefined-module"
    );
}

// ============================================================================
// Dead Code Detection - Terminal Statement Tests
// ============================================================================

#[test]
fn test_dead_code_after_rust_panic_macro() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    panic!("error");
    let unreachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_some(),
        "Expected dead-code after panic! macro"
    );
}

#[test]
fn test_dead_code_after_rust_unreachable_macro() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    unreachable!();
    let unreachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_some(),
        "Expected dead-code after unreachable! macro"
    );
}

#[test]
fn test_dead_code_after_rust_todo_macro() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    todo!("implement this");
    let unreachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_some(),
        "Expected dead-code after todo! macro"
    );
}

#[test]
fn test_dead_code_after_rust_unimplemented_macro() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    unimplemented!();
    let unreachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_some(),
        "Expected dead-code after unimplemented! macro"
    );
}

#[test]
fn test_dead_code_after_rust_process_exit() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    process::exit(1);
    let unreachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_some(),
        "Expected dead-code after process::exit()"
    );
}

#[test]
fn test_dead_code_after_rust_std_process_exit() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    std::process::exit(1);
    let unreachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_some(),
        "Expected dead-code after std::process::exit()"
    );
}

#[test]
fn test_no_false_positive_println_macro() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    println!("hello");
    let reachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_none(),
        "println! should NOT cause dead-code detection"
    );
}

#[test]
fn test_no_false_positive_format_macro() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    format!("hello {}", "world");
    let reachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_none(),
        "format! should NOT cause dead-code detection"
    );
}

#[test]
fn test_no_false_positive_dbg_macro_dead_code() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    dbg!(42);
    let reachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_none(),
        "dbg! should NOT cause dead-code detection"
    );
}

#[test]
fn test_no_false_positive_regular_function_call() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn demo() {
    some_function();
    let reachable = 1;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    let dead_code = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("dead-code"));
    assert!(
        dead_code.is_none(),
        "Regular function calls should NOT cause dead-code detection"
    );
}

// ============================================================================
// Unified Diagnostics API Tests
// ============================================================================

#[test]
fn test_unified_diagnostics_combines_lint_and_syntax() {
    let dir = TempDir::new().unwrap();
    // Use a file with both a lint issue (unwrap) and a syntax error (missing semicolon)
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = Some(1);
    x.unwrap();
    let y =
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.diagnostics();

    // Should have at least one lint diagnostic (unwrap-call) and one syntax error
    let lint_count = diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::Lint)
        .count();
    let syntax_count = diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::Syntax)
        .count();

    assert!(lint_count > 0, "Expected at least one lint diagnostic");
    assert!(syntax_count > 0, "Expected at least one syntax diagnostic");
}

#[test]
fn test_unified_diagnostics_semantic_kind() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let _value = undefined_symbol;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.diagnostics();

    let undefined_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("undefined-symbol"));

    assert!(
        undefined_diagnostic.is_some(),
        "Expected undefined-symbol diagnostic"
    );
    assert_eq!(
        undefined_diagnostic.unwrap().kind,
        DiagnosticKind::Semantic,
        "undefined-symbol should have Semantic kind"
    );
}

#[test]
fn test_unified_diagnostics_lint_kind() {
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
    let diagnostics = tree_file.diagnostics();

    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));

    assert!(
        unwrap_diagnostic.is_some(),
        "Expected unwrap-call diagnostic"
    );
    assert_eq!(
        unwrap_diagnostic.unwrap().kind,
        DiagnosticKind::Lint,
        "unwrap-call should have Lint kind"
    );
}

#[test]
fn test_unified_diagnostics_syntax_kind() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x =
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.diagnostics();

    let syntax_diagnostic = diagnostics
        .iter()
        .find(|d| d.kind == DiagnosticKind::Syntax);

    assert!(
        syntax_diagnostic.is_some(),
        "Expected at least one syntax diagnostic"
    );
    assert!(
        syntax_diagnostic.unwrap().rule.is_none(),
        "Syntax diagnostics should not have a rule field"
    );
}

#[test]
fn test_unified_diagnostics_preserves_context() {
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
    let diagnostics = tree_file.diagnostics();

    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));

    assert!(unwrap_diagnostic.is_some());
    let diagnostic = unwrap_diagnostic.unwrap();

    // Context should be preserved from the underlying LintDiagnostic
    assert!(diagnostic.context.is_some());
    let context = diagnostic.context.as_ref().unwrap();
    assert!(context.line_text.contains("unwrap"));
}

#[test]
fn test_unified_diagnostics_empty_file() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(&dir, "test.rs", "fn main() {}\n");

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.diagnostics();

    // A clean file should have no diagnostics
    assert!(
        diagnostics.is_empty(),
        "Clean file should have no diagnostics"
    );
}

#[test]
fn test_diagnostic_kind_display() {
    assert_eq!(format!("{}", DiagnosticKind::Lint), "lint");
    assert_eq!(format!("{}", DiagnosticKind::Semantic), "semantic");
    assert_eq!(format!("{}", DiagnosticKind::Syntax), "syntax");
}

// ============================================================================
// Comment Capture Tests (Phase 1 - Ignore directives inside strings)
// ============================================================================

#[test]
fn test_ignore_directive_in_string_literal_not_processed() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    // This comment inside a string should NOT be processed as a directive
    let msg = "// tree-hugger-ignore: unwrap-call";
    let x = Some(1);
    x.unwrap();
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.lint_diagnostics();

    // The unwrap call should still be detected since the ignore was inside a string
    let unwrap_diagnostic = diagnostics
        .iter()
        .find(|d| d.rule.as_deref() == Some("unwrap-call"));
    assert!(
        unwrap_diagnostic.is_some(),
        "unwrap-call should NOT be ignored when directive is inside a string literal"
    );
}

#[test]
fn test_ignore_directive_block_comment_rust() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = Some(1);
    /* tree-hugger-ignore: unwrap-call */
    x.unwrap();
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
        "unwrap-call should be ignored by block comment directive"
    );
}

#[test]
fn test_ignore_directive_block_comment_javascript() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"function foo() {
    /* tree-hugger-ignore: debugger-statement */
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
        debugger_diagnostic.is_none(),
        "debugger-statement should be ignored by block comment directive"
    );
}

// ============================================================================
// Syntax Diagnostics Context Tests (Phase 3)
// ============================================================================

#[test]
fn test_syntax_diagnostic_has_context() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x =
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.syntax_diagnostics();

    assert!(!diagnostics.is_empty(), "Expected syntax diagnostics");

    // At least one diagnostic should have context
    let has_context = diagnostics.iter().any(|d| d.context.is_some());
    assert!(has_context, "At least one syntax diagnostic should have context");
}

#[test]
fn test_syntax_diagnostic_context_underline_position() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.js",
        r#"function demo() {
  const x = 1
  const y =
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.syntax_diagnostics();

    assert!(!diagnostics.is_empty(), "Expected syntax diagnostics");

    let diagnostic = &diagnostics[0];
    if let Some(context) = &diagnostic.context {
        // Underline should point to a valid position in the line
        assert!(
            context.underline_column <= context.line_text.len(),
            "Underline column should be within line bounds"
        );
    }
}

// ============================================================================
// Fast Path Tests (Phase 2)
// ============================================================================

#[test]
fn test_syntax_diagnostics_fast_path_valid_file() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "valid.rs",
        r#"fn main() {
    let x = 42;
    println!("{}", x);
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let diagnostics = tree_file.syntax_diagnostics();

    // Valid file should return empty diagnostics (fast path)
    assert!(
        diagnostics.is_empty(),
        "Valid file should have no syntax diagnostics"
    );
}

#[test]
fn test_syntax_diagnostics_fast_path_many_languages() {
    let valid_files = vec![
        ("valid.js", r#"function hello() { return 1; }"#),
        ("valid.py", r#"def hello():
    return 1
"#),
        ("valid.go", r#"package main
func hello() int { return 1 }
"#),
    ];

    let dir = TempDir::new().unwrap();
    for (name, content) in valid_files {
        let path = create_temp_file(&dir, name, content);
        let tree_file = TreeFile::new(&path).unwrap();
        let diagnostics = tree_file.syntax_diagnostics();
        assert!(
            diagnostics.is_empty(),
            "Valid {} file should have no syntax diagnostics",
            name
        );
    }
}
