//! Test 8: a persistent cache entry written under an earlier extractor
//! fingerprint (the schema-2 literal key, a schema-3 key at extractor
//! revisions 1 to 4, or the computed key of revision 6) is not served after
//! the upgrade, and the entry written by the
//! upgraded run is served on the next run; records of another schema version
//! are recomputed even under the current key. Works on the cache files directly,
//! with `TMPDIR` (`TMP`/`TEMP` on Windows) pointed at a test-owned directory,
//! so a stale entry can be seeded under its historical key.

use std::path::{Path, PathBuf};

use biscuit_hash::{blake3_hash, xx_hash};
use serde_json::Value;
use tempfile::TempDir;
use tree_hugger::cache::{FingerprintInputs, canonical_digest};
use tree_hugger::{
    EXTRACTOR_REVISION, ProgrammingLanguage, SourceFile, SymbolKindV2, stable_symbol_key, symbol_id_from_stable_key,
};

use crate::{PYTHON_FIXTURE, RUST_FIXTURE, Scratch, TYPESCRIPT_FIXTURE, cache_entries, json_of, stdout_of};

const FILE: &str = "typescript.ts";
const ARGS: [&str; 3] = ["--json-full", "symbols", FILE];
const STALE_NAME: &str = "staleSeededSymbol";
const RENAMED_NAME: &str = "renamedInWarmEntry";

/// The persistent-cache key `hug` used before this fix, hard-coded rather than
/// computed by the code under test: `FileCacheKey::stable_key` over an
/// `AnalyzerFingerprint` of schema `2.0`, the crate version, the grammar name,
/// the literal `locals+imports+references` and the options fingerprint the
/// CLI formats for a `symbols` run.
fn historical_key(file_path: &str, source: &str) -> String {
    let options = "experimental_semantics=false;deny=;warn=;allow=;strict=false;external_adapters=false";
    let analyzer = blake3_hash(&format!(
        "2.0|{}|typescript|locals+imports+references|{options}",
        env!("CARGO_PKG_VERSION")
    ));
    let file_hash = format!("{:x}", xx_hash(source));
    blake3_hash(&format!(
        "{}|typescript|{file_hash}|{analyzer}",
        file_path.replace('\\', "/")
    ))
}

/// The cache entry path for `key` under `project_dir` (two-character shard).
fn entry_path(project_dir: &Path, key: &str) -> PathBuf {
    project_dir.join(&key[..2]).join(format!("{key}.json"))
}

pub(crate) fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read cache entry")).expect("cache JSON")
}

fn write_json(path: &Path, value: &Value) {
    std::fs::create_dir_all(path.parent().expect("entry parent")).expect("create shard");
    std::fs::write(path, serde_json::to_vec(value).expect("serialize")).expect("write entry");
}

/// Renames the record called `from` (name and display name) in a snapshot,
/// and the reference evidence owners, candidates and reference summaries
/// naming it. An occurrence keeps its name as written, and the reader checks
/// a target's or candidate's record against it, so a record named where an
/// occurrence is written cannot be renamed consistently.
pub(crate) fn rename_record(snapshot: &mut Value, from: &str, to: &str) {
    let record = snapshot["records"]
        .as_array_mut()
        .expect("entry records")
        .iter_mut()
        .find(|record| record["identity"]["name"] == from)
        .unwrap_or_else(|| panic!("snapshot has no `{from}` record"));
    record["identity"]["name"] = Value::from(to);
    record["identity"]["display_name"] = Value::from(to);
    let id = record["id"].clone();
    let written = snapshot["occurrences"].as_array().into_iter().flatten().any(|occurrence| occurrence["target"]["name"] == from);
    assert!(!written, "`{from}` is written at an occurrence; pick a sentinel no occurrence names");
    set_evidence_references(snapshot, &id, "name", Value::from(to));
}

/// Sets `field` of every reference evidence owner and candidate, and every
/// `references`/`referenced_by` summary entry, naming the record `id`. They
/// repeat the record's name and qualified name, and the reader checks that
/// they do; a target keeps the name as written.
fn set_evidence_references(snapshot: &mut Value, id: &Value, field: &str, value: Value) {
    let mut references: Vec<&mut Value> = Vec::new();
    for (key, list) in snapshot.as_object_mut().expect("entry object") {
        let Some(list) = list.as_array_mut() else { continue };
        match key.as_str() {
            "occurrences" => {
                for occurrence in list {
                    let occurrence = occurrence.as_object_mut().expect("occurrence object");
                    for (name, reference) in occurrence.iter_mut() {
                        match name.as_str() {
                            "owner" => references.push(reference),
                            "candidates" => references.extend(reference.as_array_mut().into_iter().flatten()),
                            _ => {}
                        }
                    }
                }
            }
            "reference_coverage" => references.extend(list.iter_mut().map(|region| &mut region["owner"])),
            "records" => {
                for record in list {
                    let Some(relations) = record.get_mut("relations").and_then(Value::as_object_mut) else { continue };
                    for (summary, entries) in relations.iter_mut() {
                        if summary == "references" || summary == "referenced_by" {
                            references.extend(entries.as_array_mut().into_iter().flatten());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for reference in references {
        if reference["id"] == *id {
            reference[field] = value.clone();
        }
    }
}

/// Sets the in-file qualified name of the record called `name`, and of the
/// evidence references naming it.
fn set_qualified_name(snapshot: &mut Value, name: &str, qualified_name: &str) {
    let record = record_named(snapshot, name);
    record["identity"]["qualified_name"] = Value::from(qualified_name);
    let id = record["id"].clone();
    set_evidence_references(snapshot, &id, "qualified_name", Value::from(qualified_name));
}

/// An entry whose records are recognizably stale: every access Unknown, no
/// `static` modifier, and one record renamed. The entry and its records keep
/// the current `schema_version`, so the payload stays servable.
fn stale_snapshot(current: &Value) -> Value {
    let mut snapshot = current.clone();
    for record in snapshot["records"].as_array_mut().expect("entry records") {
        record["visibility"]["level"] = Value::from("Unknown");
        if let Some(object) = record.as_object_mut() {
            object.remove("modifiers");
        }
    }
    rename_record(&mut snapshot, "exportedFunction", STALE_NAME);
    snapshot
}

/// `(name, line, kind, visibility, is_static, container_kind)` per symbol.
fn symbol_facts(json: &Value) -> Vec<(String, Value, Value, Value, Value, Value)> {
    let mut facts: Vec<_> = json["indexes"][0]["symbols"]
        .as_array()
        .expect("indexes[0].symbols")
        .iter()
        .map(|record| {
            let is_static = record["modifiers"]
                .as_array()
                .is_some_and(|modifiers| modifiers.iter().any(|modifier| modifier["type"] == "Static"));
            (
                record["identity"]["name"].as_str().unwrap_or_default().to_string(),
                record["source"]["declaration_span"]["start"]["line"].clone(),
                record["kind"].clone(),
                record["visibility"]["level"].clone(),
                Value::Bool(is_static),
                record["identity"]["container_kind"].clone(),
            )
        })
        .collect();
    facts.sort_by(|a, b| (&a.0, a.1.to_string()).cmp(&(&b.0, b.1.to_string())));
    facts
}

fn names(json: &Value) -> Vec<String> {
    symbol_facts(json).into_iter().map(|fact| fact.0).collect()
}

/// The single entry a run left under `temp`.
pub(crate) fn only_entry(temp: &Path) -> PathBuf {
    let entries = cache_entries(temp);
    assert_eq!(entries.len(), 1, "expected one cache entry, found {entries:#?}");
    entries.into_iter().next().expect("one entry")
}

#[test]
fn warm_entry_from_the_previous_extractor_is_not_served_after_the_upgrade() {
    let scratch = Scratch::new(&[(FILE, TYPESCRIPT_FIXTURE)]);

    // Discovery run in a separate temp directory: learn the path spelling `hug`
    // keys on (canonical, OS-specific) and its project directory, and the
    // current snapshot shape to derive the stale seed from.
    let discovery = TempDir::new().expect("discovery temp");
    let fresh = json_of(&stdout_of(scratch.hug_with_temp(discovery.path(), &ARGS), &ARGS));
    let discovered_entry = only_entry(discovery.path());
    let current_snapshot = read_json(&discovered_entry);
    let file_path = current_snapshot["file"]
        .as_str()
        .expect("entry file")
        .to_string();
    let relative_project = discovered_entry
        .parent()
        .and_then(Path::parent)
        .and_then(|dir| dir.strip_prefix(discovery.path()).ok())
        .expect("entry under <temp>/tree-hugger/cache/<project_id>/<shard>/")
        .to_path_buf();

    let new_key = discovered_entry
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("entry key")
        .to_string();
    let old_key = historical_key(&file_path, TYPESCRIPT_FIXTURE);

    let stale = stale_snapshot(&current_snapshot);

    // Control: the stale payload is a servable entry. Placed under the
    // current key, `hug` serves it, so run 1's miss below is due to the key.
    let control = TempDir::new().expect("control temp");
    write_json(&entry_path(&control.path().join(&relative_project), &new_key), &stale);
    let served = json_of(&stdout_of(scratch.hug_with_temp(control.path(), &ARGS), &ARGS));
    assert!(
        names(&served).iter().any(|name| name == STALE_NAME),
        "control: a stale payload under the current key must be served (names: {:?})",
        names(&served)
    );

    // The same payload with records of an earlier or later schema is not
    // served even under the current key: the records are recomputed.
    for version in [
        serde_json::json!({ "major": 2, "minor": 0 }),
        serde_json::json!({ "major": 3, "minor": 0 }),
        serde_json::json!({ "major": 4, "minor": 0 }),
        serde_json::json!({ "major": 4, "minor": 3 }),
        serde_json::json!({ "major": 6, "minor": 0 }),
    ] {
        let mut incompatible = stale.clone();
        for record in incompatible["records"].as_array_mut().expect("entry records") {
            record["schema_version"] = version.clone();
        }
        let temp = TempDir::new().expect("incompatible temp");
        write_json(&entry_path(&temp.path().join(&relative_project), &new_key), &incompatible);
        let recomputed = json_of(&stdout_of(scratch.hug_with_temp(temp.path(), &ARGS), &ARGS));
        assert_eq!(
            recomputed, fresh,
            "records of schema {version} under the current key must be recomputed"
        );
    }

    // Seed the old-format entry where a pre-upgrade `hug` would have left it.
    let project_dir = scratch.temp.path().join(&relative_project);
    let old_entry = entry_path(&project_dir, &old_key);
    write_json(&old_entry, &stale);

    // Run 1 misses: corrected records, not the seeded ones.
    let run1 = json_of(&scratch.stdout(&ARGS));
    assert!(
        !names(&run1).iter().any(|name| name == STALE_NAME),
        "TypeScript: run 1 served the seeded old-format entry"
    );
    assert_eq!(
        symbol_facts(&run1),
        symbol_facts(&fresh),
        "TypeScript: run 1 must match an uncached run (kind, visibility, is_static, container_kind)"
    );
    let make = symbol_facts(&run1)
        .into_iter()
        .find(|fact| fact.0 == "privateStaticMethod")
        .expect("privateStaticMethod");
    assert_eq!(
        (&make.3, &make.4),
        (&Value::from("Private"), &Value::Bool(true)),
        "TypeScript `privateStaticMethod`: visibility and is_static after the upgrade"
    );
    let new_entry = entry_path(&project_dir, &new_key);
    assert!(new_entry.is_file(), "run 1 must write the entry under the new key");
    assert!(old_entry.is_file(), "the old entry is left alone, only never read");

    // Run 2 hits: alter run 1's entry and observe the alteration.
    let mut written = read_json(&new_entry);
    rename_record(&mut written, "exportedFunction", RENAMED_NAME);
    write_json(&new_entry, &written);

    let run2 = json_of(&scratch.stdout(&ARGS));
    let run2_names = names(&run2);
    assert!(
        run2_names.iter().any(|name| name == RENAMED_NAME)
            && !run2_names.iter().any(|name| name == "exportedFunction"),
        "TypeScript: run 2 did not read the persistent entry (names: {run2_names:?})"
    );
    let unchanged = |json: &Value| {
        symbol_facts(json)
            .into_iter()
            .filter(|fact| fact.0 != RENAMED_NAME && fact.0 != "exportedFunction")
            .collect::<Vec<_>>()
    };
    assert_eq!(
        unchanged(&run2),
        unchanged(&run1),
        "TypeScript: records served from the warm entry must keep run 1's kind, visibility, is_static and container_kind"
    );
}

/// One earlier extractor revision whose entry must not be served: a file,
/// the value a repair changed (`probe` reads it from `hug --json-full` output),
/// and how that revision's snapshot differs from the current one.
struct StaleRevision {
    revision: &'static str,
    key: OldKey,
    file: &'static str,
    language: &'static str,
    source: String,
    /// A record renamed in the current-key entry to prove run 2 read it.
    sentinel: &'static str,
    probe: fn(&Value) -> Value,
    repaired: Value,
    stale: Value,
    revert: fn(&mut Value),
}

/// Revisions whose pre-repair value contradicts the stored evidence it is
/// derived from, so the reader refuses that payload under any key: revision
/// 5's `is_recursive: No` beside the `Limited` region every Python callable
/// has, and revision 21's callable with no `body_span` beside the coverage
/// region only a callable with a body owns, and revision 46's comment
/// in place of the source import original, and revision 48's lost grouped
/// binding without a complete producer-inventory witness.
const CONTENT_REJECTED: &[&str] = &["5", "21", "46", "48"];

/// How an earlier revision's entry key is formed.
enum OldKey {
    /// The analyzer-fingerprint key of revisions 1 to 5, whose records
    /// carried `schema` (`3.0` through revision 4, `4.0` from revision 5).
    Analyzer { schema: &'static str },
    /// The computed `SymbolCache` key (revision 6 on): it digests the grammar
    /// and query text, so it cannot be hard-coded; [`computed_revision_key`]
    /// rebuilds it with only the revision changed.
    Computed,
}

/// The `SymbolCache` key of `file_path` (as an entry records it) when the
/// extractor revision is `revision`; every other input is the shipped one.
fn computed_revision_key(revision: &str, file_path: &str) -> String {
    let source = SourceFile::read(Path::new(file_path)).expect("read scanned file");
    let language = ProgrammingLanguage::from_path(Path::new(file_path)).expect("language");
    let mut fingerprint = FingerprintInputs::shipped(language, source.path())
        .expect("fingerprint inputs")
        .records_fingerprint();
    fingerprint.tree_hugger_version = format!("{}+rev.{revision}", env!("CARGO_PKG_VERSION"));
    canonical_digest(&[
        "symbols",
        &file_path.replace('\\', "/"),
        source.hash(),
        "detected",
        &fingerprint.digest(),
    ])
}

/// The persistent-cache key `hug` used at an earlier extractor `revision`
/// whose records carried `schema` (`3.0` through revision 4, `4.0` from
/// revision 5), hard-coded rather than computed from the current revision:
/// the same `FileCacheKey::stable_key` formula as [`historical_key`] with a
/// `query_fingerprint` of `locals+imports+references;extractor-revision=<N>`.
fn revision_key(
    schema: &str,
    revision: &str,
    file_path: &str,
    source: &str,
    language: &str,
) -> String {
    let options = "experimental_semantics=false;deny=;warn=;allow=;strict=false;external_adapters=false";
    let analyzer = blake3_hash(&format!(
        "{schema}|{}|{language}|locals+imports+references;extractor-revision={revision}|{options}",
        env!("CARGO_PKG_VERSION")
    ));
    let file_hash = format!("{:x}", xx_hash(source));
    blake3_hash(&format!(
        "{}|{language}|{file_hash}|{analyzer}",
        file_path.replace('\\', "/")
    ))
}

/// The record called `name` in a snapshot; panics unless there is exactly one.
fn record_named<'a>(snapshot: &'a mut Value, name: &str) -> &'a mut Value {
    let mut matches: Vec<_> = snapshot["records"]
        .as_array_mut()
        .expect("entry records")
        .iter_mut()
        .filter(|record| record["identity"]["name"] == name)
        .collect();
    assert_eq!(matches.len(), 1, "snapshot must hold one `{name}` record");
    matches.pop().expect("one record")
}

/// Seeds `case`'s pre-repair snapshot under its revision's key and asserts it
/// is not served. A current-key control distinguishes revision invalidation
/// from independent content rejection; the rewritten entry is served on run 2.
fn assert_stale_revision_is_not_served(case: &StaleRevision) {
    let args = ["--json-full", "symbols", case.file];
    let label = format!("revision {} `{}`", case.revision, case.file);
    let scratch = Scratch::new(&[(case.file, case.source.as_str())]);

    let discovery = TempDir::new().expect("discovery temp");
    let fresh = json_of(&stdout_of(scratch.hug_with_temp(discovery.path(), &args), &args));
    assert_eq!((case.probe)(&fresh), case.repaired, "{label}: an uncached run reports the repaired value");
    let discovered_entry = only_entry(discovery.path());
    let current_snapshot = read_json(&discovered_entry);
    let file_path = current_snapshot["file"]
        .as_str()
        .expect("entry file")
        .to_string();
    let relative_project = discovered_entry
        .parent()
        .and_then(Path::parent)
        .and_then(|dir| dir.strip_prefix(discovery.path()).ok())
        .expect("entry under <temp>/tree-hugger/cache/<project_id>/<shard>/")
        .to_path_buf();
    let current_key = discovered_entry
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("entry key")
        .to_string();
    let old_key = match case.key {
        OldKey::Analyzer { schema } => revision_key(schema, case.revision, &file_path, &case.source, case.language),
        OldKey::Computed => {
            // The formula must rebuild the key run 1 wrote before it can
            // stand for an earlier revision's.
            assert_eq!(
                computed_revision_key(EXTRACTOR_REVISION, &file_path),
                current_key,
                "{label}: computed key formula"
            );
            computed_revision_key(case.revision, &file_path)
        }
    };
    assert_ne!(old_key, current_key, "{label}: the revision must differ from the current one");

    let mut stale = current_snapshot.clone();
    (case.revert)(&mut stale);

    // Control: the stale payload is servable; under the current key the
    // pre-repair value reaches the output. A payload the reader now refuses
    // on content is recomputed there instead.
    let control = TempDir::new().expect("control temp");
    write_json(&entry_path(&control.path().join(&relative_project), &current_key), &stale);
    let served = json_of(&stdout_of(scratch.hug_with_temp(control.path(), &args), &args));
    let control_value = if CONTENT_REJECTED.contains(&case.revision) { &case.repaired } else { &case.stale };
    assert_eq!(
        (case.probe)(&served),
        *control_value,
        "control: {label} payload under the current key must be served unless refused on content"
    );

    let project_dir = scratch.temp.path().join(&relative_project);
    write_json(&entry_path(&project_dir, &old_key), &stale);

    // Run 1 misses the stale entry and reports the repaired value.
    let run1 = json_of(&scratch.stdout(&args));
    assert_eq!((case.probe)(&run1), case.repaired, "{label}: run 1 served the stale entry");
    assert_eq!(
        symbol_facts(&run1),
        symbol_facts(&fresh),
        "{label}: run 1 must match an uncached run (kind, visibility, is_static, container_kind)"
    );
    let new_entry = entry_path(&project_dir, &current_key);
    assert!(new_entry.is_file(), "{label}: run 1 must write the entry under the current key");

    // Run 2 hits the entry run 1 wrote: a renamed record proves it was read.
    let mut written = read_json(&new_entry);
    rename_record(&mut written, case.sentinel, RENAMED_NAME);
    write_json(&new_entry, &written);
    let run2 = json_of(&scratch.stdout(&args));
    let run2_names = names(&run2);
    assert!(
        run2_names.iter().any(|name| name == RENAMED_NAME)
            && !run2_names.iter().any(|name| name == case.sentinel),
        "{label}: run 2 did not read the current-key entry (names: {run2_names:?})"
    );
    assert_eq!((case.probe)(&run2), case.repaired, "{label}: the warm entry keeps run 1's value");
}

const DECOY_METHOD: &str = "decoy_decorated_method";

/// The public `is_static` of the single symbol called `DECOY_METHOD`.
fn decoy_is_static(json: &Value) -> Value {
    let facts: Vec<_> = symbol_facts(json)
        .into_iter()
        .filter(|fact| fact.0 == DECOY_METHOD)
        .collect();
    assert_eq!(facts.len(), 1, "expected one `{DECOY_METHOD}` symbol, found {facts:?}");
    facts[0].4.clone()
}

/// Revision 1 reported a method whose decorator only resembles
/// `staticmethod` as static; the repaired reader does not.
#[test]
fn entry_from_extractor_revision_one_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "1",
        key: OldKey::Analyzer { schema: "3.0" },
        file: "python.py",
        language: "python",
        source: format!(
            "{PYTHON_FIXTURE}\n\nclass DecoratorDecoy:\n    @notstaticmethod\n    def {DECOY_METHOD}(self) -> None:\n        pass\n"
        ),
        sentinel: "module_function",
        probe: decoy_is_static,
        repaired: Value::Bool(false),
        stale: Value::Bool(true),
        revert: |snapshot| {
            record_named(snapshot, DECOY_METHOD)["modifiers"] = serde_json::json!([{ "type": "Static" }]);
        },
    });
}

const SCOPE_LEAK_SOURCE: &str = "static int scope_leak(int);\n\
                                 namespace unrelated { int scope_leak(int x) { return x; } }\n\
                                 int sentinel_function(int value) { return value; }\n";

/// The public visibility of `scope_leak` in `namespace unrelated`.
fn nested_scope_leak_visibility(json: &Value) -> Value {
    let facts: Vec<_> = symbol_facts(json)
        .into_iter()
        .filter(|fact| fact.0 == "scope_leak" && fact.5 == "Namespace")
        .collect();
    assert_eq!(facts.len(), 1, "expected one nested `scope_leak` symbol, found {facts:?}");
    facts[0].3.clone()
}

/// Revision 2 attached the file-scope `static` prototype to the same-named
/// function in `namespace unrelated`, reporting it module-private.
#[test]
fn entry_from_extractor_revision_two_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "2",
        key: OldKey::Analyzer { schema: "3.0" },
        file: "linkage.cpp",
        language: "cpp",
        source: SCOPE_LEAK_SOURCE.to_string(),
        sentinel: "sentinel_function",
        probe: nested_scope_leak_visibility,
        repaired: Value::from("Public"),
        stale: Value::from("ModulePrivate"),
        revert: |snapshot| {
            let nested = snapshot["records"]
                .as_array_mut()
                .expect("entry records")
                .iter_mut()
                .find(|record| {
                    record["identity"]["name"] == "scope_leak"
                        && record["source"]["declaration_span"]["start"]["line"] == 2
                })
                .expect("snapshot has the nested `scope_leak` record");
            nested["visibility"]["level"] = Value::from("ModulePrivate");
        },
    });
}

const TRAILING_RETURN_SOURCE: &str = "template<class T> static auto trailing_differs(T) -> int;\n\
                                      template<class T> auto trailing_differs(T value) -> long { return 0; }\n\
                                      int sentinel_function(int value) { return value; }\n";

/// The public visibility of the later `trailing_differs` template.
fn trailing_differs_definition_visibility(json: &Value) -> Value {
    let facts: Vec<_> = symbol_facts(json)
        .into_iter()
        .filter(|fact| fact.0 == "trailing_differs")
        .collect();
    assert_eq!(facts.len(), 2, "expected two `trailing_differs` symbols, found {facts:?}");
    facts
        .into_iter()
        .max_by_key(|fact| fact.1.as_u64())
        .map(|fact| fact.3)
        .expect("one later declaration")
}

/// Revision 3 left a template's trailing return type out of its identity,
/// attaching the `static` prototype returning `int` to the different template
/// returning `long` and reporting it module-private.
#[test]
fn entry_from_extractor_revision_three_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "3",
        key: OldKey::Analyzer { schema: "3.0" },
        file: "templates.cpp",
        language: "cpp",
        source: TRAILING_RETURN_SOURCE.to_string(),
        sentinel: "sentinel_function",
        probe: trailing_differs_definition_visibility,
        repaired: Value::from("Public"),
        stale: Value::from("ModulePrivate"),
        revert: |snapshot| {
            let definition = snapshot["records"]
                .as_array_mut()
                .expect("entry records")
                .iter_mut()
                .filter(|record| record["identity"]["name"] == "trailing_differs")
                .max_by_key(|record| record["source"]["declaration_span"]["start_byte"].as_u64())
                .expect("snapshot has a `trailing_differs` record");
            definition["visibility"]["level"] = Value::from("ModulePrivate");
        },
    });
}

/// The `identity.keyword` of the `module_function` record in a `hug --json-full`
/// report (`null` when absent).
fn module_function_keyword(json: &Value) -> Value {
    let records: Vec<&Value> = json["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == "module_function")
        .collect();
    assert_eq!(records.len(), 1, "expected one `module_function` record");
    records[0]["identity"].get("keyword").cloned().unwrap_or(Value::Null)
}

/// Revision 4 (schema 3.0) records carried no declaring keyword or display
/// header; revision 5 (schema 4.0) adds both.
#[test]
fn entry_from_extractor_revision_four_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "4",
        key: OldKey::Analyzer { schema: "3.0" },
        file: "python.py",
        language: "python",
        source: PYTHON_FIXTURE.to_string(),
        sentinel: "MODULE_CONSTANT",
        probe: module_function_keyword,
        repaired: Value::from("def"),
        stale: Value::Null,
        revert: |snapshot| {
            let identity = record_named(snapshot, "module_function")["identity"]
                .as_object_mut()
                .expect("record identity");
            identity.remove("keyword");
        },
    });
}

/// The `semantics.is_recursive` of the `module_function` record in a
/// `hug --json-full` report (`null` when absent).
fn module_function_is_recursive(json: &Value) -> Value {
    let records: Vec<&Value> = json["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == "module_function")
        .collect();
    assert_eq!(records.len(), 1, "expected one `module_function` record");
    records[0]["semantics"].get("is_recursive").cloned().unwrap_or(Value::Null)
}

/// Revision 5 guessed `is_recursive` (`No` for a function that does not call
/// its own name); revision 6 reports `Unknown`, since name-only binding
/// proves neither.
#[test]
fn entry_from_extractor_revision_five_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "5",
        key: OldKey::Analyzer { schema: "4.0" },
        file: "python.py",
        language: "python",
        source: PYTHON_FIXTURE.to_string(),
        sentinel: "MODULE_CONSTANT",
        probe: module_function_is_recursive,
        repaired: Value::from("Unknown"),
        stale: Value::from("No"),
        revert: |snapshot| {
            record_named(snapshot, "module_function")["semantics"]["is_recursive"] = Value::from("No");
        },
    });
}

/// The `identity.qualified_name` of the `instance_method` record in a
/// `hug --json-full` report (`null` when absent).
fn instance_method_qualified_name(json: &Value) -> Value {
    let records: Vec<&Value> = json["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == "instance_method")
        .collect();
    assert_eq!(records.len(), 1, "expected one `instance_method` record");
    records[0]["identity"].get("qualified_name").cloned().unwrap_or(Value::Null)
}

/// Revision 6 spelled every qualified name `container::name`, whatever the
/// language; revision 7 builds the in-file name with the language's own
/// separators (`PlainWidget.instance_method` in Python) and derives IDs from
/// it. The report shows either behind the scan-root prefix `python.py#`.
#[test]
fn entry_from_extractor_revision_six_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "6",
        key: OldKey::Computed,
        file: "python.py",
        language: "python",
        source: PYTHON_FIXTURE.to_string(),
        sentinel: "MODULE_CONSTANT",
        probe: instance_method_qualified_name,
        repaired: Value::from("python.py#PlainWidget.instance_method"),
        stale: Value::from("python.py#PlainWidget::instance_method"),
        revert: |snapshot| {
            set_qualified_name(snapshot, "instance_method", "PlainWidget::instance_method");
        },
    });
}

/// `documented_pair`'s attached documentation in a `hug --json-full` report.
fn documented_pair_doc(output: &Value) -> Value {
    let records: Vec<&Value> = output["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == "documented_pair")
        .collect();
    assert_eq!(records.len(), 1, "expected one `documented_pair` record");
    records[0]["docs"].get("raw_doc").cloned().unwrap_or(Value::Null)
}

/// Revision 7 read a Python declaration's docs from the statements before
/// it (so a function lost its own docstring and a method took its class's);
/// revision 8 reads the docstring in the body, and attaches only doc-form
/// comments elsewhere.
#[test]
fn entry_from_extractor_revision_seven_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "7",
        key: OldKey::Computed,
        file: "python.py",
        language: "python",
        source: PYTHON_FIXTURE.to_string(),
        sentinel: "MODULE_CONSTANT",
        probe: documented_pair_doc,
        repaired: Value::from("Documented, with two parameters."),
        stale: Value::Null,
        revert: |snapshot| {
            record_named(snapshot, "documented_pair")["docs"]
                .as_object_mut()
                .expect("docs object")
                .remove("raw_doc");
        },
    });
}

/// `documented_pair`'s declared return type in a `hug --json-full` report
/// (`null` when it declares none).
fn documented_pair_return_type(output: &Value) -> Value {
    let records: Vec<&Value> = output["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == "documented_pair")
        .collect();
    assert_eq!(records.len(), 1, "expected one `documented_pair` record");
    records[0]["kind_data"]["data"]["return_info"]
        .get("type_text")
        .cloned()
        .unwrap_or(Value::Null)
}

/// Revision 8 gave a Rust function without `->` the inferred return type
/// `()` (and a Go method its parameter list); revision 9 reports only a
/// declared return type, so `--returns` never matches a type the source does
/// not state.
#[test]
fn entry_from_extractor_revision_eight_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "8",
        key: OldKey::Computed,
        file: "rust.rs",
        language: "rust",
        source: RUST_FIXTURE.to_string(),
        sentinel: "public_function",
        probe: documented_pair_return_type,
        repaired: Value::Null,
        stale: Value::from("()"),
        revert: |snapshot| {
            record_named(snapshot, "documented_pair")["kind_data"]["data"]["return_info"]["type_text"] =
                Value::from("()");
        },
    });
}

const PATTERNED_RUST: &str = "\npub fn patterned((a, b): (u32, u32), _: String) {}\n";

/// The names of `patterned`'s parameters in a `hug --json-full` report.
fn patterned_parameter_names(output: &Value) -> Value {
    let records: Vec<&Value> = output["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == "patterned")
        .collect();
    assert_eq!(records.len(), 1, "expected one `patterned` record");
    // An empty list is not serialized.
    let names: Vec<Value> = records[0]["kind_data"]["data"]["parameters"]
        .as_array()
        .map(|parameters| parameters.iter().map(|parameter| parameter["name"].clone()).collect())
        .unwrap_or_default();
    Value::from(names)
}

/// Revision 9 listed only parameters bound by an identifier, so a Rust
/// tuple or `_` pattern vanished from the signature and from the count;
/// revision 10 lists each parameter under its pattern as written.
#[test]
fn entry_from_extractor_revision_nine_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "9",
        key: OldKey::Computed,
        file: "rust.rs",
        language: "rust",
        source: format!("{RUST_FIXTURE}{PATTERNED_RUST}"),
        sentinel: "public_function",
        probe: patterned_parameter_names,
        repaired: Value::from(vec!["(a, b)", "_"]),
        stale: Value::from(Vec::<Value>::new()),
        revert: |snapshot| {
            record_named(snapshot, "patterned")["kind_data"]["data"]["parameters"] = Value::from(Vec::<Value>::new());
        },
    });
}

const FOREIGN_OWNER_RUST: &str =
    "\npub struct ForeignProbeOwner;\nimpl foreign::ForeignProbeOwner { pub fn foreign_probe(&self) {} }\n";

/// The in-file qualified name of `foreign_probe`'s parent in a
/// `hug --json-full` report, `null` without one.
fn foreign_probe_parent(output: &Value) -> Value {
    let records: Vec<&Value> = output["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == "foreign_probe")
        .collect();
    assert_eq!(records.len(), 1, "expected one `foreign_probe` record");
    records[0]["relations"]["parent"]["qualified_name"].clone()
}

/// Revision 10 made the sole local `ForeignProbeOwner` the owner of a method
/// of `impl foreign::ForeignProbeOwner`; revision 11 links no same-file owner
/// that disagrees with the method's explicit qualification.
#[test]
fn entry_from_extractor_revision_ten_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "10",
        key: OldKey::Computed,
        file: "rust.rs",
        language: "rust",
        source: format!("{RUST_FIXTURE}{FOREIGN_OWNER_RUST}"),
        sentinel: "public_function",
        probe: foreign_probe_parent,
        repaired: Value::Null,
        stale: Value::from("ForeignProbeOwner"),
        revert: |snapshot| {
            let owner = record_named(snapshot, "ForeignProbeOwner");
            let owner_ref = serde_json::json!({
                "id": owner["id"].clone(),
                "name": "ForeignProbeOwner",
                "qualified_name": "ForeignProbeOwner",
            });
            let probe = record_named(snapshot, "foreign_probe");
            probe["identity"]["container_kind"] = Value::from("Type");
            probe["relations"]["parent"] = owner_ref.clone();
            probe["relations"]["container"] = owner_ref;
        },
    });
}

/// The public `is_deprecated` of `fence_decoy` and `fence_real` in a
/// `hug --json-full` report.
fn fenced_deprecation(output: &Value) -> Value {
    let flag = |name: &str| -> Value {
        let records: Vec<&Value> = output["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
            .filter(|record| record["identity"]["name"] == name)
            .collect();
        assert_eq!(records.len(), 1, "expected one `{name}` record");
        records[0]["visibility"]["is_deprecated"].clone()
    };
    Value::from(vec![flag("fence_decoy"), flag("fence_real")])
}

/// Revision 11 toggled its code-fence state on any fence-looking doc line,
/// so a tag inside a four-backtick example deprecated `fence_decoy` and a
/// tag after the example's close was missed on `fence_real`; revision 12
/// closes a fence only on a matching run.
#[test]
fn entry_from_extractor_revision_eleven_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "11",
        key: OldKey::Computed,
        file: FILE,
        language: "typescript",
        source: format!("export function exportedFunction() {{}}\n{}", crate::filters::FENCED_DEPRECATION_TS),
        sentinel: "exportedFunction",
        probe: fenced_deprecation,
        repaired: Value::from(vec![false, true]),
        stale: Value::from(vec![true, false]),
        revert: |snapshot| {
            record_named(snapshot, "fence_decoy")["visibility"]["is_deprecated"] = Value::from(true);
            record_named(snapshot, "fence_real")["visibility"]["is_deprecated"] = Value::from(false);
        },
    });
}

/// The public `is_deprecated` of `ReviewLiteral` and `has_docs` of
/// `ReviewBlockStar` in a `hug --json-full` report.
fn go_doc_star_facts(output: &Value) -> Value {
    let record = |name: &str| -> Value {
        let records: Vec<&Value> = output["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
            .filter(|record| record["identity"]["name"] == name)
            .collect();
        assert_eq!(records.len(), 1, "expected one `{name}` record");
        records[0].clone()
    };
    Value::from(vec![
        record("ReviewLiteral")["visibility"]["is_deprecated"].clone(),
        record("ReviewBlockStar")["metrics"]["has_docs"].clone(),
    ])
}

/// Revision 12 cleaned Go comments with Javadoc decoration rules, so a
/// starred `*Deprecated:` deprecated `ReviewLiteral` and the star-only
/// `/* * */` left `ReviewBlockStar` undocumented; revision 13 keeps every
/// `*` after `/*` as text.
#[test]
fn entry_from_extractor_revision_twelve_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "12",
        key: OldKey::Computed,
        file: "stars.go",
        language: "go",
        source: crate::filters::GO_DOC_STARS.to_string(),
        sentinel: "ReviewLineStar",
        probe: go_doc_star_facts,
        repaired: Value::from(vec![false, true]),
        stale: Value::from(vec![true, false]),
        revert: |snapshot| {
            record_named(snapshot, "ReviewLiteral")["visibility"]["is_deprecated"] = Value::from(true);
            record_named(snapshot, "ReviewBlockStar")["metrics"]["has_docs"] = Value::from(false);
        },
    });
}

/// `has_docs` of `ReviewSingleVar` and `ReviewGenerate`, and
/// `is_deprecated` of `ReviewMarkedConst` and `ReviewBeforeMarker`, in a
/// `hug --json-full` report.
fn go_wrapped_doc_facts(output: &Value) -> Value {
    Value::from(vec![
        go_record(output, "ReviewSingleVar")["metrics"]["has_docs"].clone(),
        go_record(output, "ReviewMarkedConst")["visibility"]["is_deprecated"].clone(),
        go_record(output, "ReviewGenerate")["metrics"]["has_docs"].clone(),
        go_record(output, "ReviewBeforeMarker")["visibility"]["is_deprecated"].clone(),
    ])
}

fn go_record(output: &Value, name: &str) -> Value {
    let records: Vec<&Value> = output["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("index symbols"))
        .filter(|record| record["identity"]["name"] == name)
        .collect();
    assert_eq!(records.len(), 1, "expected one `{name}` record");
    records[0].clone()
}

/// Revision 14 read a Go `var` or `const` spec's doc only before the spec,
/// so a standalone declaration's doc and its `Deprecated:` paragraph were
/// missed, and kept `//go:` directives as doc text, so a directive alone
/// documented its function and one before `Deprecated:` hid the marker;
/// revision 15 reads the enclosing declaration's doc and drops directives.
#[test]
fn entry_from_extractor_revision_fourteen_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "14",
        key: OldKey::Computed,
        file: "wrapped.go",
        language: "go",
        source: format!("{}\n{}", crate::filters::GO_WRAPPED_DOCS, crate::filters::GO_DIRECTIVE_DECLARATIONS),
        sentinel: "ReviewSingleConst",
        probe: go_wrapped_doc_facts,
        repaired: Value::from(vec![true, true, false, true]),
        stale: Value::from(vec![false, false, true, false]),
        revert: |snapshot| {
            record_named(snapshot, "ReviewSingleVar")["metrics"]["has_docs"] = Value::from(false);
            record_named(snapshot, "ReviewMarkedConst")["visibility"]["is_deprecated"] = Value::from(false);
            record_named(snapshot, "ReviewGenerate")["metrics"]["has_docs"] = Value::from(true);
            record_named(snapshot, "ReviewBeforeMarker")["visibility"]["is_deprecated"] = Value::from(false);
        },
    });
}

const UPGRADE_ENUM_SCALA: &str = "enum UpgradeShape {\n  case Red\n  case Point(x: Int, y: String)\n}\n";

/// The `[name, type]` payload fields of `UpgradeShape.Point` in its enum's
/// variant list, in a `hug --json-full` report.
fn upgrade_enum_payload(output: &Value) -> Value {
    let variants = go_record(output, "UpgradeShape")["kind_data"]["data"]["variants"].clone();
    let point: Vec<&Value> =
        variants.as_array().expect("variants").iter().filter(|variant| variant["name"] == "Point").collect();
    assert_eq!(point.len(), 1, "expected one `Point` variant");
    Value::from(
        point[0]["struct_fields"]
            .as_array()
            .map(|fields| fields.iter().map(|field| serde_json::json!([field["name"], field["type_text"]])).collect::<Vec<_>>())
            .unwrap_or_default(),
    )
}

/// Revision 15 listed a Scala enum case declared with class parameters as a
/// unit variant of its enum, dropping the declared payload; revision 16
/// lists the parameters with their written types.
#[test]
fn entry_from_extractor_revision_fifteen_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "15",
        key: OldKey::Computed,
        file: "shape.scala",
        language: "scala",
        source: UPGRADE_ENUM_SCALA.to_string(),
        sentinel: "Red",
        probe: upgrade_enum_payload,
        repaired: serde_json::json!([["x", "Int"], ["y", "String"]]),
        stale: serde_json::json!([]),
        revert: |snapshot| {
            let variants = record_named(snapshot, "UpgradeShape")["kind_data"]["data"]["variants"]
                .as_array_mut()
                .expect("entry variants");
            for variant in variants.iter_mut().filter(|variant| variant["name"] == "Point") {
                variant.as_object_mut().expect("variant object").remove("struct_fields");
            }
        },
    });
}

/// `has_docs` and `is_deprecated` of `ReviewEmpty`, `ReviewDirective` and
/// `ReviewAbsent`, in a `hug --json-full` report.
fn go_empty_spec_doc_facts(output: &Value) -> Value {
    Value::from(
        ["ReviewEmpty", "ReviewDirective", "ReviewAbsent"]
            .into_iter()
            .flat_map(|name| {
                let record = go_record(output, name);
                [record["metrics"]["has_docs"].clone(), record["visibility"]["is_deprecated"].clone()]
            })
            .collect::<Vec<_>>(),
    )
}

/// Revision 16 let a Go spec whose own comment group reads as no text (a
/// blank `//`, a directive only) inherit its group's doc and `Deprecated:`
/// paragraph; revision 17 leaves it undocumented, and only a spec with no
/// attached group shares the group's doc.
#[test]
fn entry_from_extractor_revision_sixteen_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "16",
        key: OldKey::Computed,
        file: "specs.go",
        language: "go",
        source: format!("package specs\n\n{}\ntype ReviewSentinel int\n", crate::filters::GO_EMPTY_SPEC_DOCS),
        sentinel: "ReviewSentinel",
        probe: go_empty_spec_doc_facts,
        repaired: Value::from(vec![false, false, false, false, true, true]),
        stale: Value::from(vec![true, true, true, true, true, true]),
        revert: |snapshot| {
            for name in ["ReviewEmpty", "ReviewDirective"] {
                let record = record_named(snapshot, name);
                record["metrics"]["has_docs"] = Value::from(true);
                record["visibility"]["is_deprecated"] = Value::from(true);
            }
        },
    });
}

const UPGRADE_GENERICS_RS: &str = "pub struct UpgradeBag<T: Clone = String> {\n    value: T,\n}\n";

/// `UpgradeBag`'s generic parameters as `[name, constraints, default,
/// position]`, in a `hug --json-full` report.
fn upgrade_generic_params(output: &Value) -> Value {
    let params = go_record(output, "UpgradeBag")["type_info"]["generic_params"].clone();
    Value::from(
        params
            .as_array()
            .map(|params| {
                params
                    .iter()
                    .map(|param| serde_json::json!([param["name"], param["constraints"], param["default"], param["position"]]))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    )
}

/// Revision 18 left `type_info.generic_params` empty; revision 19 lists each
/// written parameter with its bounds, default and position.
#[test]
fn entry_from_extractor_revision_eighteen_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "18",
        key: OldKey::Computed,
        file: "bag.rs",
        language: "rust",
        source: UPGRADE_GENERICS_RS.to_string(),
        sentinel: "value",
        probe: upgrade_generic_params,
        repaired: serde_json::json!([["T", ["Clone"], "String", 0]]),
        stale: serde_json::json!([]),
        revert: |snapshot| {
            record_named(snapshot, "UpgradeBag")["type_info"]
                .as_object_mut()
                .expect("type_info object")
                .remove("generic_params");
        },
    });
}

const UPGRADE_MODIFIERS_RS: &str =
    "#[inline]\n#[must_use = \"keep\"]\npub const unsafe fn upgrade_written() -> u8 {\n    0\n}\n\npub fn upgrade_sentinel() {}\n";

/// `upgrade_written`'s modifiers and attributes as `[modifiers, [[name,
/// value], …]]`, in a `hug --json-full` report.
fn upgrade_written_facts(output: &Value) -> Value {
    let record = go_record(output, "upgrade_written");
    let attributes: Vec<Value> = record["attributes"]
        .as_array()
        .map(|attributes| {
            attributes
                .iter()
                .map(|attribute| serde_json::json!([attribute["name"], attribute["value"]]))
                .collect()
        })
        .unwrap_or_default();
    serde_json::json!([record["modifiers"].as_array().cloned().unwrap_or_default(), attributes])
}

/// Revision 19 wrote no `attributes` and only `static`/`async` modifiers;
/// revision 20 records every written attribute and the `const`/`unsafe`
/// tokens.
#[test]
fn entry_from_extractor_revision_nineteen_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "19",
        key: OldKey::Computed,
        file: "written.rs",
        language: "rust",
        source: UPGRADE_MODIFIERS_RS.to_string(),
        sentinel: "upgrade_sentinel",
        probe: upgrade_written_facts,
        repaired: serde_json::json!([
            [{ "type": "Const" }, { "type": "Unsafe" }],
            [["inline", null], ["must_use", "\"keep\""]]
        ]),
        stale: serde_json::json!([[], []]),
        revert: |snapshot| {
            let record = record_named(snapshot, "upgrade_written")
                .as_object_mut()
                .expect("record object");
            record.remove("modifiers");
            record.remove("attributes");
        },
    });
}

const UPGRADE_BLOCK_PACKAGE_SCALA: &str =
    "package UpgradeLeft { class UpgradeW { def probe(): Unit = () } }\nclass UpgradeSentinel\n";

/// `[qualified_name, is_namespace_rooted]` of `UpgradeW` and its `probe`, in
/// a `hug --json-full` report.
fn upgrade_block_package_names(output: &Value) -> Value {
    let facts: Vec<Value> = ["UpgradeW", "probe"]
        .into_iter()
        .map(|name| {
            let identity = &go_record(output, name)["identity"];
            serde_json::json!([identity["qualified_name"], identity["is_namespace_rooted"]])
        })
        .collect();
    Value::from(facts)
}

/// Revision 20 dropped a Scala block package (`package p { … }`) from the
/// names of the declarations in its body; revision 21 keeps it as their
/// rooting segment.
#[test]
fn entry_from_extractor_revision_twenty_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "20",
        key: OldKey::Computed,
        file: "blocks.scala",
        language: "scala",
        source: UPGRADE_BLOCK_PACKAGE_SCALA.to_string(),
        sentinel: "UpgradeSentinel",
        probe: upgrade_block_package_names,
        repaired: serde_json::json!([["UpgradeLeft.UpgradeW", true], ["UpgradeLeft.UpgradeW.probe", true]]),
        stale: serde_json::json!([["blocks.scala#UpgradeW", false], ["blocks.scala#UpgradeW.probe", false]]),
        revert: |snapshot| {
            for (name, in_file) in [("UpgradeW", "UpgradeW"), ("probe", "UpgradeW.probe")] {
                set_qualified_name(snapshot, name, in_file);
                record_named(snapshot, name)["identity"]["is_namespace_rooted"] = Value::from(false);
            }
        },
    });
}

const UPGRADE_BODY_C: &str = "int upgrade_add(int a)\n{\n  return a;\n}\nint upgrade_sentinel(void) { return 0; }\n";

/// `[body_span.start.line, metrics.lines]` of `upgrade_add`, in a
/// `hug --json-full` report.
fn upgrade_body_lines(output: &Value) -> Value {
    let record = go_record(output, "upgrade_add");
    serde_json::json!([record["source"]["body_span"]["start"]["line"], record["metrics"]["lines"]])
}

/// Revision 21 stored no body span for a C/C++ function (whose anchor is its
/// declarator) and never counted `metrics.lines`; revision 22 reads the
/// definition's body and counts it.
#[test]
fn entry_from_extractor_revision_twenty_one_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "21",
        key: OldKey::Computed,
        file: "body.c",
        language: "c",
        source: UPGRADE_BODY_C.to_string(),
        sentinel: "upgrade_sentinel",
        probe: upgrade_body_lines,
        repaired: serde_json::json!([2, 3]),
        stale: serde_json::json!([null, null]),
        revert: |snapshot| {
            let record = record_named(snapshot, "upgrade_add");
            record["source"].as_object_mut().expect("source object").remove("body_span");
            record["metrics"]["lines"] = Value::Null;
        },
    });
}

const UPGRADE_BASES_PY: &str = "class UpgradeBase:\n    pass\n\nclass UpgradeChild(UpgradeBase, other.Mixin):\n    pass\n\n\
     class UpgradeSentinel:\n    pass\n";

/// `UpgradeChild`'s `relations.extends` as `[name, has id]` pairs, in a
/// `hug --json-full` report.
fn upgrade_extends(output: &Value) -> Value {
    let record = go_record(output, "UpgradeChild");
    let extends = record["relations"]["extends"].as_array().cloned().unwrap_or_default();
    Value::from(
        extends
            .iter()
            .map(|target| serde_json::json!([target["name"], target.get("id").is_some()]))
            .collect::<Vec<_>>(),
    )
}

/// Revision 22 read no declared bases; revision 23 records each written
/// base, with a same-file ID only where the name resolves.
#[test]
fn entry_from_extractor_revision_twenty_two_is_not_served_after_the_relations() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "22",
        key: OldKey::Computed,
        file: "bases.py",
        language: "python",
        source: UPGRADE_BASES_PY.to_string(),
        sentinel: "UpgradeSentinel",
        probe: upgrade_extends,
        repaired: serde_json::json!([["UpgradeBase", true], ["other.Mixin", false]]),
        stale: serde_json::json!([]),
        revert: |snapshot| {
            let record = record_named(snapshot, "UpgradeChild");
            record["relations"].as_object_mut().expect("relations object").remove("extends");
        },
    });
}

const UPGRADE_SHADOWED_BASE_PY: &str = "class Outer:\n    class Base:\n        pass\n\n\
     def shadow(Outer):\n    class UpgradeChild(Outer.Base):\n        pass\n\n\
     class UpgradeSentinel:\n    pass\n";

/// Revision 23 resolved a written base by matching its whole qualified name
/// in any enclosing scope, so `Outer.Base` under a parameter `Outer` named
/// the file's class; revision 24 lets the parameter decide `Outer` and
/// leaves the base unresolved.
#[test]
fn entry_from_extractor_revision_twenty_three_is_not_served_after_path_meaning() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "23",
        key: OldKey::Computed,
        file: "shadowed.py",
        language: "python",
        source: UPGRADE_SHADOWED_BASE_PY.to_string(),
        sentinel: "UpgradeSentinel",
        probe: upgrade_extends,
        repaired: serde_json::json!([["Outer.Base", false]]),
        stale: serde_json::json!([["Outer.Base", true]]),
        revert: |snapshot| {
            let base = record_named(snapshot, "Base");
            let (id, qualified_name) = (base["id"].clone(), base["identity"]["qualified_name"].clone());
            let child = record_named(snapshot, "UpgradeChild");
            let target = &mut child["relations"]["extends"][0];
            target["id"] = id;
            target["qualified_name"] = qualified_name;
        },
    });
}

#[test]
fn entry_from_extractor_revision_twenty_four_is_not_served_after_python_scope_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "24",
        key: OldKey::Computed,
        file: "nested.py",
        language: "python",
        source: "class Base: pass\nclass Outer:\n    class Base: pass\n    class Middle:\n        class Leaf(Base): pass\nclass Sentinel: pass\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "Leaf")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::from("Base"),
        stale: Value::from("Outer.Base"),
        revert: |snapshot| {
            let wrong_id = snapshot["records"].as_array().expect("records").iter()
                .find(|record| record["identity"]["qualified_name"] == "Outer.Base").expect("outer base")["id"].clone();
            let target = &mut record_named(snapshot, "Leaf")["relations"]["extends"][0];
            target["id"] = wrong_id;
            target["qualified_name"] = Value::from("Outer.Base");
        },
    });
}

#[test]
fn entry_from_extractor_revision_twenty_five_is_not_served_after_mixed_base_readers() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "25",
        key: OldKey::Computed,
        file: "mixed.cs",
        language: "c_sharp",
        source: "class Child : External {}\nclass Sentinel {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "Child")["relations"]["unclassified_bases"].clone(),
        repaired: serde_json::json!([{ "name": "External" }]),
        stale: Value::Null,
        revert: |snapshot| {
            record_named(snapshot, "Child")["relations"].as_object_mut().expect("relations").remove("unclassified_bases");
        },
    });
}

const UPGRADE_PROBE_PY: &str = "\n\nclass UpgradeProbe:\n    def __init__(self, seed):\n        pass\n";

/// The stable key of `UpgradeProbe.__init__` in `deferred.py` when it reports
/// `kind`; the record's ID digests it.
fn upgrade_probe_key(kind: SymbolKindV2) -> String {
    stable_symbol_key(ProgrammingLanguage::Python, Path::new("deferred.py"), kind, "UpgradeProbe.__init__", 0)
}

/// The kind and stable key of `UpgradeProbe.__init__` in a `hug --json-full`
/// report.
fn upgrade_probe_identity(output: &Value) -> Value {
    let records: Vec<&Value> = output["indexes"][0]["symbols"]
        .as_array()
        .expect("indexes[0].symbols")
        .iter()
        .filter(|record| record["identity"]["qualified_name"] == "deferred.py#UpgradeProbe.__init__")
        .collect();
    assert_eq!(records.len(), 1, "expected one `UpgradeProbe.__init__` record");
    Value::from(vec![records[0]["kind"].clone(), records[0]["identity"]["stable_key"].clone()])
}

/// Revision 13 reported a Python `__init__` as a `Method`, so its stable key
/// and ID digested that kind; revision 14 reports a `Constructor`. The stale
/// entry carries the old kind, key and a Method-derived ID, which its
/// parameters' relations point at.
#[test]
fn entry_from_extractor_revision_thirteen_is_not_served_after_the_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "13",
        key: OldKey::Computed,
        file: "deferred.py",
        language: "python",
        source: format!("{PYTHON_FIXTURE}{UPGRADE_PROBE_PY}"),
        sentinel: "UpgradeProbe",
        probe: upgrade_probe_identity,
        repaired: Value::from(vec!["Constructor".to_string(), upgrade_probe_key(SymbolKindV2::Constructor)]),
        stale: Value::from(vec!["Method".to_string(), upgrade_probe_key(SymbolKindV2::Method)]),
        revert: |snapshot| {
            let records = snapshot["records"].as_array_mut().expect("entry records");
            let probe = records
                .iter_mut()
                .find(|record| record["identity"]["qualified_name"] == "UpgradeProbe.__init__")
                .expect("the entry holds `UpgradeProbe.__init__`");
            let current_id = probe["id"].as_str().expect("id").to_string();
            let stale_key = upgrade_probe_key(SymbolKindV2::Method);
            probe["kind"] = Value::from("Method");
            probe["identity"]["stable_key"] = Value::from(stale_key.as_str());
            let text = serde_json::to_string(snapshot)
                .expect("serialize snapshot")
                .replace(&current_id, &symbol_id_from_stable_key(&stale_key).0);
            *snapshot = serde_json::from_str(&text).expect("snapshot JSON");
        },
    });
}

/// Revision 17 wrote schema 4.0 records whose `metrics` had no `lines` key;
/// revision 18 writes schema 4.1 with the key.
#[test]
fn entry_from_extractor_revision_seventeen_is_not_served_after_the_schema_bump() {
    assert_schema_bump_entry_is_not_served(&SchemaBump {
        revision: "17",
        schema: serde_json::json!({ "major": 4, "minor": 0 }),
        lacks: "`metrics.lines`",
        revert: |entry| remove_metric(entry, "lines"),
        is_current: |entry| every_record_has_metric(entry, "lines"),
    });
}

/// Revision 26 wrote schema 4.2 records whose `metrics` had no `cyclomatic`
/// key; revision 27 writes schema 4.3 with the key.
#[test]
fn entry_from_extractor_revision_twenty_six_is_not_served_after_the_schema_bump() {
    assert_schema_bump_entry_is_not_served(&SchemaBump {
        revision: "26",
        schema: serde_json::json!({ "major": 4, "minor": 2 }),
        lacks: "`metrics.cyclomatic`",
        revert: |entry| remove_metric(entry, "cyclomatic"),
        is_current: |entry| every_record_has_metric(entry, "cyclomatic"),
    });
}

/// Revision 30 wrote schema 4.3 entries with no `occurrences` or
/// `reference_coverage` keys, whose reference summaries bound the first
/// same-named declaration; revision 31 writes schema 5.0 with both keys and
/// summaries derived from scoped occurrences.
#[test]
fn entry_from_extractor_revision_thirty_is_not_served_after_the_scoped_references_bump() {
    assert_schema_bump_entry_is_not_served(&SchemaBump {
        revision: "30",
        schema: serde_json::json!({ "major": 4, "minor": 3 }),
        lacks: "`occurrences` and `reference_coverage`",
        revert: |entry| {
            let entry = entry.as_object_mut().expect("entry object");
            entry.remove("occurrences");
            entry.remove("reference_coverage");
        },
        is_current: |entry| {
            entry["occurrences"].as_array().is_some_and(|list| !list.is_empty())
                && entry["reference_coverage"].as_array().is_some_and(|list| !list.is_empty())
        },
    });
}

/// Revision 27 counted cyclomatic complexity for Rust only, so a JavaScript
/// or TypeScript function's `metrics.cyclomatic` was `null`; revision 28
/// counts both.
#[test]
fn entry_from_extractor_revision_twenty_seven_is_not_served_after_the_ecmascript_rules() {
    for (file, language, source) in [
        ("branchy.js", "javascript", "function branchy(a) { return a ? 1 : 2; }\nfunction sentinel() {}\n"),
        ("branchy.ts", "typescript", "function branchy(a: boolean) { return a ? 1 : 2; }\nfunction sentinel(): void {}\n"),
    ] {
        assert_stale_revision_is_not_served(&StaleRevision {
            revision: "27",
            key: OldKey::Computed,
            file,
            language,
            source: source.to_string(),
            sentinel: "sentinel",
            probe: |out| go_record(out, "branchy")["metrics"]["cyclomatic"].clone(),
            repaired: Value::from(2),
            stale: Value::Null,
            revert: |snapshot| {
                record_named(snapshot, "branchy")["metrics"]["cyclomatic"] = Value::Null;
            },
        });
    }
}

/// Revision 28 counted decisions written inside type syntax in a body (an
/// array type's length, a TypeScript object type's computed key) and a
/// switch label grouped with an earlier `default` (`default: case 1:`), so
/// these functions reported 2; revision 29 reports 1 for each.
#[test]
fn entry_from_extractor_revision_twenty_eight_is_not_served_after_the_complexity_repairs() {
    for (file, language, source) in [
        ("typed.rs", "rust", "fn measured() { let _x: [u8; if true { 1 } else { 2 }]; }\nfn sentinel() {}\n"),
        ("typed.ts", "typescript", "function measured(a: boolean) { let x: { [a ? 1 : 2]: string }; }\nfunction sentinel(): void {}\n"),
        ("switched.js", "javascript", "function measured(x) { switch (x) { default: case 1: break; } }\nfunction sentinel() {}\n"),
    ] {
        assert_stale_revision_is_not_served(&StaleRevision {
            revision: "28",
            key: OldKey::Computed,
            file,
            language,
            source: source.to_string(),
            sentinel: "sentinel",
            probe: |out| go_record(out, "measured")["metrics"]["cyclomatic"].clone(),
            repaired: Value::from(1),
            stale: Value::from(2),
            revert: |snapshot| {
                record_named(snapshot, "measured")["metrics"]["cyclomatic"] = Value::from(2);
            },
        });
    }
}

/// Revision 29 read only the last expression of a Scala `import` and no
/// unbraced `as` rename, so `import lib.ReviewBase, lib.Other` did not bind
/// `ReviewBase` and the child's base took the file-level class's ID; from
/// revision 30 the imported name keeps its text alone.
#[test]
fn entry_from_extractor_revision_twenty_nine_is_not_served_after_the_binding_evidence_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "29",
        key: OldKey::Computed,
        file: "imported.scala",
        language: "scala",
        source: "class ReviewBase\nobject ReviewOuter {\n  import lib.ReviewBase, lib.Other\n  \
                 class ReviewChild extends ReviewBase\n}\nclass Sentinel\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::Null,
        stale: Value::from("ReviewBase"),
        revert: |snapshot| {
            let base = record_named(snapshot, "ReviewBase");
            let (id, qualified_name) = (base["id"].clone(), base["identity"]["qualified_name"].clone());
            let relations = &mut record_named(snapshot, "ReviewChild")["relations"];
            let mut target = relations["unclassified_bases"][0].clone();
            target["id"] = id;
            target["qualified_name"] = qualified_name;
            relations.as_object_mut().expect("relations").remove("unclassified_bases");
            relations["extends"] = Value::Array(vec![target]);
        },
    });
}

/// Revision 29 dropped a Go interface element spelled like a predeclared
/// type even when a same-file declaration redeclares that name.
#[test]
fn entry_from_extractor_revision_twenty_nine_is_not_served_with_a_shadowed_go_universe_name() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "29",
        key: OldKey::Computed,
        file: "universe.go",
        language: "go",
        source: "package review\n\ntype int interface{ M() }\ntype ReviewChild interface{ int }\n\
                 type Sentinel struct{}\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::from("review.int"),
        stale: Value::Null,
        revert: |snapshot| {
            let relations = &mut record_named(snapshot, "ReviewChild")["relations"];
            relations.as_object_mut().expect("relations").remove("extends");
        },
    });
}

/// Revision 29 dropped a Swift enum's first base spelled like a standard
/// raw-value type even when a same-file alias redeclares that name.
#[test]
fn entry_from_extractor_revision_twenty_nine_is_not_served_with_an_aliased_swift_raw_type_name() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "29",
        key: OldKey::Computed,
        file: "aliased.swift",
        language: "swift",
        source: "protocol ReviewProtocol {}\ntypealias Int = ReviewProtocol\nenum ReviewChild: Int { case a }\n\
                 struct Sentinel {}\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["unclassified_bases"][0]["name"].clone(),
        repaired: Value::from("Int"),
        stale: Value::Null,
        revert: |snapshot| {
            let relations = &mut record_named(snapshot, "ReviewChild")["relations"];
            relations.as_object_mut().expect("relations").remove("unclassified_bases");
        },
    });
}

/// Revision 29 left a C# base naming a same-file record class in
/// `unclassified_bases`, although the target's declaration proves a class.
#[test]
fn entry_from_extractor_revision_twenty_nine_is_not_served_with_an_unclassified_record_base() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "29",
        key: OldKey::Computed,
        file: "records.cs",
        language: "c_sharp",
        source: "record ReviewBase;\nrecord ReviewChild : ReviewBase;\nclass Sentinel {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::from("ReviewBase"),
        stale: Value::Null,
        revert: |snapshot| {
            let relations = &mut record_named(snapshot, "ReviewChild")["relations"];
            let target = relations["extends"][0].clone();
            relations.as_object_mut().expect("relations").remove("extends");
            relations["unclassified_bases"] = Value::Array(vec![target]);
        },
    });
}

/// Revision 31 looked a written base up by displayed qualified name, so a
/// trait declared in a finished block of `factory` (`factory::ReviewBase`)
/// claimed the later `trait ReviewChild: ReviewBase` written beside it;
/// revision 32 decides the name through the lexical scopes in view and
/// resolves the file-level trait.
#[test]
fn entry_from_extractor_revision_thirty_one_is_not_served_after_the_lexical_binding_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "31",
        key: OldKey::Computed,
        file: "blocks.rs",
        language: "rust",
        source: "trait ReviewBase {}\nfn factory() {\n    { trait ReviewBase {} }\n    trait ReviewChild: ReviewBase {}\n}\n\
                 struct Sentinel;\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::from("ReviewBase"),
        stale: Value::from("factory::ReviewBase"),
        revert: |snapshot| {
            let inner = snapshot["records"]
                .as_array()
                .expect("entry records")
                .iter()
                .find(|record| record["identity"]["qualified_name"] == "factory::ReviewBase")
                .expect("the block's trait")
                .clone();
            let target = &mut record_named(snapshot, "ReviewChild")["relations"]["extends"][0];
            target["id"] = inner["id"].clone();
            target["qualified_name"] = inner["identity"]["qualified_name"].clone();
        },
    });
}

/// Revision 32 continued a qualified path below a function-local owner by
/// displayed name, so `ReviewHolder::ReviewBase` written beside a
/// `ReviewHolder` without that member claimed the member of a same-named
/// `ReviewHolder` in a finished block; revision 33 looks the member up in the
/// selected owner's own declarations and keeps the base as written.
#[test]
fn entry_from_extractor_revision_thirty_two_is_not_served_after_the_owner_identity_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "32",
        key: OldKey::Computed,
        file: "owners.cpp",
        language: "cpp",
        source: "void factory() {\n    { struct ReviewHolder { struct ReviewBase {}; }; }\n    \
                 { struct ReviewHolder {};\n      struct ReviewChild : ReviewHolder::ReviewBase {}; }\n}\n\
                 struct Sentinel {};\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::Null,
        stale: Value::from("factory::ReviewHolder::ReviewBase"),
        revert: |snapshot| {
            let sibling = snapshot["records"]
                .as_array()
                .expect("entry records")
                .iter()
                .find(|record| record["identity"]["qualified_name"] == "factory::ReviewHolder::ReviewBase")
                .expect("the finished block's member")
                .clone();
            let target = &mut record_named(snapshot, "ReviewChild")["relations"]["extends"][0];
            target["id"] = sibling["id"].clone();
            target["qualified_name"] = sibling["identity"]["qualified_name"].clone();
        },
    });
}

/// Revision 33 continued a qualified path past its first name without the
/// selected owner's bindings, so a Python nested class that a later
/// class-body import rebinds still gave `ReviewHolder.ReviewBase` the old
/// class's ID; revision 34 reads every segment's bindings and keeps the base
/// as written.
#[test]
fn entry_from_extractor_revision_thirty_three_is_not_served_after_the_member_binding_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "33",
        key: OldKey::Computed,
        file: "members.py",
        language: "python",
        source: "class ReviewHolder:\n    class ReviewBase:\n        pass\n    \
                 from builtins import object as ReviewBase\n\
                 class ReviewChild(ReviewHolder.ReviewBase):\n    pass\n\
                 class Sentinel:\n    pass\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::Null,
        stale: Value::from("ReviewHolder.ReviewBase"),
        revert: |snapshot| {
            let member = snapshot["records"]
                .as_array()
                .expect("entry records")
                .iter()
                .find(|record| record["identity"]["qualified_name"] == "ReviewHolder.ReviewBase")
                .expect("the rebound nested class")
                .clone();
            let target = &mut record_named(snapshot, "ReviewChild")["relations"]["extends"][0];
            target["id"] = member["id"].clone();
            target["qualified_name"] = member["identity"]["qualified_name"].clone();
        },
    });
}

/// Revision 34 lost the outer parameters of nested Swift extension owners.
#[test]
fn entry_from_extractor_revision_thirty_four_is_not_served_after_nested_swift_generic_context_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "34", key: OldKey::Computed, file: "nested.swift", language: "swift",
        source: "class ReviewBase {}\nstruct ReviewOuter<ReviewBase> { struct ReviewInner {} }\nextension ReviewOuter.ReviewInner { class ReviewChild: ReviewBase {} }\nstruct Sentinel {}\n".into(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::Null, stale: Value::from("ReviewBase"),
        revert: |snapshot| {
            let base = record_named(snapshot, "ReviewBase").clone();
            let relations = &mut record_named(snapshot, "ReviewChild")["relations"];
            let mut target = relations["unclassified_bases"][0].clone();
            target["id"] = base["id"].clone();
            target["qualified_name"] = Value::from("ReviewBase");
            relations.as_object_mut().expect("relations").remove("unclassified_bases");
            relations["extends"] = Value::Array(vec![target]);
        },
    });
}

/// Revision 35 continued a path through either of two same-named Rust
/// modules as if they were one reopened namespace, so `ReviewHolder::ReviewBase`
/// took the ID of the one module's trait; revision 36 stops at the competing
/// owners and keeps the base as written.
#[test]
fn entry_from_extractor_revision_thirty_five_is_not_served_after_the_owner_ambiguity_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "35",
        key: OldKey::Computed,
        file: "owners.rs",
        language: "rust",
        source: "mod ReviewHolder { pub trait ReviewBase {} }
mod ReviewHolder {}
\
                 trait ReviewChild: ReviewHolder::ReviewBase {}
struct Sentinel;
"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::Null,
        stale: Value::from("ReviewHolder::ReviewBase"),
        revert: |snapshot| {
            let base = record_named(snapshot, "ReviewBase").clone();
            let target = &mut record_named(snapshot, "ReviewChild")["relations"]["extends"][0];
            target["id"] = base["id"].clone();
            target["qualified_name"] = base["identity"]["qualified_name"].clone();
        },
    });
}

/// Revision 36 read a sibling module's import as a binding of the one the
/// base is written in, so `ReviewBase` inside the first of two same-named
/// Rust modules stayed text only; revision 37 reads that module's own
/// imports and gives the base its own module's ID.
#[test]
fn entry_from_extractor_revision_thirty_six_is_not_served_after_the_source_owner_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "36",
        key: OldKey::Computed,
        file: "owners.rs",
        language: "rust",
        source: "mod ReviewHolder {\n    pub trait ReviewBase {}\n    pub trait ReviewChild: ReviewBase {}\n}\n\
                 mod ReviewHolder { use external::ReviewBase; }\nstruct Sentinel;\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::from("ReviewHolder::ReviewBase"),
        stale: Value::Null,
        revert: |snapshot| {
            let target = &mut record_named(snapshot, "ReviewChild")["relations"]["extends"][0];
            target["id"] = Value::Null;
            target["qualified_name"] = Value::Null;
        },
    });
}

/// Revision 37 let a call inside a JavaScript `with` body establish the
/// function around it, so `f` was proven recursive; revision 38 keeps the
/// call a suggestion (the `with` object may supply `f`) and recursion
/// unknown.
#[test]
fn entry_from_extractor_revision_thirty_seven_is_not_served_after_the_lexical_boundary_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "37",
        key: OldKey::Computed,
        file: "rec.js",
        language: "javascript",
        source: "function f(obj) { with (obj) { f(obj); } }\nfunction Sentinel() {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "f")["semantics"]["is_recursive"].clone(),
        repaired: Value::from("Unknown"),
        stale: Value::from("Yes"),
        revert: |snapshot| {
            let f = record_named(snapshot, "f");
            let reference = serde_json::json!({
                "id": f["id"].clone(),
                "name": "f",
                "qualified_name": f["identity"]["qualified_name"].clone(),
            });
            f["semantics"]["is_recursive"] = Value::from("Yes");
            f["relations"]["references"].as_array_mut().expect("references").push(reference.clone());
            f["relations"]["referenced_by"] = Value::Array(vec![reference]);
            let call = snapshot["occurrences"]
                .as_array_mut()
                .expect("occurrences")
                .iter_mut()
                .find(|occurrence| occurrence["target"]["name"] == "f")
                .expect("the self-call");
            call["target"]["id"] = call["candidates"][0]["id"].clone();
            call["candidates"] = Value::Array(Vec::new());
            call["confidence"] = Value::from("Scoped");
        },
    });
}

/// Revision 38 merged every C# `partial` declaration of one name, so
/// `ReviewChild` in the non-generic `ReviewHolder` borrowed the `ReviewBase`
/// of `ReviewHolder<T>` as a proved superclass; revision 39 merges only
/// fragments of one type identity and keeps the base text only.
#[test]
fn entry_from_extractor_revision_thirty_eight_is_not_served_after_the_partial_identity_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "38",
        key: OldKey::Computed,
        file: "partials.cs",
        language: "c_sharp",
        source: "partial class ReviewHolder { class ReviewChild : ReviewBase {} }\n\
                 partial class ReviewHolder<T> { class ReviewBase {} }\nclass Sentinel {}\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::Null,
        stale: Value::from("ReviewHolder.ReviewBase"),
        revert: |snapshot| {
            let base = record_named(snapshot, "ReviewBase");
            let (id, qualified_name) = (base["id"].clone(), base["identity"]["qualified_name"].clone());
            let relations = &mut record_named(snapshot, "ReviewChild")["relations"];
            let mut target = relations["unclassified_bases"][0].clone();
            target["id"] = id;
            target["qualified_name"] = qualified_name;
            relations.as_object_mut().expect("relations").remove("unclassified_bases");
            relations["extends"] = Value::Array(vec![target]);
        },
    });
}

/// Revision 39 chose among same-named C# types before applying the written
/// generic argument count, so `ReviewChild : ReviewBase<int>` beside a
/// non-generic `ReviewBase` kept only its base text; revision 40 selects the
/// one-parameter `ReviewBase` and proves the superclass.
#[test]
fn entry_from_extractor_revision_thirty_nine_is_not_served_after_the_written_arity_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "39",
        key: OldKey::Computed,
        file: "arities.cs",
        language: "c_sharp",
        source: "class ReviewBase {}\nclass ReviewBase<T> {}\nclass ReviewChild : ReviewBase<int> {}\nclass Sentinel {}\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "ReviewChild")["relations"]["extends"][0]["qualified_name"].clone(),
        repaired: Value::from("ReviewBase"),
        stale: Value::Null,
        revert: |snapshot| {
            let relations = &mut record_named(snapshot, "ReviewChild")["relations"];
            let mut target = relations["extends"][0].clone();
            target["id"] = Value::Null;
            target["qualified_name"] = Value::Null;
            relations.as_object_mut().expect("relations").remove("extends");
            relations["unclassified_bases"] = Value::Array(vec![target]);
        },
    });
}

/// Revision 40 placed a body `var` in the scope its function's parameter
/// defaults read, so the default's `f()` in `function g(x = f()) { var f; }`
/// bound the body's `f`; revision 41 keeps the body's `var`s apart and binds
/// the outer function.
#[test]
fn entry_from_extractor_revision_forty_is_not_served_after_the_parameter_environment_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "40",
        key: OldKey::Computed,
        file: "defaults.js",
        language: "javascript",
        source: "function f() {}\nfunction g(x = f()) { var f = () => {}; }\nfunction Sentinel() {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "g")["relations"]["references"][0]["qualified_name"].clone(),
        repaired: Value::from("f"),
        stale: Value::from("g.f"),
        revert: |snapshot| {
            let records = snapshot["records"].as_array_mut().expect("entry records");
            let reference = |record: &Value| {
                serde_json::json!({
                    "id": record["id"].clone(),
                    "name": record["identity"]["name"].clone(),
                    "qualified_name": record["identity"]["qualified_name"].clone(),
                })
            };
            let position = |qualified: &str| {
                records
                    .iter()
                    .position(|record| record["identity"]["qualified_name"] == qualified)
                    .unwrap_or_else(|| panic!("a `{qualified}` record"))
            };
            let (outer, local, caller) = (position("f"), position("g.f"), position("g"));
            let (local_ref, caller_ref) = (reference(&records[local]), reference(&records[caller]));
            records[caller]["relations"]["references"] = Value::Array(vec![local_ref.clone()]);
            records[outer]["relations"].as_object_mut().expect("relations").remove("referenced_by");
            records[local]["relations"]["referenced_by"] = Value::Array(vec![caller_ref]);
            let call = snapshot["occurrences"]
                .as_array_mut()
                .expect("occurrences")
                .iter_mut()
                .find(|occurrence| occurrence["target"]["name"] == "f")
                .expect("the default's call");
            call["target"]["id"] = local_ref["id"].clone();
        },
    });
}

/// Revision 41 did not read a TypeScript instantiation expression's operand,
/// so `(f<number>)()` was a usage that limited its owner's coverage and left
/// `g`'s recursion `Unknown`; revision 42 reads it as a call of `f`.
#[test]
fn entry_from_extractor_revision_forty_one_is_not_served_after_the_instantiation_expression_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "41",
        key: OldKey::Computed,
        file: "instantiation.ts",
        language: "typescript",
        source: "function f<T>() {}\nfunction g() { (f<number>)(); }\nfunction Sentinel() {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "g")["semantics"]["is_recursive"].clone(),
        repaired: Value::from("No"),
        stale: Value::from("Unknown"),
        revert: |snapshot| {
            let call = snapshot["occurrences"]
                .as_array_mut()
                .expect("occurrences")
                .iter_mut()
                .find(|occurrence| occurrence["target"]["name"] == "f")
                .expect("the instantiated call");
            call["kind"] = Value::from("Usage");
            let region = snapshot["reference_coverage"]
                .as_array_mut()
                .expect("coverage")
                .iter_mut()
                .find(|region| region["owner"]["name"] == "g")
                .expect("`g`'s region");
            region["state"] = Value::from("Limited");
            region["reasons"] = serde_json::json!(["indirect_dispatch"]);
            record_named(snapshot, "g")["semantics"]["is_recursive"] = Value::from("Unknown");
        },
    });
}

/// Revision 42 read a written path from the name's immediate parent only,
/// so the rooted Rust self-call `::f()` lost its root, was a usage and left
/// `f`'s recursion `No`; revision 43 keeps `::f` and reads it as a call
/// whose root binding is unproved (`Unknown`).
#[test]
fn entry_from_extractor_revision_forty_two_is_not_served_after_the_written_path_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "42",
        key: OldKey::Computed,
        file: "rooted.rs",
        language: "rust",
        source: "fn f() { ::f(); }\nfn Sentinel() {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| go_record(out, "f")["semantics"]["is_recursive"].clone(),
        repaired: Value::from("Unknown"),
        stale: Value::from("No"),
        revert: |snapshot| {
            let call = snapshot["occurrences"]
                .as_array_mut()
                .expect("occurrences")
                .iter_mut()
                .find(|occurrence| occurrence["target"]["name"] == "f")
                .expect("the rooted call");
            assert_eq!(call["target"]["qualified_name"], "::f");
            call["kind"] = Value::from("Usage");
            call["target"]["qualified_name"] = Value::Null;
            // Revision 42 wrote no `path_start_byte` at all, which the
            // current reader refuses on shape; `null` with the path keeps
            // the payload servable, so the control shows the key decides.
            call["path_start_byte"] = Value::Null;
            record_named(snapshot, "f")["semantics"]["is_recursive"] = Value::from("No");
        },
    });
}

/// Revision 43 captured every identifier child of a Python default
/// parameter, so `x=external` also declared a parameter `external` whose
/// span excluded the default's read; revision 44 captures only the name and
/// keeps that read as a usage.
#[test]
fn entry_from_extractor_revision_forty_three_is_not_served_after_the_parameter_capture_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "43",
        key: OldKey::Computed,
        file: "defaults.py",
        language: "python",
        source: "def external(): pass\ndef g(x=external): pass\ndef Sentinel(): pass\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| {
            // An empty list is not serialized.
            let reads = out["indexes"][0]["occurrences"].as_array().map_or(0, |occurrences| {
                occurrences.iter().filter(|occurrence| occurrence["target"]["name"] == "external").count()
            });
            Value::from(reads)
        },
        repaired: Value::from(1),
        stale: Value::from(0),
        revert: |snapshot| {
            let occurrences = snapshot["occurrences"].as_array_mut().expect("occurrences");
            let before = occurrences.len();
            occurrences.retain(|occurrence| occurrence["target"]["name"] != "external");
            assert_eq!(occurrences.len() + 1, before, "the default's one read");
        },
    });
}

/// Revision 44 did not look through a Swift force unwrap or a C#
/// null-forgiving `!`, so `cb!()` was a usage of `cb`; revision 45 reads
/// it as a call.
#[test]
fn entry_from_extractor_revision_forty_four_is_not_served_after_the_null_assertion_call_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "44",
        key: OldKey::Computed,
        file: "unwrap.swift",
        language: "swift",
        source: "func g(cb: (() -> Void)?) { cb!() }\nfunc Sentinel() {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| {
            let occurrences = out["indexes"][0]["occurrences"].as_array().expect("occurrences");
            let unwrapped = occurrences
                .iter()
                .find(|occurrence| occurrence["target"]["name"] == "cb")
                .expect("the unwrapped call");
            unwrapped["kind"].clone()
        },
        repaired: Value::from("Call"),
        stale: Value::from("Usage"),
        revert: |snapshot| {
            let call = snapshot["occurrences"]
                .as_array_mut()
                .expect("occurrences")
                .iter_mut()
                .find(|occurrence| occurrence["target"]["name"] == "cb")
                .expect("the unwrapped call");
            assert_eq!(call["kind"], "Call");
            call["kind"] = Value::from("Usage");
        },
    });
}

/// Revision 45 read a Swift precedence group's directive and associativity
/// value (`associativity: left`) as usages, and took a Java static import
/// of a member named `var` as a legacy type; revision 46 excludes those
/// syntax markers. Only occurrences change, so the stale entry differs from
/// a current one by the two marker occurrences alone.
#[test]
fn entry_from_extractor_revision_forty_five_is_not_served_after_the_syntax_marker_repair() {
    const SOURCE: &str = "precedencegroup A {\n    associativity: left\n}\nfunc Sentinel() {}\n";
    fn markers(out: &Value) -> usize {
        // An empty list is not serialized.
        out["occurrences"].as_array().map_or(0, |occurrences| {
            occurrences
                .iter()
                .filter(|occurrence| matches!(occurrence["target"]["name"].as_str(), Some("associativity" | "left")))
                .count()
        })
    }
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "45",
        key: OldKey::Computed,
        file: "precedence.swift",
        language: "swift",
        source: SOURCE.to_string(),
        sentinel: "Sentinel",
        probe: |out| Value::from(markers(&out["indexes"][0])),
        repaired: Value::from(0),
        stale: Value::from(2),
        revert: |snapshot| {
            let root = snapshot["reference_coverage"]
                .as_array()
                .expect("coverage")
                .iter()
                .find(|region| region["owner"].is_null())
                .expect("the file region")["span"]
                .clone();
            let occurrences = snapshot["occurrences"].as_array_mut().expect("occurrences");
            for (name, column, start) in [("associativity", 5_u64, 24_u64), ("left", 20, 39)] {
                let width = name.len() as u64;
                assert_eq!(&SOURCE[start as usize..(start + width) as usize], name);
                occurrences.push(serde_json::json!({
                    "owner": null,
                    "owner_span": root,
                    "target": { "id": null, "name": name, "qualified_name": null },
                    "path_start_byte": null,
                    "import_span": null,
                    "import_start_byte": null,
                    "candidates": [],
                    "span": {
                        "start": { "line": 2, "column": column },
                        "end": { "line": 2, "column": column + width },
                        "start_byte": start,
                        "end_byte": start + width,
                    },
                    "kind": "Usage",
                    "confidence": "Unresolved",
                }));
            }
            occurrences.sort_by_key(|occurrence| occurrence["span"]["start_byte"].as_u64());
            assert_eq!(markers(snapshot), 2);
        },
    });
}

/// Revision 46 read a TypeScript entity alias's original as the alias's
/// second named child, so `import D /* note */ = N.S` had the original
/// `/* note */`, the dependency of `S` named the comment and `S`'s
/// recursion was a false `No`; revision 47 reads the entity after `=`
/// without comments (`N.S`, recursion `Unknown`).
#[test]
fn entry_from_extractor_revision_forty_six_is_not_served_after_the_commented_import_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "46",
        key: OldKey::Computed,
        file: "alias.ts",
        language: "typescript",
        source: "import D /* note */ = N.S;\nfunction S() { D(); }\nfunction Sentinel() {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| out["indexes"][0]["imports"][0]["symbol"]["qualified_name"].clone(),
        repaired: Value::from("N.S"),
        stale: Value::from("/* note */"),
        revert: |snapshot| {
            let original = &mut snapshot["imports"][0]["symbol"]["qualified_name"];
            assert_eq!(*original, "N.S");
            *original = Value::from("/* note */");
            let record = snapshot["records"]
                .as_array_mut()
                .expect("entry records")
                .iter_mut()
                .find(|record| record["identity"]["name"] == "S")
                .expect("S");
            for relation in ["references", "dependencies"] {
                record["relations"][relation][0]["qualified_name"] = Value::from("/* note */");
            }
            assert_eq!(record["semantics"]["is_recursive"], "Unknown");
            record["semantics"]["is_recursive"] = Value::from("No");
        },
    });
}

/// Commented generic construction must not reuse a pre-repair Usage.
#[test]
fn entry_from_extractor_revision_forty_seven_is_not_served_after_the_generic_comment_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "47", key: OldKey::Computed, file: "generic.swift", language: "swift",
        source: "class C<T> {}\nfunc run() { _ = C /* note */ <Int>() }\nfunc Sentinel() {}\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| out["indexes"][0]["occurrences"].as_array().expect("occurrences").iter().find(|occ| occ["target"]["name"] == "C").expect("C")["kind"].clone(),
        repaired: Value::from("Call"), stale: Value::from("Usage"),
        revert: |snapshot| {
            let occurrence = snapshot["occurrences"].as_array_mut().expect("occurrences").iter_mut().find(|occ| occ["target"]["name"] == "C").expect("C");
            assert_eq!(occurrence["kind"], "Call");
            occurrence["kind"] = Value::from("Usage");
        },
    });
}

#[test]
fn entry_from_extractor_revision_forty_eight_is_not_served_after_import_inventory_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "48", key: OldKey::Computed, file: "group.ts", language: "typescript",
        source: "import {f as D,g as E} from 'a'; function Sentinel() {}".to_string(),
        sentinel: "Sentinel",
        probe: |out| Value::from(out["indexes"][0]["imports"].as_array().expect("imports").len()),
        repaired: Value::from(2), stale: Value::from(1),
        revert: |snapshot| {
            assert_eq!(snapshot["imports"].as_array().expect("imports").len(), 2);
            snapshot["imports"].as_array_mut().expect("imports").pop();
            snapshot.as_object_mut().expect("entry").remove("import_inventory");
        },
    });
}

/// Revision 50 stamped library `undefined-symbol` diagnostics `Error` rather
/// than the registry's `Warning`, and a hit replays a stored severity. Symbols
/// output shows no severity and `hug lint` restamps it from the registry, so
/// this row proves the entry is a miss through a renamed `compute` record;
/// `symbols_cache` in the library checks the severity itself.
#[test]
fn entry_from_extractor_revision_fifty_is_not_served_after_the_registry_severity_repair() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "50", key: OldKey::Computed, file: "undefined.py", language: "python",
        source: "def compute(items):\n    return items\n\ndef Sentinel():\n    pass\n\ntotal = compute_total(items)\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| Value::from(names(out).iter().any(|name| name == "compute")),
        repaired: Value::Bool(true), stale: Value::Bool(false),
        revert: |snapshot| rename_record(snapshot, "compute", "stale_compute"),
    });
}

/// Revision 51 wrote schema 5.1 entries with no `reexports` key, no
/// `imports[].reexported` marker and no `reexports` capability; revision 52
/// writes schema 5.2 with all three.
#[test]
fn entry_from_extractor_revision_fifty_one_is_not_served_after_the_reexports_bump() {
    assert_schema_bump_entry_is_not_served(&SchemaBump {
        revision: "51",
        schema: serde_json::json!({ "major": 5, "minor": 1 }),
        lacks: "`reexports`, `imports[].reexported` and the `reexports` capability",
        revert: |entry| {
            let entry = entry.as_object_mut().expect("entry object");
            entry.remove("reexports");
            entry
                .get_mut("completed_capabilities")
                .and_then(Value::as_array_mut)
                .expect("capabilities")
                .retain(|capability| capability != "reexports");
            for import in entry.get_mut("imports").and_then(Value::as_array_mut).expect("imports") {
                import.as_object_mut().expect("import").remove("reexported");
            }
        },
        is_current: |entry| {
            entry["reexports"].is_object()
                && entry["completed_capabilities"]
                    .as_array()
                    .is_some_and(|list| list.iter().any(|capability| capability == "reexports"))
                && entry["imports"]
                    .as_array()
                    .is_some_and(|list| list.iter().all(|import| import["reexported"].is_boolean()))
        },
    });
}

/// Revision 52 stored the placeholder report (a whole-file `grammar_gap`
/// limit, no entries) for every supported language; revision 53 stores the
/// language's real forwarding entries.
#[test]
fn entry_from_extractor_revision_fifty_two_is_not_served_after_the_language_extractors_land() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "52", key: OldKey::Computed, file: "forward.py", language: "python",
        source: "from m import a\n\ndef Sentinel():\n    pass\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| Value::from(out["indexes"][0]["reexports"]["entries"].as_array().expect("entries").len()),
        repaired: Value::from(1), stale: Value::from(0),
        revert: |snapshot| {
            let span = snapshot["reexports"]["entries"][0]["statement_span"].clone();
            snapshot["reexports"] = serde_json::json!({
                "coverage": { "state": "limited", "limits": [{ "reason": "grammar_gap", "span": span }] },
                "entries": [],
            });
        },
    });
}

/// Revision 53 forwarded both of two sequential Python imports of one name
/// (and linked a Scala export to an import a nearer member definition hid);
/// revision 54 forwards only the final import binding.
#[test]
fn entry_from_extractor_revision_fifty_three_is_not_served_after_binding_scope_repairs() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "53", key: OldKey::Computed, file: "forward.py", language: "python",
        source: "from old import a\nfrom new import a\n\ndef Sentinel():\n    pass\n".to_string(),
        sentinel: "Sentinel",
        probe: |out| {
            let entries = out["indexes"][0]["reexports"]["entries"].as_array().expect("entries");
            Value::from(entries.iter().map(|entry| entry["source"].clone()).collect::<Vec<_>>())
        },
        repaired: serde_json::json!(["new"]), stale: serde_json::json!(["old", "new"]),
        revert: |snapshot| {
            let replaced = snapshot["imports"][0].clone();
            let mut forwarded = snapshot["reexports"]["entries"][0].clone();
            forwarded["source"] = Value::from("old");
            forwarded["span"] = replaced["span"].clone();
            forwarded["import_span"] = replaced["span"].clone();
            forwarded["statement_span"] = replaced["statement_span"].clone();
            snapshot["reexports"]["entries"].as_array_mut().expect("entries").insert(0, forwarded);
        },
    });
}

/// Revision 54 folded a conditional `__all__.append`/`.remove` into the
/// final list as if it always ran, so a guarded removal read as a complete
/// negative; revision 55 limits coverage with `computed_export_list`.
#[test]
fn entry_from_extractor_revision_fifty_four_is_not_served_after_conditional_list_edits_are_limited() {
    assert_stale_revision_is_not_served(&StaleRevision {
        revision: "54", key: OldKey::Computed, file: "forward.py", language: "python",
        source: "from m import read\n__all__ = [\"read\"]\nif enabled:\n    __all__.remove(\"read\")\n\ndef Sentinel():\n    pass\n"
            .to_string(),
        sentinel: "Sentinel",
        probe: |out| out["indexes"][0]["reexports"]["coverage"]["state"].clone(),
        repaired: Value::from("limited"), stale: Value::from("complete"),
        revert: |snapshot| {
            snapshot["reexports"]["coverage"] = serde_json::json!({ "state": "complete", "limits": [] });
        },
    });
}

/// An earlier revision that wrote another schema, and what its entries
/// lacked.
struct SchemaBump {
    revision: &'static str,
    schema: Value,
    lacks: &'static str,
    /// Removes from a current entry what `revision` did not write.
    revert: fn(&mut Value),
    /// Whether an entry holds what the bump added.
    is_current: fn(&Value) -> bool,
}

fn remove_metric(entry: &mut Value, metric: &str) {
    for record in entry["records"].as_array_mut().expect("entry records") {
        record["metrics"]
            .as_object_mut()
            .expect("a complete index stores metrics on every record")
            .remove(metric);
    }
}

fn every_record_has_metric(entry: &Value, metric: &str) -> bool {
    entry["records"].as_array().expect("records").iter().all(|record| {
        record["metrics"].as_object().is_some_and(|metrics| metrics.contains_key(metric))
    })
}

/// An entry an earlier revision wrote in its schema, without what the bump
/// added. Unlike the repairs above, the old payload is not readable even
/// under the current key (its versions and shape are rejected), so it is
/// recomputed both there and under its own key, and the rewritten entry
/// holds current records and what the bump added.
fn assert_schema_bump_entry_is_not_served(bump: &SchemaBump) {
    const PROBE: &str = "python.py";
    let (revision, schema, lacks) = (bump.revision, &bump.schema, bump.lacks);
    let label = format!("revision {revision}");
    let args = ["--json-full", "symbols", PROBE];
    let scratch = Scratch::new(&[(PROBE, PYTHON_FIXTURE)]);

    let discovery = TempDir::new().expect("discovery temp");
    let fresh = json_of(&stdout_of(scratch.hug_with_temp(discovery.path(), &args), &args));
    let discovered_entry = only_entry(discovery.path());
    let current = read_json(&discovered_entry);
    let file_path = current["file"].as_str().expect("entry file").to_string();
    let relative_project = discovered_entry
        .parent()
        .and_then(Path::parent)
        .and_then(|dir| dir.strip_prefix(discovery.path()).ok())
        .expect("entry under <temp>/tree-hugger/cache/<project_id>/<shard>/")
        .to_path_buf();
    let current_key = discovered_entry
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("entry key")
        .to_string();
    assert_eq!(computed_revision_key(EXTRACTOR_REVISION, &file_path), current_key, "computed key formula");
    let old_key = computed_revision_key(revision, &file_path);
    assert_ne!(old_key, current_key, "{label} has its own key");

    let mut stale = current.clone();
    stale["schema_version"] = schema.clone();
    for record in stale["records"].as_array_mut().expect("entry records") {
        record["schema_version"] = schema.clone();
    }
    (bump.revert)(&mut stale);
    rename_record(&mut stale, "module_function", STALE_NAME);

    // Under the current key: rejected, recomputed.
    let control = TempDir::new().expect("control temp");
    write_json(&entry_path(&control.path().join(&relative_project), &current_key), &stale);
    let recomputed = json_of(&stdout_of(scratch.hug_with_temp(control.path(), &args), &args));
    assert_eq!(recomputed, fresh, "a {label} payload under the current key is recomputed");

    // Under its own key: never read; run 1 writes the current entry.
    let project_dir = scratch.temp.path().join(&relative_project);
    write_json(&entry_path(&project_dir, &old_key), &stale);
    let run1 = json_of(&scratch.stdout(&args));
    assert!(!names(&run1).iter().any(|name| name == STALE_NAME), "run 1 served the {label} entry");
    assert_eq!(run1, fresh, "run 1 matches an uncached run");
    let new_entry = entry_path(&project_dir, &current_key);
    let written = read_json(&new_entry);
    let current_schema = serde_json::to_value(tree_hugger::SchemaVersion::CURRENT).expect("schema");
    assert_eq!(written["schema_version"], current_schema, "the rewritten entry is current");
    assert!(
        written["records"]
            .as_array()
            .expect("records")
            .iter()
            .all(|record| record["schema_version"] == current_schema),
        "every rewritten record is current"
    );
    assert!((bump.is_current)(&written), "the rewritten entry holds {lacks}");

    // Run 2 hits the rewritten entry.
    let mut renamed = written.clone();
    rename_record(&mut renamed, "module_function", RENAMED_NAME);
    write_json(&new_entry, &renamed);
    let run2 = json_of(&scratch.stdout(&args));
    assert!(names(&run2).iter().any(|name| name == RENAMED_NAME), "run 2 reads the rewritten entry");
}
