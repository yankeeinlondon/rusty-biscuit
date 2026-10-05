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

/// Renames the record called `from` (name and display name) in a snapshot.
pub(crate) fn rename_record(snapshot: &mut Value, from: &str, to: &str) {
    let record = snapshot["records"]
        .as_array_mut()
        .expect("entry records")
        .iter_mut()
        .find(|record| record["identity"]["name"] == from)
        .unwrap_or_else(|| panic!("snapshot has no `{from}` record"));
    record["identity"]["name"] = Value::from(to);
    record["identity"]["display_name"] = Value::from(to);
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
        serde_json::json!({ "major": 4, "minor": 1 }),
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
/// is not served, while the same payload under the current key is (so the
/// miss is due to the key) and the entry run 1 writes is served on run 2.
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
    // pre-repair value reaches the output.
    let control = TempDir::new().expect("control temp");
    write_json(&entry_path(&control.path().join(&relative_project), &current_key), &stale);
    let served = json_of(&stdout_of(scratch.hug_with_temp(control.path(), &args), &args));
    assert_eq!((case.probe)(&served), case.stale, "control: {label} payload under the current key must be served");

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
            record_named(snapshot, "instance_method")["identity"]["qualified_name"] =
                Value::from("PlainWidget::instance_method");
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
        source: format!("{TYPESCRIPT_FIXTURE}{}", crate::filters::FENCED_DEPRECATION_TS),
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
