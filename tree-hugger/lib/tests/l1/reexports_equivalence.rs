//! The re-export report answers the same forwarding question through every
//! public surface: the direct API, the complete index, a selective query, and
//! the persistent cache (cold, hit, narrowed, widened; the other-scan-root case is a CLI test).
//! It also pins the work a request may do and that extraction never reads the
//! files a statement forwards from.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tree_hugger::cache::{CacheLookup, PersistentCache, SymbolCache};
use tree_hugger::{
    Capability, ProgrammingLanguage, SourceFile, SymbolKindV2, SymbolQuery, TreeFile, work_counters,
};

use crate::reexports_support::snippet;
use crate::symbols_fixtures::primary_fixture;

const FORWARDING_TS: &str = "import { a } from './a';\nexport { a };\nexport * from './b';\nexport { c as d } from './c';\nexport function own() {}\n";

fn reexports() -> SymbolQuery {
    SymbolQuery::new().capabilities([Capability::Reexports])
}

fn markers(index: &tree_hugger::FileSymbolIndex) -> Vec<bool> {
    index.imports.iter().map(|import| import.reexported).collect()
}

fn cache_in(dir: &Path) -> SymbolCache {
    SymbolCache::new(PersistentCache::open(dir.join("cache"), true))
}

crate::language_cases::language_tests!(every_surface_agrees_for_every_language => check_every_surface_agrees_for_every_language);

fn check_every_surface_agrees_for_every_language(language: ProgrammingLanguage) {
    let path = primary_fixture(language).expect("primary fixture");
    let file = TreeFile::new(&path).expect("parse");
    let direct = file.reexports().expect("direct");

    let complete = file.query(&SymbolQuery::complete_index()).expect("complete");
    let selective = file.query(&reexports()).expect("selective");
    let kind_filtered = file
        .query(&SymbolQuery::new().kinds([SymbolKindV2::Function]).capabilities([Capability::Reexports]))
        .expect("kind filtered");
    let declarations_only = file.query(&SymbolQuery::new()).expect("declarations");

    for (label, result) in [("complete", &complete), ("selective", &selective), ("kinds", &kind_filtered)] {
        assert_eq!(result.index.reexports.as_ref(), Some(&direct), "{language:?} {label}");
        assert_eq!(markers(&result.index), markers(&complete.index), "{language:?} {label}: markers");
    }
    // Asking for the report changes neither the declarations nor their ids.
    assert_eq!(
        serde_json::to_value(&selective.index.symbols).unwrap(),
        serde_json::to_value(&declarations_only.index.symbols).unwrap(),
        "{language:?}: declarations"
    );
    assert!(declarations_only.index.reexports.is_none(), "{language:?}: unrequested report is absent");
    // A marker is set exactly where an entry links to an import.
    let marked = markers(&complete.index).into_iter().filter(|marked| *marked).count();
    assert!(marked <= complete.index.imports.len());
    if direct.entries.is_empty() {
        assert_eq!(marked, 0, "{language:?}: no entries, no markers");
    }
}

crate::language_cases::language_tests!(the_cache_serves_the_uncached_report_for_every_language => check_the_cache_serves_the_uncached_report_for_every_language);

fn check_the_cache_serves_the_uncached_report_for_every_language(language: ProgrammingLanguage) {
    let path = primary_fixture(language).expect("primary fixture");
    let uncached = SourceFile::read(&path).unwrap().query(&reexports()).unwrap();
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(dir.path());

    let cold = cache.query(SourceFile::read(&path).unwrap(), &reexports()).unwrap();
    assert!(matches!(cold.lookup, CacheLookup::Miss(_)), "{language:?}: {:?}", cold.lookup);
    let hit = cache.query(SourceFile::read(&path).unwrap(), &reexports()).unwrap();
    assert!(matches!(hit.lookup, CacheLookup::Hit), "{language:?}: {:?}", hit.lookup);
    // The wider complete index entry narrows back to the same report.
    let wide = cache.query(SourceFile::read(&path).unwrap(), &SymbolQuery::complete_index()).unwrap();
    let narrowed = cache.query(SourceFile::read(&path).unwrap(), &reexports()).unwrap();
    assert!(matches!(narrowed.lookup, CacheLookup::Hit), "{language:?}: {:?}", narrowed.lookup);

    for (label, outcome) in [("cold", &cold), ("hit", &hit), ("wide", &wide), ("narrowed", &narrowed)] {
        assert_eq!(outcome.result.index.reexports, uncached.index.reexports, "{language:?} {label}");
        assert_eq!(markers(&outcome.result.index), markers(&uncached.index), "{language:?} {label}: markers");
    }
}

#[test]
fn a_narrow_entry_is_widened_by_one_extraction_and_a_hit_extracts_nothing() {
    let dir = TempDir::new().expect("temp");
    let path: PathBuf = dir.path().join("index.ts");
    std::fs::write(&path, FORWARDING_TS).unwrap();
    let cache_dir = TempDir::new().expect("temp");
    let cache = cache_in(cache_dir.path());

    cache.query(SourceFile::read(&path).unwrap(), &SymbolQuery::new()).unwrap();
    let before = work_counters();
    let widened = cache.query(SourceFile::read(&path).unwrap(), &reexports()).unwrap();
    let work = work_counters().since(before);
    assert!(!matches!(widened.lookup, CacheLookup::Hit), "{:?}", widened.lookup);
    assert_eq!(work.reexport_extractions, 1);
    assert_eq!(work.reference_captures, 0);
    assert_eq!(work.occurrence_resolutions, 0);
    assert_eq!(work.diagnostics_runs, 0);

    let before = work_counters();
    let hit = cache.query(SourceFile::read(&path).unwrap(), &reexports()).unwrap();
    let work = work_counters().since(before);
    assert!(matches!(hit.lookup, CacheLookup::Hit), "{:?}", hit.lookup);
    assert_eq!(work.reexport_extractions, 0);
    assert_eq!(work.parser_constructions, 0);
    assert_eq!(work.definition_captures, 0);
}

#[test]
fn extraction_reads_no_forwarding_target_and_shares_one_definitions_pass() {
    let bare = TempDir::new().expect("temp");
    let linked = TempDir::new().expect("temp");
    for dir in [&bare, &linked] {
        std::fs::write(dir.path().join("index.ts"), FORWARDING_TS).unwrap();
    }
    for target in ["a.ts", "b.ts", "c.ts"] {
        std::fs::write(linked.path().join(target), "export const unrelated = 1;\nexport * from './elsewhere';\n").unwrap();
    }

    let report_of = |dir: &TempDir| {
        let before = work_counters();
        let file = TreeFile::new(dir.path().join("index.ts")).expect("parse");
        let result = file.query(&reexports()).expect("query");
        (result.index.reexports.expect("report"), work_counters().since(before))
    };
    let (absent, absent_work) = report_of(&bare);
    let (present, present_work) = report_of(&linked);

    assert_eq!(absent.entries.len(), 3);
    assert_eq!(absent, present, "target contents never change the report");
    // Only the forwarding file is parsed, with or without its targets.
    assert_eq!(absent_work.parser_constructions, 1);
    assert_eq!(present_work.parser_constructions, 1);

    // The report shares the one definitions/import pass: requesting it adds an
    // extraction, never a second definitions query.
    let file = TreeFile::new(bare.path().join("index.ts")).expect("parse");
    let measure = |query: &SymbolQuery| {
        let before = work_counters();
        file.query(query).expect("query");
        work_counters().since(before)
    };
    let without = measure(&SymbolQuery::new());
    let with = measure(&reexports());
    assert_eq!(with.definition_captures, without.definition_captures);
    assert_eq!(with.reexport_extractions, 1);
    assert_eq!(with.reference_captures, 0);
    assert_eq!(with.occurrence_resolutions, 0);
    assert_eq!(with.diagnostics_runs, 0);

    // Indirect requests: diagnostics need the markers and extract exactly once.
    let diagnostics = measure(&SymbolQuery::new().capabilities([Capability::Diagnostics]));
    assert_eq!(diagnostics.reexport_extractions, 1);
    let complete = measure(&SymbolQuery::complete_index());
    assert_eq!(complete.reexport_extractions, 1);
    assert_eq!(complete.definition_captures, 1);
}

#[test]
fn a_dependency_free_snippet_reports_without_any_other_file() {
    let snippet = snippet("index.ts", FORWARDING_TS);
    assert_eq!(snippet.report().entries.len(), 3);
    assert_eq!(snippet.indexed().reexports, Some(snippet.report()));
}
