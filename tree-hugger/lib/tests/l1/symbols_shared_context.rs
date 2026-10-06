//! Work-count regressions for context a file's declarations share
//! (`2026-10-03-symbol-filters-and-reports`, review 8): the file-level
//! namespace in force, sibling-scoped containers, inner `#![cfg(test)]`
//! attributes, C++ access sections, same-file Rust traits, C/C++ `static`
//! prototypes, Scala package segments, same-file owners and type members.
//!
//! Each is derived once per file (or per module or class body), so
//! `WorkCounters::shared_context_steps` grows linearly with the number of
//! declarations. A per-declaration rescan doubles the declarations and
//! quadruples the steps; these tests compare the step counts of a generated
//! file with `SMALL` and with twice as many declarations, never elapsed time.
//!
//! The counters are process-wide; nextest runs each test in its own process,
//! so no other analysis runs between two snapshots.

use std::fmt::Write as _;

use tempfile::TempDir;
use tree_hugger::{TreeFile, work_counters};

const SMALL: usize = 200;
const LARGE: usize = 2 * SMALL;

/// One generated file shape: a header, then `body(i)` for every `i < n`,
/// then a footer.
struct Shape {
    label: &'static str,
    file_name: &'static str,
    header: &'static str,
    body: fn(usize) -> String,
    footer: &'static str,
}

const SHAPES: &[Shape] = &[
    Shape {
        label: "Rust flat functions, no inner attribute",
        file_name: "flat.rs",
        header: "",
        body: |i| format!("fn f{i}() {{}}\n"),
        footer: "",
    },
    Shape {
        label: "Rust descendants of one module",
        file_name: "module.rs",
        header: "mod m {\n",
        body: |i| format!("    fn f{i}() {{}}\n"),
        footer: "}\n",
    },
    Shape {
        label: "Rust members of impls of a same-file trait",
        file_name: "traits.rs",
        header: "pub trait T { fn m(&self); }\n",
        body: |i| format!("struct S{i};\nimpl T for S{i} {{ fn m(&self) {{}} }}\n"),
        footer: "",
    },
    Shape {
        label: "TypeScript flat functions",
        file_name: "flat.ts",
        header: "",
        body: |i| format!("function f{i}() {{}}\n"),
        footer: "",
    },
    Shape {
        label: "Python flat functions",
        file_name: "flat.py",
        header: "",
        body: |i| format!("def f{i}():\n    pass\n"),
        footer: "",
    },
    Shape {
        label: "Java package, then classes with a field",
        file_name: "Flat.java",
        header: "package p;\n",
        body: |i| format!("class C{i} {{ int field{i}; }}\n"),
        footer: "",
    },
    Shape {
        label: "Go package clause, then types with a receiver method",
        file_name: "flat.go",
        header: "package p\n",
        body: |i| format!("type T{i} struct{{}}\nfunc (t T{i}) M() {{}}\n"),
        footer: "",
    },
    Shape {
        label: "PHP namespace statement, then functions",
        file_name: "flat.php",
        header: "<?php\nnamespace A;\n",
        body: |i| format!("function f{i}() {{}}\n"),
        footer: "",
    },
    Shape {
        label: "Scala stacked package clauses, then objects with qualified access",
        file_name: "flat.scala",
        header: "package a\npackage b\n",
        body: |i| format!("object O{i} {{ private[a] def f() = 1 }}\n"),
        footer: "",
    },
    Shape {
        label: "C# file-scoped namespace, then classes",
        file_name: "Flat.cs",
        header: "namespace N;\n",
        body: |i| format!("class C{i} {{ }}\n"),
        footer: "",
    },
    Shape {
        label: "Perl package statement, then subs",
        file_name: "flat.pl",
        header: "package P;\n",
        body: |i| format!("sub f{i} {{ }}\n"),
        footer: "",
    },
    Shape {
        label: "C static prototypes, then their definitions",
        file_name: "flat.c",
        header: "",
        body: |i| format!("static void f{i}(void);\nvoid f{i}(void) {{}}\n"),
        footer: "",
    },
    Shape {
        label: "C++ struct members without an access section, then namespace functions",
        file_name: "flat.cpp",
        header: "struct S {\n",
        body: |i| format!("    void m{i}();\n"),
        footer: "};\nnamespace n { void g(); }\n",
    },
];

fn source_for(shape: &Shape, declarations: usize) -> String {
    let mut source = String::from(shape.header);
    for index in 0..declarations {
        source.push_str(&(shape.body)(index));
    }
    source.push_str(shape.footer);
    source
}

/// The shared-context steps of one complete analysis (every pass, flags
/// and metrics included) of `shape` with `declarations` declarations, and
/// the number of records it produced.
fn steps_for(shape: &Shape, declarations: usize) -> (u64, usize) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(shape.file_name);
    std::fs::write(&path, source_for(shape, declarations)).expect("write source");
    let file = TreeFile::new(&path).expect("parse");
    let before = work_counters();
    let index = file.symbol_index_v2().expect("complete index");
    let steps = work_counters().since(before).shared_context_steps;
    (steps, index.symbols.len())
}

#[test]
fn shared_context_work_grows_linearly_with_declarations() {
    let mut failures = String::new();
    for shape in SHAPES {
        let (small_steps, small_records) = steps_for(shape, SMALL);
        let (large_steps, large_records) = steps_for(shape, LARGE);
        assert!(
            large_records >= small_records + SMALL,
            "{}: the larger file must add a record per declaration ({small_records} then {large_records})",
            shape.label
        );
        // Linear growth doubles the steps (plus the header's constant
        // share); a per-declaration rescan would quadruple them.
        let linear = large_steps <= 2 * small_steps + 64;
        // A bounded number of steps per record, whatever the shape.
        let bounded = large_steps <= 16 * large_records as u64;
        if !(linear && bounded) {
            let _ = writeln!(
                failures,
                "{}: {small_steps} steps for {SMALL} declarations ({small_records} records), \
                 {large_steps} for {LARGE} ({large_records} records)",
                shape.label
            );
        }
    }
    assert!(failures.is_empty(), "shared context grew faster than the declarations:\n{failures}");
}

/// The namespace index answers what the backward scan answered: Scala
/// clauses nest, a later PHP or Perl namespace replaces an earlier one, and
/// a declaration before any statement has no namespace.
#[test]
fn file_level_namespaces_keep_their_rules_at_every_position() {
    let cases: [(&str, &str, &[&str]); 4] = [
        (
            "nest.scala",
            "object Before\npackage a\nobject InA\npackage b\nobject InAB\n",
            &["Before", "a.InA", "a.b.InAB"],
        ),
        (
            "replace.php",
            "<?php\nfunction before() {}\nnamespace A;\nfunction inA() {}\nnamespace B;\nfunction inB() {}\n",
            &["before", "A\\inA", "B\\inB"],
        ),
        (
            "replace.pl",
            "sub before { }\npackage A;\nsub in_a { }\npackage B;\nsub in_b { }\n",
            &["before", "A::in_a", "B::in_b"],
        ),
        (
            "Flat.java",
            "package p.q;\nclass One { }\nclass Two { }\n",
            &["p.q.One", "p.q.Two"],
        ),
    ];
    for (file_name, source, expected) in cases {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(file_name);
        std::fs::write(&path, source).expect("write source");
        let records = TreeFile::new(&path).expect("parse").symbol_records().expect("records");
        let names: Vec<&str> = records
            .iter()
            .map(|record| record.identity.qualified_name.as_str())
            .collect();
        for name in expected {
            assert!(names.contains(name), "{file_name}: no {name:?} in {names:?}");
        }
    }
}
