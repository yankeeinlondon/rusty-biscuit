use tempfile::TempDir;

use crate::artifact::OutputFormat;
use crate::cache::file_cache::{FileCache, MERMAID_BACKEND};
use crate::cache::VisualizationKind;

/// Builds a cache rooted at a fresh temp directory.
///
/// The returned [`TempDir`] must be kept alive for the cache's lifetime;
/// dropping it deletes the backing directory.
fn isolated_cache() -> (FileCache, TempDir) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let cache = FileCache::with_base_dir(dir.path().join("v1"));
    (cache, dir)
}

#[test]
fn cache_key_stability() {
    let key1 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->B",
        r#"{"theme":"dark"}"#,
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    let key2 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->B",
        r#"{"theme":"dark"}"#,
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    assert_eq!(key1, key2, "Same inputs should produce same cache key");
}

#[test]
fn cache_key_different_source() {
    let key1 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->B",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    let key2 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->C",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    assert_ne!(key1, key2, "Different source should produce different key");
}

#[test]
fn cache_key_different_options() {
    let key1 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->B",
        r#"{"theme":"dark"}"#,
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    let key2 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->B",
        r#"{"theme":"light"}"#,
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    assert_ne!(key1, key2, "Different options should produce different key");
}

#[test]
fn cache_key_different_format() {
    let key1 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->B",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    let key2 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "graph TD; A-->B",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Svg,
    );

    assert_ne!(key1, key2, "Different format should produce different key");
}

#[test]
fn cache_key_different_kind() {
    let key1 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "source",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    let key2 = FileCache::cache_key(
        VisualizationKind::Graph,
        "source",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    assert_ne!(
        key1, key2,
        "Different visualization kind should produce different key"
    );
}

#[test]
fn cache_roundtrip() {
    let (cache, _dir) = isolated_cache();
    let data = b"test data";

    let key = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "source",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Svg,
    );

    // Store data
    let stored_path = cache
        .store(VisualizationKind::Mermaid, &key, OutputFormat::Svg, data)
        .expect("Failed to store data");

    assert!(
        stored_path.exists(),
        "Stored file should exist at path: {:?}",
        stored_path
    );

    // Retrieve data
    let retrieved_path = cache
        .get(VisualizationKind::Mermaid, &key, OutputFormat::Svg)
        .expect("Failed to retrieve cached data");

    assert_eq!(stored_path, retrieved_path, "Paths should match");

    // Verify content
    let content = std::fs::read(&retrieved_path).expect("Failed to read cached file");
    assert_eq!(content, data, "Cached content should match original");
}

#[test]
fn cache_directory_layout() {
    let (cache, _dir) = isolated_cache();
    let data = b"test";

    let key = FileCache::cache_key(
        VisualizationKind::Graph,
        "src",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    let path = cache
        .store(VisualizationKind::Graph, &key, OutputFormat::Png, data)
        .expect("Failed to store");

    // Verify the path structure
    let path_str = path.to_string_lossy();
    assert!(
        path_str.contains("graph"),
        "Path should contain visualization kind"
    );
    assert!(
        path_str.contains("png"),
        "Path should contain format directory"
    );
    assert!(
        path_str.ends_with(".png"),
        "File should have correct extension"
    );
}

#[test]
fn cache_entry_count() {
    let (cache, _dir) = isolated_cache();

    let initial_count = cache.entry_count();
    assert_eq!(initial_count, 0, "Cache should be empty after clear");

    // Store some entries
    let key1 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "src1",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );
    cache
        .store(
            VisualizationKind::Mermaid,
            &key1,
            OutputFormat::Png,
            b"data1",
        )
        .expect("Failed to store");

    let key2 = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "src2",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Svg,
    );
    cache
        .store(
            VisualizationKind::Mermaid,
            &key2,
            OutputFormat::Svg,
            b"data2",
        )
        .expect("Failed to store");

    let count = cache.entry_count();
    assert_eq!(count, 2, "Cache should contain 2 entries");
}

#[test]
fn cache_size_bytes() {
    let (cache, _dir) = isolated_cache();

    let initial_size = cache.size_bytes();
    assert_eq!(initial_size, 0, "Cache should be empty after clear");

    let data = b"test data with some content";
    let key = FileCache::cache_key(
        VisualizationKind::Graph,
        "src",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );

    cache
        .store(VisualizationKind::Graph, &key, OutputFormat::Png, data)
        .expect("Failed to store");

    let size = cache.size_bytes();
    assert_eq!(
        size,
        data.len() as u64,
        "Cache size should equal stored data size"
    );
}

#[test]
fn cache_clear() {
    let (cache, _dir) = isolated_cache();

    // Store some data
    let key = FileCache::cache_key(
        VisualizationKind::Mermaid,
        "src",
        "{}",
        MERMAID_BACKEND,
        OutputFormat::Png,
    );
    cache
        .store(VisualizationKind::Mermaid, &key, OutputFormat::Png, b"data")
        .expect("Failed to store");

    assert!(cache.entry_count() > 0, "Cache should have entries");

    // Clear cache
    cache.clear().expect("Failed to clear cache");

    assert_eq!(cache.entry_count(), 0, "Cache should be empty after clear");
    assert_eq!(
        cache.size_bytes(),
        0,
        "Cache size should be zero after clear"
    );
}

#[test]
fn cache_get_nonexistent() {
    let (cache, _dir) = isolated_cache();

    let result = cache.get(
        VisualizationKind::Mermaid,
        "nonexistent_key",
        OutputFormat::Png,
    );

    assert!(result.is_none(), "Should return None for nonexistent key");
}

#[test]
fn visualization_kind_as_str() {
    assert_eq!(VisualizationKind::Mermaid.as_str(), "mermaid");
    assert_eq!(VisualizationKind::Graph.as_str(), "graph");
}
