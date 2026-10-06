//! Work-count regressions for context a file's declarations share
//! (`2026-10-03-symbol-filters-and-reports`, review 8): the file-level
//! namespace in force, sibling-scoped containers, inner `#![cfg(test)]`
//! attributes, C++ access sections, same-file Rust traits, C/C++ `static`
//! prototypes, Scala package segments, same-file owners and type members.
//! Review 9 added repeated names: same-named traits, owners and tables in
//! successive modules, namespace-distinct and overloaded C++ functions,
//! namespace-qualified definitions, growing Scala block packages, same-named
//! export targets, and the containment consumer that links members to those
//! owners.
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
use tree_hugger::{
    Capability, SymbolKindV2, SymbolQuery, SymbolRecord, SymbolRef, TreeFile, VisibilityLevel, work_counters,
};

/// Stamps one `#[test]` per labeled shape in the module `$module`, each
/// calling the sibling function `$run` with its label, so every shape of a
/// growing shape table is scheduled and timed on its own. `STAMPED` lists
/// the labels for [`assert_every_shape_stamped`].
macro_rules! shape_tests {
    ($module:ident => $run:ident { $($name:ident => $label:literal,)+ }) => {
        pub(crate) mod $module {
            pub const STAMPED: &[&str] = &[$($label),+];
            $(
                #[test]
                fn $name() {
                    super::$run($label);
                }
            )+
        }
    };
}
pub(crate) use shape_tests;

/// Every label in `labels` is unique and stamped exactly once, so a new
/// shape cannot go untested.
pub fn assert_every_shape_stamped<'a>(table: &str, labels: impl Iterator<Item = &'a str>, stamped: &[&str]) {
    let mut labels: Vec<&str> = labels.collect();
    labels.sort_unstable();
    let mut unique = labels.clone();
    unique.dedup();
    assert_eq!(unique, labels, "{table}: labels are unique");
    let mut stamped = stamped.to_vec();
    stamped.sort_unstable();
    assert_eq!(stamped, labels, "{table}: stamp one test per shape");
}

/// The labeled entry of `table`.
pub fn shape_labeled<'a, T>(table: &'a [T], label: &str, label_of: fn(&T) -> &str) -> &'a T {
    table.iter().find(|entry| label_of(entry) == label).unwrap_or_else(|| panic!("no shape labeled {label:?}"))
}

pub const SMALL: usize = 200;
pub const LARGE: usize = 2 * SMALL;

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
    let (steps, records) = analyze(shape, declarations, Analysis::Complete);
    (steps, records.len())
}

/// Which public entry point [`analyze`] measures.
#[derive(Clone, Copy)]
enum Analysis {
    /// `symbol_index_v2`: every pass, in-file qualified names.
    Complete,
    /// `TreeFile::query` asking only for containment.
    Containment,
}

/// The shared-context steps of one analysis of `shape` with `declarations`
/// declarations, and the records it produced.
fn analyze(shape: &Shape, declarations: usize, analysis: Analysis) -> (u64, Vec<SymbolRecord>) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(shape.file_name);
    std::fs::write(&path, source_for(shape, declarations)).expect("write source");
    let file = TreeFile::new(&path).expect("parse");
    let before = work_counters();
    let records = match analysis {
        Analysis::Complete => file.symbol_index_v2().expect("complete index").symbols,
        Analysis::Containment => {
            let query = SymbolQuery::new().capabilities([Capability::Containment]);
            file.query(&query).expect("containment query").index.symbols
        }
    };
    let steps = work_counters().since(before).shared_context_steps;
    (steps, records)
}

/// Linear growth doubles the steps (plus the header's constant share); a
/// per-declaration rescan would quadruple them. The steps per record are
/// bounded whatever the shape.
pub fn grows_linearly(small_steps: u64, large_steps: u64, large_records: usize) -> bool {
    grows_linearly_within(small_steps, large_steps, large_records, 16)
}

/// [`grows_linearly`] with `per_record` steps allowed per record.
pub fn grows_linearly_within(small_steps: u64, large_steps: u64, large_records: usize, per_record: u64) -> bool {
    large_steps <= 2 * small_steps + 64 && large_steps <= per_record * large_records as u64
}

shape_tests!(shared_context_work_grows_linearly_with_declarations => shape_work_grows_linearly {
    rust_flat_functions => "Rust flat functions, no inner attribute",
    rust_module_descendants => "Rust descendants of one module",
    rust_same_file_trait_impls => "Rust members of impls of a same-file trait",
    typescript_flat_functions => "TypeScript flat functions",
    python_flat_functions => "Python flat functions",
    java_package_classes => "Java package, then classes with a field",
    go_package_types => "Go package clause, then types with a receiver method",
    php_namespace_functions => "PHP namespace statement, then functions",
    scala_stacked_packages => "Scala stacked package clauses, then objects with qualified access",
    csharp_file_scoped_namespace => "C# file-scoped namespace, then classes",
    perl_package_subs => "Perl package statement, then subs",
    c_static_prototypes => "C static prototypes, then their definitions",
    cpp_members_and_namespace_functions => "C++ struct members without an access section, then namespace functions",
});

fn shape_work_grows_linearly(label: &str) {
    let mut failures = String::new();
    {
        let shape = shape_labeled(SHAPES, label, |shape| shape.label);
        let (small_steps, small_records) = steps_for(shape, SMALL);
        let (large_steps, large_records) = steps_for(shape, LARGE);
        assert!(
            large_records >= small_records + SMALL,
            "{}: the larger file must add a record per declaration ({small_records} then {large_records})",
            shape.label
        );
        if !grows_linearly(small_steps, large_steps, large_records) {
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

/// Checks the records of a file with `n` declarations.
type Check = fn(&[SymbolRecord], usize) -> Result<(), String>;

/// A generated shape whose declarations repeat one name (or grow one shared
/// list), with the public results each copy must keep.
struct RepeatedShape {
    shape: Shape,
    check: Check,
}

/// The records whose in-file qualified name (without the `file#` prefix a
/// finished result adds) is `qualified_name`.
fn named<'a>(records: &'a [SymbolRecord], qualified_name: &str) -> Vec<&'a SymbolRecord> {
    records
        .iter()
        .filter(|record| {
            let name = &record.identity.qualified_name;
            name.split_once('#').map_or(name.as_str(), |(_, in_file)| in_file) == qualified_name
        })
        .collect()
}

fn one_of<'a>(
    records: &'a [SymbolRecord],
    qualified_name: &str,
    kind: SymbolKindV2,
) -> Result<&'a SymbolRecord, String> {
    let found: Vec<&SymbolRecord> = named(records, qualified_name)
        .into_iter()
        .filter(|record| record.kind == kind)
        .collect();
    match found.as_slice() {
        [only] => Ok(only),
        other => Err(format!("expected one {kind:?} `{qualified_name}`, found {}", other.len())),
    }
}

fn level_is(record: &SymbolRecord, expected: VisibilityLevel) -> Result<(), String> {
    if record.visibility.level == expected {
        Ok(())
    } else {
        Err(format!(
            "`{}`: expected {expected:?}, found {:?}",
            record.identity.qualified_name, record.visibility.level
        ))
    }
}

/// `member`'s container and parent are `owner`.
fn owned_by(member: &SymbolRecord, owner: &SymbolRecord) -> Result<(), String> {
    let linked = |link: &Option<SymbolRef>| {
        link.as_ref().and_then(|link| link.id.as_ref()) == Some(&owner.id)
    };
    if linked(&member.relations.container) && linked(&member.relations.parent) {
        Ok(())
    } else {
        Err(format!(
            "`{}` at byte {}: not linked to its owner `{}` at byte {}",
            member.identity.qualified_name,
            member.source.declaration_span.start_byte,
            owner.identity.qualified_name,
            owner.source.declaration_span.start_byte
        ))
    }
}

/// Every `static` declaration and every later definition of the same
/// function is ModulePrivate: the definition's own earlier declaration, not
/// a same-named function of another namespace or overload, decides it.
fn check_cpp_functions(records: &[SymbolRecord], n: usize) -> Result<(), String> {
    let functions: Vec<&SymbolRecord> =
        records.iter().filter(|record| record.kind == SymbolKindV2::Function).collect();
    if functions.len() != 2 * n {
        return Err(format!("expected {} function records, found {}", 2 * n, functions.len()));
    }
    functions
        .into_iter()
        .try_for_each(|function| level_is(function, VisibilityLevel::ModulePrivate))
}

/// `Probe` repeated in every module: the impl method's level comes from the
/// same-file traits, which all agree on Public, and its `implements` link
/// names its own module's `Probe`, the one the impl's scope sees.
fn check_rust_traits(records: &[SymbolRecord], n: usize) -> Result<(), String> {
    for i in 0..n {
        let probe = one_of(records, &format!("r9_{i}::Probe"), SymbolKindV2::Trait)?;
        level_is(probe, VisibilityLevel::Public)?;
        let method = one_of(records, &format!("r9_{i}::S{i}::m"), SymbolKindV2::Method)?;
        level_is(method, VisibilityLevel::Public)?;
        match method.relations.implements.as_slice() {
            [link] if link.name == "Probe" && link.id.as_ref() == Some(&probe.id) => {}
            other => return Err(format!("`r9_{i}::S{i}::m` implements {other:?}")),
        }
    }
    Ok(())
}

/// Each module's `impl Probe` method is Private and belongs to that module's
/// `struct Probe`.
fn check_rust_owners(records: &[SymbolRecord], n: usize) -> Result<(), String> {
    for i in 0..n {
        let owner = one_of(records, &format!("r9_{i}::Probe"), SymbolKindV2::Type)?;
        let method = one_of(records, &format!("r9_{i}::Probe::m"), SymbolKindV2::Method)?;
        level_is(method, VisibilityLevel::Private)?;
        owned_by(method, owner)?;
    }
    Ok(())
}

/// Every `function Probe:m()` belongs to the nearest preceding
/// `local Probe = {}`.
fn check_lua_tables(records: &[SymbolRecord], n: usize) -> Result<(), String> {
    let mut tables: Vec<&SymbolRecord> = named(records, "Probe")
        .into_iter()
        .filter(|record| record.kind == SymbolKindV2::Variable)
        .collect();
    let mut methods: Vec<&SymbolRecord> = named(records, "Probe:m");
    if tables.len() != n || methods.len() != n {
        return Err(format!("expected {n} tables and methods, found {} and {}", tables.len(), methods.len()));
    }
    tables.sort_by_key(|record| record.source.declaration_span.start_byte);
    methods.sort_by_key(|record| record.source.declaration_span.start_byte);
    for (table, method) in tables.iter().zip(&methods) {
        owned_by(method, table)?;
    }
    Ok(())
}

/// Each namespace's declaration and definition are both `r9_i::probe`.
fn check_cpp_namespaces(records: &[SymbolRecord], n: usize) -> Result<(), String> {
    for i in 0..n {
        let found = named(records, &format!("r9_{i}::probe")).len();
        if found != 2 {
            return Err(format!("expected two `r9_{i}::probe` records, found {found}"));
        }
    }
    check_cpp_functions(records, n)
}

/// Each module's `pub fn probe` is its own export target.
fn check_rust_exports(records: &[SymbolRecord], n: usize) -> Result<(), String> {
    for i in 0..n {
        let probe = one_of(records, &format!("r9_{i}::probe"), SymbolKindV2::Function)?;
        level_is(probe, VisibilityLevel::Public)?;
        if !probe.visibility.is_exported {
            return Err(format!("`r9_{i}::probe` is not exported"));
        }
    }
    Ok(())
}

fn check_scala_packages(records: &[SymbolRecord], n: usize) -> Result<(), String> {
    for i in 0..n {
        level_is(one_of(records, &format!("r9_{i}.P.f"), SymbolKindV2::Method)?, VisibilityLevel::Package)?;
    }
    Ok(())
}

const RUST_REPEATED_OWNER: Shape = Shape {
    label: "Rust struct `Probe` and its impl repeated per module",
    file_name: "owners.rs",
    header: "",
    body: |i| format!("mod r9_{i} {{ struct Probe; impl Probe {{ fn m(&self) {{}} }} }}\n"),
    footer: "",
};

const LUA_REPEATED_TABLE: Shape = Shape {
    label: "Lua `local Probe` table and its method repeated",
    file_name: "tables.lua",
    header: "",
    body: |i| format!("local Probe = {{}}\nfunction Probe:m() return {i} end\n"),
    footer: "return Probe\n",
};

const REPEATED_SHAPES: &[RepeatedShape] = &[
    RepeatedShape {
        shape: Shape {
            label: "Rust trait `Probe` repeated per module",
            file_name: "traits.rs",
            header: "",
            body: |i| {
                format!(
                    "mod r9_{i} {{ pub trait Probe {{ fn m(&self); }} struct S{i};\n    \
                     impl Probe for S{i} {{ fn m(&self) {{}} }} }}\n"
                )
            },
            footer: "",
        },
        check: check_rust_traits,
    },
    RepeatedShape { shape: RUST_REPEATED_OWNER, check: check_rust_owners },
    RepeatedShape { shape: LUA_REPEATED_TABLE, check: check_lua_tables },
    RepeatedShape {
        shape: Shape {
            label: "C++ `static void probe(); void probe() {}` repeated per namespace",
            file_name: "namespaces.cpp",
            header: "",
            body: |i| format!("namespace r9_{i} {{ static void probe(); void probe() {{}} }}\n"),
            footer: "",
        },
        check: check_cpp_namespaces,
    },
    RepeatedShape {
        shape: Shape {
            label: "C++ out-of-body `void r9_i::probe() {}` after a namespaced `static`",
            file_name: "out_of_body.cpp",
            header: "",
            body: |i| format!("namespace r9_{i} {{ static void probe(); }}\nvoid r9_{i}::probe() {{}}\n"),
            footer: "",
        },
        check: check_cpp_functions,
    },
    RepeatedShape {
        shape: Shape {
            label: "C++ overloads `static void probe(R9_i); void probe(R9_i) {}`",
            file_name: "overloads.cpp",
            header: "",
            body: |i| format!("struct R9_{i} {{}};\nstatic void probe(R9_{i});\nvoid probe(R9_{i}) {{}}\n"),
            footer: "",
        },
        check: check_cpp_functions,
    },
    RepeatedShape {
        shape: Shape {
            label: "C++ distinct namespace-qualified definitions",
            file_name: "qualified.cpp",
            header: "",
            body: |i| format!("namespace r9_{i} {{ static void probe{i}(); }}\nvoid r9_{i}::probe{i}() {{}}\n"),
            footer: "",
        },
        check: check_cpp_functions,
    },
    RepeatedShape {
        shape: Shape {
            label: "Rust `pub fn probe` exported from every module",
            file_name: "exports.rs",
            header: "",
            body: |i| format!("mod r9_{i} {{ pub fn probe() {{}} }}\n"),
            footer: "",
        },
        check: check_rust_exports,
    },
    RepeatedShape {
        shape: Shape {
            label: "Scala sibling block packages with package-qualified access",
            file_name: "packages.scala",
            header: "",
            body: |i| format!("package r9_{i} {{ object P {{ private[r9_{i}] def f() = 1 }} }}\n"),
            footer: "",
        },
        check: check_scala_packages,
    },
];

shape_tests!(repeated_names_keep_shared_context_linear_and_results_intact => repeated_shape_stays_linear {
    rust_traits_per_module => "Rust trait `Probe` repeated per module",
    rust_owner_per_module => "Rust struct `Probe` and its impl repeated per module",
    lua_repeated_table => "Lua `local Probe` table and its method repeated",
    cpp_static_per_namespace => "C++ `static void probe(); void probe() {}` repeated per namespace",
    cpp_out_of_body_definitions => "C++ out-of-body `void r9_i::probe() {}` after a namespaced `static`",
    cpp_overloads => "C++ overloads `static void probe(R9_i); void probe(R9_i) {}`",
    cpp_qualified_definitions => "C++ distinct namespace-qualified definitions",
    rust_exports_per_module => "Rust `pub fn probe` exported from every module",
    scala_block_packages => "Scala sibling block packages with package-qualified access",
});

fn repeated_shape_stays_linear(label: &str) {
    let mut failures = String::new();
    {
        let RepeatedShape { shape, check } = shape_labeled(REPEATED_SHAPES, label, |repeated| repeated.shape.label);
        let (small_steps, small_records) = analyze(shape, SMALL, Analysis::Complete);
        let (large_steps, large_records) = analyze(shape, LARGE, Analysis::Complete);
        for (n, records) in [(SMALL, &small_records), (LARGE, &large_records)] {
            if let Err(problem) = check(records, n) {
                let _ = writeln!(failures, "{} (n = {n}): {problem}", shape.label);
            }
        }
        if !grows_linearly(small_steps, large_steps, large_records.len()) {
            let _ = writeln!(
                failures,
                "{}: {small_steps} steps for {SMALL} declarations ({} records), {large_steps} for {LARGE} ({} records)",
                shape.label,
                small_records.len(),
                large_records.len()
            );
        }
    }
    assert!(failures.is_empty(), "repeated names broke results or linear shared context:\n{failures}");
}

/// Every shape table here and in `symbols_relation_work` and
/// `symbols_relations` has one stamped test per labeled shape.
#[test]
fn every_linear_work_shape_has_its_own_test() {
    assert_every_shape_stamped(
        "SHAPES",
        SHAPES.iter().map(|shape| shape.label),
        shared_context_work_grows_linearly_with_declarations::STAMPED,
    );
    assert_every_shape_stamped(
        "REPEATED_SHAPES",
        REPEATED_SHAPES.iter().map(|repeated| repeated.shape.label),
        repeated_names_keep_shared_context_linear_and_results_intact::STAMPED,
    );
    assert_every_shape_stamped(
        "WORK_SHAPES",
        crate::symbols_relation_work::WORK_SHAPES.iter().map(|shape| shape.label),
        crate::symbols_relation_work::repeated_names_keep_relation_work_linear::STAMPED,
    );
    assert_every_shape_stamped(
        "RELATION_SHAPES",
        crate::symbols_relations::RELATION_SHAPES.iter().map(|shape| shape.label),
        crate::symbols_relations::relation_resolution_grows_linearly_with_declarations::STAMPED,
    );
}

/// The containment consumer links each member to its same-module (Rust) or
/// nearest preceding (Lua) owner without revisiting every same-named owner.
#[test]
fn containment_of_repeated_owners_stays_linear() {
    let mut failures = String::new();
    let shapes: [(&Shape, Check); 2] =
        [(&RUST_REPEATED_OWNER, check_rust_owners), (&LUA_REPEATED_TABLE, check_lua_tables)];
    for (shape, check) in shapes {
        let (small_steps, small_records) = analyze(shape, SMALL, Analysis::Containment);
        let (large_steps, large_records) = analyze(shape, LARGE, Analysis::Containment);
        for (n, records) in [(SMALL, &small_records), (LARGE, &large_records)] {
            if let Err(problem) = check(records, n) {
                let _ = writeln!(failures, "{} (n = {n}): {problem}", shape.label);
            }
        }
        if !grows_linearly(small_steps, large_steps, large_records.len()) {
            let _ = writeln!(
                failures,
                "{}: {small_steps} containment steps for {SMALL} declarations, {large_steps} for {LARGE}",
                shape.label
            );
        }
    }
    assert!(failures.is_empty(), "containment of repeated owners:\n{failures}");
}
