//! R18 of the more-context feature: until a `ContentPolicy` exists, compose
//! persists no local artifact. A configured cache root may hold raw remote-URL
//! bodies only; composed `::file` children, `::code` and `::toc-linking`
//! results, and document snapshots are never read from or written to it, so a
//! warm cache cannot replay an earlier execution's composed output.
//!
//! See `darkmatter/fixes/2026-09-16-content-policy-no-cache/spec.md`.

use std::path::Path;
use std::time::Duration;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{
    CacheAccessMode, CacheFreshnessMode, CacheStats, ComposeOptions,
};
use darkmatter::markdown::reference::ReferenceGraphOptions;

/// Artifact classes that hold composed or derived local content.
const LOCAL_ARTIFACT_DIRS: [&str; 3] = ["snapshot", "composed", "operation"];

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

/// A root exercising every local artifact class: a `::file` child rendering a
/// value the persistent key deliberately excludes (`ctx.timestamp_ms`), plus a
/// `::code` and a `::toc-linking` operation.
fn write_graph(dir: &Path) -> std::path::PathBuf {
    write(
        dir,
        "root.md",
        "# Root\n\n::file ./child.md\n\n::code ./main.rs\n\n::toc-linking ./toc.md\n",
    );
    write(dir, "child.md", "stamp=[{{ ctx.timestamp_ms }}]\n");
    write(dir, "main.rs", "fn main() {}\n");
    write(dir, "toc.md", "# Toc\n\n## Section\n");
    dir.join("root.md")
}

fn stamp(content: &str) -> String {
    let start = content.find("stamp=[").expect("child rendered") + "stamp=[".len();
    let end = start + content[start..].find(']').expect("stamp closes");
    content[start..end].to_string()
}

/// Every `manifests/<class>` directory created under `cache_root`, at any depth.
fn local_artifact_dirs(cache_root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![cache_root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let is_local_class = path.parent().and_then(Path::file_name)
                == Some("manifests".as_ref())
                && LOCAL_ARTIFACT_DIRS.iter().any(|class| path.file_name() == Some(class.as_ref()));
            if is_local_class {
                found.push(path.clone());
            }
            pending.push(path);
        }
    }
    found
}

fn compose(root: &Path, options: ComposeOptions) -> (String, CacheStats) {
    let markdown = Markdown::try_from(root).expect("root loads");
    let (composed, report) = markdown
        .compose_with(options.with_source_file(root))
        .expect("the graph composes");
    (composed.content().to_string(), report.cache_stats.expect("cache stats"))
}

#[test]
fn a_warm_cache_root_never_replays_composed_local_output() {
    for mode in [
        CacheFreshnessMode::Strict,
        CacheFreshnessMode::Fallback,
        CacheFreshnessMode::Optimistic,
        CacheFreshnessMode::Forced,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let root = write_graph(directory.path());
        let options = || {
            ComposeOptions::new()
                .with_cache_access_mode(CacheAccessMode::ReadWrite)
                .with_cache_freshness_mode(mode)
                .with_cache_root(cache.path())
        };

        let (cold, cold_stats) = compose(&root, options());
        std::thread::sleep(Duration::from_millis(5));
        let (warm, warm_stats) = compose(&root, options());

        assert!(cold.contains("fn main() {}"), "{mode:?}: ::code rendered: {cold}");
        assert!(cold.contains("Section"), "{mode:?}: ::toc-linking rendered: {cold}");
        assert_ne!(
            stamp(&cold),
            stamp(&warm),
            "{mode:?}: the warm run replayed the cold run's composed child"
        );
        for stats in [&cold_stats, &warm_stats] {
            assert_eq!(stats.persistent_writes, 0, "{mode:?}: {stats:?}");
            assert_eq!(stats.persistent_hits, 0, "{mode:?}: {stats:?}");
        }
        assert_eq!(
            local_artifact_dirs(cache.path()),
            Vec::<std::path::PathBuf>::new(),
            "{mode:?}: a local artifact was persisted"
        );
    }
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
    assert_eq!(local_artifact_dirs(cache.path()), Vec::<std::path::PathBuf>::new());
}
