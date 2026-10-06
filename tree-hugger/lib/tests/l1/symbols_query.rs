//! Selective analysis (`2026-10-03-symbol-improvements`, Phase 3): pass
//! planning, the work counters, `SourceFile`, `SymbolQuery` kind and
//! capability selection, and the bind-pass behaviors that changed with it.
//!
//! The counter tests read process-wide counts; nextest runs each test in its
//! own process, so no other analysis runs between two snapshots.

use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;
use tree_hugger::{
    AnalysisPass, Capability, MetricsFacet, SourceFile, SymbolKindV2, SymbolQuery, SymbolRecord, TreeFile,
    TreeHuggerError, TriState, VisibilityLevel, WorkCounters, work_counters,
};

use crate::symbols_fixtures::{all_fixtures, display_name, multi_fixtures};
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
    }

    assert_eq!(
        SymbolQuery::new()
            .capabilities([Capability::Semantics])
            .required_capabilities(),
        [
            Capability::Declarations,
            Capability::Containment,
            Capability::References,
            Capability::Semantics,
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
        Some(MetricsFacet { parameters: Some(1), has_docs: Some(true), lines: None })
    );
    assert!(old.relations.references.is_empty(), "references were not bound");
    assert!(old.relations.parent.is_none(), "containment was not requested");
    assert_eq!(old.semantics.is_recursive, None, "semantics were not requested");
    assert!(old.docs.parsed.is_none(), "structured docs were not requested");
    assert_eq!(
        named("helper").metrics,
        Some(MetricsFacet { parameters: Some(0), has_docs: Some(false), lines: None })
    );

    let plain = tree_file.query(&SymbolQuery::new().kinds([SymbolKindV2::Function])).expect("query");
    assert!(plain.index.symbols.iter().all(|record| record.metrics.is_none()
        && !record.visibility.is_deprecated
        && !record.visibility.is_test));
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

/// For every fixture of every language, a narrow query returns exactly the
/// selected kinds' records of a full query with the same capabilities:
/// identity, spans, container, metadata, relations and semantics alike.
#[test]
fn narrow_queries_equal_the_filtered_records_of_a_full_query() {
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

    for path in all_fixtures().into_iter().chain(multi_fixtures()) {
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

    for ((set_name, _), count) in kind_sets.iter().zip(&compared) {
        assert!(*count > 0, "{set_name}: no fixture record was compared");
    }
    assert!(
        parents_outside_selection > 20,
        "selected methods keep their unselected owners as parents ({parents_outside_selection})"
    );
    assert!(
        classes_fields_with_access > 20,
        "classes keep field access and `static` without Field/Property/Constant selected ({classes_fields_with_access})"
    );
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
fn distinct_unresolved_references_are_all_kept() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        &dir,
        "two_imports.py",
        "from os import path\nfrom sys import argv\n\n\ndef run():\n    path.join(\"a\")\n    path.join(\"b\")\n    return argv\n",
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
    assert_eq!(names(&run.relations.references), expected, "both unresolved imports, `path` once");
    assert_eq!(names(&run.relations.dependencies), expected);
}

#[test]
fn is_recursive_is_unknown_when_semantics_run_and_absent_otherwise() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        &dir,
        "recursion.rs",
        "fn countdown(n: u32) -> u32 { if n == 0 { 0 } else { countdown(n - 1) } }\nfn leaf() -> u32 { 1 }\nstruct Holder;\n",
    );
    let tree_file = TreeFile::new(&path).expect("parse");
    let recursion = |records: &[SymbolRecord]| -> Vec<(String, Option<TriState>)> {
        records
            .iter()
            .map(|record| (record.identity.name.clone(), record.semantics.is_recursive))
            .collect()
    };

    let with_semantics = tree_file.symbol_index_v2().expect("index");
    assert_eq!(
        recursion(&with_semantics.symbols),
        [
            ("countdown".to_string(), Some(TriState::Unknown)),
            ("n".to_string(), None),
            ("leaf".to_string(), Some(TriState::Unknown)),
            ("Holder".to_string(), None),
        ],
        "a guessed Yes (countdown) or No (leaf) is never reported"
    );

    let without = tree_file
        .query(&SymbolQuery::new().capabilities([Capability::References]))
        .expect("query");
    assert!(
        recursion(&without.index.symbols).iter().all(|(_, value)| value.is_none()),
        "no semantics requested, no value"
    );
}

/// A wide result narrowed to a query equals that query run uncached, for
/// every fixture and each command's kind set; a query the wide result does
/// not cover is refused rather than answered from partial data.
#[test]
fn a_wide_result_narrows_to_exactly_the_uncached_narrow_query() {
    let kind_sets: [(&str, &[SymbolKindV2]); 3] = [
        ("functions", &FUNCTION_KINDS),
        ("types", &TYPE_KINDS),
        ("classes", &CLASS_KINDS),
    ];
    let mut compared = 0usize;
    for path in all_fixtures().into_iter().chain(multi_fixtures()) {
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
    assert!(compared > 100, "too few records compared ({compared})");
}
