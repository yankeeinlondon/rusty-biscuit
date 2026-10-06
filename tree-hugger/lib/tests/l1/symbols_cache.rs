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
    Capability, EXTRACTOR_REVISION, NamePattern, ProgrammingLanguage, SourceFile, SymbolKindV2,
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

/// The "Cache correctness" row of the per-language matrix: for every fixture,
/// the cold run stores an entry and the warm run is a complete hit that
/// parses nothing and equals an uncached run.
#[test]
fn every_fixture_is_a_complete_warm_hit_equal_to_an_uncached_run() {
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let query = SymbolQuery::complete_index();
    let fixtures: Vec<PathBuf> = all_fixtures().into_iter().chain(multi_fixtures()).collect();

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

/// Reference evidence survives repeated persistence: for every fixture, a
/// functions-only `Occurrences` entry is written, read and widened to the
/// complete index (rewritten), then read again as a `Hit`; each result equals
/// an uncached run, and both writes store the same whole-file evidence.
#[test]
fn stored_reference_evidence_round_trips_through_rewrites() {
    let narrow = SymbolQuery::new().kinds(FUNCTION_KINDS).capabilities([Capability::Occurrences]);
    let complete = SymbolQuery::complete_index();
    let evidence = |entry: &Value| json!([entry["occurrences"], entry["reference_coverage"]]);
    let fixtures: Vec<PathBuf> = all_fixtures().into_iter().chain(multi_fixtures()).collect();

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
#[test]
fn projected_results_equal_uncached_queries() {
    assert_projections_equal_uncached_queries(all_fixtures());
}

/// [`projected_results_equal_uncached_queries`] over the `multi/` tree.
#[test]
fn projected_results_over_multi_equal_uncached_queries() {
    assert_projections_equal_uncached_queries(multi_fixtures());
}

fn assert_projections_equal_uncached_queries(fixtures: Vec<PathBuf>) {
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

        for (kinds, capabilities) in cells {
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
    assert!(fixture_count >= 16, "only {fixture_count} fixtures");
    assert_eq!(compared, fixture_count * cells.len());
    assert!(stripped_references > 0, "no projection removed references");
    assert!(stripped_occurrences > 0, "no projection removed occurrences");
    assert!(stripped_parsed_docs > 0, "no projection removed parsed docs");
    assert!(stripped_flags > 0, "no projection removed a set flag");
    assert!(stripped_metrics > 0, "no projection removed metrics");
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

#[test]
fn widening_capabilities_runs_only_the_missing_passes() {
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

    for row in rows {
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
    let current = json!({ "major": 5, "minor": 0 });
    for version in [
        json!({ "major": 2, "minor": 0 }),
        json!({ "major": 3, "minor": 0 }),
        json!({ "major": 4, "minor": 0 }),
        json!({ "major": 4, "minor": 1 }),
        json!({ "major": 4, "minor": 2 }),
        json!({ "major": 4, "minor": 3 }),
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

/// For every source of the deferred kind matrix: a filtered query of the
/// deferred kinds stores every record of those kinds and records only those
/// kinds as its coverage, never the whole file; the unfiltered query is then
/// a hit, an every-kind query widens the kinds once, and a containment query
/// widens the capabilities without diagnostics. Afterwards each deferred kind,
/// with and without containment, is a hit that does no work and equals an
/// uncached query.
#[test]
fn deferred_kind_entries_never_claim_complete_coverage_and_warm_hits_do_no_work() {
    let deferred = SymbolQuery::new().kinds(DEFERRED_KINDS);
    let filtered = deferred.clone().name(NamePattern::new("no_declaration_has_this_name"));
    let mut sorted_kinds = DEFERRED_KINDS.to_vec();
    sorted_kinds.sort();
    let sources = TempDir::new().expect("sources");
    let (mut stored_records, mut warm_records) = (0, 0);
    for (ordinal, (language, source)) in deferred_kind_sources().into_iter().enumerate() {
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
    assert!(stored_records > 50, "too few stored records ({stored_records})");
    assert!(warm_records > 100, "too few warm records ({warm_records})");
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

/// Every source of the generics, modifiers, body-lines and relations
/// matrices (and the Rust impls) is stored complete, served warm as a real
/// `Hit` with no work, and projected to each narrower query exactly as an
/// uncached query computes it: generic parameters, modifiers, attributes and
/// relation targets kept, Rust impl containers and `metrics` dropped where
/// the query does not ask for them.
#[test]
fn declaration_fact_sources_are_warm_hits_and_project_to_uncached_queries() {
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

    for (label, path) in crate::symbols_relations::write_declaration_fact_sources(sources.path()) {
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
    assert!(dropped_containers > 0, "no projection removed a container");
    assert!(dropped_lines > 0, "no projection removed `metrics.lines`");
    assert!(kept_relations > 0, "no projection kept a relation");
}

/// Widening a declarations-only entry of the same sources: Containment is
/// added from the stored records alone, except in Rust and Swift, where an
/// impl member's container is its impl's resolved self type and an
/// extension member's its extension's resolved owner, which no record
/// retains, so it parses once; Metrics always parses once. Each widened
/// result equals an uncached query, the entry then records every request,
/// and each request (and the declarations-only narrowing) is a `Hit` with
/// no work.
#[test]
fn declaration_fact_entries_widen_by_containment_and_metrics() {
    let sources = TempDir::new().expect("sources");
    let declarations = SymbolQuery::new();
    let containment = SymbolQuery::new().capabilities([Capability::Containment]);
    let metrics = SymbolQuery::new().capabilities([Capability::Containment, Capability::Metrics]);
    let mut rust_rows = 0usize;

    for (label, path) in crate::symbols_relations::write_declaration_fact_sources(sources.path()) {
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
    assert!(rust_rows >= 5, "the Rust rows exercise the parsing path ({rust_rows})");
}

/// An entry of the Rust impl source whose records carry the previous schema
/// version (4.0, without `metrics.lines`) is never served or widened.
#[test]
fn a_declaration_fact_entry_of_an_older_record_schema_is_a_miss() {
    let sources = TempDir::new().expect("sources");
    let (label, path) = crate::symbols_relations::write_declaration_fact_sources(sources.path())
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

/// The Rust impl rows of the path-meaning matrix, cached declarations-only
/// and then asked for Containment: the widened result parses once and
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
        assert_eq!(widened.lookup, CacheLookup::WidenedCapabilities { parsed: true }, "{label}");
        assert_eq!(work.parser_constructions, 1, "{label}: one parse");
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

/// Every path-meaning row ([`path_meaning_rows`](crate::symbols_relations::path_meaning_rows))
/// with written bases, stored complete, is served warm as a real `Hit`
/// on the complete index and on a type-only query, and each owner keeps
/// exactly its uncached targets and target IDs.
#[test]
fn path_meaning_rows_hold_on_warm_hits() {
    use crate::symbols_relations::{check_owner_targets, path_meaning_rows};

    let sources = TempDir::new().expect("sources");
    let complete_query = SymbolQuery::complete_index();
    let types_only = SymbolQuery::new().kinds(TYPE_KINDS);
    let mut rows = 0;
    for (idx, row) in path_meaning_rows().filter(|row| !row.owners.is_empty()).enumerate() {
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
    assert!(rows >= 30, "the written-base rows ran ({rows})");
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

/// One test per generic reader; each analyzes a source of about half a
/// megabyte once.
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
