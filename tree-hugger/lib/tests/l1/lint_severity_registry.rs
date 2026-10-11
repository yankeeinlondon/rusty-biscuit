//! A diagnostic the library returns carries its rule's registry default
//! severity, for every registered rule and whether or not the rule is enabled.

use std::collections::BTreeSet;
use std::fs;

use tempfile::TempDir;
use tree_hugger::{DiagnosticSeverity, RuleRegistry, TreeFile};

/// One source per registered rule that makes the library emit it.
const CASES: &[(&str, &str, &str)] = &[
    ("invalid-syntax", "case.rs", "fn main( {\n"),
    ("unwrap-call", "case.rs", "fn main() {\n    let value = Some(1).unwrap();\n}\n"),
    ("expect-call", "case.rs", "fn main() {\n    let value = Some(1).expect(\"value\");\n}\n"),
    ("dbg-macro", "case.rs", "fn main() {\n    dbg!(1);\n}\n"),
    ("eval-call", "case.js", "eval('console.log(1)');\n"),
    ("exec-call", "case.py", "exec('x = 1')\n"),
    ("debugger-statement", "case.js", "function demo() {\n  debugger;\n}\n"),
    ("breakpoint-call", "case.py", "breakpoint()\n"),
    ("console-log", "case.js", "console.log('hello');\n"),
    ("print-call", "case.py", "print('hello')\n"),
    ("fmt-println", "case.go", "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n"),
    ("undefined-symbol", "case.py", "total = compute_total(items)\n"),
    ("unused-symbol", "case.rs", "struct Holder {\n    unused_value: i32,\n}\n\nfn main() {\n    let _ = 1;\n}\n"),
    ("unused-import", "case.go", "package main\n\nimport \"fmt\"\n\nfunc main() {\n\t_ = 1\n}\n"),
    ("undefined-module", "case.rs", "fn main() {\n    let _ = unknown_module::function();\n}\n"),
    ("dead-code", "case.js", "function demo() {\n  return;\n  const neverReached = 1;\n}\n"),
];

/// `(severity, metadata default severity, enabled by default)` for each
/// diagnostic of `rule` the library returns for `source`.
fn emitted(rule: &str, file: &str, source: &str) -> Vec<(DiagnosticSeverity, DiagnosticSeverity, bool)> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(file);
    fs::write(&path, source).unwrap();
    let mut tree_file = TreeFile::new(&path).unwrap();
    tree_file.experimental_semantics = true;
    tree_file
        .diagnostics()
        .into_iter()
        .filter(|d| {
            let id = d.rule.as_deref().unwrap_or("invalid-syntax");
            RuleRegistry::builtin().get(id).is_some_and(|r| r.id == rule)
        })
        .map(|d| {
            let metadata = d.metadata.expect("library diagnostics carry metadata");
            (d.severity, metadata.default_severity, metadata.is_enabled_by_default)
        })
        .collect()
}

/// Every canonical rule has a case, so a rule cannot pass by never firing.
#[test]
fn every_registered_rule_has_an_emitting_case() {
    let registered: BTreeSet<&str> = RuleRegistry::builtin().rule_ids().map(String::as_str).collect();
    let covered: BTreeSet<&str> = CASES.iter().map(|(rule, _, _)| *rule).collect();
    assert_eq!(covered, registered);
}

#[test]
fn emitted_severity_equals_registry_default_for_every_rule() {
    let registry = RuleRegistry::builtin();
    let mut saw_disabled_rule = false;
    for (rule, file, source) in CASES {
        let metadata = registry.get(rule).expect("cases name registered rules");
        let diagnostics = emitted(rule, file, source);
        assert!(!diagnostics.is_empty(), "{rule}: the case emits nothing");
        for (severity, default_severity, enabled) in diagnostics {
            assert_eq!(severity, metadata.default_severity, "{rule}: emitted severity");
            assert_eq!(default_severity, metadata.default_severity, "{rule}: metadata default severity");
            saw_disabled_rule |= !enabled;
        }
    }
    assert!(saw_disabled_rule, "at least one case must be a rule that is not enabled by default");
}

/// The library side of the library/`hug lint` agreement; the CLI side reads
/// the same fixture in `cli/tests/lint_severity_agreement.rs`.
#[test]
fn undefined_symbol_in_the_shared_fixture_is_a_warning() {
    let source = include_str!("../fixtures/lint/undefined_symbol.py");
    let diagnostics = emitted("undefined-symbol", "undefined_symbol.py", source);
    assert_eq!(
        diagnostics.iter().map(|(severity, _, enabled)| (*severity, *enabled)).collect::<Vec<_>>(),
        vec![(DiagnosticSeverity::Warning, false); 2],
        "`compute_total` and `items` are each a disabled-by-default Warning"
    );
}
