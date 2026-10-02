//! Acceptance criterion 8 of `darkmatter/fixes/2026-09-16-content-policy-no-cache`:
//! until a `ContentPolicy` exists, no semantic result (composed document,
//! `::file` child, `::code` / `::toc-linking` result, document snapshot) may be
//! persisted. The run-local cache is memory-only, and the file-backed store is
//! reachable only from the remote transport cache.
//!
//! Scope rule (`source_scan::production_sources`). Every `.rs` file under
//! this crate's `src/` is scanned as production code after three exclusions:
//!
//! - comments and string/char literals, blanked by the CLI crate's shared
//!   sanitizer (`cli/tests/common/source_scan.rs`, included by path so the two
//!   structural gates cannot drift apart);
//! - every item or statement annotated `#[cfg(test)]` or `#[cfg(all(test, ..))]`,
//!   up to its first `;` or its matching `}`;
//! - every file such an annotation pulls in with `mod name;` (honoring a
//!   `#[path = ".."]` on the declaration), and everything below that module.
//!
//! Identifiers match on identifier boundaries, so `FileStoreX` is not
//! `FileStore`.

#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use source_scan::{ident_offsets, line_at, matching_brace, production_sources};

/// Production files that may name `FileStore`, with the exact occurrence
/// count, keyed by `/`-separated path relative to `src/`. Every one belongs to
/// the remote transport cache; a new file or a moved count is a failure.
const FILE_STORE_ALLOWLIST: &[(&str, usize)] = &[
    // The store itself.
    ("markdown/compose/cache/store.rs", 2),
    // The `pub(crate)` re-export.
    ("markdown/compose/cache/mod.rs", 1),
    // Read, write, and purge of remote-URL entries.
    ("markdown/compose/cache/remote_cache.rs", 6),
    // The fetch runtime owns the optional transport store.
    ("markdown/compose/remote_fetch.rs", 4),
    // Resolves the configured cache root into the fetch runtime's store.
    ("markdown/compose/context/options.rs", 3),
];

/// The run-local cache struct, which must never hold a store.
const RUN_LOCAL_CACHE_FILE: &str = "markdown/compose/cache/runtime.rs";
const RUN_LOCAL_CACHE: &str = "RunLocalCache";
/// A field of any of these types would give the run-local cache a path to disk.
const FORBIDDEN_RUN_LOCAL_FIELDS: &[&str] = &["FileStore", "RemoteFetchRuntime"];

/// The semantic-result persistence surface this fix deleted. Reintroducing
/// any of these names in production code is how that path would come back.
const REMOVED_SURFACE: &[&str] = &[
    "with_persistent",
    "try_persistent_read_compose",
    "try_persistent_write_compose",
    "try_persistent_read_operation",
    "try_persistent_write_operation",
    "try_write_document_snapshot",
    "DocumentSnapshotManifest",
    "ComposedDocumentManifest",
    "OperationResultManifest",
    "PersistentContext",
    "OperationPersistentContext",
    "ContextClosureIdentity",
    "DependencyRef",
    "CacheFreshnessMode",
    "with_cache_freshness_mode",
    "persistent_cache_eligible",
];

fn lines(text: &str, offsets: &[usize]) -> Vec<usize> {
    offsets.iter().map(|&offset| line_at(text, offset)).collect()
}

/// The run-local cache struct exists exactly once and names no store type.
fn check_run_local_cache(sources: &BTreeMap<String, String>) -> Vec<String> {
    let Some(text) = sources.get(RUN_LOCAL_CACHE_FILE) else {
        return vec![format!("{RUN_LOCAL_CACHE_FILE} is gone; point the guard at the run-local cache")];
    };
    let definitions: Vec<usize> = ident_offsets(text, RUN_LOCAL_CACHE)
        .into_iter()
        .filter(|&offset| text[..offset].trim_end().ends_with("struct"))
        .collect();
    let [definition] = definitions[..] else {
        return vec![format!(
            "expected one `struct {RUN_LOCAL_CACHE}` in {RUN_LOCAL_CACHE_FILE}, found {}",
            definitions.len()
        )];
    };
    let open = definition + text[definition..].find('{').expect("a struct with fields");
    let body = &text[open..=matching_brace(text.as_bytes(), open)];
    FORBIDDEN_RUN_LOCAL_FIELDS
        .iter()
        .filter(|forbidden| !ident_offsets(body, forbidden).is_empty())
        .map(|forbidden| format!("`{RUN_LOCAL_CACHE}` names `{forbidden}`: the run-local cache must stay memory-only"))
        .collect()
}

/// `FileStore` appears only where the allowlist expects it, exactly.
fn check_file_store_reach(sources: &BTreeMap<String, String>, allowlist: &[(&str, usize)]) -> Vec<String> {
    let allowed: BTreeMap<&str, usize> = allowlist.iter().copied().collect();
    assert_eq!(allowed.len(), allowlist.len(), "duplicate allowlist entry");
    let mut problems = Vec::new();
    let mut found = BTreeSet::new();
    for (path, text) in sources {
        let offsets = ident_offsets(text, "FileStore");
        if offsets.is_empty() {
            continue;
        }
        found.insert(path.as_str());
        match allowed.get(path.as_str()) {
            None => problems.push(format!(
                "`FileStore` reached from {path} at lines {:?}; only the remote transport cache may use it",
                lines(text, &offsets)
            )),
            Some(&expected) if expected != offsets.len() => problems.push(format!(
                "stale allowlist count for {path}: expected {expected}, found {} at lines {:?}",
                offsets.len(),
                lines(text, &offsets)
            )),
            Some(_) => {}
        }
    }
    for (path, _) in allowlist {
        if !found.contains(path) {
            problems.push(format!("stale allowlist entry: {path} no longer names `FileStore`"));
        }
    }
    problems
}

/// No deleted persistence symbol reappears.
fn check_removed_surface(sources: &BTreeMap<String, String>) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, text) in sources {
        for ident in REMOVED_SURFACE {
            let offsets = ident_offsets(text, ident);
            if !offsets.is_empty() {
                problems.push(format!("removed persistence symbol `{ident}` in {path} at lines {:?}", lines(text, &offsets)));
            }
        }
    }
    problems
}

fn all_problems(src: &Path, allowlist: &[(&str, usize)]) -> Vec<String> {
    let sources = production_sources(src);
    let mut problems = check_run_local_cache(&sources);
    problems.extend(check_file_store_reach(&sources, allowlist));
    problems.extend(check_removed_surface(&sources));
    problems
}

#[test]
fn semantic_results_have_no_path_to_disk() {
    let src = biscuit_test_harness::manifest_dir!().join("src");
    let problems = all_problems(&src, FILE_STORE_ALLOWLIST);
    assert!(problems.is_empty(), "semantic-result persistence guard:\n  {}", problems.join("\n  "));
}

/// The guard catches each violation it exists for and ignores test code,
/// comments, and literals, so it cannot pass vacuously.
#[test]
fn the_guard_catches_planted_violations_and_honors_the_scope_rule() {
    let root = tempfile::tempdir().unwrap();
    let write = |relative: &str, text: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        RUN_LOCAL_CACHE_FILE,
        "// A FileStore in a comment.\n\
         pub(crate) struct RunLocalCache {\n    persistent: Option<Arc<FileStore>>,\n}\n\
         #[cfg(test)]\nmod tests {\n    fn t() { FileStore::at(p); with_persistent(); }\n}\n",
    );
    write("markdown/compose/cache/store.rs", "pub(crate) struct FileStore;\n");
    write("markdown/compose/cache/mod.rs", "#[cfg(test)]\nmod helpers;\n");
    write("markdown/compose/cache/helpers.rs", "fn h() { FileStore::at(p); }\n");
    write("markdown/compose/other.rs", "#[cfg(test)]\nmod nested;\n#[cfg(test)]\n#[path = \"elsewhere.rs\"]\nmod moved;\n");
    write("markdown/compose/other/nested/deep.rs", "fn with_persistent() {}\n");
    write("markdown/compose/elsewhere.rs", "struct DependencyRef;\n");
    write(
        "markdown/compose/leak.rs",
        "let s = \"with_persistent\";\nfn with_persistent() {}\nstruct FileStoreLike;\n#[cfg(all(test, unix))]\nfn skip() { DependencyRef }\n",
    );

    let allowlist = [("markdown/compose/cache/store.rs", 1), ("markdown/compose/gone.rs", 1)];
    let mut problems = all_problems(root.path(), &allowlist);
    problems.sort();
    let expected = [
        "`FileStore` reached from markdown/compose/cache/runtime.rs at lines [3]; only the remote transport cache may use it",
        "`RunLocalCache` names `FileStore`: the run-local cache must stay memory-only",
        "removed persistence symbol `with_persistent` in markdown/compose/leak.rs at lines [2]",
        "stale allowlist entry: markdown/compose/gone.rs no longer names `FileStore`",
    ];
    assert_eq!(problems, expected);

    write(RUN_LOCAL_CACHE_FILE, "pub(crate) struct Renamed {}\n");
    let problems = check_run_local_cache(&production_sources(root.path()));
    assert_eq!(problems, [format!("expected one `struct {RUN_LOCAL_CACHE}` in {RUN_LOCAL_CACHE_FILE}, found 0")]);
}
