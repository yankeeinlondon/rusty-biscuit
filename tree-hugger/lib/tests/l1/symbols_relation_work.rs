//! Relation resolution does bounded work per declaration and path.
//!
//! Each shape repeats one name so that every lookup would meet an
//! input-sized bucket if it scanned one: record-less owners binding the same
//! parameter name, same-named owners in sibling blocks, a reopened
//! namespace, a growing written-base list. The counted work
//! (`shared_context_steps`, which also counts candidate visits and
//! deduplication) must grow with the file, never with the bucket, and the
//! results must stay exact: every target ID and its source order is checked
//! at both sizes. Parameter lists contribute input units even when their
//! owners have no record. The budget stays 16 steps per input unit. Nested
//! inline namespaces are measured by depth; many inline namespaces sharing
//! one intermediate spelling are measured with a distinct final name per
//! path, so no suffix may revisit the intermediate's candidates.

use std::fmt::Write as _;

use tempfile::TempDir;
use tree_hugger::{SymbolId, SymbolKindV2, SymbolQuery, SymbolRecord, SymbolRef, TreeFile, work_counters};

use crate::symbols_shared_context::{LARGE, SMALL, grows_linearly};

/// What every `child` record's resolved bases must be.
#[derive(Clone, Copy)]
enum Expect {
    /// Each child resolves one base: the file's only `base` record.
    EachToTheOne,
    /// The k-th child (source order) resolves the k-th `base` record.
    EachToItsOwn,
    /// The only child resolves every `base` record, in source order.
    OneToAll,
    Unresolved,
}

struct WorkShape {
    label: &'static str,
    extension: &'static str,
    source: fn(usize) -> String,
    /// Name prefixes of the records written with bases and of their
    /// targets (type kinds only).
    child: &'static str,
    base: &'static str,
    expect: Expect,
    /// Parameters per repetition, including lists on record-less owners.
    parameter_units: usize,
}

fn repeat(header: &str, body: fn(usize) -> String, footer: &str, n: usize) -> String {
    let mut source = header.to_string();
    for i in 0..n {
        source.push_str(&body(i));
    }
    source.push_str(footer);
    source
}

/// `n` inline namespaces in `ReviewNs`, each declaring a `Shared`
/// namespace (`opening` opens the scopes around the Base) with a distinct
/// Base, then `child(i)` for every Base, inside the namespace `around` opens
/// when given.
fn shared_intermediate(n: usize, opening: &str, around: &str, child: fn(usize) -> String) -> String {
    let closing = "} ".repeat(opening.matches('{').count());
    let mut source = "namespace ReviewNs {\n".to_string();
    for i in 0..n {
        let _ = writeln!(source, "inline namespace v{i} {{ {opening} struct ReviewBase{i} {{}}; {closing}}}");
    }
    source.push_str("}\n");
    source.push_str(around);
    for i in 0..n {
        source.push_str(&child(i));
    }
    if !around.is_empty() {
        source.push_str("}\n");
    }
    source
}

const WORK_SHAPES: &[WorkShape] = &[
    WorkShape {
        label: "C++ broad inline namespaces: injected", extension: "cpp",
        source: |n| repeat("", |i| format!("namespace ReviewNs {{ inline namespace v{i} {{ struct ReviewBase{i} {{}}; }} }}\nstruct ReviewChild{i} : ReviewNs::ReviewBase{i} {{}};\n"), "", n),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },

    WorkShape {
        label: "C++ broad inline namespaces: rooted", extension: "cpp",
        source: |n| repeat("", |i| format!("namespace ReviewNs {{ inline namespace v{i} {{ struct ReviewBase{i} {{}}; }} }}\nstruct ReviewChild{i} : ::ReviewNs::ReviewBase{i} {{}};\n"), "", n),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },

    WorkShape {
        label: "C++ broad inline namespaces: bare", extension: "cpp",
        source: |n| repeat("", |i| format!("namespace ReviewNs {{ inline namespace v{i} {{ struct ReviewBase{i} {{}}; }} }}\nnamespace ReviewNs {{ struct ReviewChild{i} : ReviewBase{i} {{}}; }}\n"), "", n),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },

    WorkShape {
        label: "C++ broad inline namespaces: explicit", extension: "cpp",
        source: |n| repeat("", |i| format!("namespace ReviewNs {{ inline namespace v{i} {{ struct ReviewBase{i} {{}}; }} }}\nstruct ReviewChild{i} : ReviewNs::v{i}::ReviewBase{i} {{}};\n"), "", n),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },

    WorkShape {
        label: "C++ broad inline namespaces: ambiguous", extension: "cpp",
        source: |n| repeat("", |i| format!("namespace ReviewNs {{ inline namespace v{i} {{ struct ReviewBase {{}}; }} }}\nstruct ReviewChild{i} : ReviewNs::ReviewBase {{}};\n"), "", n),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::Unresolved,
        parameter_units: 0,
    },

    WorkShape {
        label: "C++ broad inline namespaces: ordinary", extension: "cpp",
        source: |n| repeat("", |i| format!("namespace ReviewNs {{ namespace v{i} {{ struct ReviewBase{i} {{}}; }} }}\nstruct ReviewChild{i} : ReviewNs::v{i}::ReviewBase{i} {{}};\n"), "", n),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },

    WorkShape {
        label: "C++ inline namespaces sharing an intermediate namespace: qualified", extension: "cpp",
        source: |n| shared_intermediate(n, "namespace Shared {", "", |i| format!("struct ReviewChild{i} : ReviewNs::Shared::ReviewBase{i} {{}};\n")),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::Unresolved,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ inline namespaces sharing an intermediate namespace: rooted", extension: "cpp",
        source: |n| shared_intermediate(n, "namespace Shared {", "", |i| format!("struct ReviewChild{i} : ::ReviewNs::Shared::ReviewBase{i} {{}};\n")),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::Unresolved,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ inline namespaces sharing an intermediate namespace: bare", extension: "cpp",
        source: |n| shared_intermediate(n, "namespace Shared {", "namespace ReviewNs {\n", |i| format!("struct ReviewChild{i} : Shared::ReviewBase{i} {{}};\n")),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::Unresolved,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ inline namespaces sharing an intermediate namespace: deeper", extension: "cpp",
        source: |n| shared_intermediate(n, "namespace Shared { namespace Inner {", "", |i| format!("struct ReviewChild{i} : ReviewNs::Shared::Inner::ReviewBase{i} {{}};\n")),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::Unresolved,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ inline namespaces sharing an intermediate namespace: explicit", extension: "cpp",
        source: |n| shared_intermediate(n, "namespace Shared {", "", |i| format!("struct ReviewChild{i} : ReviewNs::v{i}::Shared::ReviewBase{i} {{}};\n")),
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ ordinary namespaces sharing an intermediate namespace: explicit", extension: "cpp",
        source: |n| {
            let mut source = "namespace ReviewNs {\n".to_string();
            for i in 0..n { let _ = writeln!(source, "namespace v{i} {{ namespace Shared {{ struct ReviewBase{i} {{}}; }} }}"); }
            source.push_str("}\n");
            for i in 0..n { let _ = writeln!(source, "struct ReviewChild{i} : ReviewNs::v{i}::Shared::ReviewBase{i} {{}};"); }
            source
        },
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ inline namespaces sharing an intermediate namespace: same final name", extension: "cpp",
        source: |n| {
            let mut source = "namespace ReviewNs {\n".to_string();
            for i in 0..n { let _ = writeln!(source, "inline namespace v{i} {{ namespace Shared {{ struct ReviewBase {{}}; }} }}"); }
            source.push_str("}\n");
            for i in 0..n { let _ = writeln!(source, "struct ReviewChild{i} : ReviewNs::Shared::ReviewBase {{}};"); }
            source
        },
        child: "ReviewChild", base: "ReviewBase", expect: Expect::Unresolved,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ inline namespaces sharing an intermediate struct", extension: "cpp",
        source: |n| {
            let mut source = "namespace ReviewNs {\n".to_string();
            for i in 0..n { let _ = writeln!(source, "inline namespace v{i} {{ struct Shared {{ struct ReviewBase{i} {{}}; }}; }}"); }
            source.push_str("}\n");
            for i in 0..n { let _ = writeln!(source, "struct ReviewChild{i} : ReviewNs::Shared::ReviewBase{i} {{}};"); }
            source
        },
        child: "ReviewChild", base: "ReviewBase", expect: Expect::Unresolved,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ inline namespaces declaring functions named like the one intermediate namespace", extension: "cpp",
        source: |n| {
            let mut source = "namespace ReviewNs {\n".to_string();
            for i in 0..n { let _ = writeln!(source, "inline namespace v{i} {{ void Shared(); }}"); }
            source.push_str("namespace Shared {\n");
            for i in 0..n { let _ = writeln!(source, "struct ReviewBase{i} {{}};"); }
            source.push_str("}\n}\n");
            for i in 0..n { let _ = writeln!(source, "struct ReviewChild{i} : ReviewNs::Shared::ReviewBase{i} {{}};"); }
            source
        },
        child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "Swift shared generic owner across conformance extensions", extension: "swift",
        source: |n| {
            let params: Vec<_> = (0..n).map(|i| format!("P{i}")).collect();
            let mut source = format!("struct ReviewSelf<{}> {}\n", params.join(", "), "{}");
            for i in 0..n { let _ = writeln!(source, "protocol ReviewTrait{i} {{}}\nextension ReviewSelf: ReviewTrait{i} {{}}"); }
            source
        }, child: "ReviewSelf", base: "ReviewTrait", expect: Expect::OneToAll,
        parameter_units: 1,
    },
    WorkShape {
        label: "Swift shared nested generic owner across extension bodies", extension: "swift",
        source: |n| {
            let params: Vec<_> = (0..n).map(|i| format!("P{i}")).collect();
            let mut source = format!("class ReviewBase {{}}\nstruct ReviewOuter<{}> {{ struct ReviewInner {{}} }}\n", params.join(", "));
            for i in 0..n { let _ = writeln!(source, "extension ReviewOuter.ReviewInner {{ class ReviewChild{i}: ReviewBase {{}} }}"); }
            source
        }, child: "ReviewChild", base: "ReviewBase", expect: Expect::EachToTheOne,
        parameter_units: 1,
    },

    WorkShape {
        label: "Rust record-less impl parameters named like a file-level trait",
        extension: "rs",
        source: |n| {
            repeat(
                "pub trait ReviewBase {}\npub struct ReviewHolder<T>(T);\n",
                |i| format!("impl<ReviewBase> ReviewHolder<ReviewBase> {{}}\npub trait ReviewChild{i}: ReviewBase {{}}\n"),
                "",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToTheOne,
        parameter_units: 1,
    },
    WorkShape {
        label: "TypeScript record-less signature parameters named like a file-level interface",
        extension: "ts",
        source: |n| {
            repeat(
                "interface ReviewBase {}\n",
                |i| {
                    format!(
                        "declare function reviewProbe{i}<ReviewBase>(): void;\n\
                         interface ReviewChild{i} extends ReviewBase {{}}\n"
                    )
                },
                "",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToTheOne,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ sibling blocks with same-named Holder/Base",
        extension: "cpp",
        source: |n| {
            repeat(
                "void reviewProbe() {\n",
                |_| {
                    "{ struct ReviewHolder { struct ReviewBase {}; }; \
                     struct ReviewChild : ReviewHolder::ReviewBase {}; }\n"
                        .to_string()
                },
                "}\n",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "Rust sibling blocks with same-named local modules",
        extension: "rs",
        source: |n| {
            repeat(
                "fn review_probe() {\n",
                |_| {
                    "{ mod review_holder { pub trait ReviewBase {} } \
                     trait ReviewChild: review_holder::ReviewBase {} }\n"
                        .to_string()
                },
                "}\n",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "Java sibling blocks with same-named local classes",
        extension: "java",
        source: |n| {
            repeat(
                "class ReviewProbe {\n  void probe() {\n",
                |_| {
                    "    { class ReviewHolder { class ReviewBase {} } \
                     class ReviewChild extends ReviewHolder.ReviewBase {} }\n"
                        .to_string()
                },
                "  }\n}\n",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "Swift sibling `do` blocks with same-named local classes",
        extension: "swift",
        source: |n| {
            repeat(
                "func reviewProbe() {\n",
                |_| {
                    "    do {\n        class ReviewHolder { class ReviewBase {} }\n        \
                     class ReviewChild: ReviewHolder.ReviewBase {}\n    }\n"
                        .to_string()
                },
                "}\n",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "Scala sibling blocks with same-named local objects",
        extension: "scala",
        source: |n| {
            repeat(
                "object ReviewProbe {\n  def probe(): Unit = {\n",
                |_| {
                    "    {\n      object ReviewHolder { class ReviewBase }\n      \
                     class ReviewChild extends ReviewHolder.ReviewBase\n    }\n"
                        .to_string()
                },
                "  }\n}\n",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "Python repeated same-named functions with a Holder/Base",
        extension: "py",
        source: |n| {
            repeat(
                "",
                |_| {
                    "def review_probe():\n    class ReviewHolder:\n        class ReviewBase:\n            pass\n    \
                     class ReviewChild(ReviewHolder.ReviewBase):\n        pass\n"
                        .to_string()
                },
                "",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "C++ namespace reopened per declaration",
        extension: "cpp",
        source: |n| {
            repeat(
                "",
                |i| {
                    format!(
                        "namespace ReviewNs {{ struct ReviewBase{i} {{}}; \
                         struct ReviewChild{i} : ReviewNs::ReviewBase{i} {{}}; }}\n"
                    )
                },
                "",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "TypeScript namespace reopened per declaration",
        extension: "ts",
        source: |n| {
            repeat(
                "",
                |i| {
                    format!(
                        "namespace ReviewNs {{ export interface ReviewBase{i} {{}} \
                         export interface ReviewChild{i} extends ReviewNs.ReviewBase{i} {{}} }}\n"
                    )
                },
                "",
                n,
            )
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 0,
    },
    WorkShape {
        label: "Java generic owner with one type parameter and one nested pair per declaration",
        extension: "java",
        source: |n| {
            let params: Vec<String> = (0..n).map(|i| format!("P{i}")).collect();
            let mut source = format!("class ReviewOuter<{}> {{\n", params.join(", "));
            for i in 0..n {
                let _ = writeln!(source, "  class ReviewBase{i} {{}}\n  class ReviewChild{i} extends ReviewBase{i} {{}}");
            }
            source.push_str("}\n");
            source
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::EachToItsOwn,
        parameter_units: 1,
    },
    WorkShape {
        label: "Rust trait with one supertrait per declaration",
        extension: "rs",
        source: |n| {
            let mut source = String::new();
            for i in 0..n {
                let _ = writeln!(source, "pub trait ReviewBase{i} {{}}");
            }
            let bounds: Vec<String> = (0..n).map(|i| format!("ReviewBase{i}")).collect();
            let _ = writeln!(source, "pub trait ReviewChild: {} {{}}", bounds.join(" + "));
            source
        },
        child: "ReviewChild",
        base: "ReviewBase",
        expect: Expect::OneToAll,
        parameter_units: 0,
    },
    WorkShape {
        label: "Rust self type with one positive trait impl per declaration",
        extension: "rs",
        source: |n| {
            repeat(
                "pub struct ReviewSelf;\n",
                |i| format!("pub trait ReviewTrait{i} {{}}\nimpl ReviewTrait{i} for ReviewSelf {{}}\n"),
                "",
                n,
            )
        },
        child: "ReviewSelf",
        base: "ReviewTrait",
        expect: Expect::OneToAll,
        parameter_units: 0,
    },
];

fn is_type_kind(kind: SymbolKindV2) -> bool {
    matches!(
        kind,
        SymbolKindV2::Type | SymbolKindV2::Class | SymbolKindV2::Interface | SymbolKindV2::Trait | SymbolKindV2::Enum
    )
}

/// The type records whose name starts with `prefix`, in source order.
fn named<'r>(records: &'r [SymbolRecord], prefix: &str) -> Vec<&'r SymbolRecord> {
    let mut found: Vec<&SymbolRecord> = records
        .iter()
        .filter(|record| is_type_kind(record.kind) && record.identity.name.starts_with(prefix))
        .collect();
    found.sort_by_key(|record| record.source.declaration_span.start_byte);
    found
}

/// Every base a record writes, in list order.
fn bases(record: &SymbolRecord) -> Vec<&SymbolRef> {
    let relations = &record.relations;
    relations.extends.iter().chain(&relations.implements).chain(&relations.unclassified_bases).collect()
}

fn check(shape: &WorkShape, records: &[SymbolRecord], n: usize) -> Result<(), String> {
    let children = named(records, shape.child);
    let targets = named(records, shape.base);
    let ids = |record: &SymbolRecord| -> Vec<Option<SymbolId>> { bases(record).iter().map(|base| base.id.clone()).collect() };
    match shape.expect {
        Expect::EachToTheOne => {
            let [only] = targets[..] else { return Err(format!("{} `{}` records", targets.len(), shape.base)) };
            if children.len() != n {
                return Err(format!("{} children for {n}", children.len()));
            }
            for child in children {
                if ids(child) != [Some(only.id.clone())] {
                    return Err(format!("`{}` resolves {:?}", child.identity.name, ids(child)));
                }
            }
        }
        Expect::EachToItsOwn => {
            if children.len() != n || targets.len() != n {
                return Err(format!("{} children and {} bases for {n}", children.len(), targets.len()));
            }
            for (position, (child, target)) in children.iter().zip(&targets).enumerate() {
                if ids(child) != [Some(target.id.clone())] {
                    return Err(format!("child {position} resolves {:?}, not its own base", ids(child)));
                }
            }
        }
        Expect::Unresolved => {
            if children.len() != n { return Err(format!("{} children for {n}", children.len())); }
            for child in children {
                if ids(child) != [None] || bases(child)[0].qualified_name.is_some() {
                    return Err(format!("`{}` must retain an ambiguous textual base", child.identity.name));
                }
            }
        }
        Expect::OneToAll => {
            let [only] = children[..] else { return Err(format!("{} `{}` records", children.len(), shape.child)) };
            let expected: Vec<Option<SymbolId>> = targets.iter().map(|target| Some(target.id.clone())).collect();
            if expected.len() != n {
                return Err(format!("{} bases for {n}", expected.len()));
            }
            if ids(only) != expected {
                return Err("the bases are not every target in source order".to_string());
            }
            let written: Vec<&str> = bases(only).iter().map(|base| base.name.as_str()).collect();
            let declared: Vec<&str> = targets.iter().map(|target| target.identity.name.as_str()).collect();
            if written != declared {
                return Err("the written bases are not in source order".to_string());
            }
        }
    }
    Ok(())
}

/// One declarations-only analysis of `source`: its counted steps and records.
fn measure(dir: &TempDir, name: &str, source: &str) -> (u64, Vec<SymbolRecord>) {
    let path = dir.path().join(name);
    std::fs::write(&path, source).expect("write source");
    let file = TreeFile::new(&path).expect("parse");
    let before = work_counters();
    let records = file.query(&SymbolQuery::new()).expect("query").index.symbols;
    let work = work_counters().since(before);
    assert_eq!(work.parser_constructions, 0, "{name}: the query reuses the parse");
    assert_eq!(work.definition_captures, 1, "{name}: one definitions query");
    assert_eq!(work.relation_links, 1, "{name}: one relation link");
    (work.shared_context_steps, records)
}

#[test]
fn repeated_names_keep_relation_work_linear() {
    let dir = TempDir::new().expect("temp dir");
    let mut failures = String::new();
    for (position, shape) in WORK_SHAPES.iter().enumerate() {
        let mut measured = Vec::new();
        for n in [SMALL, LARGE] {
            let (steps, records) = measure(&dir, &format!("shape{position}_{n}.{}", shape.extension), &(shape.source)(n));
            if let Err(problem) = check(shape, &records, n) {
                let _ = writeln!(failures, "{} (n = {n}): {problem}", shape.label);
            }
            measured.push((steps, records.len()));
        }
        let [(small, _), (large, records)] = measured[..] else { unreachable!("two sizes") };
        if !grows_linearly(small, large, records + shape.parameter_units * LARGE) {
            let _ = writeln!(
                failures,
                "{}: {small} steps for {SMALL}, {large} for {LARGE} ({records} records)",
                shape.label
            );
        }
    }
    assert!(failures.is_empty(), "relation work or results broke on repeated names:\n{failures}");
}

/// `depth` nested inline namespaces inside `ReviewNs` around one Base; one
/// child names it by the outer namespace alone, one through the innermost
/// inline namespace only.
fn inline_source(depth: usize) -> String {
    let mut source = "namespace ReviewNs {\n".to_string();
    for level in 0..depth {
        let _ = writeln!(source, "inline namespace v{level} {{");
    }
    source.push_str("struct ReviewBase {};\n");
    source.push_str(&"}\n".repeat(depth + 1));
    let innermost = depth - 1;
    let full = (0..depth).map(|level| format!("v{level}")).collect::<Vec<_>>().join("::");
    let _ = writeln!(source, "struct ReviewChildOuter : ReviewNs::ReviewBase {{}};");
    let _ = writeln!(source, "struct ReviewChildInner : ReviewNs::v{innermost}::ReviewBase {{}};");
    let _ = writeln!(source, "struct ReviewChildFull : ReviewNs::{full}::ReviewBase {{}};");
    source
}

/// Each written path resolves without materializing the `2^depth` spellings
/// of the Base: the counted work grows linearly in the depth.
#[test]
fn nested_inline_namespaces_cost_linear_work_in_their_depth() {
    let dir = TempDir::new().expect("temp dir");
    let mut steps = Vec::new();
    for depth in [4, 8, 16] {
        let (counted, records) = measure(&dir, &format!("inline_{depth}.cpp"), &inline_source(depth));
        let [base] = named(&records, "ReviewBase")[..] else { panic!("depth {depth}: one Base") };
        for child in named(&records, "ReviewChild") {
            let resolved: Vec<Option<&SymbolId>> = bases(child).iter().map(|target| target.id.as_ref()).collect();
            assert_eq!(resolved, [Some(&base.id)], "depth {depth}: `{}` names the Base", child.identity.name);
        }
        steps.push((depth, counted));
    }
    for pair in steps.windows(2) {
        let [(depth, small), (deeper, large)] = pair else { unreachable!("pairs") };
        assert!(
            *large * 2 <= *small * 5 + 64,
            "inline-namespace work grew faster than the depth: {small} steps at {depth}, {large} at {deeper}"
        );
    }
}

/// Bound subject lookup grows with parameters and clauses, including both
/// Swift clause locations. Counters must actually visit the parameter lists.
#[test]
fn separately_written_generic_bounds_cost_linear_work() {
    let dir = TempDir::new().expect("temp dir");
    let mut failures = Vec::new();
    for extension in ["rs", "cs", "swift", "inside.swift"] {
        let mut steps = Vec::new();
        for n in [SMALL, LARGE] {
            let names = (0..n).map(|i| format!("T{i}")).collect::<Vec<_>>().join(", ");
            let bounds = (0..n).map(|i| match extension {
                "rs" => format!("T{i}: Clone"), "cs" => format!("where T{i}: class"), _ => format!("T{i}: ReviewP"),
            }).collect::<Vec<_>>().join(if extension == "cs" { " " } else { ", " });
            let source = match extension {
                "rs" => format!("fn review_probe<{names}>() where {bounds} {{}}"),
                "cs" => format!("class ReviewProbe<{names}> {bounds} {{}}"),
                "inside.swift" => format!("protocol ReviewP {{}}\nstruct ReviewProbe<{names} where {bounds}> {{}}"),
                _ => format!("protocol ReviewP {{}}\nstruct ReviewProbe<{names}> where {bounds} {{}}"),
            };
            let (count, records) = measure(&dir, &format!("bounds_{n}.{extension}"), &source);
            let owner = records.iter().find(|record| record.identity.name.starts_with("review_probe") || record.identity.name == "ReviewProbe").expect("generic owner");
            assert_eq!(owner.type_info.generic_params.len(), n, "{extension}");
            for (i, param) in owner.type_info.generic_params.iter().enumerate() {
                assert_eq!(param.position as usize, i);
                assert_eq!(param.name, format!("T{i}"));
                assert_eq!(param.constraints, [match extension { "rs" => "Clone", "cs" => "class", _ => "ReviewP" }]);
            }
            assert!(count >= n as u64, "{extension}: {count} does not count {n} parameters");
            steps.push(count);
        }
        if steps[1] > steps[0] * 2 + 64 { failures.push(format!("{extension}: nonlinear bound work {steps:?}")); }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

