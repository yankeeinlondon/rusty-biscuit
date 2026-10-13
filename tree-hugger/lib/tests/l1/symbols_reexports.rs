//! The re-export capability seam: what the planner computes, what stays
//! uncomputed, and how the result survives the persistent cache.
//!
//! Language extractors land one at a time behind this seam; until a language
//! has one, its report must say `limited`, never a `complete` negative.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;
use tree_hugger::cache::{CacheLookup, CacheMiss, PersistentCache, SymbolCache};
use tree_hugger::{
    Capability, ProgrammingLanguage, ReexportCoverageState, SourceFile,
    SymbolKindV2, SymbolQuery, TreeFile, work_counters,
};

use crate::symbols_fixtures::primary_fixture;

const NOT_APPLICABLE: [ProgrammingLanguage; 5] = [
    ProgrammingLanguage::Java,
    ProgrammingLanguage::C,
    ProgrammingLanguage::Bash,
    ProgrammingLanguage::Zsh,
    ProgrammingLanguage::Php,
];

fn fixture(language: ProgrammingLanguage) -> PathBuf {
    primary_fixture(language).expect("primary fixture")
}

fn only_reexports() -> SymbolQuery {
    SymbolQuery::new().kinds([]).capabilities([Capability::Reexports])
}

fn cache_in(dir: &TempDir) -> SymbolCache {
    SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true))
}

fn entry_path(dir: &TempDir) -> PathBuf {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "json") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(dir.path(), &mut found);
    assert_eq!(found.len(), 1, "expected one entry, found {found:#?}");
    found.remove(0)
}

fn read_entry(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read entry")).expect("entry JSON")
}

fn write_entry(path: &Path, entry: &Value) {
    std::fs::write(path, serde_json::to_vec(entry).expect("serialize entry")).expect("write entry");
}

#[test]
fn capability_closure_and_order_are_pinned() {
    assert_eq!(Capability::ALL.last(), Some(&Capability::Reexports));
    assert_eq!(serde_json::to_value(Capability::Reexports).unwrap(), json!("reexports"));

    let reexports = SymbolQuery::new().capabilities([Capability::Reexports]);
    assert_eq!(
        reexports.required_capabilities(),
        [Capability::Declarations, Capability::Reexports]
    );
    // Forwarding never needs references, occurrences or semantics.
    for unrelated in [Capability::References, Capability::Occurrences, Capability::Semantics] {
        assert!(!reexports.requires(unrelated), "{unrelated:?}");
    }

    let diagnostics = SymbolQuery::new().capabilities([Capability::Diagnostics]);
    assert!(diagnostics.requires(Capability::Reexports), "diagnostics read proven forwarding");
}

#[test]
fn an_unrequested_report_is_absent_not_empty() {
    let file = TreeFile::new(fixture(ProgrammingLanguage::Python)).expect("parse");
    let result = file.query(&SymbolQuery::new()).expect("query");
    assert!(result.index.reexports.is_none());
    assert!(!result.completed_capabilities.contains(&Capability::Reexports));
    assert!(result.index.imports.iter().all(|import| !import.reexported));

    let json = serde_json::to_value(&result.index).unwrap();
    assert!(json.get("reexports").is_none(), "an uncomputed report writes no key");
    assert!(json["imports"].as_array().unwrap().iter().all(|import| import["reexported"] == false));
}

/// One forwarding snippet per supported language: the seam reaches every
/// extractor, and none is left as the whole-file `grammar_gap` placeholder.
const FORWARDING_SNIPPETS: [(ProgrammingLanguage, &str, &str); 11] = [
    (ProgrammingLanguage::Rust, "lib.rs", "pub use std::io::Read;\n"),
    (ProgrammingLanguage::JavaScript, "m.js", "export { a } from \"m\";\n"),
    (ProgrammingLanguage::TypeScript, "m.ts", "export type { Shape } from \"./shapes\";\n"),
    (ProgrammingLanguage::Python, "m.py", "from m import a\n__all__ = [\"a\"]\n"),
    (ProgrammingLanguage::Go, "m.go", "package p\nimport \"x/pkg\"\ntype X = pkg.Y\n"),
    (
        ProgrammingLanguage::Perl,
        "M.pm",
        "package M;\nuse parent 'Exporter';\nuse Other qw(f);\nour @EXPORT_OK = ('f');\n1;\n",
    ),
    (ProgrammingLanguage::Lua, "m.lua", "return require(\"m\")\n"),
    (ProgrammingLanguage::Swift, "m.swift", "@_exported import Foo\n"),
    (ProgrammingLanguage::Scala, "m.scala", "object A { export impl.run }\n"),
    (ProgrammingLanguage::Cpp, "m.hpp", "namespace a { using b::c; }\n"),
    (
        ProgrammingLanguage::CSharp,
        "m.cs",
        "[assembly: System.Runtime.CompilerServices.TypeForwardedTo(typeof(N.T))]\n",
    ),
];

#[test]
fn every_language_reports_not_applicable_or_its_forwarding_entries() {
    let supported: Vec<_> = FORWARDING_SNIPPETS.iter().map(|(language, ..)| *language).collect();
    for language in ProgrammingLanguage::all() {
        assert_ne!(
            supported.contains(&language),
            NOT_APPLICABLE.contains(&language),
            "{language:?} must be in exactly one of the two tables"
        );
    }

    for language in NOT_APPLICABLE {
        let file = TreeFile::new(fixture(language)).expect("parse");
        let report = file.reexports().expect("reexports");
        let label = language.query_name();
        assert_eq!(report.coverage.state, ReexportCoverageState::NotApplicable, "{label}");
        assert!(report.coverage.limits.is_empty() && report.entries.is_empty(), "{label}");
    }

    for (language, name, source) in FORWARDING_SNIPPETS {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write");
        let report = TreeFile::new(&path).expect("parse").reexports().expect("reexports");
        let label = language.query_name();
        assert_eq!(report.coverage.state, ReexportCoverageState::Complete, "{label}: {report:#?}");
        assert!(!report.entries.is_empty(), "{label}: a forward is reported");
    }
}

#[test]
fn a_language_without_forwarding_syntax_is_a_complete_negative_not_a_placeholder() {
    for (language, name, source) in [
        (ProgrammingLanguage::Rust, "lib.rs", "pub fn f() {}\n"),
        (ProgrammingLanguage::Python, "m.py", "x = 1\n"),
        (ProgrammingLanguage::Go, "m.go", "package p\n"),
        (ProgrammingLanguage::Cpp, "m.cpp", "int f();\n"),
    ] {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write");
        let report = TreeFile::new(&path).expect("parse").reexports().expect("reexports");
        assert_eq!(report.coverage.state, ReexportCoverageState::Complete, "{language:?}");
        assert!(report.entries.is_empty() && report.coverage.limits.is_empty(), "{language:?}");
    }
}

#[test]
fn the_report_describes_the_whole_file_whatever_kinds_are_selected() {
    let file = TreeFile::new(fixture(ProgrammingLanguage::Python)).expect("parse");
    let whole = file.query(&SymbolQuery::new().capabilities([Capability::Reexports])).expect("whole");
    let narrow = file
        .query(
            &SymbolQuery::new()
                .kinds([SymbolKindV2::Class])
                .capabilities([Capability::Reexports]),
        )
        .expect("narrow");
    assert!(whole.index.reexports.is_some());
    assert_eq!(whole.index.reexports, narrow.index.reexports);
    assert_eq!(whole.index.reexports, Some(file.reexports().expect("direct")));
}

#[test]
fn the_complete_index_carries_the_report_and_the_capability() {
    let file = TreeFile::new(fixture(ProgrammingLanguage::Rust)).expect("parse");
    let result = file.query(&SymbolQuery::complete_index()).expect("query");
    assert!(result.completed_capabilities.contains(&Capability::Reexports));
    assert_eq!(result.index.reexports, Some(file.reexports().expect("direct")));
    assert_eq!(file.symbol_index_v2().expect("index").reexports, result.index.reexports);
}

#[test]
fn extraction_runs_once_and_only_when_required() {
    let file = TreeFile::new(fixture(ProgrammingLanguage::Python)).expect("parse");
    let run = |query: &SymbolQuery| {
        let before = work_counters();
        file.query(query).expect("query");
        work_counters().since(before)
    };

    assert_eq!(run(&SymbolQuery::new()).reexport_extractions, 0);
    assert_eq!(run(&SymbolQuery::new().capabilities([Capability::Metrics, Capability::Flags])).reexport_extractions, 0);

    let direct = run(&only_reexports());
    assert_eq!(direct.reexport_extractions, 1);
    // No reference captures, resolution or diagnostics for a cheap request.
    assert_eq!(direct.reference_captures, 0);
    assert_eq!(direct.occurrence_resolutions, 0);
    assert_eq!(direct.diagnostics_runs, 0);

    let diagnostics = run(&SymbolQuery::new().capabilities([Capability::Diagnostics]));
    assert_eq!(diagnostics.reexport_extractions, 1);
    assert_eq!(run(&SymbolQuery::complete_index()).reexport_extractions, 1);
}

#[test]
fn a_cached_report_equals_the_uncached_report_across_a_narrowing_and_a_widening() {
    let path = fixture(ProgrammingLanguage::Python);
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let wanted = SymbolQuery::new().capabilities([Capability::Reexports]);
    let uncached = SourceFile::read(&path).unwrap().query(&wanted).unwrap();

    let cold = cache.query(SourceFile::read(&path).unwrap(), &wanted).unwrap();
    assert!(matches!(cold.lookup, CacheLookup::Miss(_)), "{:?}", cold.lookup);
    assert_eq!(cold.result.index.reexports, uncached.index.reexports);
    let warm = cache.query(SourceFile::read(&path).unwrap(), &wanted).unwrap();
    assert!(matches!(warm.lookup, CacheLookup::Hit), "{:?}", warm.lookup);
    assert_eq!(warm.result.index.reexports, uncached.index.reexports);

    // A wider entry narrows to a query that does not ask for the report.
    let plain = cache.query(SourceFile::read(&path).unwrap(), &SymbolQuery::new().kinds([])).unwrap();
    assert!(plain.result.index.reexports.is_none());
    assert!(plain.result.index.imports.iter().all(|import| !import.reexported));

    // An entry without the report is widened by computing it.
    let other = TempDir::new().expect("temp");
    let narrow_cache = cache_in(&other);
    narrow_cache.query(SourceFile::read(&path).unwrap(), &SymbolQuery::new()).unwrap();
    let widened = narrow_cache.query(SourceFile::read(&path).unwrap(), &wanted).unwrap();
    assert!(!matches!(widened.lookup, CacheLookup::Hit), "{:?}", widened.lookup);
    assert_eq!(widened.result.index.reexports, uncached.index.reexports);
}

#[test]
fn an_entry_whose_report_and_capability_disagree_is_recomputed() {
    let path = fixture(ProgrammingLanguage::Python);
    let wanted = SymbolQuery::new().capabilities([Capability::Reexports]);
    let uncached = SourceFile::read(&path).unwrap().query(&wanted).unwrap();

    for (label, edit) in [
        ("report removed", (|entry: &mut Value| entry["reexports"] = Value::Null) as fn(&mut Value)),
        ("capability removed", |entry: &mut Value| {
            entry["completed_capabilities"]
                .as_array_mut()
                .unwrap()
                .retain(|capability| capability != "reexports");
        }),
        ("marker without the capability", |entry: &mut Value| {
            entry["completed_capabilities"]
                .as_array_mut()
                .unwrap()
                .retain(|capability| capability != "reexports");
            entry["reexports"] = Value::Null;
            if let Some(import) = entry["imports"].as_array_mut().unwrap().first_mut() {
                import["reexported"] = json!(true);
            }
        }),
        ("key absent", |entry: &mut Value| {
            entry.as_object_mut().unwrap().remove("reexports");
        }),
    ] {
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cache.query(SourceFile::read(&path).unwrap(), &wanted).unwrap();
        let entry_file = entry_path(&dir);
        let mut entry = read_entry(&entry_file);
        edit(&mut entry);
        write_entry(&entry_file, &entry);

        let outcome = cache.query(SourceFile::read(&path).unwrap(), &wanted).unwrap();
        assert!(
            matches!(outcome.lookup, CacheLookup::Miss(CacheMiss::Inconsistent(_) | CacheMiss::Malformed(_))),
            "{label}: {:?}",
            outcome.lookup
        );
        assert_eq!(outcome.result.index.reexports, uncached.index.reexports, "{label}");
        assert_eq!(read_entry(&entry_file)["reexports"], serde_json::to_value(&uncached.index.reexports).unwrap(), "{label}: rewritten");
    }
}
