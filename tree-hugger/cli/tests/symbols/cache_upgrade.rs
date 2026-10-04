//! Test 8: a persistent cache entry written under the previous extractor
//! fingerprint is not served after the upgrade, and the entry written by the
//! upgraded run is served on the next run. Works on the cache files directly,
//! with `TMPDIR` (`TMP`/`TEMP` on Windows) pointed at a test-owned directory.
//!
//! The stderr `cache:` line cannot show this: it counts only in-process hits,
//! and a persistent hit records nothing.

use std::path::{Path, PathBuf};

use biscuit_hash::{blake3_hash, xx_hash};
use serde_json::Value;
use tempfile::TempDir;

use crate::{Scratch, TYPESCRIPT_FIXTURE, cache_entries, json_of, stdout_of};

const FILE: &str = "typescript.ts";
const ARGS: [&str; 3] = ["--json", "symbols", FILE];
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

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read cache entry")).expect("cache JSON")
}

fn write_json(path: &Path, value: &Value) {
    std::fs::create_dir_all(path.parent().expect("entry parent")).expect("create shard");
    std::fs::write(path, serde_json::to_vec(value).expect("serialize")).expect("write entry");
}

/// Renames the record called `from` (name and display name) in a snapshot.
fn rename_record(snapshot: &mut Value, from: &str, to: &str) {
    let record = snapshot["symbols"]
        .as_array_mut()
        .expect("snapshot symbols")
        .iter_mut()
        .find(|record| record["identity"]["name"] == from)
        .unwrap_or_else(|| panic!("snapshot has no `{from}` record"));
    record["identity"]["name"] = Value::from(to);
    record["identity"]["display_name"] = Value::from(to);
}

/// A snapshot whose records are recognizably stale: schema `2.0` throughout,
/// every access Unknown, no `static` modifier, and one record renamed.
fn stale_snapshot(current: &Value) -> Value {
    let mut snapshot = current.clone();
    let old_schema = serde_json::json!({ "major": 2, "minor": 0 });
    snapshot["key"]["analyzer_fingerprint"]["schema_version"] = old_schema.clone();
    for record in snapshot["symbols"].as_array_mut().expect("snapshot symbols") {
        record["schema_version"] = old_schema.clone();
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
    let mut facts: Vec<_> = json["files"][0]["symbols"]
        .as_array()
        .expect("files[0].symbols")
        .iter()
        .map(|symbol| {
            (
                symbol["name"].as_str().unwrap_or_default().to_string(),
                symbol["range"]["start_line"].clone(),
                symbol["kind"].clone(),
                symbol.get("visibility").cloned().unwrap_or(Value::Null),
                symbol.get("is_static").cloned().unwrap_or(Value::Bool(false)),
                symbol.get("container_kind").cloned().unwrap_or(Value::Null),
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
fn only_entry(temp: &Path) -> PathBuf {
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
    let file_path = current_snapshot["key"]["file_path"]
        .as_str()
        .expect("snapshot key file_path")
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
