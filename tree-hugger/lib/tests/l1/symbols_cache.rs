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
    Capability, EXTRACTOR_REVISION, ProgrammingLanguage, SourceFile, SymbolKindV2, SymbolQuery,
    SymbolQueryResult, TreeFile, WorkCounters, work_counters,
};

use crate::symbols_fixtures::{all_fixtures, display_name, multi_fixtures, primary_fixture};

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
    let cells: [(Option<&[SymbolKindV2]>, &[Capability]); 9] = [
        (None, &[Capability::Declarations]),
        (None, &[Capability::Containment]),
        (None, &[Capability::References]),
        (None, &[Capability::Semantics]),
        (None, &[Capability::StructuredDocs]),
        (None, &[Capability::Diagnostics]),
        (None, &Capability::ALL),
        (Some(method), &[Capability::Declarations]),
        (Some(method), &Capability::ALL),
    ];
    let dir = TempDir::new().expect("temp");
    let cache = cache_in(&dir);
    let mut compared = 0usize;
    let mut stripped_references = 0usize;
    let mut stripped_parsed_docs = 0usize;

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
                if !query.requires(Capability::StructuredDocs) {
                    stripped_parsed_docs +=
                        count_records(&full, |record| !record["docs"]["parsed"].is_null());
                }
            }
        }
    }
    assert!(fixture_count >= 16, "only {fixture_count} fixtures");
    assert_eq!(compared, fixture_count * cells.len());
    assert!(stripped_references > 0, "no projection removed references");
    assert!(stripped_parsed_docs > 0, "no projection removed parsed docs");
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
        WideningRow {
            language: ProgrammingLanguage::TypeScript,
            first: SymbolQuery::new().capabilities([Capability::References]),
            second: SymbolQuery::new().capabilities([Capability::Semantics]),
            parsed: false,
            diagnostics_runs: 0,
        },
        WideningRow {
            language: ProgrammingLanguage::Python,
            first: SymbolQuery::new(),
            second: SymbolQuery::new().capabilities([Capability::Containment]),
            parsed: false,
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
