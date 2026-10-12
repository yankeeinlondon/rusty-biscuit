//! The persistent symbol cache (`2026-10-03-symbol-improvements`, Phase 4):
//! one entry per file whose recorded coverage decides what it serves.
//!
//! Every cached result is compared with an uncached query of the same file,
//! as a whole serialized index plus completed capabilities. Lookups are
//! asserted by outcome (`Hit`, widened, `Miss` with its reason), so an entry
//! that silently fails to decode and is recomputed is caught even though its
//! output would be equal. Counter tests read process-wide counts; nextest
//! runs each test in its own process.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};

use serde_json::{Value, json};
use tempfile::TempDir;
use tree_hugger::cache::{
    CacheLookup, CacheMiss, FingerprintInputs, PersistentCache, SymbolCache,
};
use tree_hugger::queries::QueryKind;
use tree_hugger::{
    Capability, EXTRACTOR_REVISION, NamePattern, Occurrence, OccurrenceKind, ProgrammingLanguage, SourceFile, SymbolKindV2,
    SymbolQuery, SymbolQueryResult, TreeFile, WorkCounters, stable_symbol_key,
    symbol_id_from_stable_key, work_counters,
};

use crate::symbols_fixtures::{all_fixtures, display_name, in_file_name, multi_fixtures, primary_fixture};
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

fn counted<T>(work: impl FnOnce() -> T) -> (T, WorkCounters) {
    let before = work_counters();
    let value = work();
    (value, work_counters().since(before))
}

fn cache_in(dir: &TempDir) -> SymbolCache {
    SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true))
}

fn read(path: &Path) -> SourceFile {
    SourceFile::read(path).expect("read source")
}

/// The serialized result: the whole index and the completed capabilities.
fn as_value(result: &SymbolQueryResult) -> Value {
    json!({
        "index": serde_json::to_value(&result.index).expect("serialize index"),
        "completed_capabilities": result.completed_capabilities,
    })
}

fn uncached(path: &Path, query: &SymbolQuery) -> Value {
    as_value(&read(path).query(query).expect("uncached query"))
}

/// Runs `query` through `cache`, returning the lookup, the result and the
/// work it did.
fn cached(
    cache: &SymbolCache,
    source: SourceFile,
    query: &SymbolQuery,
) -> (CacheLookup, Value, WorkCounters) {
    let (outcome, work) = counted(|| cache.query(source, query).expect("cached query"));
    assert!(outcome.write_error.is_none(), "write failed: {:?}", outcome.write_error);
    (outcome.lookup, as_value(&outcome.result), work)
}

/// Every file under `dir` with `extension`.
fn files_with_extension(dir: &Path, extension: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, extension: &str, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, extension, found);
            } else if path.extension().is_some_and(|ext| ext == extension) {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(dir, extension, &mut found);
    found.sort();
    found
}

fn only_entry(dir: &TempDir) -> PathBuf {
    let entries = files_with_extension(dir.path(), "json");
    assert_eq!(entries.len(), 1, "expected one entry, found {entries:#?}");
    entries.into_iter().next().expect("one entry")
}

fn read_entry(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read entry")).expect("entry JSON")
}

fn write_entry(path: &Path, entry: &Value) {
    std::fs::write(path, serde_json::to_vec(entry).expect("serialize entry")).expect("write entry");
}

/// The query an entry's recorded coverage describes.
fn coverage_query(entry: &Value) -> SymbolQuery {
    let capabilities: Vec<Capability> =
        serde_json::from_value(entry["completed_capabilities"].clone()).expect("capabilities");
    let query = SymbolQuery::new().capabilities(capabilities);
    if entry["covered_kinds"] == "all" {
        query
    } else {
        let kinds: Vec<SymbolKindV2> =
            serde_json::from_value(entry["covered_kinds"].clone()).expect("kinds");
        query.kinds(kinds)
    }
}

fn fixture(language: ProgrammingLanguage) -> PathBuf {
    primary_fixture(language).expect("primary fixture")
}

crate::language_cases::language_tests!(every_fixture_is_a_complete_warm_hit_equal_to_an_uncached_run => check_every_fixture_is_a_complete_warm_hit_equal_to_an_uncached_run);

/// The "Cache correctness" row of the per-language matrix: for every fixture,
/// the cold run stores an entry and the warm run is a complete hit that
/// parses nothing and equals an uncached run.
fn check_every_fixture_is_a_complete_warm_hit_equal_to_an_uncached_run(language: ProgrammingLanguage) {
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let query = SymbolQuery::complete_index();
    let fixtures = crate::symbols_fixtures::fixtures_for(language, true);

    for path in &fixtures {
        let label = display_name(path);
        let expected = uncached(path, &query);
        let (cold_lookup, cold, _) = cached(&cache, read(path), &query);
        assert_eq!(cold_lookup, CacheLookup::Miss(CacheMiss::Absent), "{label}: cold");
        assert_eq!(cold, expected, "{label}: cold result");

        let (warm_lookup, warm, work) = cached(&cache, read(path), &query);
        assert_eq!(warm_lookup, CacheLookup::Hit, "{label}: warm");
        assert_eq!(warm, expected, "{label}: warm result");
        assert_eq!(work, WorkCounters::default(), "{label}: a complete hit did work");
    }
    assert_eq!(
        files_with_extension(dir.path(), "json").len(),
        fixtures.len(),
        "one entry per file"
    );
    assert!(files_with_extension(dir.path(), "tmp").is_empty(), "no temporary file left");
}

crate::language_cases::language_tests!(stored_reference_evidence_round_trips_through_rewrites => check_stored_reference_evidence_round_trips_through_rewrites);

/// Reference evidence survives repeated persistence: for every fixture, a
/// functions-only `Occurrences` entry is written, read and widened to the
/// complete index (rewritten), then read again as a `Hit`; each result equals
/// an uncached run, and both writes store the same whole-file evidence.
fn check_stored_reference_evidence_round_trips_through_rewrites(language: ProgrammingLanguage) {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let complete = SymbolQuery::complete_index();
    let evidence = |entry: &Value| json!([entry["occurrences"], entry["reference_coverage"]]);
    let fixtures = crate::symbols_fixtures::fixtures_for(language, true);

    for path in &fixtures {
        let label = display_name(path);
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);

        let (lookup, served, _) = cached(&cache, read(path), &narrow);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent), "{label}: narrow write");
        assert_eq!(served, uncached(path, &narrow), "{label}: narrow result");
        let first = evidence(&read_entry(&only_entry(&dir)));
        assert!(
            first[1].as_array().is_some_and(|coverage| !coverage.is_empty()),
            "{label}: the entry stores coverage"
        );

        let (lookup, served, _) = cached(&cache, read(path), &narrow);
        assert_eq!(lookup, CacheLookup::Hit, "{label}: narrow read");
        assert_eq!(served, uncached(path, &narrow), "{label}: narrow read result");

        let (lookup, served, _) = cached(&cache, read(path), &complete);
        assert_eq!(lookup, CacheLookup::WidenedKinds, "{label}: widened");
        assert_eq!(served, uncached(path, &complete), "{label}: widened result");
        assert_eq!(evidence(&read_entry(&only_entry(&dir))), first, "{label}: rewritten evidence");

        let (lookup, served, work) = cached(&cache, read(path), &complete);
        assert_eq!(lookup, CacheLookup::Hit, "{label}: complete read");
        assert_eq!(served, uncached(path, &complete), "{label}: complete read result");
        assert_eq!(work, WorkCounters::default(), "{label}: a hit did work");
    }
}

/// A complete entry serves every narrower query exactly as an uncached query
/// of that query computes it: unselected kinds dropped, and relations,
/// semantics, parsed docs, export targets, diagnostics and each record's
/// `parse_pass` reduced to what the narrower query's passes produce.
mod projected_results_equal_uncached_queries {
    use super::*;
    macro_rules! projection_language_tests {
        ($($test:ident => $language:ident),* $(,)?) => {
            const LANGUAGES: &[ProgrammingLanguage] = &[$(ProgrammingLanguage::$language),*];
            $(#[test] fn $test() {
                let language = ProgrammingLanguage::$language;
                let fixtures = all_fixtures().into_iter()
                    .filter(|path| ProgrammingLanguage::from_path(path) == Some(language)).collect();
                assert_projections_equal_uncached_queries(fixtures, false);
            })*
        };
    }
    projection_language_tests! {
        rust => Rust,
        javascript => JavaScript,
        typescript => TypeScript,
        go => Go,
        python => Python,
        java => Java,
        php => Php,
        perl => Perl,
        bash => Bash,
        zsh => Zsh,
        c => C,
        cpp => Cpp,
        csharp => CSharp,
        swift => Swift,
        scala => Scala,
        lua => Lua,
    }
    #[test]
    fn projections_exercise_every_optional_facet() {
        let dir = TempDir::new().expect("fixture temp");
        let path = dir.path().join("facets.rs");
        std::fs::write(&path, "fn helper() {}\n/// Calls helper.\n#[deprecated]\npub fn caller() { helper(); }\n").expect("write fixture");
        assert_projections_equal_uncached_queries(vec![path], true);
    }

    #[test]
    fn every_fixture_language_is_scheduled() {
        assert_eq!(LANGUAGES, ProgrammingLanguage::all());
        let fixtures = all_fixtures();
        assert!(fixtures.len() >= 16);
        assert!(fixtures.iter().all(|path| LANGUAGES.contains(&ProgrammingLanguage::from_path(path).expect("fixture language"))));
    }
}

crate::language_cases::language_tests!(projected_results_over_multi_equal_uncached_queries => check_projected_results_over_multi_equal_uncached_queries);

/// Complete-entry projections over the `multi/` tree.
fn check_projected_results_over_multi_equal_uncached_queries(language: ProgrammingLanguage) {
    assert_projections_equal_uncached_queries(multi_fixtures().into_iter().filter(|path| ProgrammingLanguage::from_path(path) == Some(language)).collect(), false);
}

fn assert_projections_equal_uncached_queries(fixtures: Vec<PathBuf>, require_populated_facets: bool) {
    // Each capability alone (each projection rule on its own) and all of
    // them, over every kind; kind selection itself is a plain filter, so a
    // member kind whose owners are unselected (the case that needs owners
    // from outside the selection) is checked at both ends.
    let method: &[SymbolKindV2] = &[SymbolKindV2::Method];
    let cells: [(Option<&[SymbolKindV2]>, &[Capability]); 13] = [
        (None, &[Capability::Declarations]),
        (None, &[Capability::Containment]),
        (None, &[Capability::References]),
        (None, &[Capability::Occurrences]),
        (None, &[Capability::Semantics]),
        (None, &[Capability::StructuredDocs]),
        (None, &[Capability::Diagnostics]),
        (None, &[Capability::Flags]),
        (None, &[Capability::Metrics]),
        (None, &Capability::ALL),
        (Some(method), &[Capability::Declarations]),
        // Occurrences describe the whole file under any kind selection.
        (Some(method), &[Capability::Semantics]),
        (Some(method), &Capability::ALL),
    ];
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let mut compared = 0usize;
    let mut stripped_references = 0usize;
    let mut stripped_occurrences = 0usize;
    let mut stripped_parsed_docs = 0usize;
    let mut stripped_flags = 0usize;
    let mut stripped_metrics = 0usize;

    let fixture_count = fixtures.len();
    for path in fixtures {
        let label = display_name(&path);
        let complete = SymbolQuery::complete_index();
        let (lookup, full, _) = cached(&cache, read(&path), &complete);
        assert!(matches!(lookup, CacheLookup::Miss(_)), "{label}: first run");
        let tree_file = TreeFile::new(&path).expect("parse fixture");

        for &(kinds, capabilities) in &cells {
            {
                let mut query = SymbolQuery::new().capabilities(capabilities.iter().copied());
                if let Some(kinds) = kinds {
                    query = query.kinds(kinds.iter().copied());
                }
                let expected = as_value(&tree_file.query(&query).expect("uncached query"));
                let (lookup, served, work) = cached(&cache, read(&path), &query);
                let cell = format!("{label}: kinds {kinds:?}, capabilities {capabilities:?}");
                assert_eq!(lookup, CacheLookup::Hit, "{cell}");
                assert_eq!(work.parser_constructions, 0, "{cell}: parsed");
                assert_eq!(served, expected, "{cell}");
                compared += 1;
                if !query.requires(Capability::References) {
                    stripped_references += count_records(&full, |record| {
                        record["relations"]["references"].as_array().is_some_and(|refs| !refs.is_empty())
                    });
                }
                if !query.requires(Capability::Occurrences) {
                    stripped_occurrences += full["index"]["occurrences"].as_array().map_or(0, Vec::len);
                }
                if !query.requires(Capability::StructuredDocs) {
                    stripped_parsed_docs +=
                        count_records(&full, |record| !record["docs"]["parsed"].is_null());
                }
                if !query.requires(Capability::Flags) {
                    stripped_flags += count_records(&full, |record| {
                        record["visibility"]["is_test"] == json!(true)
                            || record["visibility"]["is_deprecated"] == json!(true)
                    });
                }
                if !query.requires(Capability::Metrics) {
                    stripped_metrics += count_records(&full, |record| record["metrics"].is_object());
                }
            }
        }
    }
    assert!(fixture_count > 0, "no fixtures for a scheduled language");
    assert_eq!(compared, fixture_count * cells.len());
    if require_populated_facets {
        assert!(stripped_references > 0, "no projection removed references");
        assert!(stripped_occurrences > 0, "no projection removed occurrences");
        assert!(stripped_parsed_docs > 0, "no projection removed parsed docs");
        assert!(stripped_flags > 0, "no projection removed a set flag");
        assert!(stripped_metrics > 0, "no projection removed metrics");
    }
}

fn count_records(result: &Value, predicate: impl Fn(&Value) -> bool) -> usize {
    result["index"]["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .filter(|record| predicate(record))
        .count()
}

/// One capability-widening row: after `first`, `second` widens the entry by
/// its missing capabilities only; the entry then covers both, and both (and
/// their union) are complete hits.
struct WideningRow {
    language: ProgrammingLanguage,
    first: SymbolQuery,
    second: SymbolQuery,
    parsed: bool,
    diagnostics_runs: u64,
}

crate::language_cases::language_tests!(widening_capabilities_runs_only_the_missing_passes => check_widening_capabilities_runs_only_the_missing_passes);

fn check_widening_capabilities_runs_only_the_missing_passes(language: ProgrammingLanguage) {
    let functions = || SymbolQuery::new().kinds(FUNCTION_KINDS);
    let rows = [
        WideningRow {
            language: ProgrammingLanguage::Rust,
            first: SymbolQuery::new(),
            second: SymbolQuery::new().capabilities([Capability::StructuredDocs]),
            parsed: false,
            diagnostics_runs: 0,
        },
        // Recursion is read from the occurrences: reference edges alone
        // cannot tell a proven self-call from a same-named one.
        WideningRow {
            language: ProgrammingLanguage::TypeScript,
            first: SymbolQuery::new().capabilities([Capability::References]),
            second: SymbolQuery::new().capabilities([Capability::Semantics]),
            parsed: true,
            diagnostics_runs: 0,
        },
        WideningRow {
            language: ProgrammingLanguage::Rust,
            first: SymbolQuery::new().capabilities([Capability::Occurrences]),
            second: SymbolQuery::new().capabilities([Capability::Semantics]),
            parsed: false,
            diagnostics_runs: 0,
        },
        WideningRow {
            language: ProgrammingLanguage::TypeScript,
            first: functions().capabilities([Capability::References]),
            second: functions().capabilities([Capability::Occurrences]),
            parsed: true,
            diagnostics_runs: 0,
        },
        WideningRow {
            language: ProgrammingLanguage::Python,
            first: SymbolQuery::new(),
            second: SymbolQuery::new().capabilities([Capability::Containment]),
            parsed: false,
            diagnostics_runs: 0,
        },
        // A Rust impl member's container is its impl's resolved self type,
        // which no stored record retains.
        WideningRow {
            language: ProgrammingLanguage::Rust,
            first: SymbolQuery::new(),
            second: SymbolQuery::new().capabilities([Capability::Containment]),
            parsed: true,
            diagnostics_runs: 0,
        },
        // Containment over a narrow entry needs the owners it does not hold.
        WideningRow {
            language: ProgrammingLanguage::Java,
            first: functions(),
            second: functions().capabilities([Capability::Containment]),
            parsed: true,
            diagnostics_runs: 0,
        },
        WideningRow {
            language: ProgrammingLanguage::Go,
            first: SymbolQuery::new(),
            second: SymbolQuery::new().capabilities([Capability::References]),
            parsed: true,
            diagnostics_runs: 0,
        },
        WideningRow {
            language: ProgrammingLanguage::CSharp,
            first: functions().capabilities([Capability::StructuredDocs]),
            second: functions().capabilities([Capability::Diagnostics]),
            parsed: true,
            diagnostics_runs: 1,
        },
        // Flags and metrics are read from the syntax tree; neither runs
        // diagnostics.
        WideningRow {
            language: ProgrammingLanguage::Rust,
            first: SymbolQuery::new(),
            second: SymbolQuery::new().capabilities([Capability::Flags]),
            parsed: true,
            diagnostics_runs: 0,
        },
        WideningRow {
            language: ProgrammingLanguage::Go,
            first: functions().capabilities([Capability::Flags]),
            second: functions().capabilities([Capability::Metrics]),
            parsed: true,
            diagnostics_runs: 0,
        },
    ];

    for row in rows.into_iter().filter(|row| row.language == language) {
        let path = fixture(row.language);
        let label = format!("{}: {:?} then {:?}", display_name(&path), row.first, row.second);
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &row.first);

        let (lookup, widened, work) = cached(&cache, read(&path), &row.second);
        assert_eq!(lookup, CacheLookup::WidenedCapabilities { parsed: row.parsed }, "{label}");
        assert_eq!(widened, uncached(&path, &row.second), "{label}: widened result");
        let parses = u64::from(row.parsed);
        assert_eq!(work.parser_constructions, parses, "{label}: parses");
        assert_eq!(work.definition_captures, parses, "{label}: captures");
        assert_eq!(work.diagnostics_runs, row.diagnostics_runs, "{label}: diagnostics runs");

        let entry = read_entry(&only_entry(&dir));
        let union = row.first.union_for_test(&row.second);
        assert_eq!(
            coverage_query(&entry),
            union,
            "{label}: the entry records both requests"
        );
        for query in [&row.first, &row.second, &union] {
            let (lookup, served, work) = cached(&cache, read(&path), query);
            assert_eq!(lookup, CacheLookup::Hit, "{label}: then {query:?}");
            assert_eq!(work, WorkCounters::default(), "{label}: then {query:?}");
            assert_eq!(served, uncached(&path, query), "{label}: then {query:?}");
        }
    }
}

#[test]
fn widening_kinds_reextracts_once_for_the_union() {
    let path = fixture(ProgrammingLanguage::Rust);
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let functions = SymbolQuery::new().kinds(FUNCTION_KINDS);
    let types = SymbolQuery::new()
        .kinds(TYPE_KINDS)
        .capabilities([Capability::Containment]);
    cached(&cache, read(&path), &functions);

    let (lookup, widened, work) = cached(&cache, read(&path), &types);
    assert_eq!(lookup, CacheLookup::WidenedKinds);
    assert_eq!(widened, uncached(&path, &types));
    assert_eq!((work.parser_constructions, work.definition_captures), (1, 1));

    let entry = read_entry(&only_entry(&dir));
    let mut union: Vec<SymbolKindV2> = FUNCTION_KINDS.iter().chain(&TYPE_KINDS).copied().collect();
    union.sort();
    assert_eq!(entry["covered_kinds"], json!(union));
    assert_eq!(entry["completed_capabilities"], json!(["declarations", "containment"]));

    for query in [
        functions.clone(),
        types.clone(),
        functions.clone().capabilities([Capability::Containment]),
    ] {
        let (lookup, served, work) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Hit, "{query:?}");
        assert_eq!(work, WorkCounters::default(), "{query:?}");
        assert_eq!(served, uncached(&path, &query), "{query:?}");
    }
}

#[test]
fn each_key_input_and_stored_identity_decides_a_miss() {
    let dir = TempDir::new().expect("temp");
    let source_dir = TempDir::new().expect("sources");
    let path = source_dir.path().join("rust.rs");
    std::fs::copy(fixture(ProgrammingLanguage::Rust), &path).expect("copy fixture");
    let query = SymbolQuery::new();
    let cache = cache_in(&dir);
    cached(&cache, read(&path), &query);
    let key = cache.key(&read(&path)).expect("key");

    // Edited locals query text: another key, so no entry.
    let locals = FingerprintInputs::shipped(ProgrammingLanguage::Rust, &path)
        .expect("inputs")
        .records_fingerprint();
    let edited = cache_in(&dir).with_query_text(
        ProgrammingLanguage::Rust,
        QueryKind::Locals,
        "(function_item name: (identifier) @local.definition.function)",
    );
    assert_ne!(edited.key(&read(&path)).expect("key"), key, "query text");
    assert_eq!(cached(&edited, read(&path), &query).0, CacheLookup::Miss(CacheMiss::Absent));

    // The extractor revision is part of the fingerprint.
    assert_eq!(
        locals.tree_hugger_version,
        format!("{}+rev.{EXTRACTOR_REVISION}", env!("CARGO_PKG_VERSION"))
    );
    let mut other_revision = locals.clone();
    other_revision.tree_hugger_version = format!("{}+rev.0", env!("CARGO_PKG_VERSION"));
    assert_ne!(other_revision.digest(), locals.digest(), "extractor revision");

    // A forced language is a key input even when it equals the detected one.
    let forced = SourceFile::read_with_language(&path, Some(ProgrammingLanguage::Rust)).expect("read");
    assert_ne!(cache.key(&forced).expect("key"), key, "forced language");
    assert_eq!(cached(&cache, forced, &query).0, CacheLookup::Miss(CacheMiss::Absent));

    // Edited lint query text changes only the diagnostics fingerprint: a
    // listing is still served, a diagnostics request recomputes them.
    let complete = SymbolQuery::complete_index();
    cached(&cache, read(&path), &complete);
    let lint_edited = cache_in(&dir).with_query_text(
        ProgrammingLanguage::Rust,
        QueryKind::Lint,
        "; no lint patterns",
    );
    assert_eq!(lint_edited.key(&read(&path)).expect("key"), key, "lint text is not a key input");
    assert_eq!(cached(&lint_edited, read(&path), &query).0, CacheLookup::Hit);
    assert_eq!(
        cached(&lint_edited, read(&path), &complete).0,
        CacheLookup::WidenedCapabilities { parsed: true }
    );
    // Experimental semantic rules likewise.
    let experimental = read(&path).experimental_semantics(true);
    let expected = as_value(
        &read(&path)
            .experimental_semantics(true)
            .query(&complete)
            .expect("uncached experimental"),
    );
    let (lookup, served, _) = cached(&cache, experimental, &complete);
    assert_eq!(lookup, CacheLookup::WidenedCapabilities { parsed: true }, "experimental semantics");
    assert_eq!(served, expected, "experimental semantics");

    // Stored schema, fingerprint and source hash must match the request.
    let entry_path = files_with_extension(dir.path(), "json")
        .into_iter()
        .find(|entry| entry.file_stem().is_some_and(|stem| stem == key.as_str()))
        .expect("the detected-language entry");
    let original = read_entry(&entry_path);
    for (field, value, reason) in [
        ("schema_version", json!({ "major": 3, "minor": 0 }), "schema version"),
        ("schema_version", json!({ "major": 4, "minor": 0 }), "schema version"),
        ("schema_version", json!({ "major": 4, "minor": 3 }), "schema version"),
        ("fingerprint", json!("0000"), "fingerprint"),
        ("source_hash", json!("0000"), "source hash"),
        ("file", json!("elsewhere.rs"), "file"),
        ("language", json!("Python"), "language"),
    ] {
        let mut edited = original.clone();
        edited[field] = value;
        write_entry(&entry_path, &edited);
        let (lookup, served, _) = cached(&cache, read(&path), &complete);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Mismatched(reason)), "{field}");
        assert_eq!(served, uncached(&path, &complete), "{field}");
    }

    // An edited source has a new hash, so a new key.
    std::fs::write(&path, "fn edited() {}\n").expect("edit source");
    assert_eq!(cached(&cache, read(&path), &query).0, CacheLookup::Miss(CacheMiss::Absent));
}

/// A record whose `schema_version` is not exactly the current one makes the
/// whole entry a miss, for the same request and for a wider one (never
/// widened), whichever record carries it; the rewritten entry holds only
/// current records and is a hit next time.
#[test]
fn incompatible_record_schema_versions_are_misses_never_served_or_widened() {
    let path = fixture(ProgrammingLanguage::Python);
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS);
    let complete = SymbolQuery::complete_index();
    let current = json!({ "major": 5, "minor": 2 });
    for version in [
        json!({ "major": 2, "minor": 0 }),
        json!({ "major": 3, "minor": 0 }),
        json!({ "major": 4, "minor": 0 }),
        json!({ "major": 4, "minor": 1 }),
        json!({ "major": 4, "minor": 2 }),
        json!({ "major": 4, "minor": 3 }),
        json!({ "major": 5, "minor": 0 }),
        json!({ "major": 5, "minor": 1 }),
        json!({ "major": 6, "minor": 0 }),
    ] {
        for (position, request) in [("first", &narrow), ("last", &narrow), ("first", &complete)] {
            let label = format!("record {version} on the {position} record, request {request:?}");
            let dir = TempDir::new().expect("temp");
            let cache = cache_in(&dir);
            cached(&cache, read(&path), &narrow);
            let entry_path = only_entry(&dir);
            let mut entry = read_entry(&entry_path);
            let records = entry["records"].as_array_mut().expect("entry records");
            assert!(records.len() > 1, "{label}: the fixture stores several records");
            let record = if position == "first" {
                records.first_mut()
            } else {
                records.last_mut()
            };
            record.expect("a record")["schema_version"] = version.clone();
            write_entry(&entry_path, &entry);

            let (lookup, served, _) = cached(&cache, read(&path), request);
            assert_eq!(
                lookup,
                CacheLookup::Miss(CacheMiss::Mismatched("record schema version")),
                "{label}"
            );
            assert_eq!(served, uncached(&path, request), "{label}");

            let rewritten = read_entry(&entry_path);
            assert!(
                rewritten["records"]
                    .as_array()
                    .expect("rewritten records")
                    .iter()
                    .all(|record| record["schema_version"] == current),
                "{label}: the rewritten entry holds only current records"
            );
            assert_eq!(cached(&cache, read(&path), request).0, CacheLookup::Hit, "{label}");
        }
    }
}

#[test]
fn a_corrupt_entry_is_a_miss_and_is_replaced() {
    let path = fixture(ProgrammingLanguage::Python);
    let query = SymbolQuery::complete_index();
    let expected = uncached(&path, &query);
    for (label, corrupt) in [
        ("truncated", None),
        ("empty", Some(Vec::new())),
        ("not JSON", Some(b"not json".to_vec())),
    ] {
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &query);
        let entry = only_entry(&dir);
        let bytes = std::fs::read(&entry).expect("read entry");
        let corrupt = corrupt.unwrap_or_else(|| bytes[..bytes.len() / 2].to_vec());
        std::fs::write(&entry, corrupt).expect("corrupt entry");

        let (lookup, served, _) = cached(&cache, read(&path), &query);
        assert!(matches!(lookup, CacheLookup::Miss(CacheMiss::Malformed(_))), "{label}: {lookup:?}");
        assert_eq!(served, expected, "{label}: never an empty result");
        let (lookup, served, _) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Hit, "{label}: the entry was replaced");
        assert_eq!(served, expected, "{label}");
    }
}

/// The reported reason names the field and tells an absent field from an
/// explicit `null`.
#[test]
fn absent_and_null_entry_fields_report_different_reasons() {
    let path = fixture(ProgrammingLanguage::Python);
    let query = SymbolQuery::complete_index();
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    cached(&cache, read(&path), &query);
    let entry_path = only_entry(&dir);
    let original = read_entry(&entry_path);

    for field in [
        "schema_version",
        "fingerprint",
        "source_hash",
        "covered_kinds",
        "completed_capabilities",
        "diagnostics_fingerprint",
        "records",
    ] {
        let mut absent = original.clone();
        absent.as_object_mut().expect("entry object").remove(field);
        write_entry(&entry_path, &absent);
        let (lookup, ..) = cached(&cache, read(&path), &query);
        let CacheLookup::Miss(CacheMiss::Malformed(absent_reason)) = lookup else {
            panic!("{field} absent: {lookup:?}");
        };
        assert!(absent_reason.contains(&format!("missing field `{field}`")), "{field}: {absent_reason}");

        let mut null = original.clone();
        null[field] = Value::Null;
        write_entry(&entry_path, &null);
        let (lookup, ..) = cached(&cache, read(&path), &query);
        let null_reason = match lookup {
            CacheLookup::Miss(CacheMiss::Malformed(reason)) => reason,
            // `null` is the encoding of "no diagnostics", which contradicts
            // the stored diagnostics capability.
            CacheLookup::Miss(CacheMiss::Inconsistent(reason)) if field == "diagnostics_fingerprint" => {
                reason.to_string()
            }
            other => panic!("{field} null: {other:?}"),
        };
        assert!(!null_reason.contains("missing field"), "{field}: {null_reason}");
        assert_ne!(absent_reason, null_reason, "{field}");
    }
}

/// The list form of `covered_kinds`, from a narrow entry the cache wrote.
#[test]
fn covered_kind_lists_are_served_only_in_canonical_form() {
    let path = fixture(ProgrammingLanguage::Rust);
    let query = SymbolQuery::new().kinds([SymbolKindV2::Function, SymbolKindV2::Method]);
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    cached(&cache, read(&path), &query);
    let entry_path = only_entry(&dir);
    let original = read_entry(&entry_path);
    assert_eq!(original["covered_kinds"], json!(["Function", "Method"]), "control shape");

    for (label, value, served) in [
        ("control", json!(["Function", "Method"]), true),
        ("one wrong element", json!(["Function", "Method", 123]), false),
        ("every element wrong", json!([123, 456]), false),
        ("unknown kind", json!(["Function", "Methods"]), false),
        ("empty", json!([]), false),
        ("unordered", json!(["Method", "Function"]), false),
        ("repeated", json!(["Function", "Function", "Method"]), false),
        ("other string", json!("every"), false),
        ("wrong type", json!(123), false),
        ("object", json!({}), false),
    ] {
        let mut edited = original.clone();
        edited["covered_kinds"] = value;
        write_entry(&entry_path, &edited);
        let (lookup, result, _) = cached(&cache, read(&path), &query);
        if served {
            assert_eq!(lookup, CacheLookup::Hit, "{label}");
        } else {
            assert!(matches!(lookup, CacheLookup::Miss(CacheMiss::Malformed(_))), "{label}: {lookup:?}");
        }
        assert_eq!(result, uncached(&path, &query), "{label}");
    }

    // A record of a kind the entry does not cover contradicts its coverage.
    let mut edited = original.clone();
    edited["records"][0]["kind"] = json!("Variable");
    write_entry(&entry_path, &edited);
    assert_eq!(
        cached(&cache, read(&path), &query).0,
        CacheLookup::Miss(CacheMiss::Inconsistent("a record of a kind the entry does not cover"))
    );

    // So do a metrics facet and a set flag that no capability of the entry
    // computed; the result is recomputed, without them.
    for (label, pointer, value, reason) in [
        (
            "metrics facet",
            "/records/0/metrics",
            json!({ "parameters": 1, "has_docs": false, "lines": null, "cyclomatic": null }),
            "metrics stored without the metrics capability",
        ),
        ("is_test", "/records/0/visibility/is_test", json!(true), "a flag set without the flags capability"),
        (
            "is_deprecated",
            "/records/0/visibility/is_deprecated",
            json!(true),
            "a flag set without the flags capability",
        ),
    ] {
        let mut edited = original.clone();
        let (parent, key) = pointer.rsplit_once('/').expect("pointer");
        edited_object(&mut edited, parent).insert(key.to_string(), value);
        write_entry(&entry_path, &edited);
        let (lookup, result, _) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Inconsistent(reason)), "{label}");
        assert_eq!(result, uncached(&path, &query), "{label}");
    }
}

fn edited_object<'a>(
    value: &'a mut serde_json::Value,
    pointer: &str,
) -> &'a mut serde_json::Map<String, serde_json::Value> {
    value
        .pointer_mut(pointer)
        .and_then(serde_json::Value::as_object_mut)
        .expect("object at pointer")
}

/// An entry that claims flags and metrics serves the stored values as
/// computed; one whose record lost its metrics facet contradicts the claim.
#[test]
fn flags_and_metrics_are_served_only_under_their_capabilities() {
    let path = fixture(ProgrammingLanguage::Java);
    let dir = TempDir::new().expect("temp");
    let query = SymbolQuery::new()
        .kinds(FUNCTION_KINDS)
        .capabilities([Capability::Flags, Capability::Metrics]);
    let cache = cache_in(&dir);
    let (cold, _, _) = cached(&cache, read(&path), &query);
    assert!(matches!(cold, CacheLookup::Miss(CacheMiss::Absent)), "{cold:?}");
    let entry_path = only_entry(&dir);
    let original = read_entry(&entry_path);
    assert_eq!(
        original["completed_capabilities"],
        json!(["declarations", "flags", "metrics"])
    );
    let records = original["records"].as_array().expect("records");
    assert!(records.iter().all(|record| record["metrics"].is_object()), "every record has metrics");
    assert!(
        records.iter().any(|record| record["visibility"]["is_deprecated"] == json!(true)),
        "the Java fixture holds a deprecated method"
    );

    let (lookup, result, _) = cached(&cache, read(&path), &query);
    assert_eq!(lookup, CacheLookup::Hit);
    assert_eq!(result, uncached(&path, &query));

    // Flags alone are served from the same entry, with metrics projected away.
    let flags_only = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Flags]);
    let (lookup, result, _) = cached(&cache, read(&path), &flags_only);
    assert_eq!(lookup, CacheLookup::Hit);
    assert_eq!(result, uncached(&path, &flags_only));

    let mut edited = original.clone();
    edited_object(&mut edited, "/records/0").remove("metrics");
    write_entry(&entry_path, &edited);
    let (lookup, result, _) = cached(&cache, read(&path), &query);
    assert_eq!(
        lookup,
        CacheLookup::Miss(CacheMiss::Inconsistent("a record without metrics under the metrics capability"))
    );
    assert_eq!(result, uncached(&path, &query));
}

/// Writers racing on one file, each with its own cache handle (as separate
/// processes would be): whatever entry is left is readable and serves its
/// recorded coverage as a complete hit equal to an uncached query.
#[test]
fn concurrent_writers_leave_a_readable_entry_whose_coverage_matches() {
    let path = fixture(ProgrammingLanguage::TypeScript);
    let dir = TempDir::new().expect("temp");
    let queries = [
        SymbolQuery::new(),
        SymbolQuery::new().kinds(FUNCTION_KINDS),
        SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Containment]),
        SymbolQuery::new().capabilities([Capability::StructuredDocs]),
        SymbolQuery::new().capabilities([Capability::Semantics]),
        SymbolQuery::complete_index(),
    ];
    let rounds = 5;
    let barrier = Arc::new(Barrier::new(queries.len()));

    std::thread::scope(|scope| {
        for query in &queries {
            let (barrier, path, dir) = (Arc::clone(&barrier), &path, &dir);
            scope.spawn(move || {
                let cache = cache_in(dir);
                for _ in 0..rounds {
                    barrier.wait();
                    // A write may lose a rename race (Windows can refuse
                    // one); the result is complete either way.
                    let outcome = cache.query(read(path), query).expect("query");
                    assert_eq!(as_value(&outcome.result), uncached(path, query));
                }
            });
        }
    });

    assert!(files_with_extension(dir.path(), "tmp").is_empty(), "no temporary file left");
    let entry = read_entry(&only_entry(&dir));
    let coverage = coverage_query(&entry);
    let (lookup, served, work) = cached(&cache_in(&dir), read(&path), &coverage);
    assert_eq!(lookup, CacheLookup::Hit, "the surviving entry serves its coverage");
    assert_eq!(work, WorkCounters::default());
    assert_eq!(served, uncached(&path, &coverage), "its contents match its coverage");
}

/// `--no-cache` in the library: a disabled store never reads or writes.
#[test]
fn a_disabled_store_neither_reads_nor_writes() {
    let path = fixture(ProgrammingLanguage::Go);
    let dir = TempDir::new().expect("temp");
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), false));
    let query = SymbolQuery::new();
    for _ in 0..2 {
        let (lookup, served, work) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent));
        assert_eq!(work.parser_constructions, 1);
        assert_eq!(served, uncached(&path, &query));
    }
    assert!(files_with_extension(dir.path(), "json").is_empty());
}

/// The kinds the deferred declarations report.
const DEFERRED_KINDS: [SymbolKindV2; 5] = [
    SymbolKindV2::Constructor,
    SymbolKindV2::EnumVariant,
    SymbolKindV2::TypeAlias,
    SymbolKindV2::Field,
    SymbolKindV2::Namespace,
];

/// The IDs of a serialized result's records, in order.
fn record_ids(result: &Value) -> Vec<Value> {
    result["index"]["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .map(|record| record["id"].clone())
        .collect()
}

crate::language_cases::language_tests!(deferred_kind_entries_never_claim_complete_coverage_and_warm_hits_do_no_work => check_deferred_kind_entries_never_claim_complete_coverage_and_warm_hits_do_no_work);

/// For every source of the deferred kind matrix: a filtered query of the
/// deferred kinds stores every record of those kinds and records only those
/// kinds as its coverage, never the whole file; the unfiltered query is then
/// a hit, an every-kind query widens the kinds once, and a containment query
/// widens the capabilities without diagnostics. Afterwards each deferred kind,
/// with and without containment, is a hit that does no work and equals an
/// uncached query.
fn check_deferred_kind_entries_never_claim_complete_coverage_and_warm_hits_do_no_work(language: ProgrammingLanguage) {
    let deferred = SymbolQuery::new().kinds(DEFERRED_KINDS);
    let filtered = deferred.clone().name(NamePattern::new("no_declaration_has_this_name"));
    let mut sorted_kinds = DEFERRED_KINDS.to_vec();
    sorted_kinds.sort();
    let sources = TempDir::new().expect("sources");
    let (mut stored_records, mut warm_records) = (0, 0);
    for (ordinal, (row_language, source)) in deferred_kind_sources().into_iter().enumerate().filter(|(_, row)| row.0 == language) {
        assert_eq!(row_language, language);
        let path = sources.path().join(format!("deferred_{ordinal}.{}", language.extensions()[0]));
        std::fs::write(&path, source).expect("write source");
        let label = format!("{language:?} {source:?}");
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);

        let (lookup, served, _) = cached(&cache, read(&path), &filtered);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent), "{label}");
        assert_eq!(served, uncached(&path, &filtered), "{label}: filtered");
        assert_eq!(served["index"]["symbols"], json!([]), "{label}: the name filter keeps nothing");
        let entry = read_entry(&only_entry(&dir));
        assert_eq!(entry["covered_kinds"], json!(sorted_kinds), "{label}: the entry covers the deferred kinds only");
        let stored: Vec<Value> = entry["records"].as_array().expect("records").iter().map(|record| record["id"].clone()).collect();
        assert_eq!(stored, record_ids(&uncached(&path, &deferred)), "{label}: every deferred-kind record is stored");
        stored_records += stored.len();

        let (lookup, served, work) = cached(&cache, read(&path), &deferred);
        assert_eq!(lookup, CacheLookup::Hit, "{label}: unfiltered");
        assert_eq!(work, WorkCounters::default(), "{label}: unfiltered");
        assert_eq!(served, uncached(&path, &deferred), "{label}: unfiltered");

        let every_kind = SymbolQuery::new();
        let (lookup, served, work) = cached(&cache, read(&path), &every_kind);
        assert_eq!(lookup, CacheLookup::WidenedKinds, "{label}: a narrow entry cannot answer every kind");
        assert_eq!(served, uncached(&path, &every_kind), "{label}: every kind");
        assert_eq!(
            (work.parser_constructions, work.definition_captures, work.diagnostics_runs),
            (1, 1, 0),
            "{label}: one re-extraction for the union"
        );
        assert_eq!(read_entry(&only_entry(&dir))["covered_kinds"], "all", "{label}");

        let contained = |kind: SymbolKindV2| SymbolQuery::new().kinds([kind]).capabilities([Capability::Containment]);
        let (lookup, served, work) = cached(&cache, read(&path), &contained(SymbolKindV2::Constructor));
        assert!(matches!(lookup, CacheLookup::WidenedCapabilities { .. }), "{label}: {lookup:?}");
        assert_eq!(served, uncached(&path, &contained(SymbolKindV2::Constructor)), "{label}: containment");
        assert!(work.definition_captures <= 1 && work.diagnostics_runs == 0, "{label}: {work:?}");

        for kind in DEFERRED_KINDS {
            for query in [SymbolQuery::new().kinds([kind]), contained(kind)] {
                let (lookup, served, work) = cached(&cache, read(&path), &query);
                assert_eq!(lookup, CacheLookup::Hit, "{label}: {query:?}");
                assert_eq!(work, WorkCounters::default(), "{label}: {query:?}");
                assert_eq!(served, uncached(&path, &query), "{label}: {query:?}");
                warm_records += record_ids(&served).len();
            }
        }
    }
    let minimum = crate::language_cases::corpus_minimums(language).deferred;
    assert!(stored_records >= minimum, "{language}: too few stored records ({stored_records})");
    assert!(warm_records >= 2 * minimum, "{language}: too few warm records ({warm_records})");
}

/// `(language, source, name, qualified name, kind before the deferred
/// kinds landed, kind now)`.
type IdentityRow = (ProgrammingLanguage, &'static str, &'static str, &'static str, SymbolKindV2, SymbolKindV2);

/// A record's ID digests its kind and qualified name, so a reclassified
/// declaration gets a new ID while its parameters, a namespace's members and
/// unrelated records keep theirs: a captured namespace renames nothing. The
/// earlier kinds and names are what extractor revision 13 reported.
#[test]
fn reclassified_records_get_new_ids_and_unrelated_ids_stay() {
    use ProgrammingLanguage as L;
    use SymbolKindV2 as K;
    const PY: &str = "class C:\n    def __init__(self, value):\n        pass\n";
    const CPP: &str = "class W {\npublic:\n    W(int x);\n};\n";
    const PHP_CLASS: &str = "<?php\nclass P {\n    public function __construct($x) {}\n}\n";
    const PHP_NAMESPACE: &str = "<?php\nnamespace App;\n\nfunction inside() {}\n";
    const TS: &str = "namespace Space {\n  export function inside() {}\n}\n";
    const SCALA: &str = "class K {\n  val width = 1\n}\n";
    const C: &str = "enum E { RED };\n";
    const GO: &str = "package p\n\ntype S struct {\n\tX int\n}\n";
    let rows: &[IdentityRow] = &[
        (L::Python, PY, "__init__", "C.__init__", K::Method, K::Constructor),
        (L::Python, PY, "value", "C.__init__.value", K::Parameter, K::Parameter),
        (L::Python, PY, "C", "C", K::Class, K::Class),
        (L::Cpp, CPP, "W", "W::W", K::Method, K::Constructor),
        (L::Cpp, CPP, "x", "W::W::x", K::Parameter, K::Parameter),
        (L::Php, PHP_CLASS, "__construct", "P::__construct", K::Method, K::Constructor),
        (L::Php, PHP_NAMESPACE, "inside", "App\\inside", K::Function, K::Function),
        (L::TypeScript, TS, "inside", "Space.inside", K::Function, K::Function),
        (L::Scala, SCALA, "width", "K.width", K::Variable, K::Field),
        (L::C, C, "RED", "E.RED", K::Variable, K::EnumVariant),
        (L::Go, GO, "S", "p.S", K::Type, K::Type),
    ];
    let dir = TempDir::new().expect("temp");
    for (ordinal, &(language, source, name, qualified_name, before, now)) in rows.iter().enumerate() {
        let path = dir.path().join(format!("identity_{ordinal}.{}", language.extensions()[0]));
        std::fs::write(&path, source).expect("write source");
        let index = TreeFile::new(&path).expect("parse").symbol_index_v2().expect("index");
        let label = format!("{language:?} `{qualified_name}`");
        // The name is unchanged: exactly one `name` is called `qualified_name`.
        let matching: Vec<_> = index
            .symbols
            .iter()
            .filter(|record| record.identity.name == name && in_file_name(record) == qualified_name)
            .collect();
        assert_eq!(matching.len(), 1, "{label} in {source:?}");
        let record = matching[0];
        assert_eq!(record.kind, now, "{label}");
        let id_as = |kind: SymbolKindV2| {
            symbol_id_from_stable_key(&stable_symbol_key(language, &record.source.file_path, kind, qualified_name, 0))
        };
        // For an unrelated record `before == now`: the revision-13 ID.
        assert_eq!(record.id, id_as(now), "{label}: the ID digests the kind and name");
        if before != now {
            assert_ne!(record.id, id_as(before), "{label}: a reclassified record gets a new ID");
        }
    }
}

trait UnionForTest {
    fn union_for_test(&self, other: &SymbolQuery) -> SymbolQuery;
}

/// The union of two queries through the public builder, so the expected
/// coverage is not computed by the code under test.
impl UnionForTest for SymbolQuery {
    fn union_for_test(&self, other: &SymbolQuery) -> SymbolQuery {
        let capabilities: BTreeSet<Capability> = self
            .required_capabilities()
            .into_iter()
            .chain(other.required_capabilities())
            .collect();
        let union = SymbolQuery::new().capabilities(capabilities);
        match (self.selected_kinds(), other.selected_kinds()) {
            (Some(mine), Some(theirs)) => union.kinds(mine.into_iter().chain(theirs)),
            _ => union,
        }
    }
}

/// The declared payload of a Scala enum case (named fields with their
/// written types, every parameter clause) is stored and served from a warm
/// entry, for the complete index and for an enum-only query, exactly as an
/// uncached run computes it.
#[test]
fn warm_enum_case_payloads_equal_uncached_results() {
    use crate::symbols_field_lists::{SCALA_BRACE_ENUM, SCALA_INDENTED_ENUM};
    let sources = TempDir::new().expect("sources");
    let cases = [
        (SCALA_BRACE_ENUM, "Shape", "Curried", json!([["x", "Int"], ["order", "Ordering[T]"]])),
        (SCALA_INDENTED_ENUM, "Mode", "Level", json!([["value", "Int"], ["render", "Int => String"]])),
    ];
    for (ordinal, (source, enum_name, variant, expected_fields)) in cases.into_iter().enumerate() {
        let path = sources.path().join(format!("enum_{ordinal}.scala"));
        std::fs::write(&path, source).expect("write source");
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        let complete = SymbolQuery::complete_index();
        let (lookup, _, _) = cached(&cache, read(&path), &complete);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent), "{enum_name}: cold");

        for query in [complete.clone(), SymbolQuery::new().kinds([SymbolKindV2::Enum])] {
            let (lookup, served, work) = cached(&cache, read(&path), &query);
            assert_eq!(lookup, CacheLookup::Hit, "{enum_name}: warm {query:?}");
            assert_eq!(work.parser_constructions, 0, "{enum_name}: warm {query:?}");
            assert_eq!(served, uncached(&path, &query), "{enum_name}: warm {query:?}");
            let owner: Vec<&Value> = served["index"]["symbols"]
                .as_array()
                .expect("symbols")
                .iter()
                .filter(|record| record["kind"] == "Enum" && record["identity"]["name"] == enum_name)
                .collect();
            assert_eq!(owner.len(), 1, "{enum_name}: one enum record");
            let entry: Vec<&Value> = owner[0]["kind_data"]["data"]["variants"]
                .as_array()
                .expect("variants")
                .iter()
                .filter(|entry| entry["name"] == variant)
                .collect();
            assert_eq!(entry.len(), 1, "{enum_name}: one `{variant}` entry");
            let fields: Vec<Value> = entry[0]["struct_fields"]
                .as_array()
                .expect("struct fields")
                .iter()
                .map(|field| json!([field["name"], field["type_text"]]))
                .collect();
            assert_eq!(Value::from(fields), expected_fields, "{enum_name}: `{variant}` payload {query:?}");
            assert!(entry[0].get("discriminant").is_none(), "{enum_name}: `{variant}` has no discriminant");
        }
    }
}

crate::symbols_relations::declaration_fact_group_tests!(
    declaration_fact_sources_are_warm_hits_and_project_to_uncached_queries => warm_hits_project_to_uncached_queries
);

/// Every source of `group` of the declaration-fact corpus is stored
/// complete, served warm as a real `Hit` with no work, and projected to each
/// narrower query exactly as an uncached query computes it: generic
/// parameters, modifiers, attributes and relation targets kept, Rust impl
/// containers and `metrics` dropped where the query does not ask for them.
/// The Rust group must exercise each projection.
fn warm_hits_project_to_uncached_queries(group: usize) {
    let method: &[SymbolKindV2] = &[SymbolKindV2::Method];
    let cells: [(Option<&[SymbolKindV2]>, &[Capability]); 6] = [
        (None, &[Capability::Declarations]),
        (None, &[Capability::Containment]),
        (None, &[Capability::Metrics]),
        (Some(method), &[Capability::Declarations]),
        (Some(method), &[Capability::Containment]),
        (Some(&TYPE_KINDS), &[Capability::Containment, Capability::Metrics]),
    ];
    let sources = TempDir::new().expect("sources");
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let complete = SymbolQuery::complete_index();
    let mut dropped_containers = 0usize;
    let mut dropped_lines = 0usize;
    let mut kept_relations = 0usize;

    for (label, path) in crate::symbols_relations::write_declaration_fact_group(sources.path(), group) {
        let expected = uncached(&path, &complete);
        let (lookup, cold, _) = cached(&cache, read(&path), &complete);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent), "{label}: cold");
        assert_eq!(cold, expected, "{label}: cold result");
        let (lookup, warm, work) = cached(&cache, read(&path), &complete);
        assert_eq!(lookup, CacheLookup::Hit, "{label}: warm");
        assert_eq!(work, WorkCounters::default(), "{label}: a complete hit did work");
        assert_eq!(warm, expected, "{label}: warm result");

        for (kinds, capabilities) in cells {
            let mut query = SymbolQuery::new().capabilities(capabilities.iter().copied());
            if let Some(kinds) = kinds {
                query = query.kinds(kinds.iter().copied());
            }
            let cell = format!("{label}: kinds {kinds:?}, capabilities {capabilities:?}");
            let (lookup, served, work) = cached(&cache, read(&path), &query);
            assert_eq!(lookup, CacheLookup::Hit, "{cell}");
            assert_eq!(work, WorkCounters::default(), "{cell}: work");
            assert_eq!(served, uncached(&path, &query), "{cell}");
            if !query.requires(Capability::Containment) {
                dropped_containers += count_records(&expected, |record| !record["relations"]["container"].is_null());
            }
            if !query.requires(Capability::Metrics) {
                dropped_lines += count_records(&expected, |record| record["metrics"]["lines"].is_u64());
            }
            kept_relations += count_records(&served, |record| {
                record["relations"]["extends"].as_array().is_some_and(|list| !list.is_empty())
                    || record["relations"]["implements"].as_array().is_some_and(|list| !list.is_empty())
            });
        }
    }
    if group != crate::symbols_relations::RUST_FACT_GROUP {
        return;
    }
    assert!(dropped_containers > 0, "no projection removed a container");
    assert!(dropped_lines > 0, "no projection removed `metrics.lines`");
    assert!(kept_relations > 0, "no projection kept a relation");
}

crate::symbols_relations::declaration_fact_group_tests!(
    declaration_fact_entries_widen_by_containment_and_metrics => entries_widen_by_containment_and_metrics
);

/// Widening a declarations-only entry of the same sources (one corpus
/// group): Containment is
/// added from the stored records alone, except in Rust and Swift, where an
/// impl member's container is its impl's resolved self type and an
/// extension member's its extension's resolved owner, which no record
/// retains, so it parses once; Metrics always parses once. Each widened
/// result equals an uncached query, the entry then records every request,
/// and each request (and the declarations-only narrowing) is a `Hit` with
/// no work. The Rust group must reach the parsing path through Rust.
fn entries_widen_by_containment_and_metrics(group: usize) {
    let sources = TempDir::new().expect("sources");
    let declarations = SymbolQuery::new();
    let containment = SymbolQuery::new().capabilities([Capability::Containment]);
    let metrics = SymbolQuery::new().capabilities([Capability::Containment, Capability::Metrics]);
    let mut rust_rows = 0usize;

    for (label, path) in crate::symbols_relations::write_declaration_fact_group(sources.path(), group) {
        let is_rust = path.extension().is_some_and(|extension| extension == "rs");
        rust_rows += usize::from(is_rust);
        let needs_tree = is_rust || path.extension().is_some_and(|extension| extension == "swift");
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &declarations);

        for (query, parsed) in [(&containment, needs_tree), (&metrics, true)] {
            let cell = format!("{label}: widen to {query:?}");
            let (lookup, widened, work) = cached(&cache, read(&path), query);
            assert_eq!(lookup, CacheLookup::WidenedCapabilities { parsed }, "{cell}");
            assert_eq!(work.parser_constructions, u64::from(parsed), "{cell}: parses");
            assert_eq!(work.definition_captures, u64::from(parsed), "{cell}: captures");
            assert_eq!(work.relation_links, u64::from(parsed), "{cell}: relation links");
            assert_eq!(work.diagnostics_runs, 0, "{cell}: diagnostics");
            assert_eq!(widened, uncached(&path, query), "{cell}: widened result");
        }
        let entry = read_entry(&only_entry(&dir));
        assert_eq!(coverage_query(&entry), metrics, "{label}: the entry records every request");
        for query in [&declarations, &containment, &metrics] {
            let (lookup, served, work) = cached(&cache, read(&path), query);
            assert_eq!(lookup, CacheLookup::Hit, "{label}: then {query:?}");
            assert_eq!(work, WorkCounters::default(), "{label}: then {query:?}");
            assert_eq!(served, uncached(&path, query), "{label}: then {query:?}");
        }
    }
    if group == crate::symbols_relations::RUST_FACT_GROUP {
        assert!(rust_rows >= 5, "the Rust rows exercise the parsing path ({rust_rows})");
    }
}

/// An entry of the Rust impl source whose records carry the previous schema
/// version (4.0, without `metrics.lines`) is never served or widened.
#[test]
fn a_declaration_fact_entry_of_an_older_record_schema_is_a_miss() {
    let sources = TempDir::new().expect("sources");
    let (label, path) =
        crate::symbols_relations::write_declaration_fact_group(sources.path(), crate::symbols_relations::RUST_FACT_GROUP)
            .into_iter()
            .find(|(label, _)| label.starts_with("rust impls"))
            .expect("the Rust impl source");
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    cached(&cache, read(&path), &SymbolQuery::new());
    let entry_path = only_entry(&dir);
    let mut entry = read_entry(&entry_path);
    for record in entry["records"].as_array_mut().expect("records") {
        record["schema_version"] = json!({ "major": 4, "minor": 0 });
    }
    write_entry(&entry_path, &entry);
    let query = SymbolQuery::new().capabilities([Capability::Containment]);
    let (lookup, served, _) = cached(&cache, read(&path), &query);
    assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Mismatched("record schema version")), "{label}");
    assert_eq!(served, uncached(&path, &query), "{label}");
}

/// The impl-member rows of the path-meaning matrix (Rust impls and the C#
/// containment control), cached declarations-only and then asked for
/// Containment: the widened result (parsed once for Rust) and
/// reports exactly the uncached facts (a self type through an external `::`
/// path, an enclosing generic parameter or a block import stays a textual
/// container with no parent and no membership, and the same-named
/// file-level type gains nothing); the next Containment request is a real
/// `Hit` with no work and the same facts.
#[test]
fn path_meaning_impl_rows_widen_to_containment_with_the_same_facts() {
    use crate::symbols_relations::{check_impl_facts, path_meaning_rows};

    let sources = TempDir::new().expect("sources");
    let declarations = SymbolQuery::new();
    let containment = SymbolQuery::new().capabilities([Capability::Containment]);
    let mut rows = 0;
    for (idx, row) in path_meaning_rows().filter(|row| !row.impl_members.is_empty()).enumerate() {
        rows += 1;
        let path = sources.path().join(format!("impl_row_{idx}.{}", row.extension));
        std::fs::write(&path, row.source).expect("write row source");
        let complete = read(&path).query(&SymbolQuery::complete_index()).expect("complete").index;
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &declarations);

        let (widened, work) = counted(|| cache.query(read(&path), &containment).expect("widen"));
        let label = format!("{}: widened", row.label);
        // Rust impl and Swift extension owners are resolved from the tree;
        // every other language links containers from the stored records.
        let parsed = matches!(row.language, ProgrammingLanguage::Rust | ProgrammingLanguage::Swift);
        assert_eq!(widened.lookup, CacheLookup::WidenedCapabilities { parsed }, "{label}");
        assert_eq!(work.parser_constructions, u64::from(parsed), "{label}: parses only when needed");
        assert_eq!(as_value(&widened.result), uncached(&path, &containment), "{label}: equals uncached");
        check_impl_facts(&label, row, &complete, &widened.result.index, true);

        let (warm, work) = counted(|| cache.query(read(&path), &containment).expect("warm"));
        let label = format!("{}: warm", row.label);
        assert_eq!(warm.lookup, CacheLookup::Hit, "{label}");
        assert_eq!(work, WorkCounters::default(), "{label}: no work");
        check_impl_facts(&label, row, &complete, &warm.result.index, true);
    }
    assert!(rows >= 5, "the impl rows ran ({rows})");
}

crate::symbols_relations::declaration_fact_group_tests!(shared path_meaning_rows_hold_on_warm_hits => rows_hold_on_warm_hits);

/// Every path-meaning row of `group`
/// ([`path_meaning_group`](crate::symbols_relations::path_meaning_group))
/// with written bases, stored complete, is served warm as a real `Hit`
/// on the complete index and on a type-only query, and each owner keeps
/// exactly its uncached targets and target IDs.
fn rows_hold_on_warm_hits(group: usize) {
    use crate::symbols_relations::{check_owner_targets, path_meaning_group};

    let sources = TempDir::new().expect("sources");
    let complete_query = SymbolQuery::complete_index();
    let types_only = SymbolQuery::new().kinds(TYPE_KINDS);
    let mut rows = 0;
    for (idx, row) in path_meaning_group(group).filter(|row| !row.owners.is_empty()).enumerate() {
        rows += 1;
        let path = sources.path().join(format!("row_{idx}.{}", row.extension));
        std::fs::write(&path, row.source).expect("write row source");
        let complete = read(&path).query(&complete_query).expect("complete").index;
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &complete_query);
        for (path_label, query) in [("complete", &complete_query), ("type-only", &types_only)] {
            let label = format!("{}: {}: warm {path_label}", row.language.name(), row.label);
            let (warm, work) = counted(|| cache.query(read(&path), query).expect("warm"));
            assert_eq!(warm.lookup, CacheLookup::Hit, "{label}");
            assert_eq!(work, WorkCounters::default(), "{label}: no work");
            check_owner_targets(&label, &complete, &warm.result.index, row.owners);
        }
    }
    assert!(rows > 0, "group {group}: no written-base row ran");
}

/// The member-binding repairs through the cache: a Python member rebound
/// by a class-body import (module-level and function-local holder) and a
/// Swift extension member beside a closure's same-named owner, stored
/// declarations-only, widen to Containment (Python from the stored records,
/// Swift by one parse: an extension member's owner is resolved from the
/// tree), then to the complete index, and are served as a real `Hit`; every step keeps the
/// uncached facts (no target ID for the rebound member, the extension
/// method under the file-level type).
#[test]
fn member_binding_repairs_hold_through_widening_and_warm_hits() {
    use crate::symbols_owner_identity::{NESTED_SHADOW_MATRIX, check_nested_shadow};
    use crate::symbols_relations::{check_owner_targets, path_meaning_rows};

    let sources = TempDir::new().expect("sources");
    let declarations = SymbolQuery::new();
    let containment = SymbolQuery::new().capabilities([Capability::Containment]);
    let complete_query = SymbolQuery::complete_index();
    let python: Vec<_> =
        path_meaning_rows().filter(|row| row.label.ends_with("holder: aliased `from` import")).collect();
    assert_eq!(python.len(), 2, "module-level and function-local import rows");
    let swift = &NESTED_SHADOW_MATRIX[0];
    let mut cases: Vec<(String, &str, &str)> =
        python.iter().map(|row| (row.label.to_string(), row.extension, row.source)).collect();
    cases.push((format!("Swift: {}", swift.label), "swift", swift.source));

    for (idx, (label, extension, source)) in cases.iter().enumerate() {
        let path = sources.path().join(format!("member_{idx}.{extension}"));
        std::fs::write(&path, source).expect("write source");
        let complete = read(&path).query(&complete_query).expect("complete").index;
        let check = |step: &str, result: &SymbolQueryResult| {
            let step = format!("{label}: {step}");
            match python.get(idx) {
                Some(row) => check_owner_targets(&step, &complete, &result.index, row.owners),
                None => check_nested_shadow(&step, swift, &result.index.symbols),
            }
        };
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &declarations);

        let widened = cache.query(read(&path), &containment).expect("containment");
        let parsed = *extension == "swift";
        assert_eq!(widened.lookup, CacheLookup::WidenedCapabilities { parsed }, "{label}: containment");
        assert_eq!(as_value(&widened.result), uncached(&path, &containment), "{label}: containment equals uncached");
        check("containment widened", &widened.result);

        let widened = cache.query(read(&path), &complete_query).expect("complete");
        assert!(matches!(widened.lookup, CacheLookup::WidenedCapabilities { .. }), "{label}: {:?}", widened.lookup);
        assert_eq!(as_value(&widened.result), uncached(&path, &complete_query), "{label}: complete equals uncached");
        check("complete widened", &widened.result);

        for query in [&containment, &complete_query] {
            let (warm, work) = counted(|| cache.query(read(&path), query).expect("warm"));
            assert_eq!(warm.lookup, CacheLookup::Hit, "{label}: warm {query:?}");
            assert_eq!(work, WorkCounters::default(), "{label}: warm {query:?} did work");
            check("warm", &warm.result);
        }
    }
}

/// Unproved Swift extension owners through the cache: each source, stored
/// declarations-only, widens to Containment by one parse (the owner
/// decision needs the tree), then to the complete index, and is served as a
/// real `Hit` for both; every step leaves the method with a textual
/// container, no parent and no local membership, and `ReviewP` on no type.
#[test]
fn unproved_swift_extension_owners_hold_through_widening_and_warm_hits() {
    use crate::symbols_owner_identity::{UNPROVED_OWNER_MATRIX, check_unproved_owner};

    let sources = TempDir::new().expect("sources");
    let declarations = SymbolQuery::new();
    let containment = SymbolQuery::new().capabilities([Capability::Containment]);
    let complete_query = SymbolQuery::complete_index();
    for (idx, row) in UNPROVED_OWNER_MATRIX.iter().enumerate() {
        let path = sources.path().join(format!("unproved_{idx}.swift"));
        std::fs::write(&path, row.source).expect("write source");
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &declarations);

        let (widened, work) = counted(|| cache.query(read(&path), &containment).expect("containment"));
        assert_eq!(widened.lookup, CacheLookup::WidenedCapabilities { parsed: true }, "{}: containment", row.label);
        assert_eq!(work.parser_constructions, 1, "{}: one parse", row.label);
        assert_eq!(as_value(&widened.result), uncached(&path, &containment), "{}: containment equals uncached", row.label);
        check_unproved_owner("containment widened", row, &widened.result.index.symbols);

        let widened = cache.query(read(&path), &complete_query).expect("complete");
        assert!(matches!(widened.lookup, CacheLookup::WidenedCapabilities { .. }), "{}: {:?}", row.label, widened.lookup);
        assert_eq!(as_value(&widened.result), uncached(&path, &complete_query), "{}: complete equals uncached", row.label);
        check_unproved_owner("complete widened", row, &widened.result.index.symbols);

        for query in [&containment, &complete_query] {
            let (warm, work) = counted(|| cache.query(read(&path), query).expect("warm"));
            assert_eq!(warm.lookup, CacheLookup::Hit, "{}: warm {query:?}", row.label);
            assert_eq!(work, WorkCounters::default(), "{}: warm {query:?} did work", row.label);
            check_unproved_owner("warm", row, &warm.result.index.symbols);
        }
    }
}

/// Complexity through the cache: a declarations-only entry widened by
/// Metrics parses once and walks each measured callable once; the entry then
/// stores every `cyclomatic` value, a `null` included (a language without
/// counting rules or a bodyless declaration), and a warm Metrics request is
/// a `Hit` that parses and walks nothing. So an unavailable value is cached
/// like a number, never recomputed on every run.
#[test]
fn complexity_is_stored_once_and_served_on_warm_hits() {
    let sources = TempDir::new().expect("sources");
    let declarations = SymbolQuery::new();
    let metrics = SymbolQuery::new().capabilities([Capability::Metrics]);
    for (name, source, expected) in crate::symbols_query::COMPLEXITY_SOURCES {
        let path = sources.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);
        let (_, _, work) = cached(&cache, read(&path), &declarations);
        assert_eq!(work.complexity_walks, 0, "{name}: a declarations-only entry walks nothing");

        let (lookup, widened, work) = cached(&cache, read(&path), &metrics);
        assert_eq!(lookup, CacheLookup::WidenedCapabilities { parsed: true }, "{name}: widened");
        assert_eq!(work.parser_constructions, 1, "{name}: one parse");
        let measured = expected.iter().filter(|(_, cyclomatic)| cyclomatic.is_some()).count();
        assert_eq!(work.complexity_walks, measured as u64, "{name}: one walk per measured callable");
        assert_eq!(widened, uncached(&path, &metrics), "{name}: widened result");

        let entry = read_entry(&only_entry(&dir));
        for (symbol, cyclomatic) in expected {
            let record = entry["records"]
                .as_array()
                .expect("entry records")
                .iter()
                .find(|record| record["identity"]["name"] == *symbol)
                .unwrap_or_else(|| panic!("{name}: the entry has no `{symbol}`"));
            assert_eq!(record["metrics"]["cyclomatic"], json!(cyclomatic), "{name}: stored `{symbol}`");
        }

        for query in [&metrics, &declarations] {
            let (lookup, served, work) = cached(&cache, read(&path), query);
            assert_eq!(lookup, CacheLookup::Hit, "{name}: then {query:?}");
            assert_eq!(work, WorkCounters::default(), "{name}: then {query:?}");
            assert_eq!(served, uncached(&path, query), "{name}: then {query:?}");
        }
    }
}

/// A selective entry of a source whose unselected owner declares 65,537
/// generic parameters is stored from the first query and served warm as a
/// real `Hit`, and the inner type's base keeps the owner's shadowing (no
/// target ID) both times.
fn check_oversized_owner_warm_hit(language: ProgrammingLanguage) {
    use crate::symbols_relations::{OVERFLOWS, boundary_query, boundary_row, boundary_source, check_boundary_child};

    let row = boundary_row(language);
    let sources = TempDir::new().expect("sources");
    let path = sources.path().join(format!("boundary.{}", row.extension));
    std::fs::write(&path, boundary_source(row, OVERFLOWS)).expect("write source");
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let query = boundary_query(row);
    let cold = cache.query(read(&path), &query).expect("cold");
    assert!(matches!(cold.lookup, CacheLookup::Miss(_)), "{}: cold {:?}", language.name(), cold.lookup);
    check_boundary_child(&format!("{} cold", language.name()), row, &cold.result.index);
    let (warm, work) = counted(|| cache.query(read(&path), &query).expect("warm"));
    let label = format!("{} warm", language.name());
    assert_eq!(warm.lookup, CacheLookup::Hit, "{label}");
    assert_eq!(work, WorkCounters::default(), "{label}: no work");
    check_boundary_child(&label, row, &warm.result.index);
}

/// One test per generic reader, retaining the overflowing parameter count.
mod oversized_owner_warm_hits {
    use super::*;

    #[test]
    fn rust() { check_oversized_owner_warm_hit(ProgrammingLanguage::Rust) }
    #[test]
    fn typescript() { check_oversized_owner_warm_hit(ProgrammingLanguage::TypeScript) }
    #[test]
    fn go() { check_oversized_owner_warm_hit(ProgrammingLanguage::Go) }
    #[test]
    fn python() { check_oversized_owner_warm_hit(ProgrammingLanguage::Python) }
    #[test]
    fn java() { check_oversized_owner_warm_hit(ProgrammingLanguage::Java) }
    #[test]
    fn swift() { check_oversized_owner_warm_hit(ProgrammingLanguage::Swift) }
    #[test]
    fn scala() { check_oversized_owner_warm_hit(ProgrammingLanguage::Scala) }
    #[test]
    fn csharp() { check_oversized_owner_warm_hit(ProgrammingLanguage::CSharp) }
    #[test]
    fn cpp() { check_oversized_owner_warm_hit(ProgrammingLanguage::Cpp) }
}

/// The Rust impl inside an oversized generic function, stored by a
/// method-only query and widened by Containment, keeps a textual container
/// and no parent, and the next Containment request is a real `Hit` with
/// the same facts.
#[test]
fn an_oversized_owner_impl_row_widens_and_hits_with_the_same_facts() {
    use crate::symbols_relations::{check_impl_facts, oversized_impl_row};

    let row = oversized_impl_row();
    let sources = TempDir::new().expect("sources");
    let path = sources.path().join("oversized_impl.rs");
    std::fs::write(&path, row.source).expect("write source");
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let methods = SymbolQuery::new().kinds([SymbolKindV2::Method]);
    let contained = methods.clone().capabilities([Capability::Containment]);
    let (_, cold, _) = cached(&cache, read(&path), &methods);
    assert!(cold["index"]["symbols"].as_array().is_some_and(|symbols| !symbols.is_empty()), "stored");
    let widened = cache.query(read(&path), &contained).expect("widen");
    assert_eq!(widened.lookup, CacheLookup::WidenedCapabilities { parsed: true }, "widened");
    check_impl_facts("widened", &row, &widened.result.index, &widened.result.index, true);
    let (warm, work) = counted(|| cache.query(read(&path), &contained).expect("warm"));
    assert_eq!(warm.lookup, CacheLookup::Hit, "warm");
    assert_eq!(work, WorkCounters::default(), "warm: no work");
    check_impl_facts("warm", &row, &warm.result.index, &warm.result.index, true);
}

/// Restored Swift generic contexts survive parsed containment widening,
/// parsed complete widening, and complete/type-only genuine warm hits.
#[test]
fn swift_nested_generic_extensions_hold_through_widening_and_warm_hits() {
    use crate::symbols_relations::{check_owner_targets, path_meaning_rows};
    let sources = TempDir::new().expect("sources");
    let declarations = SymbolQuery::new();
    let containment = SymbolQuery::new().capabilities([Capability::Containment]);
    let complete_query = SymbolQuery::complete_index();
    let types = SymbolQuery::new().kinds(TYPE_KINDS);
    let labels = ["outer parameter", "outer parameter conformance", "Int"];
    for label in labels {
        let full_label = format!("Swift nested generic extension: {label}");
        let row = path_meaning_rows().find(|row| row.label == full_label).expect("shared Swift regression");
        let path = sources.path().join(format!("{}.swift", label.replace(' ', "_")));
        std::fs::write(&path, row.source).expect("write source");
        let complete = read(&path).query(&complete_query).expect("complete").index;
        let dir = TempDir::new().expect("cache");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), &declarations);
        let widened = cache.query(read(&path), &containment).expect("containment");
        assert_eq!(widened.lookup, CacheLookup::WidenedCapabilities { parsed: true });
        check_owner_targets(label, &complete, &widened.result.index, row.owners);
        assert_eq!(as_value(&widened.result), uncached(&path, &containment));
        let widened = cache.query(read(&path), &complete_query).expect("complete widening");
        assert_eq!(widened.lookup, CacheLookup::WidenedCapabilities { parsed: true });
        check_owner_targets(label, &complete, &widened.result.index, row.owners);
        for query in [&complete_query, &types] {
            let (warm, work) = counted(|| cache.query(read(&path), query).expect("warm"));
            assert_eq!(warm.lookup, CacheLookup::Hit);
            assert_eq!(work, WorkCounters::default());
            check_owner_targets(label, &complete, &warm.result.index, row.owners);
            assert_eq!(as_value(&warm.result), uncached(&path, query));
        }
    }
}

/// One edit to a written entry and the lookup it must get: `None` = `Hit`,
/// `Some(what)` = `Miss(Inconsistent(what))`.
struct EvidenceEdit {
    label: &'static str,
    edit: fn(&mut Value),
    refused: Option<&'static str>,
}

fn first_occurrence_of<'a>(entry: &'a mut Value, name: &str) -> &'a mut Value {
    entry["occurrences"]
        .as_array_mut()
        .expect("occurrences")
        .iter_mut()
        .find(|occurrence| occurrence["target"]["name"] == name)
        .unwrap_or_else(|| panic!("no occurrence of `{name}`"))
}

fn entry_record_id(entry: &Value, name: &str) -> Value {
    entry["records"]
        .as_array()
        .expect("records")
        .iter()
        .find(|record| record["identity"]["name"] == name)
        .unwrap_or_else(|| panic!("no `{name}` record"))["id"]
        .clone()
}

const NAME_MISMATCH: &str = "an occurrence whose target ID and candidates contradict its confidence";
const REGION_MISMATCH: &str = "a callable whose coverage region does not match its body";
const OWNER_MISMATCH: &str = "a region owner that does not name its record";

/// A well-formed ID no declaration has.
fn phantom_id() -> Value {
    json!("0".repeat(64))
}

/// Adds, right after the region owned by `name`, a copy of it owned by a
/// declaration no record is: order and spans stay valid.
fn add_phantom_region_after(entry: &mut Value, name: &str) {
    let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
    let at = regions.iter().position(|region| region["owner"]["name"] == name).expect("owned region");
    let mut phantom = regions[at].clone();
    phantom["owner"] = json!({ "id": phantom_id(), "name": "phantom", "qualified_name": "phantom" });
    regions.insert(at + 1, phantom);
}

fn remove_region_of(entry: &mut Value, name: &str) {
    let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
    let before = regions.len();
    regions.retain(|region| region["owner"]["name"] != name);
    assert_eq!(regions.len(), before - 1, "one region owned by `{name}`");
}
const MALFORMED_ID: &str = "an ID that is not a generated declaration ID";

/// Writes `source` with `query`, then serves each edit of the written entry
/// (restored before every edit) and checks the lookup it gets; a refused or
/// unedited entry's result equals an uncached run.
fn assert_evidence_edits(file: &str, source: &str, query: &SymbolQuery, edits: &[EvidenceEdit]) {
    let sources = TempDir::new().expect("sources");
    let path = sources.path().join(file);
    std::fs::write(&path, source).expect("write source");
    let expected = uncached(&path, query);
    let dir = TempDir::new().expect("cache");
    let cache = cache_in(&dir);
    assert_eq!(cached(&cache, read(&path), query).0, CacheLookup::Miss(CacheMiss::Absent), "{file}: cold");
    let entry = only_entry(&dir);
    let written = read_entry(&entry);
    for row in edits {
        let label = format!("{file}: {}", row.label);
        let mut edited = written.clone();
        (row.edit)(&mut edited);
        write_entry(&entry, &edited);
        let (lookup, served, _) = cached(&cache, read(&path), query);
        let wanted = row.refused.map_or(CacheLookup::Hit, |what| CacheLookup::Miss(CacheMiss::Inconsistent(what)));
        assert_eq!(lookup, wanted, "{label}");
        // A served edit is served as written.
        if row.refused.is_some() || edited == written {
            assert_eq!(served, expected, "{label}: result");
        }
    }
}

/// The reader validates what an entry can prove and nothing more. A
/// kind-narrowed entry keeps evidence naming declarations it omits, so an ID
/// naming no retained record passes only when the entry omits a kind that
/// record could have (an owner is a callable), and every ID is checked for
/// the generated form; a retained callable has its region exactly when it
/// has a body. A language without binding rules resolves by name over every
/// record, so its name-only evidence holds every retained record of the
/// name, and in an every-kind entry nothing else.
#[test]
fn stored_evidence_is_checked_against_what_the_entry_retains() {
    const NARROW_TS: &str = "class C {}\nfunction f() { return new C(); }\nfunction g() {}\nf();\nnew C();\n";
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    assert_evidence_edits(
        "narrow.ts",
        NARROW_TS,
        &narrow,
        &[
            EvidenceEdit {
                label: "control: a target ID of an omitted class",
                edit: |_| {},
                refused: None,
            },
            EvidenceEdit {
                label: "an ownerless target naming a retained declaration of another name",
                edit: |entry| {
                    let id = entry_record_id(entry, "g");
                    first_occurrence_of(entry, "f")["target"]["id"] = id;
                },
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "an ownerless target with a malformed ID",
                edit: |entry| first_occurrence_of(entry, "f")["target"]["id"] = json!("bogus"),
                refused: Some(MALFORMED_ID),
            },
            EvidenceEdit {
                label: "an ownerless target that lost its ID",
                edit: |entry| first_occurrence_of(entry, "f")["target"]["id"] = Value::Null,
                refused: Some("a scoped occurrence with no target ID and no binding of its name"),
            },
            EvidenceEdit {
                label: "an omitted class's target with a malformed ID",
                edit: |entry| first_occurrence_of(entry, "C")["target"]["id"] = json!("bogus"),
                refused: Some(MALFORMED_ID),
            },
            EvidenceEdit {
                label: "a retained callable's region removed",
                edit: |entry| {
                    entry["reference_coverage"]
                        .as_array_mut()
                        .expect("coverage")
                        .retain(|region| region["owner"]["name"] != "g");
                },
                refused: Some(REGION_MISMATCH),
            },
            EvidenceEdit {
                label: "control: an ownerless target naming no record, of a kind the entry omits",
                edit: |entry| {
                    let occurrence = entry["occurrences"]
                        .as_array_mut()
                        .expect("occurrences")
                        .iter_mut()
                        .rfind(|occurrence| occurrence["target"]["name"] == "C")
                        .expect("ownerless `C`");
                    assert!(occurrence["owner"].is_null(), "the last `C` is ownerless");
                    occurrence["target"]["id"] = phantom_id();
                },
                refused: None,
            },
            EvidenceEdit {
                label: "a region owner naming no record although no callable kind is omitted",
                edit: |entry| add_phantom_region_after(entry, "g"),
                refused: Some(OWNER_MISMATCH),
            },
        ],
    );

    // Omitting a callable kind leaves a region owner of that kind unproved.
    const METHODS_TS: &str = "class K { m() { return 1; } }\nfunction f() {}\n";
    assert_evidence_edits(
        "functions-only.ts",
        METHODS_TS,
        &SymbolQuery::new().kinds([SymbolKindV2::Function]).capabilities([Capability::Occurrences]),
        &[
            EvidenceEdit { label: "control: a region owned by an omitted method", edit: |_| {}, refused: None },
            EvidenceEdit {
                label: "control: a region owner naming no record, of a kind the entry omits",
                edit: |entry| add_phantom_region_after(entry, "f"),
                refused: None,
            },
        ],
    );

    // An every-kind entry omits nothing: every ID names a retained record.
    const COMPLETE_TS: &str = "class K { m() { return 1; } }\n\
        function h(o: K) { return o.m() + g(); }\n\
        function g() { return 2; }\n\
        function e() {}\n\
        g();\n";
    assert_evidence_edits(
        "complete.ts",
        COMPLETE_TS,
        &SymbolQuery::complete_index(),
        &[
            EvidenceEdit { label: "control", edit: |_| {}, refused: None },
            EvidenceEdit {
                label: "an ownerless target naming no record",
                edit: |entry| {
                    let occurrence = entry["occurrences"]
                        .as_array_mut()
                        .expect("occurrences")
                        .iter_mut()
                        .find(|occurrence| occurrence["target"]["name"] == "g" && occurrence["owner"].is_null())
                        .expect("ownerless `g`");
                    occurrence["target"]["id"] = phantom_id();
                },
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "a candidate naming no record",
                edit: |entry| first_occurrence_of(entry, "m")["candidates"][0]["id"] = phantom_id(),
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "a region owner naming no record",
                edit: |entry| add_phantom_region_after(entry, "e"),
                refused: Some(OWNER_MISMATCH),
            },
            EvidenceEdit {
                label: "a reference summary naming no record",
                edit: |entry| {
                    let record = entry["records"]
                        .as_array_mut()
                        .expect("records")
                        .iter_mut()
                        .find(|record| record["identity"]["name"] == "g")
                        .expect("`g`");
                    record["relations"]["referenced_by"][0]["id"] = phantom_id();
                },
                refused: Some("a reference summary that does not name its record"),
            },
            EvidenceEdit {
                label: "an empty callable's region removed",
                edit: |entry| remove_region_of(entry, "e"),
                refused: Some(REGION_MISMATCH),
            },
        ],
    );

    // A region belongs to a callable with a body, and only to one.
    assert_evidence_edits(
        "bodiless.rs",
        "trait T {\n    fn r();\n    fn p() {}\n}\n",
        &SymbolQuery::complete_index(),
        &[
            EvidenceEdit { label: "control", edit: |_| {}, refused: None },
            EvidenceEdit {
                label: "a region owned by a callable with no body",
                edit: |entry| {
                    let id = entry_record_id(entry, "r");
                    let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
                    let at = regions.iter().position(|region| region["owner"]["name"] == "p").expect("`p`");
                    let mut region = regions[at].clone();
                    region["owner"] = json!({ "id": id, "name": "r", "qualified_name": "T::r" });
                    regions.insert(at + 1, region);
                },
                refused: Some(REGION_MISMATCH),
            },
        ],
    );

    // Python and Lua read a body as a run of children, and a Lua body can
    // have no `block` at all; a callable with a body still has its region.
    for (file, source) in [
        ("empty.py", "def e():\n    pass\n\ndef f():\n    e()\n"),
        ("empty.lua", "function e() end\n\nfunction f()\n  e()\nend\n"),
        ("comment.lua", "function e()\n  -- nothing yet\nend\n\nfunction f()\n  e()\nend\n"),
    ] {
        assert_evidence_edits(
            file,
            source,
            &SymbolQuery::complete_index(),
            &[
                EvidenceEdit { label: "control", edit: |_| {}, refused: None },
                EvidenceEdit {
                    label: "an empty callable's region removed",
                    edit: |entry| remove_region_of(entry, "e"),
                    refused: Some(REGION_MISMATCH),
                },
                EvidenceEdit {
                    label: "a region owner naming no record",
                    edit: |entry| add_phantom_region_after(entry, "e"),
                    refused: Some(OWNER_MISMATCH),
                },
            ],
        );
    }

    const PYTHON: &str = "def f():\n    g()\n\ndef g():\n    pass\n\ndef g():\n    pass\n\nclass K:\n    pass\n\nK()\n";
    assert_evidence_edits(
        "complete.py",
        PYTHON,
        &SymbolQuery::complete_index(),
        &[
            EvidenceEdit { label: "control", edit: |_| {}, refused: None },
            EvidenceEdit {
                label: "a name-only candidate dropped",
                edit: |entry| {
                    first_occurrence_of(entry, "g")["candidates"].as_array_mut().expect("candidates").pop();
                },
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "unresolved beside a declaration of its name",
                edit: |entry| {
                    let occurrence = first_occurrence_of(entry, "K");
                    occurrence["confidence"] = json!("Unresolved");
                    occurrence["candidates"] = json!([]);
                },
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "scoped in a language without binding rules",
                edit: |entry| {
                    let id = entry_record_id(entry, "K");
                    let occurrence = first_occurrence_of(entry, "K");
                    occurrence["confidence"] = json!("Scoped");
                    occurrence["candidates"] = json!([]);
                    occurrence["target"]["id"] = id;
                },
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "a region without the unsupported-language limit",
                edit: |entry| {
                    let region = &mut entry["reference_coverage"][1];
                    region["state"] = json!("Complete");
                    region["reasons"] = json!([]);
                },
                refused: Some("an unsupported-language limit that does not match the language"),
            },
        ],
    );

    // Narrowed, the same name-only evidence names a class the entry omits,
    // while every retained record of the name stays a candidate.
    assert_evidence_edits(
        "narrow.py",
        PYTHON,
        &narrow,
        &[
            EvidenceEdit { label: "control: a candidate of an omitted class", edit: |_| {}, refused: None },
            EvidenceEdit {
                label: "a retained candidate dropped",
                edit: |entry| {
                    first_occurrence_of(entry, "g")["candidates"].as_array_mut().expect("candidates").pop();
                },
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "unresolved beside a retained declaration of its name",
                edit: |entry| {
                    let occurrence = first_occurrence_of(entry, "g");
                    occurrence["confidence"] = json!("Unresolved");
                    occurrence["candidates"] = json!([]);
                },
                refused: Some(NAME_MISMATCH),
            },
        ],
    );
}

const PATH_MISMATCH: &str = "an occurrence whose written path is not the source text ending at its name";
const PATH_START: &str = "an occurrence whose written path disagrees with its stored start";
const CANDIDATE_ORDER: &str = "candidates out of declaration order";
const REPEATED_EDGE: &str = "a reference summary that repeats an edge";
const EDGE_ORDER: &str = "a reference summary out of first-occurrence order";

/// `u` calls the import `a` after a loop binding of the same name: its first
/// no-ID `a` precedes `helper`, while the import edge follows it.
const GEOMETRY_RS: &str = "struct S;\n\
    impl S {\n    fn h() {}\n    fn f() { Self::h(); }\n}\n\
    struct A;\nimpl A { fn m(&self) {} }\n\
    struct B;\nimpl B { fn m(&self) {} }\n\
    use other::a;\nuse other::b;\n\
    fn helper() {}\n\
    fn f2() {}\n\
    fn g(x: A) { helper(); a(); b(); f2(); helper(); x.m(); }\n\
    fn j() { f2(); }\n\
    fn u() { for a in [1] { a; } helper(); a(); }\n\
    fn empty() {}\n";

fn entry_record<'a>(entry: &'a mut Value, name: &str) -> &'a mut Value {
    entry["records"]
        .as_array_mut()
        .expect("records")
        .iter_mut()
        .find(|record| record["identity"]["name"] == name)
        .unwrap_or_else(|| panic!("no `{name}` record"))
}

fn relation<'a>(entry: &'a mut Value, name: &str, list: &str) -> &'a mut Vec<Value> {
    entry_record(entry, name)["relations"][list].as_array_mut().expect("summary list")
}

fn duplicate_first(list: &mut Vec<Value>) {
    list.insert(1, list[0].clone());
}

/// Source-derived facts the reader can check without a parse: a retained
/// callable's region holds its retained body, a written path is the source
/// text ending at its name and agrees with its stored start (`null` together,
/// so a path or start deleted alone is refused), candidates are in
/// declaration order, and each summary lists every edge once in
/// first-occurrence order. Narrowed controls: an omitted record's position is
/// unknown, so only the retained candidates are compared, while an omitted
/// member's written path is checked like any other; summaries are still
/// ordered by their occurrences. Deleting a path and its start together is
/// the documented coordinated-forgery limit.
#[test]
fn stored_evidence_geometry_paths_and_order_are_checked() {
    assert_evidence_edits(
        "geometry.rs",
        GEOMETRY_RS,
        &SymbolQuery::complete_index(),
        &[
            EvidenceEdit { label: "control (unqualified paths are null)", edit: |_| {}, refused: None },
            EvidenceEdit {
                label: "a callable's region shrunk to its name, outside its body",
                edit: |entry| {
                    let name_span = entry_record(entry, "empty")["source"]["name_span"].clone();
                    let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
                    let region = regions.iter_mut().find(|region| region["owner"]["name"] == "empty").expect("`empty`");
                    region["span"] = name_span;
                },
                refused: Some(REGION_MISMATCH),
            },
            EvidenceEdit {
                label: "a written path the source does not write",
                edit: |entry| first_occurrence_of(entry, "h")["target"]["qualified_name"] = json!("Other::h"),
                refused: Some(PATH_MISMATCH),
            },
            EvidenceEdit {
                label: "an empty written path on a qualified name",
                edit: |entry| first_occurrence_of(entry, "h")["target"]["qualified_name"] = json!(""),
                refused: Some(PATH_MISMATCH),
            },
            EvidenceEdit {
                label: "an empty written path on an unqualified name",
                edit: |entry| first_occurrence_of(entry, "helper")["target"]["qualified_name"] = json!(""),
                refused: Some(PATH_MISMATCH),
            },
            EvidenceEdit {
                label: "a written path that is only the name",
                edit: |entry| first_occurrence_of(entry, "helper")["target"]["qualified_name"] = json!("helper"),
                refused: Some(PATH_MISMATCH),
            },
            EvidenceEdit {
                label: "a scoped member that lost its qualifier",
                edit: |entry| first_occurrence_of(entry, "h")["target"]["qualified_name"] = Value::Null,
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "a member call that lost its receiver",
                edit: |entry| first_occurrence_of(entry, "m")["target"]["qualified_name"] = Value::Null,
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "a written path whose stored start is null",
                edit: |entry| first_occurrence_of(entry, "h")["path_start_byte"] = Value::Null,
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "a written path whose stored start is one byte off",
                edit: |entry| {
                    let occurrence = first_occurrence_of(entry, "h");
                    let start = occurrence["path_start_byte"].as_u64().expect("a stored start");
                    occurrence["path_start_byte"] = json!(start + 1);
                },
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "a stored start on an unqualified name",
                edit: |entry| {
                    let occurrence = first_occurrence_of(entry, "helper");
                    assert!(occurrence["path_start_byte"].is_null(), "an unqualified name has no stored start");
                    occurrence["path_start_byte"] = occurrence["span"]["start_byte"].clone();
                },
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "candidates reversed",
                edit: |entry| first_occurrence_of(entry, "m")["candidates"].as_array_mut().expect("candidates").reverse(),
                refused: Some(CANDIDATE_ORDER),
            },
            EvidenceEdit {
                label: "a repeated candidate",
                edit: |entry| duplicate_first(first_occurrence_of(entry, "m")["candidates"].as_array_mut().expect("candidates")),
                refused: Some(NAME_MISMATCH),
            },
            EvidenceEdit {
                label: "a repeated reference",
                edit: |entry| duplicate_first(relation(entry, "g", "references")),
                refused: Some(REPEATED_EDGE),
            },
            EvidenceEdit {
                label: "references reversed",
                edit: |entry| relation(entry, "g", "references").reverse(),
                refused: Some(EDGE_ORDER),
            },
            EvidenceEdit {
                label: "a repeated referenced-by owner",
                edit: |entry| duplicate_first(relation(entry, "f2", "referenced_by")),
                refused: Some(REPEATED_EDGE),
            },
            EvidenceEdit {
                label: "referenced-by owners reversed",
                edit: |entry| relation(entry, "f2", "referenced_by").reverse(),
                refused: Some(EDGE_ORDER),
            },
            EvidenceEdit {
                label: "a repeated dependency",
                edit: |entry| duplicate_first(relation(entry, "g", "dependencies")),
                refused: Some(REPEATED_EDGE),
            },
            EvidenceEdit {
                label: "dependencies reversed",
                edit: |entry| relation(entry, "g", "dependencies").reverse(),
                refused: Some(EDGE_ORDER),
            },
            EvidenceEdit {
                label: "an import edge moved before the binding-named occurrence's resolved edge",
                edit: |entry| relation(entry, "u", "references").reverse(),
                refused: Some(EDGE_ORDER),
            },
            EvidenceEdit {
                label: "occurrences reversed",
                edit: |entry| entry["occurrences"].as_array_mut().expect("occurrences").reverse(),
                refused: Some("occurrences out of order or repeated"),
            },
            EvidenceEdit {
                label: "a repeated occurrence",
                edit: |entry| duplicate_first(entry["occurrences"].as_array_mut().expect("occurrences")),
                refused: Some("occurrences out of order or repeated"),
            },
            EvidenceEdit {
                label: "coverage regions swapped",
                edit: |entry| entry["reference_coverage"].as_array_mut().expect("coverage").swap(1, 2),
                refused: Some("coverage out of order"),
            },
        ],
    );

    // Methods omitted: their positions and scopes are unknown, while the
    // occurrences still order every summary.
    assert_evidence_edits(
        "geometry-narrow.rs",
        GEOMETRY_RS,
        &SymbolQuery::new().kinds([SymbolKindV2::Function]).capabilities([Capability::Occurrences]),
        &[
            EvidenceEdit { label: "control", edit: |_| {}, refused: None },
            EvidenceEdit {
                label: "limit: candidates of omitted methods are not ordered",
                edit: |entry| first_occurrence_of(entry, "m")["candidates"].as_array_mut().expect("candidates").reverse(),
                refused: None,
            },
            EvidenceEdit {
                label: "an omitted member's lost qualifier",
                edit: |entry| first_occurrence_of(entry, "h")["target"]["qualified_name"] = Value::Null,
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "an omitted member's receiver lost",
                edit: |entry| first_occurrence_of(entry, "m")["target"]["qualified_name"] = Value::Null,
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "an omitted member's stored start lost",
                edit: |entry| first_occurrence_of(entry, "m")["path_start_byte"] = Value::Null,
                refused: Some(PATH_START),
            },
            EvidenceEdit {
                label: "limit: a path and its start deleted together",
                edit: |entry| {
                    let occurrence = first_occurrence_of(entry, "h");
                    occurrence["target"]["qualified_name"] = Value::Null;
                    occurrence["path_start_byte"] = Value::Null;
                },
                refused: None,
            },
            EvidenceEdit {
                label: "an omitted method's written path is still source text",
                edit: |entry| first_occurrence_of(entry, "h")["target"]["qualified_name"] = json!("Other::h"),
                refused: Some(PATH_MISMATCH),
            },
            EvidenceEdit {
                label: "references reversed",
                edit: |entry| relation(entry, "g", "references").reverse(),
                refused: Some(EDGE_ORDER),
            },
            EvidenceEdit {
                label: "a repeated dependency",
                edit: |entry| duplicate_first(relation(entry, "g", "dependencies")),
                refused: Some(REPEATED_EDGE),
            },
        ],
    );
}

/// An entry written before occurrences stored `path_start_byte` (extractor
/// revision 42 and earlier, already refused by key) is malformed even under
/// the current key: the key is required, never read as `null`.
#[test]
fn an_occurrence_without_a_stored_path_start_is_malformed() {
    let sources = TempDir::new().expect("sources");
    let path = sources.path().join("lost.rs");
    std::fs::write(&path, LOST_PATH_RS).expect("write source");
    let query = SymbolQuery::complete_index();
    let dir = TempDir::new().expect("cache");
    let cache = cache_in(&dir);
    cached(&cache, read(&path), &query);
    let entry = only_entry(&dir);
    let mut edited = read_entry(&entry);
    for occurrence in edited["occurrences"].as_array_mut().expect("occurrences") {
        occurrence.as_object_mut().expect("occurrence").remove("path_start_byte").expect("a stored start");
    }
    write_entry(&entry, &edited);
    let (lookup, served, _) = cached(&cache, read(&path), &query);
    let CacheLookup::Miss(CacheMiss::Malformed(reason)) = lookup else {
        panic!("expected a malformed entry, got {lookup:?}");
    };
    assert!(reason.contains("missing field `path_start_byte`"), "{reason}");
    assert_eq!(served, uncached(&path, &query));
}

/// `g` calls the unresolved item path `other::external`; `S::m` is a member
/// the narrowed query omits, reached through its receiver.
const LOST_PATH_RS: &str = "fn g() { other::external(); }\nstruct S;\nimpl S { fn m(&self) {} }\nfn k(s: S) { s.m(); }\n";

/// A written path deleted from a stored entry (`target.qualified_name` set to
/// `null`, its stored start kept) is a miss whose recomputed result writes the
/// path again, whether the entry and query are complete, narrowed to
/// functions (the member's record omitted), or a complete entry projected to
/// the narrowed query; the rewritten entry is then a hit with the path.
#[test]
fn a_deleted_written_path_is_recomputed_in_complete_narrowed_and_projected_results() {
    let sources = TempDir::new().expect("sources");
    let path = sources.path().join("lost.rs");
    std::fs::write(&path, LOST_PATH_RS).expect("write source");
    let complete = SymbolQuery::complete_index();
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let paths = |result: &Value| -> Vec<Value> {
        result["index"]["occurrences"]
            .as_array()
            .expect("occurrences")
            .iter()
            .filter(|occurrence| ["external", "m"].contains(&occurrence["target"]["name"].as_str().unwrap_or("")))
            .map(|occurrence| occurrence["target"]["qualified_name"].clone())
            .collect()
    };
    let drop_paths = |entry: &mut Value| {
        for name in ["external", "m"] {
            first_occurrence_of(entry, name)["target"]["qualified_name"] = Value::Null;
        }
    };
    // (step, query that writes the entry, query that reads the edited entry)
    for (step, writer, reader) in [("complete", &complete, &complete), ("narrowed", &narrow, &narrow), ("projected", &complete, &narrow)] {
        let dir = TempDir::new().expect("cache");
        let cache = cache_in(&dir);
        cached(&cache, read(&path), writer);
        let entry = only_entry(&dir);
        let mut edited = read_entry(&entry);
        drop_paths(&mut edited);
        write_entry(&entry, &edited);
        let expected = uncached(&path, reader);
        assert_eq!(paths(&expected), [json!("other::external"), json!("s.m")], "{step}: uncached");
        let (lookup, served, _) = cached(&cache, read(&path), reader);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Inconsistent(PATH_START)), "{step}");
        assert_eq!(served, expected, "{step}: recomputed");
        let (lookup, served, work) = cached(&cache, read(&path), reader);
        assert_eq!(lookup, CacheLookup::Hit, "{step}: rewritten");
        assert_eq!(work, WorkCounters::default(), "{step}: rewritten");
        assert_eq!(served, expected, "{step}: rewritten");
    }
}

/// A call of a Rust value (`Self::C()` with `const C: fn()`) stored as
/// occurrences keeps its `indirect_dispatch` limit when semantics are
/// widened from the stored evidence without a parse, and on a warm hit:
/// `f` stays `Unknown`, beside a proven self-call (`Yes`) and a call of
/// another member function (`No`).
#[test]
fn stored_value_calls_widen_to_unknown_recursion() {
    use tree_hugger::{Confidence, CoverageState, LimitReason, OccurrenceKind, TriState};

    const SOURCE: &str = "struct S;\nimpl S {\n    const C: fn() = Self::f;\n    fn f() { Self::C(); }\n    fn h() {}\n    fn g() { Self::h(); }\n    fn r() { Self::r(); }\n}\n";
    let sources = TempDir::new().expect("sources");
    let path = sources.path().join("values.rs");
    std::fs::write(&path, SOURCE).expect("write source");
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);

    let check = |result: &SymbolQueryResult, label: &str| {
        let calls: Vec<(u32, &str, Confidence)> = result
            .index
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.kind == OccurrenceKind::Call)
            .map(|occurrence| (occurrence.span.start.line, occurrence.target.name.as_str(), occurrence.confidence))
            .collect();
        assert_eq!(
            calls,
            [(4, "C", Confidence::Scoped), (6, "h", Confidence::Scoped), (7, "r", Confidence::Scoped)],
            "{label}: calls"
        );
        let region = result
            .index
            .reference_coverage
            .iter()
            .find(|coverage| coverage.owner.as_ref().is_some_and(|owner| owner.name == "f"))
            .expect("`f` region");
        assert_eq!(
            (region.state, region.reasons.as_slice()),
            (CoverageState::Limited, &[LimitReason::IndirectDispatch][..]),
            "{label}: `f` coverage"
        );
    };
    let recursion = |result: &SymbolQueryResult, name: &str| {
        result
            .index
            .symbols
            .iter()
            .find(|record| record.identity.name == name)
            .and_then(|record| record.semantics.is_recursive)
    };
    let verdicts = |result: &SymbolQueryResult, label: &str| {
        assert_eq!(recursion(result, "f"), Some(TriState::Unknown), "{label}: `f`");
        assert_eq!(recursion(result, "g"), Some(TriState::No), "{label}: `g`");
        assert_eq!(recursion(result, "r"), Some(TriState::Yes), "{label}: `r`");
    };

    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let outcome = cache.query(read(&path), &narrow).expect("narrow");
    assert_eq!(outcome.lookup, CacheLookup::Miss(CacheMiss::Absent));
    check(&outcome.result, "narrow");

    let semantics = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Semantics]);
    let outcome = cache.query(read(&path), &semantics).expect("widened");
    assert_eq!(outcome.lookup, CacheLookup::WidenedCapabilities { parsed: false });
    check(&outcome.result, "widened");
    verdicts(&outcome.result, "widened");
    assert_eq!(as_value(&outcome.result), uncached(&path, &semantics));

    let (outcome, work) = counted(|| cache.query(read(&path), &semantics).expect("warm"));
    assert_eq!(outcome.lookup, CacheLookup::Hit);
    assert_eq!(work, WorkCounters::default());
    check(&outcome.result, "warm");
    verdicts(&outcome.result, "warm");
}

/// TypeScript instantiation-expression calls (`(f<number>)()`,
/// `new (C<number>)()`, `(obj.m<number>)()`) survive a narrow entry, its
/// widening and a warm hit as calls with the same recursion verdicts: each
/// step is checked against the expected evidence, not only against an
/// uncached run.
#[test]
fn instantiation_expression_calls_survive_narrow_widened_and_warm_entries() {
    use tree_hugger::{Confidence, OccurrenceKind, TriState};

    const SOURCE: &str = "function f<T>() { (f<number>)(); }\nclass W { m<T>() {} }\nclass C<T> {}\nfunction g(obj: W) {\n  ((f<number>))();\n  new (C<number>)();\n  (obj.m<number>)();\n  const x = f<number>;\n}\nfunction h() { (f<number>)(); new (C<number>)(); }\n";
    let sources = TempDir::new().expect("sources");
    let path = sources.path().join("instantiation.ts");
    std::fs::write(&path, SOURCE).expect("write source");
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);

    let check = |result: &SymbolQueryResult, label: &str| {
        let calls: Vec<(u32, &str, Confidence)> = result
            .index
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.kind == OccurrenceKind::Call)
            .map(|occurrence| (occurrence.span.start.line, occurrence.target.name.as_str(), occurrence.confidence))
            .collect();
        assert_eq!(
            calls,
            [
                (1, "f", Confidence::Scoped),
                (5, "f", Confidence::Scoped), (6, "C", Confidence::Scoped), (7, "m", Confidence::NameOnly),
                (10, "f", Confidence::Scoped),
                (10, "C", Confidence::Scoped),
            ],
            "{label}: calls"
        );
    };
    let recursion = |result: &SymbolQueryResult, name: &str| {
        result
            .index
            .symbols
            .iter()
            .find(|record| record.identity.name == name)
            .and_then(|record| record.semantics.is_recursive)
    };

    let narrow = SymbolQuery::new()
        .kinds([SymbolKindV2::Class])
        .capabilities([Capability::Occurrences]);
    let outcome = cache.query(read(&path), &narrow).expect("narrow");
    assert_eq!(outcome.lookup, CacheLookup::Miss(CacheMiss::Absent));
    check(&outcome.result, "narrow");

    let functions = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Semantics]);
    let outcome = cache.query(read(&path), &functions).expect("widened");
    assert_eq!(outcome.lookup, CacheLookup::WidenedKinds);
    check(&outcome.result, "widened");
    assert_eq!(recursion(&outcome.result, "f"), Some(TriState::Yes), "widened");
    assert_eq!(recursion(&outcome.result, "h"), Some(TriState::No), "widened");
    assert_eq!(recursion(&outcome.result, "g"), Some(TriState::Unknown), "widened: the member call stays indirect");
    assert_eq!(as_value(&outcome.result), uncached(&path, &functions));

    let (outcome, work) = counted(|| cache.query(read(&path), &functions).expect("warm"));
    assert_eq!(outcome.lookup, CacheLookup::Hit);
    assert_eq!(work, WorkCounters::default());
    check(&outcome.result, "warm");
    assert_eq!(recursion(&outcome.result, "f"), Some(TriState::Yes), "warm");
    assert_eq!(recursion(&outcome.result, "h"), Some(TriState::No), "warm");
    assert_eq!(recursion(&outcome.result, "g"), Some(TriState::Unknown), "warm");
}

/// The `eval` call forms that bind nothing (optional, tagged, through
/// another value) keep the nested self-call and the `Base` target
/// established, and `f` `Yes`, through a narrow entry, its widening and a
/// warm hit; a direct `eval` keeps them uncertain at every step. Each step
/// is checked against the expected evidence, not only an uncached run.
#[test]
fn eval_call_modes_survive_narrow_widened_and_warm_entries() {
    use tree_hugger::{Confidence, TriState};

    use crate::symbols_occurrences::{EVAL_CALL_FORMS, EvalMode, eval_source};

    for &(label, extensions, statement, mode) in EVAL_CALL_FORMS {
        for &extension in extensions {
            let label = format!("{extension} {label} eval");
            let sources = TempDir::new().expect("sources");
            let path = sources.path().join(format!("eval.{extension}"));
            std::fs::write(&path, eval_source(statement)).expect("write source");
            let dir = TempDir::new().expect("temp");
            let cache = cache_in(&dir);
            let expected_recursion = if mode == EvalMode::Direct { TriState::Unknown } else { TriState::Yes };

            let check = |result: &SymbolQueryResult, step: &str| {
                let f_id = result
                    .index
                    .occurrences
                    .iter()
                    .find_map(|occurrence| {
                        occurrence
                            .owner
                            .as_ref()
                            .filter(|owner| owner.name == "f")
                            .and_then(|owner| owner.id.clone())
                    })
                    .expect("an occurrence owned by `f`");
                let at = |line: u32, name: &str| {
                    result
                        .index
                        .occurrences
                        .iter()
                        .find(|occurrence| occurrence.span.start.line == line && occurrence.target.name == name)
                        .unwrap_or_else(|| panic!("{label} {step}: no `{name}` on line {line}"))
                };
                let (call, base) = (at(6, "f"), at(5, "Base"));
                if mode == EvalMode::Direct {
                    assert_eq!((call.confidence == Confidence::Scoped, call.target.id.as_ref()), (false, None), "{label} {step}");
                    assert_eq!((base.confidence == Confidence::Scoped, base.target.id.as_ref()), (false, None), "{label} {step}");
                } else {
                    assert_eq!((call.confidence, call.target.id.as_ref()), (Confidence::Scoped, Some(&f_id)), "{label} {step}");
                    assert_eq!(base.confidence, Confidence::Scoped, "{label} {step}");
                    assert!(base.target.id.is_some(), "{label} {step}");
                }
                let region = result
                    .index
                    .reference_coverage
                    .iter()
                    .find(|coverage| coverage.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(&f_id))
                    .expect("`f` region");
                assert_eq!(region.reasons.as_slice(), mode.reasons(), "{label} {step}");
            };
            let recursion = |result: &SymbolQueryResult| {
                result
                    .index
                    .symbols
                    .iter()
                    .find(|record| record.identity.name == "f")
                    .and_then(|record| record.semantics.is_recursive)
            };

            let narrow = SymbolQuery::new().kinds([SymbolKindV2::Class]).capabilities([Capability::Occurrences]);
            let outcome = cache.query(read(&path), &narrow).expect("narrow");
            assert_eq!(outcome.lookup, CacheLookup::Miss(CacheMiss::Absent), "{label}");
            check(&outcome.result, "narrow");

            let functions = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Semantics]);
            let outcome = cache.query(read(&path), &functions).expect("widened");
            assert_eq!(outcome.lookup, CacheLookup::WidenedKinds, "{label}");
            check(&outcome.result, "widened");
            assert_eq!(recursion(&outcome.result), Some(expected_recursion), "{label} widened");
            assert_eq!(as_value(&outcome.result), uncached(&path, &functions), "{label}");

            let (outcome, work) = counted(|| cache.query(read(&path), &functions).expect("warm"));
            assert_eq!(outcome.lookup, CacheLookup::Hit, "{label}");
            assert_eq!(work, WorkCounters::default(), "{label}");
            check(&outcome.result, "warm");
            assert_eq!(recursion(&outcome.result), Some(expected_recursion), "{label} warm");
        }
    }
}

/// One file per path-shape family (`R4-F1`): the rooted Rust self-call and
/// shadowing local, C++ roots/right-nested/generic paths, C# alias and
/// conditional paths, PHP nullsafe and namespace paths, and Perl package
/// and receiver paths, with `(line, name, nth, written path)` known from
/// the source.
/// `(line, name, nth on that line, written path)`.
type ExpectedPath = (u32, &'static str, usize, &'static str);

const WRITTEN_PATH_SOURCES: &[(&str, &str, &[ExpectedPath])] = &[
    (
        "rooted.rs",
        "fn f() {\n    ::f();\n}\nfn g() {\n    let f = || {};\n    ::f();\n    ::m::f();\n}\n",
        &[(2, "f", 0, "::f"), (6, "f", 0, "::f"), (7, "m", 0, "::m"), (7, "f", 0, "::m::f")],
    ),
    (
        "paths.cpp",
        "class W {};\nvoid g() {\n    ::f();\n    A::B::f();\n    r4::f<int>();\n    obj.template m<int>();\n    N::C<int>::member();\n}\n",
        &[
            (3, "f", 0, "::f"),
            (4, "B", 0, "A::B"),
            (4, "f", 0, "A::B::f"),
            (5, "f", 0, "r4::f"),
            (6, "m", 0, "obj.template m"),
            (7, "C", 0, "N::C"),
            (7, "member", 0, "N::C<int>::member"),
        ],
    ),
    (
        "paths.cs",
        "class G { void g(W obj) {\n    obj?.A?.Invoke();\n    global::N.C h;\n    obj.M<int>();\n} }\n",
        &[(2, "A", 0, "obj?.A"), (2, "Invoke", 0, "obj?.A?.Invoke"), (3, "N", 0, "global::N"), (3, "C", 0, "global::N.C"), (4, "M", 0, "obj.M")],
    ),
    (
        "paths.php",
        "<?php\nclass W {}\nfunction g($o) {\n    $o?->m();\n    \\A\\B\\f();\n}\n",
        &[(4, "m", 0, "$o?->m"), (5, "B", 0, "\\A\\B"), (5, "f", 0, "\\A\\B\\f")],
    ),
    (
        "paths.pl",
        "package W;\nsub g {\n    R4::r4f();\n    R4->r4f();\n    $obj->r4f();\n}\n",
        &[(3, "r4f", 0, "R4::r4f"), (4, "r4f", 0, "R4->r4f"), (5, "r4f", 0, "$obj->r4f")],
    ),
];

/// Written paths survive a narrow (types-only) entry, its widening to the
/// callables, a warm `Hit` and a name-filtered warm `Hit`, each checked
/// against the paths the source writes; the rooted Rust self-call stays
/// `Unknown` (never `No`) at every step and the shadowing local is never
/// its target.
#[test]
fn written_paths_survive_narrow_widened_and_warm_entries() {
    use tree_hugger::{Confidence, TriState};

    let sources = TempDir::new().expect("sources");
    for &(name, source, expected) in WRITTEN_PATH_SOURCES {
        let path = sources.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let dir = TempDir::new().expect("temp");
        let cache = cache_in(&dir);

        let check = |result: &SymbolQueryResult, step: &str| {
            for &(line, written, nth, expected_path) in expected {
                let occurrence = result
                    .index
                    .occurrences
                    .iter()
                    .filter(|occurrence| occurrence.span.start.line == line && occurrence.target.name == written)
                    .nth(nth)
                    .unwrap_or_else(|| panic!("{name} {step}: no `{written}` on line {line}"));
                assert_eq!(
                    occurrence.target.qualified_name.as_deref(),
                    Some(expected_path),
                    "{name} {step}: `{written}` on line {line}"
                );
            }
            if name.ends_with(".rs") {
                let local = result
                    .index
                    .symbols
                    .iter()
                    .find(|record| record.identity.name == "f" && record.kind == SymbolKindV2::Variable)
                    .map(|record| record.id.clone());
                for occurrence in result.index.occurrences.iter().filter(|occurrence| occurrence.target.qualified_name.is_some()) {
                    assert_ne!(occurrence.confidence, Confidence::Scoped, "{name} {step}: {occurrence:#?}");
                    assert!(local.is_none() || occurrence.candidates.iter().all(|candidate| candidate.id != local));
                }
            }
        };
        let recursion = |result: &SymbolQueryResult, step: &str| {
            if !name.ends_with(".rs") {
                return;
            }
            let verdict = |callable: &str| {
                result
                    .index
                    .symbols
                    .iter()
                    .find(|record| record.identity.name == callable && record.kind == SymbolKindV2::Function)
                    .and_then(|record| record.semantics.is_recursive)
            };
            assert_eq!(verdict("f"), Some(TriState::Unknown), "{name} {step}: the rooted self-call");
            assert_eq!(verdict("g"), Some(TriState::No), "{name} {step}: `::f` is not `g`");
        };

        let narrow = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
        let outcome = cache.query(read(&path), &narrow).expect("narrow");
        assert_eq!(outcome.lookup, CacheLookup::Miss(CacheMiss::Absent), "{name}");
        check(&outcome.result, "narrow");

        let functions = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Semantics]);
        let outcome = cache.query(read(&path), &functions).expect("widened");
        assert_eq!(outcome.lookup, CacheLookup::WidenedKinds, "{name}");
        check(&outcome.result, "widened");
        recursion(&outcome.result, "widened");
        assert_eq!(as_value(&outcome.result), uncached(&path, &functions), "{name}");

        let (outcome, work) = counted(|| cache.query(read(&path), &functions).expect("warm"));
        assert_eq!(outcome.lookup, CacheLookup::Hit, "{name}");
        assert_eq!(work, WorkCounters::default(), "{name}");
        check(&outcome.result, "warm");
        recursion(&outcome.result, "warm");

        let named = functions.clone().name(NamePattern::new("g"));
        let outcome = cache.query(read(&path), &named).expect("named warm");
        assert_eq!(outcome.lookup, CacheLookup::Hit, "{name}");
        check(&outcome.result, "named warm");
    }
}

type KindRow = (&'static str, &'static str, &'static [(u32, &'static str, OccurrenceKind)]);

/// `(file name, source, [(line, name, kind)])`: invocation repairs of
/// review 4 (R4-F2) whose kind the cache must store and serve as computed.
const INVOCATION_KIND_SOURCES: &[KindRow] = &[
    (
        "rooted.rs",
        "fn f() {\n    ::f();\n    ((::f))();\n}\n",
        &[(2, "f", OccurrenceKind::Call), (3, "f", OccurrenceKind::Call)],
    ),
    (
        "rooted.cpp",
        "void f() {\n    ::f();\n    ((::f))();\n}\n",
        &[(2, "f", OccurrenceKind::Call), (3, "f", OccurrenceKind::Call)],
    ),
    (
        "conditional.cs",
        "class W {\n    void F(W obj) {\n        obj?.F();\n        obj?.A?.Invoke();\n    }\n}\n",
        &[
            (3, "obj", OccurrenceKind::Usage),
            (3, "F", OccurrenceKind::Call),
            (4, "A", OccurrenceKind::Usage),
            (4, "Invoke", OccurrenceKind::Call),
        ],
    ),
    (
        "callables.php",
        "<?php\nfunction f() {}\nfunction g($cb, $o) {\n    f(...);\n    $o->m(...);\n    $cb();\n    ($cb)();\n    f(...$cb);\n}\n",
        &[
            (4, "f", OccurrenceKind::Usage),
            (5, "m", OccurrenceKind::Usage),
            (6, "cb", OccurrenceKind::Call),
            (7, "cb", OccurrenceKind::Call),
            (8, "f", OccurrenceKind::Call),
        ],
    ),
    (
        "coderef.pl",
        "sub g {\n    $cb->();\n    &$cb();\n    $cb->m();\n}\n",
        &[
            (2, "$cb", OccurrenceKind::Call),
            (3, "$cb", OccurrenceKind::Call),
            (4, "$cb", OccurrenceKind::Usage),
            (4, "m", OccurrenceKind::Call),
        ],
    ),
];

/// Asserts each expected kind in a served result's occurrences.
fn assert_invocation_kinds(served: &Value, expected: &[(u32, &str, OccurrenceKind)], cell: &str) {
    let occurrences: Vec<Occurrence> =
        serde_json::from_value(served["index"]["occurrences"].clone()).expect("occurrences");
    for &(line, name, kind) in expected {
        let found = occurrences
            .iter()
            .find(|occurrence| occurrence.span.start.line == line && occurrence.target.name == name)
            .unwrap_or_else(|| panic!("{cell}: no `{name}` on line {line}: {occurrences:#?}"));
        assert_eq!(found.kind, kind, "{cell}: `{name}` on line {line}");
    }
}

/// R4-F2.C4: rooted, conditional, PHP variable/callable-creation and Perl
/// code-reference kinds are served as computed by a narrow entry, its warm
/// `Hit`, the entry widened by kinds, the widened entry's warm `Hit` and a
/// narrower query projected from it; each equals an uncached run.
#[test]
fn invocation_kinds_hold_through_narrow_widened_projected_and_warm_results() {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(name, source, expected) in INVOCATION_KIND_SOURCES {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup); 5] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("widened", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        for (step, query, expected_lookup) in steps {
            let cell = format!("{name}: {step}");
            let (lookup, served, _) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");
            assert_invocation_kinds(&served, expected, &cell);
        }
    }
}

/// R5-F1 sources: constructed generic types, braced construction and
/// implicit members, with their type arguments, qualifiers, receivers and
/// arguments as usages.
const CONSTRUCTED_KIND_SOURCES: &[KindRow] = &[
    ("commented.swift", "class C<T> {}\nfunc run() {\n _ = C /* n */ <Int>()\n _ = C/*n*/<Int>()\n _ = N.C /* n */ <Int>()\n _ = C<Int /* n */>()\n}\nfunc annotation(x: C /* n */ <Int>) {}\n", &[(3, "C", OccurrenceKind::Call), (3, "Int", OccurrenceKind::Usage), (4, "C", OccurrenceKind::Call), (4, "Int", OccurrenceKind::Usage), (5, "C", OccurrenceKind::Call), (5, "N", OccurrenceKind::Usage), (5, "Int", OccurrenceKind::Usage), (6, "C", OccurrenceKind::Call), (6, "Int", OccurrenceKind::Usage), (8, "C", OccurrenceKind::Usage), (8, "Int", OccurrenceKind::Usage)]),
    (
        "construct.java",
        "class C<T> { C() {} }\nclass G {\n    void g(Object y) {\n        new C<String>();\n        new a.b.C<>();\n        outer.new C<String>(y);\n    }\n}\n",
        &[
            (4, "C", OccurrenceKind::Call),
            (4, "String", OccurrenceKind::Usage),
            (5, "a", OccurrenceKind::Usage),
            (5, "C", OccurrenceKind::Call),
            (6, "outer", OccurrenceKind::Usage),
            (6, "C", OccurrenceKind::Call),
            (6, "y", OccurrenceKind::Usage),
        ],
    ),
    (
        "construct.scala",
        "class C[T]\nobject G {\n  def g(y: Int): Unit = {\n    new C[Int](y)\n    new a.b.C[Int] {}\n    new C with T\n  }\n}\n",
        &[
            (4, "C", OccurrenceKind::Call),
            (4, "Int", OccurrenceKind::Usage),
            (4, "y", OccurrenceKind::Usage),
            (5, "a", OccurrenceKind::Usage),
            (5, "C", OccurrenceKind::Call),
            (6, "C", OccurrenceKind::Call),
            (6, "T", OccurrenceKind::Usage),
        ],
    ),
    (
        "construct.swift",
        "func g(x: Int) {\n    _ = N.C<Int>()\n    let _: E = .some(x)\n    let _: E = .none\n    _ = !f()\n}\n",
        &[
            (2, "N", OccurrenceKind::Usage),
            (2, "C", OccurrenceKind::Call),
            (2, "Int", OccurrenceKind::Usage),
            (3, "some", OccurrenceKind::Call),
            (3, "x", OccurrenceKind::Usage),
            (4, "none", OccurrenceKind::Usage),
            (5, "f", OccurrenceKind::Call),
        ],
    ),
    (
        "construct.cpp",
        "struct Plain { Plain(int) {} };\nvoid g(int x) {\n    new N::C<int>();\n    Plain{x};\n    new Plain[2];\n}\n",
        &[
            (3, "N", OccurrenceKind::Usage),
            (3, "C", OccurrenceKind::Call),
            (4, "Plain", OccurrenceKind::Call),
            (4, "x", OccurrenceKind::Usage),
            (5, "Plain", OccurrenceKind::Usage),
        ],
    ),
];

/// R5-F1.C4: construction and implicit-member kinds are served as computed
/// by a functions-only entry, its warm `Hit`, a record-only semantic
/// widening, a kind widening, the complete warm `Hit` and a projected
/// type-only result; each equals an uncached run.
#[test]
fn constructed_and_implicit_member_kinds_hold_through_the_cache() {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(name, source, expected) in CONSTRUCTED_KIND_SOURCES {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup); 6] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }),
            ("kind widening", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        for (step, query, expected_lookup) in steps {
            let cell = format!("{name}: {step}");
            let (lookup, served, _) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");
            assert_invocation_kinds(&served, expected, &cell);
            if name == "commented.swift" {
                let index: tree_hugger::FileSymbolIndex = serde_json::from_value(served["index"].clone()).expect("index");
                for occurrence in index.occurrences.iter().filter(|occ| (3..=6).contains(&occ.span.start.line) && occ.target.name == "C") {
                    assert_eq!(occurrence.confidence, tree_hugger::Confidence::NameOnly, "{cell}");
                    assert_eq!(occurrence.target.id, None, "{cell}");
                    assert_eq!(occurrence.candidates.len(), 1, "{cell}");
                    assert_eq!(occurrence.candidates[0].name, "C", "{cell}");
                    assert_eq!(occurrence.owner.as_ref().map(|owner| owner.name.as_str()), Some("run"), "{cell}");
                }
                for record in index.symbols.iter().filter(|record| record.identity.name == "run") {
                    assert!(record.relations.references.is_empty(), "{cell}");
                    if matches!(step, "semantic widening" | "kind widening" | "complete warm") {
                        assert_eq!(record.semantics.is_recursive, Some(tree_hugger::TriState::Unknown), "{cell}");
                    }
                }
            }

        }
    }
}

/// `(file name, source, [(line, name, written path, kind)])`.
type WrittenPathCacheRow = (&'static str, &'static str, &'static [(u32, &'static str, Option<&'static str>, OccurrenceKind)]);

/// R5-F3.C4 sources: Java receivers before `new` (simple, parenthesized,
/// `this`, evaluated, chained, generic, anonymous body) and Swift prefix
/// operators kept out of the member chain (R5-F3.S1).
const WRITTEN_PATH_CACHE_SOURCES: &[WrittenPathCacheRow] = &[
    (
        "receivers.java",
        "class G {\n    class C {}\n    void g(G outer) {\n        outer.new C();\n        (outer).new C();\n        this.new C();\n        factory().new C();\n        holder.outer.new C();\n        outer.new C<String>();\n        outer.new C() {};\n    }\n    G factory() { return this; }\n}\n",
        &[
            (4, "C", Some("outer.new C"), OccurrenceKind::Call),
            (5, "C", Some("(outer).new C"), OccurrenceKind::Call),
            (6, "C", Some("this.new C"), OccurrenceKind::Call),
            (7, "factory", None, OccurrenceKind::Call),
            (7, "C", Some("factory().new C"), OccurrenceKind::Call),
            (8, "outer", Some("holder.outer"), OccurrenceKind::Usage),
            (8, "C", Some("holder.outer.new C"), OccurrenceKind::Call),
            (9, "C", Some("outer.new C"), OccurrenceKind::Call),
            (9, "String", None, OccurrenceKind::Usage),
            (10, "C", Some("outer.new C"), OccurrenceKind::Call),
        ],
    ),
    (
        "operators.swift",
        "func isEmpty() -> Bool { true }\nfunc g(x: [Int]) {\n    _ = !x.isEmpty()\n    _ = -x.y.f()\n    _ = -.some.f()\n}\n",
        &[
            (3, "isEmpty", Some("x.isEmpty"), OccurrenceKind::Call),
            (4, "y", Some("x.y"), OccurrenceKind::Usage),
            (4, "f", Some("x.y.f"), OccurrenceKind::Call),
            (5, "some", None, OccurrenceKind::Usage),
            (5, "f", Some(".some.f"), OccurrenceKind::Call),
        ],
    ),
    // R5-F1.S17: a Swift subscript's operand is never a call target.
    (
        "subscripts.swift",
        "func f() -> [Int] { [] }\nfunc g(a: [Int]) {\n    _ = a[0]\n    _ = !a[0]\n    _ = x.items[0]\n    _ = f()[0]\n}\n",
        &[
            (3, "a", None, OccurrenceKind::Usage),
            (4, "a", None, OccurrenceKind::Usage),
            (5, "items", Some("x.items"), OccurrenceKind::Usage),
            (6, "f", None, OccurrenceKind::Call),
        ],
    ),
    // R5-X.06, R5-X.10: a Python starred target unpacks the whole attribute;
    // a PHP static member keeps its class scope.
    (
        "starred.py",
        "def g(r):\n    [*x.s] = r\n    *x.s, y = r\n",
        &[(2, "s", Some("x.s"), OccurrenceKind::Usage), (3, "s", Some("x.s"), OccurrenceKind::Usage)],
    ),
    (
        "statics.php",
        "<?php\nfunction g() {\n    $a = X::K;\n    ++X::$f;\n    $c = \\N\\X::K;\n}\n",
        &[
            (3, "K", Some("X::K"), OccurrenceKind::Usage),
            (4, "f", Some("X::$f"), OccurrenceKind::Usage),
            (5, "K", Some("\\N\\X::K"), OccurrenceKind::Usage),
        ],
    ),
];

/// R5-F3.C4: a written receiver path is stored and served as computed by a
/// functions-only entry, its warm `Hit`, a record-only semantic widening, a
/// kind widening, the complete warm `Hit` and a projected type-only result;
/// each equals an uncached run, and each `Hit` proves the stored path passed
/// the entry's path validation.
#[test]
fn written_receiver_paths_hold_through_the_cache() {
    assert_paths_and_kinds_hold_through_the_cache(WRITTEN_PATH_CACHE_SOURCES);
}

/// R6-F1 sources: Swift force-unwrapped and C# null-forgiving invocations
/// at several grouping depths, with their receivers, written paths and a
/// non-invoked control.
const NULL_ASSERTION_CACHE_SOURCES: &[WrittenPathCacheRow] = &[
    (
        "unwrap.swift",
        "class X { var cb: (() -> Void)? }\nfunc g(cb: (() -> Void)??, x: X) {\n    (cb!)!()\n    ((cb!)!)()\n    x.cb!()\n    (x.cb!)?()\n    _ = x.cb!\n}\n",
        &[
            (3, "cb", None, OccurrenceKind::Call),
            (4, "cb", None, OccurrenceKind::Call),
            (5, "x", None, OccurrenceKind::Usage),
            (5, "cb", Some("x.cb"), OccurrenceKind::Call),
            (6, "x", None, OccurrenceKind::Usage),
            (6, "cb", Some("x.cb"), OccurrenceKind::Call),
            (7, "cb", Some("x.cb"), OccurrenceKind::Usage),
        ],
    ),
    (
        "forgiving.cs",
        "class G {\n    System.Action F;\n    void g(System.Action cb, G obj) {\n        cb!();\n        ((cb!)!)();\n        ((obj.F)!)();\n        obj?.F!();\n        obj.F!!();\n        var v = obj.F!;\n        i++;\n    }\n}\n",
        &[
            (4, "cb", None, OccurrenceKind::Call),
            (5, "cb", None, OccurrenceKind::Call),
            (6, "obj", None, OccurrenceKind::Usage),
            (6, "F", Some("obj.F"), OccurrenceKind::Call),
            (7, "obj", None, OccurrenceKind::Usage),
            (7, "F", Some("obj?.F"), OccurrenceKind::Call),
            (8, "F", Some("obj.F"), OccurrenceKind::Call),
            (9, "F", Some("obj.F"), OccurrenceKind::Usage),
            (10, "i", None, OccurrenceKind::Usage),
        ],
    ),
];

/// R6-F1.C4: a Swift force-unwrapped or C# null-forgiving call keeps its
/// `Call` kind and written path through the same six cache stages.
#[test]
fn null_assertion_invocation_kinds_hold_through_the_cache() {
    assert_paths_and_kinds_hold_through_the_cache(NULL_ASSERTION_CACHE_SOURCES);
}

/// Serves every source through a functions-only `Miss`, its warm `Hit`, a
/// record-only semantic widening, a kind widening, the complete warm `Hit`
/// and a projected type-only `Hit`, and checks every expected written path
/// and kind at each stage besides equality with an uncached run.
fn assert_paths_and_kinds_hold_through_the_cache(sources: &[WrittenPathCacheRow]) {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(name, source, expected) in sources {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup); 6] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }),
            ("kind widening", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        for (step, query, expected_lookup) in steps {
            let cell = format!("{name}: {step}");
            let (lookup, served, _) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");
            let occurrences: Vec<Occurrence> =
                serde_json::from_value(served["index"]["occurrences"].clone()).expect("occurrences");
            for &(line, target, written, kind) in expected {
                let found = occurrences
                    .iter()
                    .find(|occurrence| occurrence.span.start.line == line && occurrence.target.name == target)
                    .unwrap_or_else(|| panic!("{cell}: no `{target}` on line {line}: {occurrences:#?}"));
                assert_eq!(found.target.qualified_name.as_deref(), written, "{cell}: `{target}` on line {line}");
                assert_eq!(found.kind, kind, "{cell}: `{target}` on line {line}");
                assert_eq!(
                    &source[found.span.start_byte as usize..found.span.end_byte as usize],
                    target,
                    "{cell}: the span stays the name token"
                );
            }
        }
    }
}

/// `(file name, source, every callable whose parameters are `x`, `y`, `z`
/// with defaults `1`, `2`, `3`)`: comments sit before, after and on both
/// sides of each `=`.
const COMMENTED_DEFAULT_CACHE_SOURCES: &[(&str, &str, &[&str])] = &[
    (
        "defaults.cs",
        "class K {\n    void g(int x /* b */ = 1, int y = /* a */ 2, int z /* b */ = /* a */ 3) {}\n    \
         K(int x /* b */ = 1, int y = /* a */ 2, int z /* b */ = /* a */ 3) {}\n    \
         void Outer() { void Local(int x /* b */ = 1, int y = /* a */ 2, int z /* b */ = /* a */ 3) {} }\n}\n",
        &["g", "K", "Local"],
    ),
    (
        "defaults.php",
        "<?php\nfunction g($x /* b */ = 1, $y = /* a */ 2, $z /* b */ = /* a */ 3) {}\nclass K {\n    \
         function __construct($x /* b */ = 1, $y = /* a */ 2, $z /* b */ = /* a */ 3) {}\n    \
         function method($x /* b */ = 1, $y = /* a */ 2, $z /* b */ = /* a */ 3) {}\n}\n",
        &["g", "__construct", "method"],
    ),
    (
        "defaults.scala",
        "object O {\n  def g(x: Int /* b */ = 1, y: Int = /* a */ 2, z: Int /* b */ = /* a */ 3): Int = x\n}\n\
         class K {\n  def method(x: Int /* b */ = 1, y: Int = /* a */ 2, z: Int /* b */ = /* a */ 3): Int = x\n}\n",
        &["g", "method"],
    ),
];

/// R6-F2.C4: a comment beside a default's `=` neither erases nor becomes
/// the default at any cache stage: a functions-only `Miss`, its warm `Hit`,
/// a record-only semantic widening, a kind widening, the complete warm
/// `Hit` and a functions-only projection of the complete entry.
#[test]
fn commented_parameter_defaults_hold_through_the_cache() {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(FUNCTION_KINDS);
    for &(name, source, callables) in COMMENTED_DEFAULT_CACHE_SOURCES {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup); 6] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }),
            ("kind widening", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        for (step, query, expected_lookup) in steps {
            let cell = format!("{name}: {step}");
            let (lookup, served, _) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");
            let symbols = served["index"]["symbols"].as_array().expect("symbols");
            for callable in callables {
                let parameters: Vec<Value> = symbols
                    .iter()
                    .filter(|record| record["identity"]["name"] == *callable)
                    .filter_map(|record| record["kind_data"]["data"].get("parameters").cloned())
                    .collect();
                let [Value::Array(parameters)] = parameters.as_slice() else {
                    panic!("{cell}: `{callable}` has {parameters:?}");
                };
                let actual: Vec<(&str, Option<&str>, bool)> = parameters
                    .iter()
                    .map(|parameter| {
                        (
                            parameter["name"].as_str().expect("name"),
                            parameter["default_value"].as_str(),
                            parameter["is_optional"].as_bool().expect("is_optional"),
                        )
                    })
                    .collect();
                assert_eq!(
                    actual,
                    [("x", Some("1"), true), ("y", Some("2"), true), ("z", Some("3"), true)],
                    "{cell}: `{callable}`"
                );
            }
        }
    }
}

/// `(file name, source, every Parameter record's name in the complete
/// index, every occurrence of the checked names as (line, column, name,
/// kind, candidate names))`.
type ParameterCaptureCacheRow = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static [(u32, u32, &'static str, OccurrenceKind, &'static [&'static str])],
);

/// R5-F4.C5 sources: a Python default value and lambda default and a C#
/// parameter type, each a reference whose only candidate is the real
/// declaration.
const PARAMETER_CAPTURE_CACHE_SOURCES: &[ParameterCaptureCacheRow] = &[
    (
        "defaults.py",
        "def external(): pass\ndef g(x=external): pass\nh = lambda y=external: y\n",
        &["x", "y"],
        &[
            (2, 9, "external", OccurrenceKind::Usage, &["external"]),
            (3, 14, "external", OccurrenceKind::Usage, &["external"]),
        ],
    ),
    (
        "types.cs",
        "class W {}\nclass G {\n    const int external = 1;\n    void g(W obj, int n = external) {}\n}\n",
        &["obj", "n"],
        &[
            (4, 12, "W", OccurrenceKind::Usage, &["W"]),
            (4, 27, "external", OccurrenceKind::Usage, &["external"]),
        ],
    ),
];

/// R5-F4.C5: a parameter's default value and type stay references, with
/// the real declaration as their only candidate and no parameter record of
/// their name, through a functions-only entry, its warm `Hit`, a record-only
/// semantic widening, a kind widening, the complete warm `Hit` and a
/// projected type-only result; each also equals an uncached run.
#[test]
fn parameter_defaults_and_types_stay_references_through_the_cache() {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(name, source, parameters, reads) in PARAMETER_CAPTURE_CACHE_SOURCES {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup); 6] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }),
            ("kind widening", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        let names: BTreeSet<&str> = reads.iter().map(|&(_, _, name, _, _)| name).collect();
        for (step, query, expected_lookup) in steps {
            let cell = format!("{name}: {step}");
            let (lookup, served, _) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");

            let occurrences: Vec<Occurrence> =
                serde_json::from_value(served["index"]["occurrences"].clone()).expect("occurrences");
            let actual: Vec<(u32, u32, &str, OccurrenceKind, Vec<&str>)> = occurrences
                .iter()
                .filter(|occurrence| names.contains(occurrence.target.name.as_str()))
                .map(|occurrence| {
                    let candidates = occurrence.candidates.iter().map(|candidate| candidate.name.as_str()).collect();
                    (occurrence.span.start.line, occurrence.span.start.column, occurrence.target.name.as_str(), occurrence.kind, candidates)
                })
                .collect();
            let expected: Vec<(u32, u32, &str, OccurrenceKind, Vec<&str>)> = reads
                .iter()
                .map(|&(line, column, name, kind, candidates)| (line, column, name, kind, candidates.to_vec()))
                .collect();
            assert_eq!(actual, expected, "{cell}: reads");

            let records = served["index"]["symbols"].as_array().expect("symbols");
            let declared: Vec<&str> = records
                .iter()
                .filter(|record| record["kind"] == "Parameter")
                .map(|record| record["identity"]["name"].as_str().expect("name"))
                .collect();
            let wanted: &[&str] = if matches!(step, "kind widening" | "complete warm") { parameters } else { &[] };
            assert_eq!(declared, wanted, "{cell}: parameter records");
        }
    }
}

/// `(file name, source, the declaration tokens as (line, column, name),
/// every occurrence of the checked names as (line, column, name, candidate
/// names))`.
type DeclarationCacheRow = (
    &'static str,
    &'static str,
    &'static [(u32, u32, &'static str)],
    &'static [(u32, u32, &'static str, &'static [&'static str])],
);

/// R5-F2.C4 sources: Java type parameters, Python lambda parameters and
/// pattern captures, and lowercase Scala binders, none of them a record.
const DECLARATION_CACHE_SOURCES: &[DeclarationCacheRow] = &[
    (
        "generics.java",
        "class Bound {}\nclass G<T> { T f; <U extends Bound> U m(U u) { return u; } }\n",
        &[(2, 9, "T"), (2, 20, "U")],
        &[(2, 14, "T", &[]), (2, 30, "Bound", &["Bound"]), (2, 37, "U", &[]), (2, 41, "U", &[])],
    ),
    (
        "patterns.py",
        "def g(subject):\n    cb = lambda x: x\n    match subject:\n        case [a1, *rest]:\n            return a1, rest\n",
        &[(2, 17, "x"), (4, 15, "a1"), (4, 20, "rest")],
        &[(2, 20, "x", &[]), (3, 11, "subject", &["subject"]), (5, 20, "a1", &[]), (5, 24, "rest", &[])],
    ),
    (
        "binders.scala",
        "object O {\n  def a(xs: List[Int]) = for (x <- xs) yield x\n  def c(o: Option[Int]) = o match { case Some(y) => y; case Stable => 0 }\n}\n",
        &[(2, 31, "x"), (3, 47, "y")],
        &[
            (2, 36, "xs", &["xs"]),
            (2, 46, "x", &[]),
            (3, 27, "o", &["o"]),
            (3, 42, "Some", &[]),
            (3, 53, "y", &[]),
            (3, 61, "Stable", &[]),
        ],
    ),
    // R7-F2: annotation types, a C# static constructor and accessor event,
    // a TypeScript namespace prefix, a Swift macro and selective import, a
    // qualified Java module: declared names no record (or only a whole
    // path) reports; the genuine reads beside them stay.
    (
        "Annotation.java",
        "class A {}\nclass G { @interface A { int value() default external; } @A void g() {} }\n",
        &[(2, 22, "A"), (2, 30, "value")],
        &[(2, 46, "external", &[]), (2, 59, "A", &["A"])],
    ),
    (
        "Events.cs",
        "class G { static G() {} event System.Action E {add{} remove{}} void f() { E(); } G x; }\n",
        &[(1, 18, "G"), (1, 45, "E")],
        &[(1, 75, "E", &[]), (1, 82, "G", &["G"])],
    ),
    (
        "namespace.ts",
        "class A {} namespace A.B { export function f(){} } let x:A;\n",
        &[(1, 22, "A")],
        &[(1, 58, "A", &[])],
    ),
    (
        "macro.swift",
        "func foo() {}\nmacro foo() = #externalMacro(module: \"M\", type: \"T\")\nfoo()\n",
        &[(2, 7, "foo")],
        &[(3, 1, "foo", &["foo"])],
    ),
    (
        "import.swift",
        "import struct Foo.Bar\nfunc g() { let z = Bar }\n",
        &[(1, 19, "Bar")],
        &[(1, 15, "Foo", &[]), (2, 20, "Bar", &[])],
    ),
    (
        "module-info.java",
        "module a.b { requires c.d; }\n",
        &[(1, 8, "a"), (1, 10, "b")],
        &[(1, 23, "c", &[]), (1, 25, "d", &[])],
    ),
];

/// R5-F2.C4: a declaration token no record reports is never an occurrence
/// and the reads around it stay, with the same candidates, through a
/// functions-only entry, its warm `Hit`, a record-only semantic widening, a
/// kind widening, the complete warm `Hit` and a projected type-only result;
/// each also equals an uncached run.
#[test]
fn unreported_declaration_tokens_stay_excluded_through_the_cache() {
    assert_exclusions_hold_through_the_cache(DECLARATION_CACHE_SOURCES);
}

/// R6-F3.C5 sources: Lua variable attributes, Java inferred-type `var` and
/// PHP's class-relative `static`, each beside a same-named declaration
/// whose real reads stay suggestions for it. The excluded list holds the
/// markers only.
const MARKER_CACHE_SOURCES: &[DeclarationCacheRow] = &[
    (
        "attributes.lua",
        "local const=1; local close=2\nlocal x <const> = const\nlocal y <close> = close\nlocal function f() return const, close end\n",
        &[(2, 10, "const"), (3, 10, "close")],
        &[
            (2, 19, "const", &["const"]),
            (3, 19, "close", &["close"]),
            (4, 27, "const", &["const"]),
            (4, 34, "close", &["close"]),
        ],
    ),
    (
        "Inferred.java",
        "record R() {\n    void g() { var x=factory(); int var=1; use(var); }\n    void h(java.util.List<String> xs) { for (var y : xs) { use(y); } }\n}\n",
        &[(2, 16, "var"), (3, 46, "var")],
        &[(2, 22, "factory", &[]), (2, 48, "var", &["var"]), (3, 54, "xs", &["xs"]), (3, 64, "y", &["y"])],
    ),
    (
        "relative.php",
        "<?php\nclass C {\n    function static() {}\n    function f(): static { return new static(); }\n    function g() { return $this->static(); }\n}\n",
        &[(4, 19, "static"), (4, 39, "static")],
        &[(5, 34, "static", &["static"])],
    ),
    // R7-F1: a static import of a member named `var` is no evidence of a
    // legacy type, so the inferred-type marker stays excluded and the call
    // and read of the variable keep their candidates.
    (
        "StaticImport.java",
        "import static p.Tools.var;\nrecord G() {\n    void g() { var x = var(); int var = 2; use(var); }\n}\n",
        &[(1, 23, "var"), (3, 16, "var"), (3, 35, "var")],
        &[(3, 24, "var", &["var"]), (3, 48, "var", &["var"])],
    ),
    // R7-F1: Swift precedence directives and associativity values are
    // markers; a group reference and same-named locals stay reads.
    (
        "precedence.swift",
        "precedencegroup A {\n    associativity: left\n    higherThan: AdditionPrecedence\n}\nfunc g() {\n    let left = 1, higherThan = 2\n    use(left, higherThan)\n}\n",
        &[(2, 5, "associativity"), (2, 20, "left"), (3, 5, "higherThan"), (6, 9, "left"), (6, 19, "higherThan")],
        &[(3, 17, "AdditionPrecedence", &[]), (7, 9, "left", &["left"]), (7, 15, "higherThan", &["higherThan"])],
    ),
    (
        "EveryInference.java",
        "import static p.Tools.var;\nrecord P(int a) {}\nclass S {\n    void g(java.util.List<String> xs, Object o, int var) {\n        var v = id(xs);\n        final var w = id(v);\n        for (var y : xs) { use(y); }\n        try (var r = open(xs)) { use(r); }\n        java.util.function.Function<String, String> l = (var z) -> z;\n        if (o instanceof P(var a)) { use(a); }\n        for (var i = 0; i < var; i++) {}\n    }\n}\n",
        &[(1, 23, "var"), (4, 53, "var"), (5, 9, "var"), (6, 15, "var"), (7, 14, "var"), (8, 14, "var"), (9, 58, "var"), (10, 28, "var"), (11, 14, "var")],
        &[(11, 29, "var", &["var"])],
    ),
    (
        "EveryPrecedence.swift",
        "precedencegroup A {\n    associativity: left\n    assignment: true\n    higherThan: AdditionPrecedence\n    lowerThan: MultiplicationPrecedence\n}\nprecedencegroup B {\n    associativity: right\n}\nprecedencegroup C {\n    associativity: none\n}\ninfix operator +++ : TernaryPrecedence\nfunc g() {\n    let left = 1, right = 2, none = 3\n    let associativity = 4, assignment = 5, higherThan = 6, lowerThan = 7\n    use(left, right, none, associativity, assignment, higherThan, lowerThan)\n}\n",
        &[(2, 5, "associativity"), (2, 20, "left"), (3, 5, "assignment"), (4, 5, "higherThan"), (5, 5, "lowerThan"), (8, 5, "associativity"), (8, 20, "right"), (11, 5, "associativity"), (11, 20, "none"), (15, 9, "left"), (15, 19, "right"), (15, 30, "none"), (16, 9, "associativity"), (16, 28, "assignment"), (16, 44, "higherThan"), (16, 60, "lowerThan")],
        &[(4, 17, "AdditionPrecedence", &[]), (5, 16, "MultiplicationPrecedence", &[]), (13, 22, "TernaryPrecedence", &[]), (17, 9, "left", &["left"]), (17, 15, "right", &["right"]), (17, 22, "none", &["none"]), (17, 28, "associativity", &["associativity"]), (17, 43, "assignment", &["assignment"]), (17, 55, "higherThan", &["higherThan"]), (17, 67, "lowerThan", &["lowerThan"])],
    ),
    // R7-F1.S2/.S3: availability platforms and `#if` operands are markers.
    (
        "availability.swift",
        "#if os(macOS)\nlet x = 1\n#endif\nfunc g() {\n    if #available(macOS 10.15, *) {}\n    let macOS = 1\n    use(macOS)\n}\n",
        &[(1, 8, "macOS"), (5, 19, "macOS"), (6, 9, "macOS")],
        &[(7, 9, "macOS", &["macOS"])],
    ),
    // R7-F1.S5: named-argument and tuple labels are no reads; the value
    // after each label stays one.
    (
        "keywords.py",
        "def f(x): pass\nx = 1\nf(x=x)\n",
        &[(1, 7, "x"), (2, 1, "x"), (3, 3, "x")],
        &[(3, 5, "x", &["x", "x"])],
    ),
    (
        "labels.swift",
        "func g() {\n    let x = 1\n    _ = f(x: x)\n    let t = (x: x, y: 2)\n}\n",
        &[(2, 9, "x"), (3, 11, "x"), (4, 14, "x")],
        &[(3, 14, "x", &["x"]), (4, 17, "x", &["x"])],
    ),
    // R7-F1.S6: a built-in attribute name is a marker; a property
    // wrapper's name stays a read.
    (
        "attributes.swift",
        "@propertyWrapper struct W { var wrappedValue: Int }\n@objc func h() {}\nstruct S { @W var q: Int }\nfunc g() {\n    let objc = 1\n    use(objc)\n}\n",
        &[(1, 2, "propertyWrapper"), (1, 25, "W"), (2, 2, "objc"), (5, 9, "objc")],
        &[(3, 13, "W", &["W"]), (6, 9, "objc", &["objc"])],
    ),
    // R6-F3.X1: a bare-identifier initializer is a read beside its
    // excluded declared name and the inferred-type marker.
    (
        "Initializer.java",
        "class A {\n    int xs = 1; int ys = xs;\n    void g() { int k = xs; var v = xs; }\n}\n",
        &[(2, 9, "xs"), (2, 21, "ys"), (3, 20, "k"), (3, 28, "var"), (3, 32, "v")],
        &[(2, 26, "xs", &["xs"]), (3, 24, "xs", &["xs"]), (3, 36, "xs", &["xs"])],
    ),
    // R6-F3.X2: zsh declaration-builtin operands are declaration tokens;
    // the expansion and the call stay reads.
    (
        "declare.zsh",
        "x() { :; }\nf() { local x; export x; echo $x; x; }\n",
        &[(1, 1, "x"), (2, 13, "x"), (2, 23, "x")],
        &[(2, 32, "x", &["x"]), (2, 35, "x", &["x"])],
    ),
];

/// R6-F3.C5: a contextual syntax marker is never an occurrence, and the
/// same-named reads keep their candidates, through every cache stage of
/// [`unreported_declaration_tokens_stay_excluded_through_the_cache`].
#[test]
fn contextual_syntax_markers_stay_excluded_through_the_cache() {
    assert_exclusions_hold_through_the_cache(MARKER_CACHE_SOURCES);
}

/// `(file, source, [(line, column, name, written path)] every occurrence on
/// those lines, [(import name, line, column)])`.
type QualifiedImportCacheRow = (
    &'static str,
    &'static str,
    &'static [(u32, u32, &'static str, Option<&'static str>)],
    &'static [(&'static str, u32, u32)],
);

/// R7-F3.C4 sources: a qualified annotation and module service operands are
/// reads with their paths and no imports; a genuine import keeps only its
/// final segment; every C# `using` directive is an import whose qualifiers
/// are reads (`R7-F3.S1`); Scala, Python, Rust, PHP and TS imports report
/// only the names they bind (`R7-F3.S3`-`.S6`); a C/C++ elaborated type use
/// is a read (`R7-F2.X1`); TS export clause names are no reads (`R7-F3.X2`)
/// and a Go raw-string import path is an import (`R7-F3.X3`).
const QUALIFIED_IMPORT_CACHE_SOURCES: &[QualifiedImportCacheRow] = &[
    (
        "G.java",
        "class A {}\n@p.q.A class G { void f() {} }\n@A class H {}\n",
        &[(2, 2, "p", None), (2, 4, "q", Some("p.q")), (2, 6, "A", Some("p.q.A")), (3, 2, "A", None)],
        &[],
    ),
    (
        "module-info.java",
        "module m {\n    uses a.b.C;\n    provides a.b.C with d.e.D, f.g.F;\n}\nclass C { void f() {} }\n",
        &[
            (2, 10, "a", None),
            (2, 12, "b", Some("a.b")),
            (2, 14, "C", Some("a.b.C")),
            (3, 14, "a", None),
            (3, 16, "b", Some("a.b")),
            (3, 18, "C", Some("a.b.C")),
            (3, 25, "d", None),
            (3, 27, "e", Some("d.e")),
            (3, 29, "D", Some("d.e.D")),
            (3, 32, "f", None),
            (3, 34, "g", Some("f.g")),
            (3, 36, "F", Some("f.g.F")),
        ],
        &[],
    ),
    (
        "K.java",
        "import a.b.C;\nimport static a.b.C.*;\n@p.q.A class K { void f() {} }\n",
        &[(1, 8, "a", None), (1, 10, "b", Some("a.b")), (3, 2, "p", None), (3, 4, "q", Some("p.q")), (3, 6, "A", Some("p.q.A"))],
        &[("C", 1, 12), ("C", 2, 19)],
    ),
    (
        "U.cs",
        "using A.B;\nusing E = F.G;\nusing static C.D;\nclass Z { void f() {} }\n",
        &[(1, 7, "A", None), (2, 11, "F", None), (2, 13, "G", Some("F.G")), (3, 14, "C", None)],
        &[("B", 1, 9), ("E", 2, 7), ("D", 3, 16)],
    ),
    (
        "I.scala",
        "import a.b.C\nimport a.{D, E => F}\nimport c.d.*\nobject O { def m(): Unit = { C; F } }\n",
        &[(1, 8, "a", None), (1, 10, "b", Some("a.b")), (2, 8, "a", None), (2, 14, "E", None), (3, 8, "c", None)],
        &[("C", 1, 12), ("D", 2, 11), ("F", 2, 19), ("d", 3, 10)],
    ),
    (
        "i.py",
        "import a.b\nfrom g.h import i as j\nfrom l import *\ndef m():\n    return a, j\n",
        &[(1, 10, "b", Some("a.b")), (2, 6, "g", None), (2, 8, "h", Some("g.h")), (2, 17, "i", None), (5, 12, "a", None), (5, 15, "j", None)],
        &[("a", 1, 8), ("j", 2, 22), ("l", 3, 6)],
    ),
    (
        "i.rs",
        "use a;\nuse b::*;\nuse c::{g::h, e as f};\nextern crate y as z;\nfn run() { a::x(); h(); f(); z::w(); }\n",
        &[(5, 12, "a", None), (5, 15, "x", Some("a::x")), (5, 20, "h", None), (5, 25, "f", None), (5, 30, "z", None), (5, 33, "w", Some("z::w"))],
        &[("a", 1, 5), ("b", 2, 5), ("h", 3, 12), ("f", 3, 20), ("z", 4, 19)],
    ),
    (
        "i.php",
        "<?php\nuse A;\nuse B\\C as D;\nuse E\\{F, G as H};\nfunction f() { return new D(); }\n",
        &[(3, 5, "B", None), (3, 7, "C", Some("B\\C")), (4, 5, "E", None), (4, 11, "G", None), (5, 27, "D", None)],
        &[("A", 2, 5), ("D", 3, 12), ("F", 4, 8), ("H", 4, 16)],
    ),
    (
        "i.ts",
        "import d, { a as b } from \"m\";\nimport q = require(\"m\");\nfunction g() { d(); b(); q(); }\n",
        &[(3, 16, "d", None), (3, 21, "b", None), (3, 26, "q", None)],
        &[("d", 1, 8), ("b", 1, 18), ("q", 2, 8)],
    ),
    (
        "exports.ts",
        "function k() {}\nexport { k as j } from \"m\";\nexport { k as g };\nexport * as k from \"m\";\nfunction g() { k(); }\n",
        &[(3, 10, "k", None), (5, 16, "k", None)],
        &[],
    ),
    (
        "r.go",
        "package p\nimport `c/d`\nimport x `e/f`\nfunc f() { d.A(); x.B() }\n",
        &[(4, 12, "d", None), (4, 14, "A", Some("d.A")), (4, 19, "x", None), (4, 21, "B", Some("x.B"))],
        &[("d", 2, 8), ("x", 3, 8)],
    ),
    (
        "e.c",
        "struct S { int a; };\nstruct S x;\nvoid f(struct S *p) {}\n",
        &[(2, 8, "S", None), (3, 15, "S", None)],
        &[],
    ),
    (
        "e.cpp",
        "class C {};\nclass C *c;\nvoid f(class C *p) {}\n",
        &[(2, 7, "C", None), (3, 14, "C", None)],
        &[],
    ),
];

crate::language_cases::language_tests!(qualified_reads_and_imports_hold_through_the_cache => check_qualified_reads_and_imports_hold_through_the_cache);

/// R7-F3.C4: Every Java qualified-name inventory row and the sibling import
/// readers keep their
/// offsets and paths, and no phantom import appears, through a
/// functions-only entry, its warm `Hit`, a record-only semantic widening, a
/// kind widening, the complete warm `Hit` and a projected type-only result;
/// each also equals an uncached run.
fn check_qualified_reads_and_imports_hold_through_the_cache(language: ProgrammingLanguage) {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    let sources = QUALIFIED_IMPORT_CACHE_SOURCES.iter().map(|&(name, source, reads, imports)| {
        (name, source, reads.to_vec(), imports.to_vec())
    }).chain(super::symbols_occurrences::JAVA_QUALIFIED_REFERENCE_MATRIX.iter().map(|&(name, source, reads, imports)| {
        let reads = reads.iter().map(|&(line, column, name, path, _)| (line, column, name, path)).collect();
        let imports = imports.iter().map(|&(name, _, _, line, column)| (name, line, column)).collect();
        (name, source, reads, imports)
    }));
    for (name, source, reads, imports) in sources.filter(|(name, ..)| if name.starts_with("R7-F3.") { language == ProgrammingLanguage::Java } else { ProgrammingLanguage::from_path(std::path::Path::new(name)) == Some(language) }) {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(if name.starts_with("R7-F3.") { "Matrix.java" } else { name });
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup); 6] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }),
            ("kind widening", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        let lines: BTreeSet<u32> = reads.iter().map(|&(line, ..)| line).collect();
        for (step, query, expected_lookup) in steps {
            let cell = format!("{name}: {step}");
            let (lookup, served, _) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");

            let occurrences: Vec<Occurrence> =
                serde_json::from_value(served["index"]["occurrences"].clone()).expect("occurrences");
            let actual: Vec<(u32, u32, &str, Option<&str>)> = occurrences
                .iter()
                .filter(|occurrence| lines.contains(&occurrence.span.start.line))
                .map(|occurrence| {
                    (
                        occurrence.span.start.line,
                        occurrence.span.start.column,
                        occurrence.target.name.as_str(),
                        occurrence.target.qualified_name.as_deref(),
                    )
                })
                .collect();
            assert_eq!(actual, reads, "{cell}: reads");
            if let Some(&(_, _, expected_reads, _)) = super::symbols_occurrences::JAVA_QUALIFIED_REFERENCE_MATRIX.iter().find(|&&(label, ..)| label == name) {
                for (&(line, column, target, _, name_only), occurrence) in expected_reads.iter().zip(occurrences.iter().filter(|occurrence| lines.contains(&occurrence.span.start.line))) {
                    assert_eq!(occurrence.kind, OccurrenceKind::Usage, "{cell}: {line}:{column}");
                    assert_eq!(occurrence.target.id, None, "{cell}: no invented target ID");
                    assert_eq!(occurrence.confidence, if name_only { tree_hugger::Confidence::NameOnly } else { tree_hugger::Confidence::Unresolved }, "{cell}: confidence");
                    let candidates: Vec<&str> = occurrence.candidates.iter().map(|candidate| candidate.name.as_str()).collect();
                    assert_eq!(candidates, if name_only { vec![target] } else { vec![] }, "{cell}: candidate names");
                }
                for coverage in served["index"]["reference_coverage"].as_array().expect("coverage") {
                    assert_eq!(coverage["state"], "Limited", "{cell}: coverage");
                    assert!(coverage["reasons"].as_array().expect("reasons").iter().any(|reason| reason == "unsupported_language"), "{cell}: reason");
                }
                for record in served["index"]["symbols"].as_array().expect("symbols") {
                    for relation in ["references", "referenced_by", "dependencies"] {
                        assert!(record["relations"][relation].is_null() || record["relations"][relation].as_array().is_some_and(Vec::is_empty), "{cell}: no established {relation}");
                    }
                    if !record["semantics"]["is_recursive"].is_null() {
                        assert_eq!(record["semantics"]["is_recursive"], "Unknown", "{cell}: recursion");
                    }
                }
            }


            let served_imports = served["index"]["imports"].as_array().expect("imports");
            let actual_imports: Vec<(&str, u64, u64)> = served_imports
                .iter()
                .map(|import| {
                    (
                        import["name"].as_str().expect("import name"),
                        import["span"]["start"]["line"].as_u64().expect("line"),
                        import["span"]["start"]["column"].as_u64().expect("column"),
                    )
                })
                .collect();
            let expected_imports: Vec<(&str, u64, u64)> =
                imports.iter().map(|&(import, line, column)| (import, u64::from(line), u64::from(column))).collect();
            assert_eq!(actual_imports, expected_imports, "{cell}: imports");
        }
    }
}

/// Import comments and grammar alternatives (`symbols_occurrences::for_each_import_row`,
/// review rows `R8-F1.nn`, `R8-F2.nn`) hold through a functions-only entry, its warm
/// `Hit`, a record-only semantic widening, a kind widening, the complete
/// warm `Hit` and a projected type-only result: each stage carries the
/// row's own expected imports (original and source included), every read
/// and no imported token, and for TS/JS/Rust the owner's dependencies and
/// recursion; each also equals an uncached run.
fn check_commented_import_cache(selected: &str) {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    let mut checked = 0;
    super::symbols_occurrences::for_each_import_row(|row| {
        if row.label.split(": comment at").next().expect("base label") != selected { return; }
        checked += 1;
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(format!("source.{}", row.extension));
        std::fs::write(&path, row.source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup, bool); 6] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent), false),
            ("narrow warm", &narrow, CacheLookup::Hit, false),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }, false),
            ("kind widening", &complete, CacheLookup::WidenedKinds, true),
            ("complete warm", &complete, CacheLookup::Hit, true),
            ("projected", &projected, CacheLookup::Hit, false),
        ];
        for (step, query, expected_lookup, every_kind) in steps {
            let cell = format!("{}: {step}", row.label);
            let (lookup, served, work) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            if !matches!(lookup, CacheLookup::Miss(_) | CacheLookup::WidenedKinds) {
                assert_eq!(work.parser_constructions, 0, "{cell}: served without a parse");
            }
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");
            let index: tree_hugger::FileSymbolIndex = serde_json::from_value(served["index"].clone()).expect("index");
            let evidence = tree_hugger::ReferenceEvidence {
                occurrences: index.occurrences.clone(),
                reference_coverage: index.reference_coverage.clone(),
            };
            super::symbols_occurrences::assert_commented_import_row(row, &cell, &index, &evidence, every_kind);
        }
    });
    assert!(checked > 0, "scheduled import row was absent: {selected}");
}

/// Runs each source through a functions-only entry, its warm `Hit`, a
/// record-only semantic widening, a kind widening, the complete warm `Hit`
/// and a projected type-only result; each equals an uncached run, holds no
/// excluded token and exactly the expected reads.
fn assert_exclusions_hold_through_the_cache(sources: &[DeclarationCacheRow]) {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(name, source, declarations, reads) in sources {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let steps: [(&str, &SymbolQuery, CacheLookup); 6] = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }),
            ("kind widening", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        let names: BTreeSet<&str> = declarations
            .iter()
            .map(|&(_, _, name)| name)
            .chain(reads.iter().map(|&(_, _, name, _)| name))
            .collect();
        let expected: Vec<(u32, u32, &str, Vec<&str>)> =
            reads.iter().map(|&(line, column, name, candidates)| (line, column, name, candidates.to_vec())).collect();
        for (step, query, expected_lookup) in steps {
            let cell = format!("{name}: {step}");
            let (lookup, served, _) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{cell}");
            assert_eq!(served, uncached(&path, query), "{cell}: equals an uncached run");

            let occurrences: Vec<Occurrence> =
                serde_json::from_value(served["index"]["occurrences"].clone()).expect("occurrences");
            let actual: Vec<(u32, u32, &str, Vec<&str>)> = occurrences
                .iter()
                .filter(|occurrence| names.contains(occurrence.target.name.as_str()))
                .map(|occurrence| {
                    let candidates = occurrence.candidates.iter().map(|candidate| candidate.name.as_str()).collect();
                    (occurrence.span.start.line, occurrence.span.start.column, occurrence.target.name.as_str(), candidates)
                })
                .collect();
            for &(line, column, declared) in declarations {
                assert!(
                    !actual.iter().any(|(l, c, n, _)| (*l, *c, *n) == (line, column, declared)),
                    "{cell}: the excluded `{declared}` at {line}:{column} is an occurrence"
                );
            }
            assert_eq!(actual, expected, "{cell}: reads");
        }
    }
}

crate::language_cases::language_tests!(every_declaration_exclusion_shape_holds_through_six_cache_stages => check_every_declaration_exclusion_shape_holds_through_six_cache_stages);

/// Every declaration-exclusion shape, including earlier sibling controls,
/// keeps its independently expected reads through persistence and projection.
fn check_every_declaration_exclusion_shape_holds_through_six_cache_stages(language: ProgrammingLanguage) {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(label, extension, source, declarations, reads) in crate::symbols_occurrences::DECLARATION_EXCLUSION_MATRIX.iter().filter(|row| ProgrammingLanguage::from_extension(row.1) == Some(language)) {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(format!("source.{extension}"));
        std::fs::write(&path, source).expect("write source");
        let cache = cache_in(&dir);
        let names: BTreeSet<&str> = declarations.iter().map(|&(_, _, name)| name)
            .chain(reads.iter().map(|&(_, _, name, _)| name)).collect();
        let steps = [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent)),
            ("narrow warm", &narrow, CacheLookup::Hit),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }),
            ("kind widening", &complete, CacheLookup::WidenedKinds),
            ("complete warm", &complete, CacheLookup::Hit),
            ("projected", &projected, CacheLookup::Hit),
        ];
        for (stage, query, expected_lookup) in steps {
            let (lookup, served, work) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected_lookup, "{label}: {stage}");
            if matches!(lookup, CacheLookup::Hit | CacheLookup::WidenedCapabilities { parsed: false }) {
                assert_eq!(work.parser_constructions, 0, "{label}: {stage}");
            }
            assert_eq!(served, uncached(&path, query), "{label}: {stage}");
            let occurrences: Vec<Occurrence> = serde_json::from_value(served["index"].get("occurrences").cloned().unwrap_or_else(|| json!([])))
                .unwrap_or_else(|_| panic!("{label}: {stage}: occurrences"));
            let actual: Vec<_> = occurrences.iter().filter(|occurrence| names.contains(occurrence.target.name.as_str()))
                .map(|occurrence| (occurrence.span.start.line, occurrence.span.start.column,
                    occurrence.target.name.as_str(), occurrence.kind)).collect();
            assert_eq!(actual, reads, "{label}: {stage}: exact reads and excluded declarations");
            if label.starts_with("R7-F2.12a typescript dotted namespace;") {
                let usage = occurrences.iter().find(|occurrence| occurrence.target.name == "A").expect("type read");
                assert_eq!(usage.confidence, tree_hugger::Confidence::Scoped, "{label}: {stage}");
                assert!(usage.target.id.is_some(), "{label}: {stage}: class identity survives omitted kinds");
            }
            if label.starts_with("R7-F2.12b") {
                for record in served["index"]["symbols"].as_array().expect("symbols") {
                    assert!(record["relations"]["references"].as_array().is_none_or(Vec::is_empty), "{label}: {stage}");
                    assert!(record["relations"]["referenced_by"].as_array().is_none_or(Vec::is_empty), "{label}: {stage}");
                }
            }
        }
    }
}


#[test]
fn type_only_imports_preserve_categories_through_six_cache_stages() {
    use crate::symbols_occurrences::{TYPE_ONLY_IMPORT_MATRIX, check_type_only_import_index, type_only_import_source};
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(label, import, name, mixed) in TYPE_ONLY_IMPORT_MATRIX {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join("imports.ts");
        std::fs::write(&path, type_only_import_source(import, name, mixed)).expect("source");
        let cache = cache_in(&dir);
        for (stage, query, expected, semantics) in [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent), false),
            ("narrow warm", &narrow, CacheLookup::Hit, false),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }, true),
            ("kind widening", &complete, CacheLookup::WidenedKinds, true),
            ("complete warm", &complete, CacheLookup::Hit, true),
            ("projected", &projected, CacheLookup::Hit, false),
        ] {
            let (lookup, served, counters) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected, "{label}: {stage}");
            assert_eq!(served, uncached(&path, query), "{label}: {stage}");
            if matches!(lookup, CacheLookup::Hit | CacheLookup::WidenedCapabilities { parsed: false }) {
                assert_eq!(counters.parser_constructions, 0, "{label}: {stage}");
            }
            let index = serde_json::from_value(served["index"].clone()).expect("index");
            check_type_only_import_index(&index, name, mixed, semantics);
        }
    }
}


#[test]
fn type_only_import_controls_hold_through_six_cache_stages() {
    use tree_hugger::{Confidence, CoverageState, FileSymbolIndex, TriState};
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    let sources = [
        ("R9-F2.09.namespace", "import type * as NS from './types';\nlet value: NS.F;\nfunction run() {}\n", "NS"),
        ("R9-F2.09.require", "import type NS = require('./types');\nlet value: NS.F;\nfunction run() {}\n", "NS"),
        ("R9-F2.09.namespace.comments", "import /* a */ type /* b */ * as /* c */ NS from './types';\nlet value: NS.F;\nfunction run() {}\n", "NS"),
        ("R9-F2.09.require.comments", "import /* a */ type /* b */ NS = require('./types');\nlet value: NS.F;\nfunction run() {}\n", "NS"),
        ("R9-F2.10", "import { type as g } from './types';\nfunction run() { g(); }\n", "g"),
        ("R9-F2.11", "import type { T } from './types';\nimport { G as g } from './types';\nfunction f(x: T) { f(x); g(); }\n", "T"),
    ];
    for &(label, source, imported) in &sources {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join("controls.ts");
        std::fs::write(&path, source).expect("source");
        let cache = cache_in(&dir);
        for (stage, query, expected, semantics) in [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent), false),
            ("narrow warm", &narrow, CacheLookup::Hit, false),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }, true),
            ("kind widening", &complete, CacheLookup::WidenedKinds, true),
            ("complete warm", &complete, CacheLookup::Hit, true),
            ("projected", &projected, CacheLookup::Hit, false),
        ] {
            let (lookup, served, counters) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected, "{label}: {stage}");
            assert_eq!(served, uncached(&path, query), "{label}: {stage}");
            if matches!(lookup, CacheLookup::Hit | CacheLookup::WidenedCapabilities { parsed: false }) {
                assert_eq!(counters.parser_constructions, 0, "{label}: {stage}");
            }
            let index: FileSymbolIndex = serde_json::from_value(served["index"].clone()).expect("index");
            let import = index.occurrences.iter().find(|occ| occ.target.name == imported).expect("import read");
            assert_eq!(import.confidence, Confidence::Scoped, "{label}: {stage}");
            assert_eq!(import.target.id, None);
            assert!(import.candidates.is_empty());
            assert!(index.reference_coverage.iter().all(|region| region.state == CoverageState::Complete));
            if imported == "NS" {
                assert_eq!(index.occurrences.len(), 2, "namespace and terminal type reads only");
                assert_eq!(import.kind, OccurrenceKind::Usage);
                let terminal = index.occurrences.iter().find(|occ| occ.target.name == "F").expect("terminal type read");
                assert_eq!(terminal.kind, OccurrenceKind::Usage);
                assert_eq!(terminal.confidence, Confidence::Unresolved);
                assert_eq!(terminal.target.qualified_name.as_deref(), Some("NS.F"));
                assert_eq!(terminal.target.id, None);
            } else if imported == "g" {
                assert_eq!(index.occurrences.len(), 1, "ordinary exported type is a call");
                assert_eq!(import.kind, OccurrenceKind::Call);
                assert_eq!(index.imports[0].symbol.qualified_name.as_deref(), Some("type"));
            } else {
                assert_eq!(import.kind, OccurrenceKind::Usage);
                let call = index.occurrences.iter().find(|occ| occ.target.name == "f").expect("self-call");
                let owner = call.owner.as_ref().and_then(|owner| owner.id.as_ref()).expect("f ID");
                assert_eq!(call.kind, OccurrenceKind::Call);
                assert_eq!(call.confidence, Confidence::Scoped);
                assert_eq!(call.target.id.as_ref(), Some(owner));
                let g = index.occurrences.iter().find(|occ| occ.target.name == "g").expect("ordinary import");
                assert_eq!(g.kind, OccurrenceKind::Call);
                assert_eq!(g.confidence, Confidence::Scoped);
                assert_eq!(g.target.id, None);
            }
            if let Some(function) = index.symbols.iter().find(|record| record.kind == SymbolKindV2::Function) {
                if semantics { assert_eq!(function.semantics.is_recursive, Some(if imported == "T" { TriState::Yes } else { TriState::No })); }
                let expected = if imported == "NS" { vec![] } else if imported == "g" { vec!["g"] } else { vec!["T", "g"] };
                assert_eq!(function.relations.dependencies.iter().map(|target| target.name.as_str()).collect::<Vec<_>>(), expected);
            }
        }
    }
}

/// An import whose only spelling is an encoded token (a TS/JS Unicode
/// escape, a Rust raw identifier) still witnesses its unbound uses, so the
/// warm read is a hit, served as computed.
#[test]
fn encoded_binding_witnesses_keep_unbound_uses_served() {
    for (file, source, name) in [
        ("escaped.ts", "import { \\u0078 } from \"m\";\nfunction f(){ return x; }\n", "x"),
        ("escaped_braced.js", "import { \\u{79} } from \"m\";\nfunction f(){ return y; }\n", "y"),
        ("raw.rs", "use m::r#match_;\nfn f() { match_(); }\n", "match_"),
    ] {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(file);
        std::fs::write(&path, source).expect("source");
        let cache = cache_in(&dir);
        let query = SymbolQuery::new().capabilities([Capability::Occurrences]);
        let (lookup, control, _) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent), "{file}");
        let unbound = control["index"]["occurrences"].as_array().expect("occurrences").iter()
            .find(|occurrence| occurrence["target"]["name"] == name).expect("a use of the local binding");
        assert_eq!(unbound["confidence"], "Scoped", "{file}");
        assert!(unbound["target"]["id"].is_null(), "{file}");
        let (lookup, served, _) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Hit, "{file}");
        assert_eq!(served, control, "{file}");
    }
}

/// An unrelated encoded binding cannot excuse a lost ID on a retained target.
#[test]
fn encoded_binding_sources_preserve_lost_id_rejection() {
    for (file, source) in [
        ("raw.rs", "fn g(){} fn f(){g();} fn r#other(){}"),
        ("escaped.ts", r"function g(){} function f(){g();} function \u006fther(){}"),
        ("raw_ownerless.rs", "const fn r#f() {} const X: () = f();"),
        ("escaped_ownerless.ts", r"function \u0066(){} const X = f();"),
    ] {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(file);
        std::fs::write(&path, source).expect("source");
        let cache = cache_in(&dir);
        let query = SymbolQuery::new().capabilities([Capability::Occurrences]);
        let (lookup, control, _) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent));
        let entry_path = only_entry(&dir);
        let mut entry = read_entry(&entry_path);
        let call = entry["occurrences"].as_array_mut().expect("occurrences").iter_mut()
            .find(|o| o["target"]["name"] == "g" || o["target"]["name"] == "f").expect("retained target call");
        let expected_id = call["target"]["id"].clone();
        assert!(expected_id.is_string());
        call["target"]["id"] = Value::Null;
        for record in entry["records"].as_array_mut().expect("records") {
            record["relations"]["references"] = json!([]);
            record["relations"]["referenced_by"] = json!([]);
        }
        write_entry(&entry_path, &entry);
        let (lookup, repaired, _) = cached(&cache, read(&path), &query);
        assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Inconsistent("a scoped occurrence with no target ID and no binding of its name")), "{file}");
        assert_eq!(repaired, control);
        assert_eq!(repaired["index"]["occurrences"][0]["target"]["id"], expected_id);
    }
}

#[test]
fn selected_import_provenance_holds_through_six_cache_stages() {
    use crate::symbols_occurrences::{IMPORT_PROVENANCE_MATRIX, check_import_provenance};
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences, Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds(TYPE_KINDS).capabilities([Capability::Occurrences]);
    for &(label, file, source, owner, name, shadow) in IMPORT_PROVENANCE_MATRIX {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(file);
        std::fs::write(&path, source).expect("source");
        let cache = cache_in(&dir);
        for (stage, query, expected, semantics) in [
            ("narrow cold", &narrow, CacheLookup::Miss(CacheMiss::Absent), false),
            ("narrow warm", &narrow, CacheLookup::Hit, false),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }, true),
            ("kind widening", &complete, CacheLookup::WidenedKinds, true),
            ("complete warm", &complete, CacheLookup::Hit, true),
            ("projected", &projected, CacheLookup::Hit, false),
        ] {
            let (lookup, served, counters) = cached(&cache, read(&path), query);
            assert_eq!(lookup, expected, "{label}: {stage}");
            assert_eq!(served, uncached(&path, query), "{label}: {stage}");
            if matches!(lookup, CacheLookup::Hit | CacheLookup::WidenedCapabilities { parsed: false }) {
                assert_eq!(counters.parser_constructions, 0, "{label}: {stage}");
            }
            let index = serde_json::from_value(served["index"].clone()).expect("index");
            check_import_provenance(&index, label, owner, name, shadow, semantics);
        }
    }
}

macro_rules! stored_import_cases {
    ($($name:ident => $row:literal => $label:literal),+ $(,)?) => {
        mod stored_selected_import_provenance_matrix_recomputes_public_evidence {
            $(#[test] fn $name() { super::check_stored_import_provenance($row); })+
            #[test]
            fn every_source_has_a_case() {
                let registered = [$($label),+];
                let actual: Vec<_> = crate::symbols_occurrences::IMPORT_PROVENANCE_MATRIX.iter().map(|row| row.0).collect();
                assert_eq!(registered.as_slice(), actual);
            }
        }
    };
}
stored_import_cases! {
    row_0 => 0 => "R9-F4.01.rs",
    row_1 => 1 => "R9-F4.01.rs.renamed",
    row_2 => 2 => "R9-F4.02.ts",
    row_3 => 3 => "R9-F4.02.ts.renamed",
    row_4 => 4 => "R9-F4.03.ts",
    row_5 => 5 => "R9-F4.03.js",
    row_6 => 6 => "R9-F4.03.rs",
    row_7 => 7 => "R9-F4.05.rs.self",
    row_8 => 8 => "R9-F4.05.ts.self",
    row_9 => 9 => "R9-F4.05.rs.raw",
    row_10 => 10 => "R9-F4.05.ts.escaped",
    row_11 => 11 => "R9-F4.05.js.escaped",
    row_12 => 12 => "R9-F4.05.js.unique",
}

fn check_stored_import_provenance(row: usize) {
    use crate::symbols_occurrences::{IMPORT_PROVENANCE_MATRIX, check_import_provenance};
    let query = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let fields = ["import_span", "import_start_byte", "import_span/start", "import_span/end", "import_span/start_byte", "import_span/end_byte", "import_span/start/line", "import_span/start/column", "import_span/end/line", "import_span/end/column"];
    let &(label, file, source, owner, name, shadow) = &IMPORT_PROVENANCE_MATRIX[row];
    {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(file);
        std::fs::write(&path, source).expect("source");
        let cache = cache_in(&dir);
        let (_, expected, _) = cached(&cache, read(&path), &query);
        let entry_path = only_entry(&dir);
        let written = read_entry(&entry_path);
        let occurrences = written["occurrences"].as_array().expect("occurrences");
        let at = occurrences.iter().position(|o| o["target"]["name"] == name && o["owner"]["name"] == owner && o["kind"] == "Call").expect("import call");
        assert!(occurrences[at]["import_span"].is_object(), "load-bearing positive control");
        let local = occurrences.iter().position(|o| o["target"]["name"] == name && o["owner"]["name"] == owner && o["kind"] == "Call" && o["import_span"].is_null());
        for at in std::iter::once(at).chain(local) {
        for field in fields {
            if field.starts_with("import_span/") && occurrences[at]["import_span"].is_null() { continue; }
            let pointer = format!("/occurrences/{at}/{field}");
            let (parent_pointer, key) = pointer.rsplit_once('/').expect("field");
            for shape in ["control", "absent", "null", "wrong type", "empty object", "empty list", "empty string", "zero", "out-of-source number", "duplicate key", "invalid content"] {
                let mut edited = written.clone();
                let value = edited.pointer(&pointer).expect("field").clone();
                let mut bytes = None;
                match shape {
                    "control" => {}
                    "absent" => { edited.pointer_mut(parent_pointer).expect("parent").as_object_mut().expect("object").remove(key); }
                    "null" => *edited.pointer_mut(&pointer).expect("field") = Value::Null,
                    "wrong type" => *edited.pointer_mut(&pointer).expect("field") = json!(true),
                    "empty object" => *edited.pointer_mut(&pointer).expect("field") = json!({}),
                    "empty list" => *edited.pointer_mut(&pointer).expect("field") = json!([]),
                    "empty string" => *edited.pointer_mut(&pointer).expect("field") = json!(""),
                    "zero" => *edited.pointer_mut(&pointer).expect("field") = json!(0),
                    "out-of-source number" => *edited.pointer_mut(&pointer).expect("field") = json!(999999999),
                    "duplicate key" => {
                        let parent = serde_json::to_string(edited.pointer(parent_pointer).expect("parent")).expect("parent JSON");
                        let duplicate = format!("{{\"{key}\":{},{}", serde_json::to_string(&value).expect("value JSON"), &parent[1..]);
                        let occurrence_json = serde_json::to_string(&edited["occurrences"][at]).expect("occurrence JSON");
                        let duplicate_occurrence = occurrence_json.replacen(&parent, &duplicate, 1);
                        bytes = Some(serde_json::to_string(&edited).expect("JSON").replacen(&occurrence_json, &duplicate_occurrence, 1));
                    }
                    "invalid content" => bytes = Some(format!("{} garbage", serde_json::to_string(&edited).expect("JSON"))),
                    _ => unreachable!(),
                }
                if let Some(bytes) = bytes { std::fs::write(&entry_path, bytes).expect("write malformed JSON"); }
                else { write_entry(&entry_path, &edited); }
                let (lookup, served, _) = cached(&cache, read(&path), &query);
                if shape == "control" || shape == "zero" && value == json!(0) || shape == "null" && value.is_null() {
                    assert_eq!(lookup, CacheLookup::Hit, "{label} {field} {shape}");
                } else { assert!(matches!(lookup, CacheLookup::Miss(_)), "{label} {field} {shape}: {lookup:?}"); }
                assert_eq!(served, expected, "{label} {field} {shape}: recomputed control");
                let index = serde_json::from_value(served["index"].clone()).expect("index");
                check_import_provenance(&index, label, owner, name, shadow, false);
            }
        }
        }
        if label == "R9-F4.01.rs" || label == "R9-F4.02.ts" {
            let mut edited = written.clone();
            let other_import = edited["imports"].as_array().expect("imports").iter()
                .find(|i| i["symbol"]["qualified_name"] == if owner == "f" { "f" } else { "N.S" }).expect("other scope import")["span"].clone();
            edited["occurrences"][at]["import_span"] = other_import.clone();
            edited["occurrences"][at]["import_start_byte"] = other_import["start_byte"].clone();
            write_entry(&entry_path, &edited);
            let (lookup, served, _) = cached(&cache, read(&path), &query);
            assert!(matches!(lookup, CacheLookup::Miss(_)), "{label}: other scope cannot supply the original");
            assert_eq!(served, expected);
        }
        if shadow {
            let local = local.expect("local call");
            assert!(occurrences[local]["import_span"].is_null());
            for field in ["import_span", "import_start_byte"] {
                let mut edited = written.clone();
                edited["occurrences"][local][field] = occurrences[at][field].clone();
                write_entry(&entry_path, &edited);
                let (lookup, served, _) = cached(&cache, read(&path), &query);
                assert!(matches!(lookup, CacheLookup::Miss(_)), "{label}: invented {field}");
                assert_eq!(served, expected);
            }
        }
    }
}

/// The identity a retaining query must give back unchanged.
fn source_identity(source: &SourceFile) -> (PathBuf, String, String, ProgrammingLanguage, Option<ProgrammingLanguage>, bool) {
    (
        source.path().to_path_buf(),
        source.hash().to_string(),
        source.source().to_string(),
        source.language(),
        source.forced_language(),
        source.uses_experimental_semantics(),
    )
}

#[test]
fn retaining_queries_give_back_the_source_they_analyzed_on_every_lookup() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("lib.rs");
    std::fs::write(&path, "pub fn probe() -> u32 { helper() }\nfn helper() -> u32 { 1 }\npub struct W;\n")
        .expect("write source");
    let cache = cache_in(&dir);
    let functions = SymbolQuery::new().kinds(FUNCTION_KINDS);
    let every_kind = SymbolQuery::new();
    let occurrences = SymbolQuery::new().capabilities([Capability::Occurrences]);
    type ExpectedLookup = fn(&CacheLookup) -> bool;
    let steps: [(&SymbolQuery, ExpectedLookup); 4] = [
        (&functions, |lookup| matches!(lookup, CacheLookup::Miss(_))),
        (&functions, |lookup| matches!(lookup, CacheLookup::Hit)),
        (&every_kind, |lookup| matches!(lookup, CacheLookup::WidenedKinds)),
        (&occurrences, |lookup| matches!(lookup, CacheLookup::WidenedCapabilities { parsed: true })),
    ];

    for (query, expected_lookup) in steps {
        let source = read(&path);
        let identity = source_identity(&source);
        let (outcome, back) = cache.query_retaining(source, query).expect("retaining cached query");
        assert!(expected_lookup(&outcome.lookup), "unexpected lookup {:?}", outcome.lookup);
        assert_eq!(as_value(&outcome.result), uncached(&path, query));
        assert_eq!(source_identity(&back), identity);
    }

    let forced = SourceFile::read_with_language(&path, Some(ProgrammingLanguage::Rust))
        .expect("read source")
        .experimental_semantics(true);
    let identity = source_identity(&forced);
    let (result, back) = forced.query_retaining(&occurrences).expect("retaining uncached query");
    assert_eq!(as_value(&result), uncached(&path, &occurrences));
    assert_eq!(source_identity(&back), identity);
}

/// The stored `undefined-symbol` diagnostic of an entry whose index is `value`.
fn undefined_symbol_severities(value: &Value) -> Vec<Value> {
    value["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter(|diagnostic| diagnostic["rule"] == "undefined-symbol")
        .map(|diagnostic| diagnostic["severity"].clone())
        .collect()
}

/// A warm hit serves each diagnostic's stored severity; it never restamps it.
/// A change to how severity is stamped therefore reaches cached files only
/// through a fingerprint change, which is what `EXTRACTOR_REVISION` is for.
#[test]
fn a_warm_hit_replays_the_stored_diagnostic_severity() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("undefined.py");
    std::fs::write(&path, "total = compute_total(items)\n").expect("write source");
    let source = || read(&path).experimental_semantics(true);
    let query = SymbolQuery::new().capabilities([Capability::Diagnostics]);
    let cache = cache_in(&dir);

    let (lookup, cold, _) = cached(&cache, source(), &query);
    assert!(matches!(lookup, CacheLookup::Miss(_)), "cold run: {lookup:?}");
    let uncached = as_value(&source().query(&query).expect("uncached query"));
    assert_eq!(cold, uncached, "the cold result equals an uncached run");
    let computed = undefined_symbol_severities(&cold["index"]);
    assert_eq!(computed, vec![json!("Warning"); 2], "`compute_total` and `items` carry the registry severity");

    let entry_path = files_with_extension(&dir.path().join("cache"), "json")
        .into_iter()
        .next()
        .expect("one stored entry");
    let mut entry = read_entry(&entry_path);
    assert_eq!(undefined_symbol_severities(&entry), computed, "the entry stores the computed severity");

    let (lookup, warm, _) = cached(&cache, source(), &query);
    assert_eq!(lookup, CacheLookup::Hit);
    assert_eq!(undefined_symbol_severities(&warm["index"]), computed);

    // A severity no current build stamps on `undefined-symbol` proves the
    // hit serves what is stored rather than recomputing it.
    for diagnostic in entry["diagnostics"].as_array_mut().expect("diagnostics") {
        if diagnostic["rule"] == "undefined-symbol" {
            diagnostic["severity"] = json!("Info");
        }
    }
    write_entry(&entry_path, &entry);
    let (lookup, replayed, _) = cached(&cache, source(), &query);
    assert_eq!(lookup, CacheLookup::Hit);
    assert!(
        undefined_symbol_severities(&replayed["index"]).iter().all(|severity| severity == "Info"),
        "the hit replays the stored severity: {replayed:#}"
    );
}

/// An entry written at extractor revision 50, which stamped `undefined-symbol`
/// `Error`, is refused even under the current key: it is recomputed with the
/// registry's `Warning`, and the rewritten entry is then a hit equal to an
/// uncached run. A control proves the same `Error` payload under current
/// fingerprints would be served.
#[test]
fn a_revision_fifty_entry_never_replays_its_error_severity() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("undefined.py");
    std::fs::write(&path, "total = compute_total(items)\n").expect("write source");
    let source = || read(&path).experimental_semantics(true);
    let query = SymbolQuery::new().capabilities([Capability::Diagnostics]);
    let cache = cache_in(&dir);
    let uncached = as_value(&source().query(&query).expect("uncached query"));
    assert_eq!(undefined_symbol_severities(&uncached["index"]), vec![json!("Warning"); 2]);

    cached(&cache, source(), &query);
    let entry_path = files_with_extension(&dir.path().join("cache"), "json")
        .into_iter()
        .next()
        .expect("one stored entry");
    let mut entry = read_entry(&entry_path);
    for diagnostic in entry["diagnostics"].as_array_mut().expect("diagnostics") {
        if diagnostic["rule"] == "undefined-symbol" {
            diagnostic["severity"] = json!("Error");
        }
    }

    // Control: under current fingerprints the `Error` payload is served.
    write_entry(&entry_path, &entry);
    let (lookup, control, _) = cached(&cache, source(), &query);
    assert_eq!(lookup, CacheLookup::Hit);
    assert_eq!(undefined_symbol_severities(&control["index"]), vec![json!("Error"); 2]);

    // Revision 50's fingerprints, as that build wrote them.
    let inputs = FingerprintInputs::shipped(ProgrammingLanguage::Python, &path).expect("inputs");
    let at_revision_fifty = |mut fingerprint: tree_hugger::cache::PassFingerprint| {
        fingerprint.tree_hugger_version = format!("{}+rev.50", env!("CARGO_PKG_VERSION"));
        fingerprint.digest()
    };
    entry["fingerprint"] = json!(at_revision_fifty(inputs.records_fingerprint()));
    entry["diagnostics_fingerprint"] = json!(at_revision_fifty(inputs.diagnostics_fingerprint(true)));
    write_entry(&entry_path, &entry);

    let (lookup, recomputed, _) = cached(&cache, source(), &query);
    assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Mismatched("fingerprint")));
    assert_eq!(recomputed, uncached, "the revision-50 entry is recomputed, not served");
    assert_eq!(undefined_symbol_severities(&read_entry(&entry_path)), vec![json!("Warning"); 2]);

    let (lookup, warm, _) = cached(&cache, source(), &query);
    assert_eq!(lookup, CacheLookup::Hit);
    assert_eq!(warm, uncached, "the rewritten entry is a hit equal to an uncached run");
}

mod commented_imports_hold_through_the_cache {
    use super::*;
    macro_rules! import_row_tests {
        ($($test:ident => $label:literal),* $(,)?) => {
            const LISTED: &[&str] = &[$($label),*];
            $(#[test] fn $test() { check_commented_import_cache($label); })*
        };
    }
    import_row_tests! {
        r8_f2_07_ts_string_named_original => "R8-F2.07.ts string-named original",
        r8_f2_07_js_string_named_original => "R8-F2.07.js string-named original",
        r8_f1_01_ts_entity_alias_block_comment_before => "R8-F1.01 TS entity alias, block comment before `=`",
        r8_f1_01_ts_entity_alias_line_comment_before => "R8-F1.01 TS entity alias, line comment before `=`",
        r8_f1_01_export_ts_exported_entity_alias_comment_before => "R8-F1.01.export TS exported entity alias, comment before `=`",
        r8_f1_02_ts_entity_alias_block_comment_after => "R8-F1.02 TS entity alias, block comment after `=`",
        r8_f1_02_ts_entity_alias_line_comment_after => "R8-F1.02 TS entity alias, line comment after `=`",
        r8_f1_s1_ts_entity_alias_comment_before_the_bound_name => "R8-F1.S1 TS entity alias, comment before the bound name",
        r8_f1_s2_ts_entity_alias_comment_inside_the_target_path => "R8-F1.S2 TS entity alias, comment inside the target path",
        r8_f1_09_ts_clean_ts_import_forms_beside_comments => "R8-F1.09.ts clean TS import forms beside comments",
        r8_f1_09_ts_named_default_and_namespace_imports_beside_comments => "R8-F1.09.ts named, default and namespace imports beside comments",
        r8_f1_09_js_named_default_and_namespace_imports_beside_comments => "R8-F1.09.js named, default and namespace imports beside comments",
        r8_f1_03_scala_block_comment_between_path_segments => "R8-F1.03 Scala block comment between path segments",
        r8_f1_03_scala_line_comment_between_path_segments => "R8-F1.03 Scala line comment between path segments",
        r8_f1_03_scala_comment_in_a_braced_and_an_on_demand_path => "R8-F1.03 Scala comment in a braced and an on-demand path",
        r8_f1_04a_scala_comment_before_a_plain_selector_group => "R8-F1.04a Scala comment before a plain selector group",
        r8_f1_04b_scala_comment_before_a_renamed_selector_and_as => "R8-F1.04b Scala comment before a renamed selector (`=>` and `as`)",
        r8_f1_05_star_scala_comment_before_a_wildcard_block_and_line => "R8-F1.05.star Scala comment before a `*` wildcard (block and line)",
        r8_f1_05_underscore_scala_comment_before_a_wildcard => "R8-F1.05.underscore Scala comment before a `_` wildcard",
        r8_f1_05_given_scala_comment_before_a_direct_given => "R8-F1.05.given Scala comment before a direct `given`",
        r8_f1_06_mixed_scala_comment_before_given_t => "R8-F1.06.mixed Scala comment before `{_, given T}`",
        r8_f1_06_given_scala_comment_before_given_t => "R8-F1.06.given Scala comment before `{given T}`",
        r8_f1_07_simple_scala_comment_before_after_a_one_segment_path => "R8-F1.07.simple Scala comment before `,` after a one-segment path",
        r8_f1_07_qualified_scala_comment_before_after_a_qualified_path => "R8-F1.07.qualified Scala comment before `,` after a qualified path",
        r8_f1_08_scala_comments_the_binding_never_depended_on => "R8-F1.08 Scala comments the binding never depended on",
        r8_f1_10_rs_rust_use_paths_lists_renames_globs_and_extern_aliases_with_comments => "R8-F1.10.rs Rust use paths, lists, renames, globs and extern aliases with comments",
        r8_f1_10_java_java_qualified_imports_with_comments => "R8-F1.10.java Java qualified imports with comments",
        r8_f1_10_cs_c_alias_static_and_global_usings_with_comments => "R8-F1.10.cs C# alias, static and global usings with comments",
        r8_f1_10_php_php_use_paths_aliases_and_groups_with_comments => "R8-F1.10.php PHP use paths, aliases and groups with comments",
        r8_f1_10_swift_swift_selective_and_whole_module_imports_with_comments => "R8-F1.10.swift Swift selective and whole-module imports with comments",
        r8_f1_10_py_python_parenthesized_from_import_with_a_line_comment => "R8-F1.10.py Python parenthesized from-import with a line comment",
        r8_f1_10_go_go_aliased_blank_dot_and_raw_string_imports_with_comments => "R8-F1.10.go Go aliased, blank, dot and raw-string imports with comments",
        r8_f2_01_single => "R8-F2.01.single",
        r8_f2_01_multiple => "R8-F2.01.multiple",
        r8_f2_01_nested => "R8-F2.01.nested",
        r8_f2_02_escaped => "R8-F2.02.escaped",
        r8_f2_02_original => "R8-F2.02.original",
        r8_f2_03_direct => "R8-F2.03.direct",
        r8_f2_03_multiple => "R8-F2.03.multiple",
        r8_f2_03_braced => "R8-F2.03.braced",
        r8_f2_04_arrow => "R8-F2.04.arrow",
        r8_f2_04_as => "R8-F2.04.as",
        r8_f2_04_as_mixed => "R8-F2.04.as.mixed",
        r8_f2_05_direct_root => "R8-F2.05.direct.root",
        r8_f2_05_direct => "R8-F2.05.direct",
        r8_f2_05_braced => "R8-F2.05.braced",
        r8_f2_05_wildcard => "R8-F2.05.wildcard",
        r8_f2_06_hidden => "R8-F2.06.hidden",
        r8_f2_06_backticks => "R8-F2.06.backticks",
        r8_f2_06_this => "R8-F2.06.this",
        r8_f2_06_super => "R8-F2.06.super",
        r8_f2_06_root => "R8-F2.06.root",
        r8_f2_06_given => "R8-F2.06.given",
    }
    #[test]
    fn every_import_row_is_scheduled() {
        let mut labels = Vec::new();
        super::super::symbols_occurrences::for_each_import_row(|row| {
            let label = row.label.split(": comment at").next().expect("base label").to_string();
            if labels.last() != Some(&label) { labels.push(label); }
        });
        assert_eq!(labels, LISTED);
    }
}
