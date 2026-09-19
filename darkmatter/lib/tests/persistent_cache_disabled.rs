//! R18 of the more-context feature: until a `ContentPolicy` exists, compose
//! persists no local artifact. A configured cache root may hold raw remote-URL
//! bodies only; composed `::file` children, `::code` and `::toc-linking`
//! results, and document snapshots are never read from or written to it, so a
//! warm cache cannot replay an earlier execution's composed output, its
//! execution identity, or a probe's result (AC36).
//!
//! See `darkmatter/fixes/2026-09-16-content-policy-no-cache/spec.md`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{CacheAccessMode, ComposeOptions};
use darkmatter::markdown::reference::ReferenceGraphOptions;

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

/// A root exercising every semantic-result kind: the execution identity, a
/// `::file` child rendering a value the compose key deliberately excludes
/// (`ctx.timestamp_ms`), plus a `::code` and a `::toc-linking` operation.
fn write_graph(dir: &Path) -> std::path::PathBuf {
    write(
        dir,
        "root.md",
        "# Root\n\nid=[{{ ctx.id }}] sid=[{{ ctx.sid }}]\n\n::file ./child.md\n\n::code ./main.rs\n\n::toc-linking ./toc.md\n",
    );
    write(dir, "child.md", "stamp=[{{ ctx.timestamp_ms }}]\n");
    write(dir, "main.rs", "fn main() {}\n");
    write(dir, "toc.md", "# Toc\n\n## Section\n");
    dir.join("root.md")
}

/// The `name=[value]` probe on the composed text.
fn probe(content: &str, name: &str) -> String {
    let start = content
        .find(&format!("{name}=["))
        .unwrap_or_else(|| panic!("no `{name}` probe in {content}"))
        + name.len()
        + 2;
    let end = start + content[start..].find(']').expect("probe closes");
    content[start..end].to_string()
}

fn compose(root: &Path, options: ComposeOptions) -> String {
    let markdown = Markdown::try_from(root).expect("root loads");
    let (composed, _) = markdown
        .compose_with(options.with_source_file(root))
        .expect("the graph composes");
    composed.content().to_string()
}

/// Acceptance criterion 2: cold and warm runs recompose under every remaining
/// cache control — each `CacheAccessMode`, with no cache root, a cache root,
/// and a namespaced cache root — and nothing is written under the root.
#[test]
fn a_warm_cache_root_never_replays_composed_local_output() {
    let access_modes = [
        CacheAccessMode::Off,
        CacheAccessMode::ReadOnly,
        CacheAccessMode::ReadWrite,
        CacheAccessMode::Refresh,
    ];
    for access_mode in access_modes {
        for root_variant in ["no root", "root", "namespaced root"] {
            assert_warm_run_recomposes(access_mode, root_variant);
        }
    }
}

fn assert_warm_run_recomposes(access_mode: CacheAccessMode, root_variant: &str) {
    let label = format!("{access_mode:?} / {root_variant}");
    let directory = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let root = write_graph(directory.path());
    let options = || {
        let options = ComposeOptions::new().with_cache_access_mode(access_mode);
        match root_variant {
            "no root" => options,
            "root" => options.with_cache_root(cache.path()),
            "namespaced root" => {
                options.with_cache_root(cache.path()).with_cache_namespace("branch")
            }
            other => unreachable!("unknown root variant {other}"),
        }
    };

    let cold = compose(&root, options());
    std::thread::sleep(Duration::from_millis(5));
    // A probe whose result changes between executions.
    write(directory.path(), "main.rs", "fn main() { warm() }\n");
    let warm = compose(&root, options());

    assert!(cold.contains("fn main() {}"), "{label}: ::code rendered: {cold}");
    assert!(cold.contains("Section"), "{label}: ::toc-linking rendered: {cold}");
    assert!(
        warm.contains("fn main() { warm() }") && !warm.contains("fn main() {}"),
        "{label}: the warm run replayed the cold run's ::code result: {warm}"
    );
    assert_ne!(
        probe(&cold, "stamp"),
        probe(&warm, "stamp"),
        "{label}: the warm run replayed the cold run's composed child"
    );
    for key in ["id", "sid"] {
        assert!(!probe(&cold, key).is_empty(), "{label}: ctx.{key} rendered: {cold}");
        assert_ne!(
            probe(&cold, key),
            probe(&warm, key),
            "{label}: the warm run replayed the cold run's ctx.{key}"
        );
    }
    assert_ne!(probe(&warm, "id"), probe(&warm, "sid"), "{label}: {warm}");
    assert_eq!(snapshot_tree(cache.path()), Vec::new(), "{label}: an artifact was persisted");
}

/// A reference graph built against a cache root persists nothing either, and
/// a rebuild still sees an edited child.
#[test]
fn a_reference_graph_with_a_cache_root_persists_no_local_artifact() {
    let directory = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let root = write_graph(directory.path());
    let options = ReferenceGraphOptions::with_compose(
        ComposeOptions::new().with_cache_root(cache.path()).with_cache_namespace("branch"),
    );
    let markdown = Markdown::try_from(root.as_path()).unwrap();

    let first = markdown.reference_graph(options.clone()).unwrap();
    write(directory.path(), "child.md", "# Edited\n\n[link](https://example.com)\n");
    let second = markdown.reference_graph(options).unwrap();

    assert_eq!(first.node_count(), second.node_count());
    assert!(
        second
            .iter()
            .any(|node| !node.local_references.hyperlinks().is_empty()),
        "the rebuilt graph must read the edited child from disk"
    );
    assert_eq!(snapshot_tree(cache.path()), Vec::new(), "an artifact was persisted");
}

/// Relative path, directory flag, and bytes for every entry under `root`,
/// sorted. Modification times are excluded: directory mtimes vary by
/// filesystem (notably Windows and WSL2 `drvfs`).
fn snapshot_tree(root: &Path) -> Vec<(PathBuf, bool, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, bool, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.push((relative, true, Vec::new()));
                walk(root, &path, out);
            } else {
                out.push((relative, false, std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// `symlink_metadata` also catches a dangling link left at `path`.
fn assert_absent(path: &Path, label: &str) {
    let metadata = std::fs::symlink_metadata(path);
    assert!(
        metadata.as_ref().is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "{label}: configuring a cache root created {}: {metadata:?}",
        path.display()
    );
}

/// The ways a caller can attach a cache root to a local-only compose.
const CACHE_ROOT_VARIANTS: [&str; 4] =
    ["root only", "namespaced", "shared remote runtime", "read-write run-local cache"];

/// Built per root: `with_shared_remote_fetch` resolves the store from the root
/// already configured, so the root cannot be swapped in afterwards.
fn cache_root_options(variant: &str, cache_root: &Path) -> ComposeOptions {
    let base = ComposeOptions::new().with_cache_root(cache_root);
    match variant {
        "root only" => base,
        "namespaced" => base.with_cache_namespace("branch"),
        "shared remote runtime" => base.with_shared_remote_fetch(),
        "read-write run-local cache" => base.with_cache_access_mode(CacheAccessMode::ReadWrite),
        other => unreachable!("unknown variant {other}"),
    }
}

fn assert_local_only_compose(root: &Path, options: ComposeOptions, label: &str) {
    let markdown = Markdown::try_from(root).expect("root loads");
    let (composed, report) = markdown
        .compose_with(options.with_source_file(root))
        .unwrap_or_else(|error| panic!("{label}: the graph composes: {error}"));
    let content = composed.content();
    assert!(content.contains("fn main() {}"), "{label}: ::code rendered: {content}");
    assert!(content.contains("Section"), "{label}: ::toc-linking rendered: {content}");

    let summary = report.summary();
    assert!(report.cache_stats.is_some(), "{label}: cache stats");
    if let Some(remote) = report.remote_fetch_stats {
        assert_eq!(remote.fetched + remote.cache_hits, 0, "{label}: {remote:?}");
    }
    assert!(!summary.contains("write"), "{label}: the summary claims a cache write: {summary}");
}

/// Acceptance criterion 1: a cache root that does not exist is still absent
/// after a local-only compose or reference-graph build.
#[test]
fn a_nonexistent_cache_root_is_never_created_by_local_only_work() {
    let directory = tempfile::tempdir().unwrap();
    let root = write_graph(directory.path());
    let parent = tempfile::tempdir().unwrap();

    for (index, label) in CACHE_ROOT_VARIANTS.into_iter().enumerate() {
        // A fresh missing path per variant, so an earlier variant cannot mask a later one.
        let cache_root = parent.path().join(format!("never-created-{index}"));
        assert_local_only_compose(&root, cache_root_options(label, &cache_root), label);
        assert_absent(&cache_root, label);
    }

    let cache_root = parent.path().join("never-created-by-graph");
    let options = ReferenceGraphOptions::with_compose(
        ComposeOptions::new().with_cache_root(&cache_root).with_cache_namespace("branch"),
    );
    Markdown::try_from(root.as_path()).unwrap().reference_graph(options).unwrap();
    assert_absent(&cache_root, "reference graph");

    assert_eq!(
        snapshot_tree(parent.path()),
        Vec::new(),
        "no sibling of the configured cache root was created either"
    );
}

/// Acceptance criterion 1: an existing cache root — empty, partially
/// populated, or holding another run's remote entry — keeps the same entries
/// and bytes after a local-only compose.
#[test]
fn an_existing_cache_root_is_left_byte_identical_by_local_only_work() {
    let directory = tempfile::tempdir().unwrap();
    let root = write_graph(directory.path());

    let seeds: [(&str, &[(&str, &str)]); 3] = [
        ("empty", &[]),
        (
            "sentinels",
            &[("sentinel.txt", "keep me\n"), ("nested/deeper/note.md", "# Note\n")],
        ),
        (
            "partial store",
            &[
                (".darkmatter/cache/v1/manifests/remote/ab/cd/abcd000000000000.json", "{}"),
                (".darkmatter/cache/v1/branch/README", "namespace sentinel\n"),
            ],
        ),
    ];
    for (seed, files) in seeds {
        for variant in CACHE_ROOT_VARIANTS {
            let cache = tempfile::tempdir().unwrap();
            for (relative, content) in files {
                let path = cache.path().join(relative);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, content).unwrap();
            }
            let before = snapshot_tree(cache.path());
            let label = format!("{seed} / {variant}");

            assert_local_only_compose(&root, cache_root_options(variant, cache.path()), &label);

            assert_eq!(snapshot_tree(cache.path()), before, "{label}: the cache root changed");
        }
    }
}

/// A cache root that names a regular file is not an error for local-only work,
/// and the file is untouched.
#[test]
fn a_cache_root_that_is_a_file_does_not_fail_local_only_work() {
    let directory = tempfile::tempdir().unwrap();
    let root = write_graph(directory.path());
    let parent = tempfile::tempdir().unwrap();
    let file = parent.path().join("not-a-directory");
    std::fs::write(&file, "plain file\n").unwrap();

    for label in CACHE_ROOT_VARIANTS {
        assert_local_only_compose(&root, cache_root_options(label, &file), label);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "plain file\n", "{label}");
        assert!(std::fs::metadata(&file).unwrap().is_file(), "{label}");
    }
}
