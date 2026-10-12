//! Selective analysis (`2026-10-03-symbol-improvements`, Phase 3): pass
//! planning, the work counters, `SourceFile`, `SymbolQuery` kind and
//! capability selection, and the bind-pass behaviors that changed with it.
//!
//! The counter tests read process-wide counts; nextest runs each test in its
//! own process, so no other analysis runs between two snapshots.

use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;
use tree_hugger::ProgrammingLanguage;
use tree_hugger::{
    AnalysisPass, Capability, MetricsFacet, SourceFile, SymbolId, SymbolKindV2, SymbolQuery, SymbolRecord, TreeFile,
    TreeHuggerError, VisibilityLevel, WorkCounters, work_counters,
};

use crate::symbols_fixtures::{display_name, multi_fixtures};
use crate::symbols_kinds::deferred_kind_sources;

const FUNCTION_KINDS: [SymbolKindV2; 3] = [
    SymbolKindV2::Function,
    SymbolKindV2::Method,
    SymbolKindV2::Constructor,
];
const TYPE_KINDS: [SymbolKindV2; 6] = [
    SymbolKindV2::Type,
    SymbolKindV2::Class,
    SymbolKindV2::Interface,
    SymbolKindV2::Enum,
    SymbolKindV2::Trait,
    SymbolKindV2::TypeAlias,
];
const CLASS_KINDS: [SymbolKindV2; 6] = [
    SymbolKindV2::Class,
    SymbolKindV2::Type,
    SymbolKindV2::Interface,
    SymbolKindV2::Trait,
    SymbolKindV2::Method,
    SymbolKindV2::Constructor,
];

fn counted<T>(work: impl FnOnce() -> T) -> (T, WorkCounters) {
    let before = work_counters();
    let value = work();
    (value, work_counters().since(before))
}

fn write(dir: &TempDir, name: &str, source: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, source).expect("write source");
    path
}

#[test]
fn a_symbol_query_over_multi_runs_no_diagnostics_and_one_capture_per_file() {
    let files = multi_fixtures();
    let query = SymbolQuery::new().kinds(FUNCTION_KINDS);

    let (results, work) = counted(|| {
        files
            .iter()
            .map(|path| SourceFile::read(path).expect("read").query(&query).expect("query"))
            .collect::<Vec<_>>()
    });

    let file_count = files.len() as u64;
    assert_eq!(file_count, 16, "multi/ holds one file per language");
    assert_eq!(work.diagnostics_runs, 0, "a symbol listing ran diagnostics");
    assert_eq!(work.parser_constructions, file_count, "one parse per file");
    assert_eq!(work.definition_captures, file_count, "one definition capture per file");

    for (path, result) in files.iter().zip(&results) {
        let label = display_name(path);
        assert_eq!(result.index.completed_passes, [AnalysisPass::Parse], "{label}");
        assert_eq!(result.completed_capabilities, [Capability::Declarations], "{label}");
        assert!(result.index.diagnostics.is_empty(), "{label}: diagnostics without the pass");
        assert!(
            result
                .index
                .symbols
                .iter()
                .all(|record| FUNCTION_KINDS.contains(&record.kind)),
            "{label}: a record outside the selected kinds"
        );
    }
    assert!(
        results.iter().filter(|result| !result.index.symbols.is_empty()).count() >= 14,
        "most multi/ files declare a function"
    );
}

#[test]
fn the_complete_index_and_lint_still_run_diagnostics_from_one_capture() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        &dir,
        "lint_me.py",
        "import os\n\n\ndef unreachable():\n    return 1\n    value = 2\n",
    );
    let tree_file = TreeFile::new(&path).expect("parse");

    let (index, work) = counted(|| tree_file.symbol_index_v2().expect("index"));
    assert_eq!(work.diagnostics_runs, 1, "symbol_index_v2() runs diagnostics");
    assert_eq!(work.definition_captures, 1, "records, references and diagnostics share one capture");
    assert_eq!(work.parser_constructions, 0, "the file is already parsed");
    assert_eq!(
        index.completed_passes,
        [
            AnalysisPass::Parse,
            AnalysisPass::Bind,
            AnalysisPass::Semantic,
            AnalysisPass::Docs,
            AnalysisPass::Diagnostics,
        ]
    );

    let (direct, work) = counted(|| tree_file.try_diagnostics().expect("diagnostics"));
    assert_eq!(work.diagnostics_runs, 1);
    assert_eq!(work.definition_captures, 1, "try_diagnostics() captures definitions once");
    let rules = |diagnostics: &[tree_hugger::Diagnostic]| {
        let mut rules: Vec<String> = diagnostics.iter().filter_map(|d| d.rule.clone()).collect();
        rules.sort();
        rules
    };
    assert_eq!(rules(&index.diagnostics), ["dead-code", "unused-import"]);
    assert_eq!(
        serde_json::to_value(&index.diagnostics).expect("serialize"),
        serde_json::to_value(&direct).expect("serialize"),
        "the diagnostics pass reports what try_diagnostics() reports"
    );

    let (lint, work) = counted(|| tree_file.lint_diagnostics());
    assert_eq!(work.diagnostics_runs, 1, "lint_diagnostics() runs diagnostics");
    assert_eq!(lint.len(), 2);

    // Without the pass, no diagnostics and no `Diagnostics` entry: an empty
    // list is not evidence that the file is clean.
    let (without, work) = counted(|| {
        tree_file
            .symbol_index_with_passes(&[
                AnalysisPass::Parse,
                AnalysisPass::Bind,
                AnalysisPass::Semantic,
                AnalysisPass::Docs,
            ])
            .expect("index")
    });
    assert_eq!(work.diagnostics_runs, 0);
    assert!(without.diagnostics.is_empty());
    assert!(!without.completed_passes.contains(&AnalysisPass::Diagnostics));
}

#[test]
fn passes_run_once_after_their_prerequisites() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(&dir, "plan.rs", "/// Docs.\nfn main() { helper(); }\nfn helper() {}\n");
    let tree_file = TreeFile::new(&path).expect("parse");
    let cases: [(&[AnalysisPass], &[AnalysisPass]); 6] = [
        (&[], &[AnalysisPass::Parse]),
        (&[AnalysisPass::Bind], &[AnalysisPass::Parse, AnalysisPass::Bind]),
        (
            &[AnalysisPass::Docs, AnalysisPass::Semantic, AnalysisPass::Semantic],
            &[
                AnalysisPass::Parse,
                AnalysisPass::Bind,
                AnalysisPass::Semantic,
                AnalysisPass::Docs,
            ],
        ),
        (&[AnalysisPass::Docs], &[AnalysisPass::Parse, AnalysisPass::Docs]),
        (&[AnalysisPass::Diagnostics], &[AnalysisPass::Parse, AnalysisPass::Diagnostics]),
        (
            &[AnalysisPass::Diagnostics, AnalysisPass::Parse, AnalysisPass::Bind],
            &[AnalysisPass::Parse, AnalysisPass::Bind, AnalysisPass::Diagnostics],
        ),
    ];
    for (requested, completed) in cases {
        let (index, work) = counted(|| tree_file.symbol_index_with_passes(requested).expect("index"));
        assert_eq!(index.completed_passes, completed, "requested {requested:?}");
        assert_eq!(
            work.diagnostics_runs,
            u64::from(completed.contains(&AnalysisPass::Diagnostics)),
            "requested {requested:?}: diagnostics run only when planned"
        );
        assert_eq!(work.definition_captures, 1, "requested {requested:?}");
        let binds = completed.contains(&AnalysisPass::Bind);
        assert_eq!(
            work.reference_captures,
            u64::from(binds || completed.contains(&AnalysisPass::Diagnostics)),
            "requested {requested:?}: one references run serves the occurrences and the diagnostics"
        );
        assert_eq!(
            work.occurrence_resolutions,
            u64::from(binds),
            "requested {requested:?}: edges and recursion read one resolution"
        );
    }

    assert_eq!(
        SymbolQuery::new()
            .capabilities([Capability::Semantics])
            .required_capabilities(),
        [
            Capability::Declarations,
            Capability::Containment,
            Capability::References,
            Capability::Occurrences,
            Capability::Semantics,
        ]
    );
    assert_eq!(
        SymbolQuery::new()
            .capabilities([Capability::Occurrences])
            .required_capabilities(),
        [
            Capability::Declarations,
            Capability::Containment,
            Capability::References,
            Capability::Occurrences,
        ]
    );
    assert_eq!(
        SymbolQuery::new()
            .capabilities([Capability::StructuredDocs, Capability::StructuredDocs])
            .required_capabilities(),
        [Capability::Declarations, Capability::StructuredDocs]
    );
    assert_eq!(SymbolQuery::complete_index().required_capabilities(), Capability::ALL);
}

/// Flags and metrics come with the declarations: no reference binding,
/// semantics, structured docs or diagnostics run for them, and records of
/// unselected kinds get neither.
#[test]
fn flags_and_metrics_run_with_the_declarations_only() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        &dir,
        "flags.rs",
        "/// Docs.\n#[deprecated]\nfn old(a: u8) { helper(); }\nfn helper() {}\nstruct S;\n",
    );
    let tree_file = TreeFile::new(&path).expect("parse");
    let query = SymbolQuery::new()
        .kinds([SymbolKindV2::Function])
        .capabilities([Capability::Flags, Capability::Metrics]);
    let (result, work) = counted(|| tree_file.query(&query).expect("query"));

    assert_eq!(result.index.completed_passes, [AnalysisPass::Parse]);
    assert_eq!(
        result.completed_capabilities,
        [Capability::Declarations, Capability::Flags, Capability::Metrics]
    );
    assert_eq!(work.diagnostics_runs, 0);
    assert_eq!(work.definition_captures, 1);
    let named = |name: &str| -> &SymbolRecord {
        result
            .index
            .symbols
            .iter()
            .find(|record| record.identity.name == name)
            .unwrap_or_else(|| panic!("no `{name}` record"))
    };
    let old = named("old");
    assert!(old.visibility.is_deprecated);
    assert!(!old.visibility.is_test, "a scratch file outside any package `tests/`");
    assert_eq!(
        old.metrics,
        Some(MetricsFacet {
            parameters: Some(1),
            has_docs: Some(true),
            lines: Some(1),
            cyclomatic: Some(1),
        })
    );
    assert!(old.relations.references.is_empty(), "references were not bound");
    assert!(old.relations.parent.is_none(), "containment was not requested");
    assert_eq!(old.semantics.is_recursive, None, "semantics were not requested");
    assert!(old.docs.parsed.is_none(), "structured docs were not requested");
    assert_eq!(
        named("helper").metrics,
        Some(MetricsFacet {
            parameters: Some(0),
            has_docs: Some(false),
            lines: Some(1),
            cyclomatic: Some(1),
        })
    );

    let plain = tree_file.query(&SymbolQuery::new().kinds([SymbolKindV2::Function])).expect("query");
    assert!(plain.index.symbols.iter().all(|record| record.metrics.is_none()
        && !record.visibility.is_deprecated
        && !record.visibility.is_test));
}

pub(crate) type ComplexitySource = (&'static str, &'static str, &'static [(&'static str, Option<u32>)]);

/// One source per complexity case: a reviewed language's callables with a
/// body (`Some`), bodyless ones (`None`), and a language without counting
/// rules (`None` for every record, the other metrics still measured), type
/// syntax beside a counted value in the body, and switch label groups.
/// `(file name, source, expected cyclomatic by name)`.
pub(crate) const COMPLEXITY_SOURCES: [ComplexitySource; 8] = [
    (
        "walks.rs",
        "fn branch(x: bool) -> u8 { if x { 1 } else { 2 } }\nstruct S;\nimpl S { fn both(&self) -> bool { true && false } }\ntrait T { fn bodyless(&self); }\n",
        &[("branch", Some(2)), ("both", Some(2)), ("bodyless", None), ("S", None), ("T", None)],
    ),
    (
        "walks.js",
        "function coalesce(a) { return a ?? 1; }\nclass K { plain() { return 1; } }\n",
        &[("coalesce", Some(2)), ("plain", Some(1)), ("K", None)],
    ),
    (
        "walks.ts",
        "function sign(a: number): number { return a > 0 ? 1 : 0; }\ninterface I { declared(): void; }\n",
        &[("sign", Some(2)), ("declared", None), ("I", None)],
    ),
    (
        "walks.go",
        "package p\n\nfunc branch(a bool) int {\n\tif a {\n\t\treturn 1\n\t}\n\treturn 0\n}\n",
        &[("branch", None)],
    ),
    (
        "types.rs",
        "fn typed(a: bool) { let _x: [u8; if true { 1 } else { 2 }] = [0; if a { 1 } else { 2 }]; }\n",
        &[("typed", Some(2))],
    ),
    (
        "types.ts",
        "function typed(a: boolean) { let x: { [a ? 1 : 2]: string } = {} as { [a ? 1 : 2]: string }; return a || x; }\n",
        &[("typed", Some(2))],
    ),
    (
        "types.tsx",
        "function typed(a: boolean) { return {} satisfies { [a ? 1 : 2]: string } ?? a; }\n",
        &[("typed", Some(2))],
    ),
    (
        "switches.js",
        "function grouped(x) { switch (x) { default: case 1: break; case 2: case 3: break; } }\n",
        &[("grouped", Some(2))],
    ),
];

/// The complexity walk runs only under `Capability::Metrics`, once per
/// measured callable of a language with counting rules: no other capability
/// (alone or all together) walks a body, a language without rules walks
/// none, and the complete index walks no more than a metrics-only query.
/// Record IDs and qualified names do not depend on whether it ran.
#[test]
fn complexity_is_walked_only_for_metrics_once_per_measured_callable() {
    let dir = TempDir::new().expect("temp");
    let without_metrics: Vec<Capability> =
        Capability::ALL.into_iter().filter(|capability| *capability != Capability::Metrics).collect();
    for (name, source, expected) in COMPLEXITY_SOURCES {
        let path = write(&dir, name, source);
        let mut identities: Option<Vec<(SymbolId, String)>> = None;
        let queries = [
            SymbolQuery::new(),
            SymbolQuery::new().capabilities([Capability::Flags]),
            SymbolQuery::new().capabilities(without_metrics.iter().copied()),
            SymbolQuery::new().capabilities([Capability::Metrics]),
            SymbolQuery::complete_index(),
        ];
        for query in queries {
            let cell = format!("{name}: {query:?}");
            let (result, work) =
                counted(|| SourceFile::read(&path).expect("read").query(&query).expect("query"));
            let records = &result.index.symbols;
            let identity: Vec<(SymbolId, String)> =
                records.iter().map(|record| (record.id.clone(), record.identity.qualified_name.clone())).collect();
            assert_eq!(identities.get_or_insert_with(|| identity.clone()), &identity, "{cell}: IDs and qualified names");
            if !query.requires(Capability::Metrics) {
                assert_eq!(work.complexity_walks, 0, "{cell}: walks without metrics");
                assert!(records.iter().all(|record| record.metrics.is_none()), "{cell}: a metrics facet");
                continue;
            }
            for (symbol, cyclomatic) in expected {
                let record = records
                    .iter()
                    .find(|record| record.identity.name == *symbol)
                    .unwrap_or_else(|| panic!("{cell}: no `{symbol}` record"));
                let metrics = record.metrics.as_ref().unwrap_or_else(|| panic!("{cell}: `{symbol}` has no facet"));
                assert_eq!(metrics.cyclomatic, *cyclomatic, "{cell}: `{symbol}` cyclomatic");
                assert!(metrics.has_docs.is_some(), "{cell}: `{symbol}` lost has_docs beside complexity");
            }
            let measured = records
                .iter()
                .filter(|record| record.metrics.as_ref().is_some_and(|metrics| metrics.cyclomatic.is_some()))
                .count();
            assert_eq!(work.complexity_walks, measured as u64, "{cell}: one walk per measured callable");
        }
    }
    // The language without rules still measures its function's other metrics.
    let go = write(&dir, "walks.go", COMPLEXITY_SOURCES[3].1);
    let metrics = TreeFile::new(&go).expect("parse").query(&SymbolQuery::new().capabilities([Capability::Metrics]))
        .expect("query")
        .index
        .symbols
        .into_iter()
        .find(|record| record.identity.name == "branch")
        .and_then(|record| record.metrics)
        .expect("Go metrics");
    assert_eq!(
        metrics,
        MetricsFacet { parameters: Some(1), has_docs: Some(false), lines: Some(6), cyclomatic: None }
    );
}

#[test]
fn containment_alone_links_members_without_binding_references() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        &dir,
        "shapes.ts",
        "export class Shape {\n  area(): number { return helper(); }\n}\nfunction helper(): number { return 1; }\n",
    );
    let tree_file = TreeFile::new(&path).expect("parse");
    let containment = tree_file
        .query(&SymbolQuery::new().capabilities([Capability::Containment]))
        .expect("containment");
    let full = tree_file.symbol_index_v2().expect("index");

    assert_eq!(containment.index.completed_passes, [AnalysisPass::Parse]);
    assert_eq!(
        containment.completed_capabilities,
        [Capability::Declarations, Capability::Containment]
    );
    let named = |records: &[SymbolRecord], name: &str| -> SymbolRecord {
        records
            .iter()
            .find(|record| record.identity.name == name)
            .cloned()
            .unwrap_or_else(|| panic!("no `{name}` record"))
    };
    let (shape, area) = (named(&containment.index.symbols, "Shape"), named(&containment.index.symbols, "area"));
    let (full_shape, full_area) = (named(&full.symbols, "Shape"), named(&full.symbols, "area"));
    assert_eq!(area.relations.parent.as_ref().map(|r| &r.id), Some(&Some(shape.id.clone())));
    assert_eq!(
        serde_json::to_value(&shape.relations.members).expect("serialize"),
        serde_json::to_value(&full_shape.relations.members).expect("serialize")
    );
    assert!(!full_area.relations.references.is_empty(), "the bind pass resolves `helper`");
    assert!(area.relations.references.is_empty(), "references were not requested");
    assert_eq!(area.semantics.is_recursive, None, "semantics were not requested");
}

#[test]
fn source_file_parses_the_bytes_it_hashed() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(&dir, "edited.rs", "fn before() {}\n");
    let source_file = SourceFile::read(&path).expect("read");
    std::fs::write(&path, "fn after() {}\n").expect("edit after read");

    assert_eq!(source_file.source(), "fn before() {}\n");
    let hash = source_file.hash().to_string();
    let (tree_file, work) = counted(|| source_file.parse().expect("parse"));
    assert_eq!(work.parser_constructions, 1);
    assert_eq!(tree_file.hash, hash, "the parsed file keeps the hash of the bytes read");
    let names: Vec<String> = tree_file
        .symbols()
        .expect("symbols")
        .into_iter()
        .map(|symbol| symbol.name)
        .collect();
    assert_eq!(names, ["before"], "parsing must not reread the edited file");

    let reread = TreeFile::new(&path).expect("reread");
    assert_ne!(reread.hash, hash, "the hash follows the content");
    assert_eq!(
        SourceFile::read(&path).expect("read").hash(),
        reread.hash,
        "SourceFile and TreeFile hash the same text the same way"
    );
}

/// `SourceFile` keeps `TreeFile`'s error contract: the same variant for each
/// failure, and a forced language overrides detection.
#[test]
fn source_file_and_tree_file_report_the_same_errors() {
    let dir = TempDir::new().expect("tempdir");
    let missing = dir.path().join("missing.rs");
    let unsupported = write(&dir, "notes.unknown", "fn f() {}\n");
    let not_utf8 = dir.path().join("latin1.rs");
    std::fs::write(&not_utf8, b"fn caf\xe9() {}\n").expect("write bytes");

    let variant = |error: &TreeHuggerError| match error {
        TreeHuggerError::Io { source, .. } => format!("io:{:?}", source.kind()),
        TreeHuggerError::UnsupportedLanguage { .. } => "unsupported_language".to_string(),
        other => format!("other:{other}"),
    };
    let cases: [(&Path, &str); 3] = [
        (&missing, "io:NotFound"),
        (&unsupported, "unsupported_language"),
        (&not_utf8, "io:InvalidData"),
    ];
    for (path, expected) in cases {
        let from_source = SourceFile::read(path).expect_err("SourceFile::read must fail");
        let from_tree = TreeFile::new(path).expect_err("TreeFile::new must fail");
        assert_eq!(variant(&from_source), expected, "{}", path.display());
        assert_eq!(variant(&from_tree), expected, "{}", path.display());
    }

    let forced = SourceFile::read_with_language(&unsupported, Some(tree_hugger::ProgrammingLanguage::Rust))
        .expect("forced language")
        .query(&SymbolQuery::new())
        .expect("query");
    assert_eq!(forced.index.language, tree_hugger::ProgrammingLanguage::Rust);
    assert_eq!(forced.index.symbols.len(), 1);
}

/// Serialized records with the fields every query computes the same way.
fn as_values(records: &[SymbolRecord]) -> Vec<Value> {
    records
        .iter()
        .map(|record| serde_json::to_value(record).expect("serialize record"))
        .collect()
}

crate::language_cases::language_tests!(narrow_queries_equal_the_filtered_records_of_a_full_query => check_narrow_queries_equal_the_filtered_records_of_a_full_query);

/// For every fixture of every language, a narrow query returns exactly the
/// selected kinds' records of a full query with the same capabilities:
/// identity, spans, container, metadata, relations and semantics alike.
fn check_narrow_queries_equal_the_filtered_records_of_a_full_query(language: ProgrammingLanguage) {
    let kind_sets: [(&str, &[SymbolKindV2]); 6] = [
        ("functions", &FUNCTION_KINDS),
        ("types", &TYPE_KINDS),
        ("classes", &CLASS_KINDS),
        ("methods only", &[SymbolKindV2::Method]),
        ("fields only", &[SymbolKindV2::Field]),
        ("variables only", &[SymbolKindV2::Variable]),
    ];
    let capability_sets: [&[Capability]; 3] = [
        &[Capability::Declarations],
        &[Capability::Containment],
        &[Capability::Semantics, Capability::StructuredDocs],
    ];
    let mut compared = vec![0usize; kind_sets.len()];
    let mut parents_outside_selection = 0;
    let mut classes_fields_with_access = 0;

    for path in crate::symbols_fixtures::fixtures_for(language, true) {
        let label = display_name(&path);
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        for capabilities in capability_sets {
            let full = tree_file
                .query(&SymbolQuery::new().capabilities(capabilities.iter().copied()))
                .expect("full query");
            for (set_idx, (set_name, kinds)) in kind_sets.iter().enumerate() {
                let query = SymbolQuery::new()
                    .kinds(kinds.iter().copied())
                    .capabilities(capabilities.iter().copied());
                let narrow = tree_file.query(&query).expect("narrow query");
                let expected: Vec<SymbolRecord> = full
                    .index
                    .symbols
                    .iter()
                    .filter(|record| kinds.contains(&record.kind))
                    .cloned()
                    .collect();
                assert_eq!(
                    as_values(&narrow.index.symbols),
                    as_values(&expected),
                    "{label}: {set_name} with {capabilities:?}"
                );
                assert_eq!(narrow.index.completed_passes, full.index.completed_passes, "{label}");
                assert_eq!(narrow.completed_capabilities, full.completed_capabilities, "{label}");
                compared[set_idx] += expected.len();

                if *set_name == "methods only" && capabilities.contains(&Capability::Containment) {
                    parents_outside_selection += narrow
                        .index
                        .symbols
                        .iter()
                        .filter(|record| record.relations.parent.as_ref().is_some_and(|parent| parent.id.is_some()))
                        .count();
                }
                if *set_name == "classes" {
                    classes_fields_with_access += narrow
                        .index
                        .symbols
                        .iter()
                        .filter_map(|record| match &record.kind_data {
                            tree_hugger::SymbolKindData::Type(data) => Some(&data.fields),
                            _ => None,
                        })
                        .flatten()
                        .filter(|field| {
                            field.is_static
                                || field
                                    .visibility
                                    .is_some_and(|level| level != VisibilityLevel::Unknown)
                        })
                        .count();
                }
            }
        }
    }

    let minimum = crate::language_cases::corpus_minimums(language);
    assert!(parents_outside_selection >= minimum.parents, "{language}: unselected parents corpus shrank");
    assert!(classes_fields_with_access >= minimum.fields, "{language}: class fields corpus shrank");
    if language != ProgrammingLanguage::Rust { return; }
    for ((set_name, _), count) in kind_sets.iter().zip(&compared) {
        assert!(*count > 0, "{set_name}: no fixture record was compared");
    }

}

/// The kinds the deferred declarations report.
const DEFERRED_KINDS: [SymbolKindV2; 5] = [
    SymbolKindV2::Constructor,
    SymbolKindV2::EnumVariant,
    SymbolKindV2::TypeAlias,
    SymbolKindV2::Field,
    SymbolKindV2::Namespace,
];

/// For every source of the deferred kind matrix, a query of one deferred
/// kind returns exactly that kind's records of a full query with the same
/// capabilities. Each narrow query runs one definitions capture, no parse and
/// no diagnostics; without a capability that needs them it binds no
/// references and computes no semantics.
#[test]
fn deferred_kind_queries_equal_the_same_kind_of_a_full_query() {
    let capability_sets: [&[Capability]; 3] = [
        &[Capability::Declarations],
        &[Capability::Containment, Capability::Flags, Capability::Metrics],
        &[Capability::Semantics, Capability::StructuredDocs],
    ];
    let dir = TempDir::new().expect("tempdir");
    let mut compared = [0usize; DEFERRED_KINDS.len()];
    for (ordinal, (language, source)) in deferred_kind_sources().into_iter().enumerate() {
        let path = write(&dir, &format!("deferred_{ordinal}.{}", language.extensions()[0]), source);
        let label = format!("{language:?} {source:?}");
        let tree_file = TreeFile::new(&path).expect("parse");
        for capabilities in capability_sets {
            let full = tree_file
                .query(&SymbolQuery::new().capabilities(capabilities.iter().copied()))
                .expect("full query");
            for (kind_idx, kind) in DEFERRED_KINDS.into_iter().enumerate() {
                let query = SymbolQuery::new().kinds([kind]).capabilities(capabilities.iter().copied());
                let (narrow, work) = counted(|| tree_file.query(&query).expect("narrow query"));
                let expected: Vec<SymbolRecord> =
                    full.index.symbols.iter().filter(|record| record.kind == kind).cloned().collect();
                assert_eq!(
                    as_values(&narrow.index.symbols),
                    as_values(&expected),
                    "{label}: {kind:?} with {capabilities:?}"
                );
                assert_eq!(narrow.index.completed_passes, full.index.completed_passes, "{label}: {kind:?}");
                assert_eq!(narrow.completed_capabilities, full.completed_capabilities, "{label}: {kind:?}");
                assert_eq!(
                    (work.parser_constructions, work.definition_captures, work.diagnostics_runs),
                    (0, 1, 0),
                    "{label}: {kind:?} with {capabilities:?}: parses, captures, diagnostics"
                );
                if capabilities == [Capability::Declarations] {
                    assert_eq!(narrow.index.completed_passes, [AnalysisPass::Parse], "{label}: {kind:?}");
                    for record in &narrow.index.symbols {
                        assert!(record.relations.references.is_empty(), "{label}: {kind:?} bound references");
                        assert_eq!(record.semantics.is_recursive, None, "{label}: {kind:?} computed semantics");
                        assert!(record.docs.parsed.is_none(), "{label}: {kind:?} parsed docs");
                    }
                }
                compared[kind_idx] += expected.len();
            }
        }
    }
    for (kind, count) in DEFERRED_KINDS.iter().zip(compared) {
        assert!(count > 0, "{kind:?}: no record was compared");
    }
}

#[test]
fn distinct_import_references_are_all_kept() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        &dir,
        "two_imports.ts",
        "import { path } from \"os\";\nimport { argv } from \"sys\";\n\nfunction run() {\n    path.join(\"a\");\n    path.join(\"b\");\n    return argv;\n}\n",
    );
    let index = TreeFile::new(&path).expect("parse").symbol_index_v2().expect("index");
    let run = index
        .symbols
        .iter()
        .find(|record| record.identity.name == "run")
        .expect("`run` record");
    let names = |refs: &[tree_hugger::SymbolRef]| -> Vec<(String, bool)> {
        refs.iter().map(|r| (r.name.clone(), r.id.is_some())).collect()
    };
    let expected = [("path".to_string(), false), ("argv".to_string(), false)];
    assert_eq!(names(&run.relations.references), expected, "both bound imports, `path` once");
    assert_eq!(names(&run.relations.dependencies), expected);
}

crate::language_cases::language_tests!(a_wide_result_narrows_to_exactly_the_uncached_narrow_query => check_a_wide_result_narrows_to_exactly_the_uncached_narrow_query);

/// A wide result narrowed to a query equals that query run uncached, for
/// every fixture and each command's kind set; a query the wide result does
/// not cover is refused rather than answered from partial data.
fn check_a_wide_result_narrows_to_exactly_the_uncached_narrow_query(language: ProgrammingLanguage) {
    let kind_sets: [(&str, &[SymbolKindV2]); 3] = [
        ("functions", &FUNCTION_KINDS),
        ("types", &TYPE_KINDS),
        ("classes", &CLASS_KINDS),
    ];
    let mut compared = 0usize;
    for path in crate::symbols_fixtures::fixtures_for(language, true) {
        let label = display_name(&path);
        let wide_query = SymbolQuery::new().capabilities([Capability::Containment]);
        let wide = SourceFile::read(&path).expect("read").query(&wide_query).expect("wide query");
        assert_eq!(wide.selected_kinds, None, "{label}: every kind is selected");

        for (set_name, kinds) in kind_sets {
            for capabilities in [&[Capability::Declarations][..], &[Capability::Containment][..]] {
                let query = SymbolQuery::new()
                    .kinds(kinds.iter().copied())
                    .capabilities(capabilities.iter().copied());
                let uncached = SourceFile::read(&path).expect("read").query(&query).expect("query");
                let narrowed = wide.clone().narrowed(&query).unwrap_or_else(|| {
                    panic!("{label}: {set_name} {capabilities:?} is covered by the wide result")
                });
                assert_eq!(
                    serde_json::to_value(&narrowed.index).expect("serialize"),
                    serde_json::to_value(&uncached.index).expect("serialize"),
                    "{label}: {set_name} with {capabilities:?}"
                );
                assert_eq!(narrowed.completed_capabilities, uncached.completed_capabilities);
                assert_eq!(narrowed.selected_kinds, uncached.selected_kinds);
                compared += uncached.index.symbols.len();
            }
        }

        let functions = SymbolQuery::new().kinds(FUNCTION_KINDS);
        let narrow = SourceFile::read(&path).expect("read").query(&functions).expect("query");
        assert!(
            narrow.clone().narrowed(&SymbolQuery::new()).is_none(),
            "{label}: a functions-only result cannot answer an every-kind query"
        );
        assert!(
            narrow.narrowed(&functions.clone().capabilities([Capability::References])).is_none(),
            "{label}: a declarations-only result cannot answer a references query"
        );
        assert!(
            wide.narrowed(&SymbolQuery::new().capabilities([Capability::Diagnostics])).is_none(),
            "{label}: a result without diagnostics cannot answer a diagnostics query"
        );
    }
    assert!(compared >= crate::language_cases::corpus_minimums(language).symbols, "{language}: too few records compared ({compared})");
}

/// The declaration facts of one record that the query and capabilities can
/// change: generic parameters, constraints, modifiers, attributes,
/// `extends`/`implements`, containment links and the `metrics` facet.
fn declaration_facts(record: &SymbolRecord) -> Value {
    serde_json::json!({
        "generic_params": record.type_info.generic_params,
        "constraints": record.type_info.constraints,
        "modifiers": record.modifiers,
        "attributes": record.attributes,
        "extends": record.relations.extends,
        "implements": record.relations.implements,
        "unclassified_bases": record.relations.unclassified_bases,
        "parent": record.relations.parent,
        "container": record.relations.container,
        "members": record.relations.members,
        "metrics": record.metrics,
    })
}

/// [`declaration_facts`] of `record` as the complete index reports it, with
/// what `query` does not compute removed.
fn expected_facts(complete: &SymbolRecord, query: &SymbolQuery) -> Value {
    let mut facts = declaration_facts(complete);
    if !query.requires(Capability::Containment) {
        facts["parent"] = Value::Null;
        facts["container"] = Value::Null;
        facts["members"] = serde_json::json!([]);
    }
    if !query.requires(Capability::Metrics) {
        facts["metrics"] = Value::Null;
    }
    facts
}

crate::symbols_relations::declaration_fact_group_tests!(
    declaration_facts_of_narrow_queries_equal_the_complete_index => narrow_queries_equal_the_complete_index
);

/// Over every source of `group` of the declaration-fact corpus, a narrow
/// query reports, for each record it selects, the same generic parameters,
/// modifiers, attributes, relations targets (IDs included), Rust impl
/// container links and `metrics` as the complete index, under each
/// capability set; facts a capability set does not compute are absent. The
/// selected records are exactly the complete index's records of the selected
/// kinds. The Rust group must exercise every compared fact.
fn narrow_queries_equal_the_complete_index(group: usize) {
    let kind_sets: [(&str, Option<&[SymbolKindV2]>); 4] = [
        ("every kind", None),
        ("functions", Some(&FUNCTION_KINDS)),
        ("types", Some(&TYPE_KINDS)),
        ("methods only", Some(&[SymbolKindV2::Method])),
    ];
    let capability_sets: [&[Capability]; 6] = [
        &[Capability::Declarations],
        &[Capability::Containment],
        &[Capability::Metrics],
        &[Capability::Flags],
        &[Capability::Containment, Capability::Metrics],
        &Capability::ALL,
    ];
    let dir = TempDir::new().expect("temp");
    let mut compared = 0usize;
    let mut nonempty = [0usize; 6];

    for (label, path) in crate::symbols_relations::write_declaration_fact_group(dir.path(), group) {
        let source = TreeFile::new(&path).expect("parse");
        assert!(source.syntax_diagnostics().is_empty(), "{label}: {:?}", source.syntax_diagnostics());
        let complete = source.query(&SymbolQuery::complete_index()).expect("complete").index;
        for record in &complete.symbols {
            nonempty[0] += usize::from(!record.type_info.generic_params.is_empty());
            nonempty[1] += usize::from(!record.modifiers.is_empty());
            nonempty[2] += usize::from(!record.attributes.is_empty());
            nonempty[3] += usize::from(!record.relations.extends.is_empty() || !record.relations.implements.is_empty());
            nonempty[4] += usize::from(record.relations.container.as_ref().is_some_and(|c| c.id.is_none()));
            nonempty[5] += usize::from(record.metrics.as_ref().is_some_and(|m| m.lines.is_some()));
        }
        for (set_name, kinds) in kind_sets {
            for capabilities in capability_sets {
                let mut query = SymbolQuery::new().capabilities(capabilities.iter().copied());
                if let Some(kinds) = kinds {
                    query = query.kinds(kinds.iter().copied());
                }
                let cell = format!("{label}: {set_name} with {capabilities:?}");
                let narrow = source.query(&query).expect("narrow query").index;
                let expected: Vec<&SymbolRecord> = complete
                    .symbols
                    .iter()
                    .filter(|record| kinds.is_none_or(|kinds| kinds.contains(&record.kind)))
                    .collect();
                assert_eq!(
                    narrow.symbols.iter().map(|record| &record.id).collect::<Vec<_>>(),
                    expected.iter().map(|record| &record.id).collect::<Vec<_>>(),
                    "{cell}: selected records"
                );
                for (record, complete_record) in narrow.symbols.iter().zip(expected) {
                    assert_eq!(
                        declaration_facts(record),
                        expected_facts(complete_record, &query),
                        "{cell}: `{}`",
                        record.identity.qualified_name
                    );
                    compared += 1;
                }
            }
        }
    }
    assert!(compared > 0, "group {group}: no record compared");
    if group != crate::symbols_relations::RUST_FACT_GROUP {
        return;
    }
    assert!(compared > 1000, "too few records compared ({compared})");
    for (fact, count) in
        ["generic parameters", "modifiers", "attributes", "relations", "textual impl containers", "body lines"]
            .iter()
            .zip(nonempty)
    {
        assert!(count > 0, "no source exercised {fact}");
    }
}

crate::symbols_relations::declaration_fact_group_tests!(
    declaration_facts_need_one_parse_one_capture_and_one_relation_link => one_parse_one_capture_and_one_relation_link
);

/// A declarations-only query over the same sources runs the parse pass
/// alone: one parse, one definitions query, one relation link (one
/// declaration index and, in Rust, one impl walk) per file, and no
/// diagnostics. The complete index links relations once too.
fn one_parse_one_capture_and_one_relation_link(group: usize) {
    let dir = TempDir::new().expect("temp");
    for (label, path) in crate::symbols_relations::write_declaration_fact_group(dir.path(), group) {
        for query in [SymbolQuery::new(), SymbolQuery::new().kinds([SymbolKindV2::Method]), SymbolQuery::complete_index()] {
            let cell = format!("{label}: {query:?}");
            let (result, work) =
                counted(|| SourceFile::read(&path).expect("read").query(&query).expect("query"));
            assert_eq!(work.parser_constructions, 1, "{cell}: parses");
            assert_eq!(work.definition_captures, 1, "{cell}: definitions queries");
            assert_eq!(work.relation_links, 1, "{cell}: relation links");
            if query.requires(Capability::Diagnostics) {
                assert_eq!(work.diagnostics_runs, 1, "{cell}: diagnostics runs");
            } else {
                assert_eq!(work.diagnostics_runs, 0, "{cell}: diagnostics runs");
                assert_eq!(result.index.completed_passes, [AnalysisPass::Parse], "{cell}: passes");
            }
        }
    }
}
