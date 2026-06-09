//! Integration tests for Phase 5 caching features.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tree_hugger::cache::*;
use tree_hugger::{AnalysisPass, FileSymbolIndex, ProgrammingLanguage, SchemaVersion, TreeFile};

fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

// ============================================================================
// PassFingerprint Tests
// ============================================================================

#[test]
fn test_pass_fingerprint_digest_is_stable() {
    let fp = PassFingerprint {
        schema_version: SchemaVersion::V2_0,
        tree_hugger_version: "0.1.0".to_string(),
        grammar_version: "0.24.0".to_string(),
        grammar_id: "rust".to_string(),
        query_hash: "abc".to_string(),
        rule_metadata_hash: "def".to_string(),
        enabled_rules_hash: "ghi".to_string(),
        config_hash: "jkl".to_string(),
        external_tool_version: None,
    };
    assert_eq!(fp.digest(), fp.digest());
}

#[test]
fn test_pass_fingerprint_changes_with_source() {
    let mut fp = PassFingerprint {
        schema_version: SchemaVersion::V2_0,
        tree_hugger_version: "0.1.0".to_string(),
        grammar_version: "0.24.0".to_string(),
        grammar_id: "rust".to_string(),
        query_hash: "abc".to_string(),
        rule_metadata_hash: "def".to_string(),
        enabled_rules_hash: "ghi".to_string(),
        config_hash: "jkl".to_string(),
        external_tool_version: None,
    };
    let base = fp.digest();
    fp.query_hash = "xyz".to_string();
    assert_ne!(base, fp.digest());
}

#[test]
fn test_pass_fingerprint_changes_with_tool_version() {
    let mut fp = PassFingerprint {
        schema_version: SchemaVersion::V2_0,
        tree_hugger_version: "0.1.0".to_string(),
        grammar_version: "0.24.0".to_string(),
        grammar_id: "rust".to_string(),
        query_hash: "abc".to_string(),
        rule_metadata_hash: "def".to_string(),
        enabled_rules_hash: "ghi".to_string(),
        config_hash: "jkl".to_string(),
        external_tool_version: Some("1.0.0".to_string()),
    };
    let base = fp.digest();
    fp.external_tool_version = Some("2.0.0".to_string());
    assert_ne!(base, fp.digest());
}

// ============================================================================
// InProcessCache Tests
// ============================================================================

#[test]
fn test_in_process_cache_parse_tree_hit_miss() {
    let cache = InProcessCache::new(8);
    let key = "test_parse";

    assert!(cache.get_parse_tree(key).is_none());

    cache.put_parse_tree(
        key.to_string(),
        ParsedFileSnapshot {
            file_path: PathBuf::from("test.rs"),
            language: ProgrammingLanguage::Rust,
            source_hash: "abc".to_string(),
            grammar_id: "rust".to_string(),
            source_text: "fn main() {}".to_string(),
        },
    );

    assert!(cache.get_parse_tree(key).is_some());
}

#[test]
fn test_in_process_cache_symbol_index_hit_miss() {
    let cache = InProcessCache::new(8);
    let key = "test_symbols";

    assert!(cache.get_symbol_index(key).is_none());

    cache.put_symbol_index(
        key.to_string(),
        FileSymbolIndex {
            schema_version: SchemaVersion::V2_0,
            file: PathBuf::from("test.rs"),
            language: ProgrammingLanguage::Rust,
            file_hash: "abc".to_string(),
            completed_passes: vec![AnalysisPass::Parse],
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            diagnostics: Vec::new(),
        },
    );

    assert!(cache.get_symbol_index(key).is_some());
}

#[test]
fn test_in_process_cache_capacity_eviction() {
    let cache = InProcessCache::new(2);

    cache.put_parse_tree(
        "key1".to_string(),
        ParsedFileSnapshot {
            file_path: PathBuf::from("a.rs"),
            language: ProgrammingLanguage::Rust,
            source_hash: "a".to_string(),
            grammar_id: "rust".to_string(),
            source_text: "fn a() {}".to_string(),
        },
    );
    cache.put_parse_tree(
        "key2".to_string(),
        ParsedFileSnapshot {
            file_path: PathBuf::from("b.rs"),
            language: ProgrammingLanguage::Rust,
            source_hash: "b".to_string(),
            grammar_id: "rust".to_string(),
            source_text: "fn b() {}".to_string(),
        },
    );
    cache.put_parse_tree(
        "key3".to_string(),
        ParsedFileSnapshot {
            file_path: PathBuf::from("c.rs"),
            language: ProgrammingLanguage::Rust,
            source_hash: "c".to_string(),
            grammar_id: "rust".to_string(),
            source_text: "fn c() {}".to_string(),
        },
    );

    // One of the first two should have been evicted
    let count = [
        cache.get_parse_tree("key1").is_some(),
        cache.get_parse_tree("key2").is_some(),
        cache.get_parse_tree("key3").is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    assert_eq!(count, 2, "Cache should only hold 2 entries");
}

#[test]
fn test_in_process_cache_stats_tracking() {
    let cache = InProcessCache::new(8);

    cache.record_hit(CacheHitInfo::hit(
        AnalysisPass::Parse,
        std::time::Duration::from_millis(1),
    ));
    cache.record_hit(CacheHitInfo::hit(
        AnalysisPass::Bind,
        std::time::Duration::from_millis(2),
    ));
    cache.record_hit(CacheHitInfo::miss(
        AnalysisPass::Semantic,
        CacheInvalidationReason::SourceChanged,
        std::time::Duration::from_millis(3),
    ));

    let stats = cache.stats();
    assert_eq!(stats.parse_hits, 1);
    assert_eq!(stats.symbol_hits, 1);
    assert_eq!(stats.diagnostic_misses, 1);
}

#[test]
fn test_in_process_cache_hit_history() {
    let cache = InProcessCache::new(8);

    cache.record_hit(CacheHitInfo::hit(
        AnalysisPass::Parse,
        std::time::Duration::from_millis(1),
    ));

    let history = cache.hit_history();
    assert_eq!(history.len(), 1);
    assert!(history[0].hit);
    assert_eq!(history[0].pass, AnalysisPass::Parse);
}

#[test]
fn test_in_process_cache_clear() {
    let cache = InProcessCache::new(8);
    cache.put_parse_tree(
        "key".to_string(),
        ParsedFileSnapshot {
            file_path: PathBuf::from("test.rs"),
            language: ProgrammingLanguage::Rust,
            source_hash: "abc".to_string(),
            grammar_id: "rust".to_string(),
            source_text: "fn main() {}".to_string(),
        },
    );

    cache.clear();
    assert!(cache.get_parse_tree("key").is_none());
    assert_eq!(cache.stats().entries, 0);
}

// ============================================================================
// PersistentCache Tests
// ============================================================================

#[test]
fn test_persistent_cache_disabled_returns_none() {
    let dir = TempDir::new().unwrap();
    let cache = PersistentCache::open(dir.path().to_path_buf(), false);
    assert!(cache.get::<SymbolSnapshot>("key").is_none());
}

#[test]
fn test_persistent_cache_roundtrip() {
    let dir = TempDir::new().unwrap();
    let cache = PersistentCache::open(dir.path().to_path_buf(), true);

    let snapshot = SymbolSnapshot {
        key: FileCacheKey {
            file_path: PathBuf::from("src/lib.rs"),
            language: ProgrammingLanguage::Rust,
            file_hash: "abc".to_string(),
            analyzer_fingerprint: AnalyzerFingerprint {
                schema_version: SchemaVersion::V2_0,
                tree_hugger_version: "0.1.0".to_string(),
                grammar_fingerprint: "g1".to_string(),
                query_fingerprint: "q1".to_string(),
                config_fingerprint: "c1".to_string(),
            },
        },
        completed_passes: vec![AnalysisPass::Parse],
        symbols: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        diagnostics: Vec::new(),
        created_at_epoch_ms: 0,
    };

    cache
        .put("test_key", &snapshot)
        .expect("put should succeed");
    let retrieved: SymbolSnapshot = cache.get("test_key").expect("get should return a value");
    assert_eq!(retrieved.key.file_hash, "abc");
}

#[test]
fn test_persistent_cache_invalidates_entry() {
    let dir = TempDir::new().unwrap();
    let cache = PersistentCache::open(dir.path().to_path_buf(), true);

    let snapshot = SymbolSnapshot {
        key: FileCacheKey {
            file_path: PathBuf::from("src/lib.rs"),
            language: ProgrammingLanguage::Rust,
            file_hash: "abc".to_string(),
            analyzer_fingerprint: AnalyzerFingerprint {
                schema_version: SchemaVersion::V2_0,
                tree_hugger_version: "0.1.0".to_string(),
                grammar_fingerprint: "g1".to_string(),
                query_fingerprint: "q1".to_string(),
                config_fingerprint: "c1".to_string(),
            },
        },
        completed_passes: vec![AnalysisPass::Parse],
        symbols: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        diagnostics: Vec::new(),
        created_at_epoch_ms: 0,
    };

    cache.put("key1", &snapshot).unwrap();
    cache.invalidate("key1");
    assert!(cache.get::<SymbolSnapshot>("key1").is_none());
}

#[test]
fn test_persistent_cache_invalidate_all() {
    let dir = TempDir::new().unwrap();
    let cache = PersistentCache::open(dir.path().to_path_buf(), true);

    let snapshot = SymbolSnapshot {
        key: FileCacheKey {
            file_path: PathBuf::from("src/lib.rs"),
            language: ProgrammingLanguage::Rust,
            file_hash: "abc".to_string(),
            analyzer_fingerprint: AnalyzerFingerprint {
                schema_version: SchemaVersion::V2_0,
                tree_hugger_version: "0.1.0".to_string(),
                grammar_fingerprint: "g1".to_string(),
                query_fingerprint: "q1".to_string(),
                config_fingerprint: "c1".to_string(),
            },
        },
        completed_passes: vec![AnalysisPass::Parse],
        symbols: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        diagnostics: Vec::new(),
        created_at_epoch_ms: 0,
    };

    cache.put("key1", &snapshot).unwrap();
    cache.put("key2", &snapshot).unwrap();
    cache.invalidate_all();
    assert!(cache.get::<SymbolSnapshot>("key1").is_none());
    assert!(cache.get::<SymbolSnapshot>("key2").is_none());
}

#[test]
fn test_persistent_cache_recomputes_on_corrupt_entry() {
    let dir = TempDir::new().unwrap();
    let cache = PersistentCache::open(dir.path().to_path_buf(), true);

    // Write invalid JSON
    let path = dir.path().join("co").join("corrupt_key.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "not valid json").unwrap();

    assert!(cache.get::<SymbolSnapshot>("corrupt_key").is_none());
}

// ============================================================================
// Cache Config Tests
// ============================================================================

#[test]
fn test_cache_config_default() {
    let config = CacheConfig::default();
    assert!(config.enabled);
    assert!(!config.persistent);
    assert_eq!(config.capacity, 256);
}

#[test]
fn test_cache_config_with_config() {
    let cache = InProcessCache::with_config(CacheConfig {
        enabled: false,
        persistent: true,
        capacity: 64,
    });
    assert_eq!(cache.stats().entries, 0);
}

// ============================================================================
// Cache Integration with TreeFile
// ============================================================================

#[test]
fn test_tree_file_symbol_index_can_be_cached() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = 42;
}
"#,
    );

    let tree_file = TreeFile::new(&path).unwrap();
    let index = tree_file.symbol_index_v2().unwrap();

    // Store in in-process cache
    let cache = InProcessCache::new(8);
    let key = format!("{}|{}", path.to_string_lossy(), tree_file.hash);
    cache.put_symbol_index(key.clone(), index.clone());

    // Retrieve from cache
    let cached = cache.get_symbol_index(&key).unwrap();
    assert_eq!(cached.file, index.file);
    assert_eq!(cached.symbols.len(), index.symbols.len());
}

#[test]
fn test_cache_invalidates_on_source_edit() {
    let dir = TempDir::new().unwrap();
    let path = create_temp_file(
        &dir,
        "test.rs",
        r#"fn main() {
    let x = 42;
}
"#,
    );

    let tree_file1 = TreeFile::new(&path).unwrap();
    let index1 = tree_file1.symbol_index_v2().unwrap();

    let cache = InProcessCache::new(8);
    let key1 = format!("{}|{}", path.to_string_lossy(), tree_file1.hash);
    cache.put_symbol_index(key1.clone(), index1);

    // Edit the file
    fs::write(&path, "fn main() { let y = 99; }\n").unwrap();

    let tree_file2 = TreeFile::new(&path).unwrap();
    // Different hash means different cache key
    let key2 = format!("{}|{}", path.to_string_lossy(), tree_file2.hash);

    // Old key should still be in cache
    assert!(cache.get_symbol_index(&key1).is_some());
    // New key should be a miss
    assert!(cache.get_symbol_index(&key2).is_none());
}

#[test]
fn test_cache_unit_enum_variants() {
    let fp = PassFingerprint {
        schema_version: SchemaVersion::V2_0,
        tree_hugger_version: "0.1.0".to_string(),
        grammar_version: "0.24.0".to_string(),
        grammar_id: "rust".to_string(),
        query_hash: "abc".to_string(),
        rule_metadata_hash: "def".to_string(),
        enabled_rules_hash: "ghi".to_string(),
        config_hash: "jkl".to_string(),
        external_tool_version: None,
    };

    let _unit = CacheUnit::Parse {
        fingerprint: fp.clone(),
        source_hash: "hash".to_string(),
        grammar_id: "rust".to_string(),
    };

    let _unit = CacheUnit::Symbols {
        fingerprint: fp.clone(),
        symbols: Vec::new(),
    };

    let _unit = CacheUnit::Diagnostics {
        fingerprint: fp,
        diagnostics: Vec::new(),
    };
}

// ============================================================================
// Query Fingerprint Tests
// ============================================================================

#[test]
fn test_query_fingerprint_deterministic() {
    let fp1 = query_fingerprint("(identifier) @id");
    let fp2 = query_fingerprint("(identifier) @id");
    assert_eq!(fp1, fp2);
}

#[test]
fn test_query_fingerprint_different_queries() {
    let fp1 = query_fingerprint("(identifier) @id");
    let fp2 = query_fingerprint("(call_expression) @call");
    assert_ne!(fp1, fp2);
}

#[test]
fn test_enabled_rules_fingerprint_deterministic() {
    let rules = vec!["unwrap-call".to_string(), "dbg-macro".to_string()];
    let fp1 = enabled_rules_fingerprint(&rules);
    let fp2 = enabled_rules_fingerprint(&rules);
    assert_eq!(fp1, fp2);
}

#[test]
fn test_enabled_rules_fingerprint_order_sensitive() {
    let fp1 = enabled_rules_fingerprint(&["unwrap-call".to_string(), "dbg-macro".to_string()]);
    let fp2 = enabled_rules_fingerprint(&["dbg-macro".to_string(), "unwrap-call".to_string()]);
    assert_ne!(fp1, fp2);
}

// ============================================================================
// Cache Invalidation Reason Tests
// ============================================================================

#[test]
fn test_cache_invalidation_reason_display() {
    assert_eq!(
        format!("{}", CacheInvalidationReason::SourceChanged),
        "source changed"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::QueryChanged),
        "query changed"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::GrammarChanged),
        "grammar changed"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::RuleMetadataChanged),
        "rule metadata changed"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::ConfigChanged),
        "config changed"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::ExternalToolChanged),
        "external tool changed"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::CorruptEntry),
        "corrupt entry"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::PassOptionsChanged),
        "pass options changed"
    );
    assert_eq!(
        format!("{}", CacheInvalidationReason::CacheDisabled),
        "cache disabled"
    );
}

// ============================================================================
// CacheHitInfo Tests
// ============================================================================

#[test]
fn test_cache_hit_info_hit() {
    let info = CacheHitInfo::hit(AnalysisPass::Parse, std::time::Duration::from_millis(5));
    assert!(info.hit);
    assert!(info.reason.is_none());
    assert_eq!(info.elapsed_ms, 5);
    assert_eq!(info.pass, AnalysisPass::Parse);
}

#[test]
fn test_cache_hit_info_miss() {
    let info = CacheHitInfo::miss(
        AnalysisPass::Bind,
        CacheInvalidationReason::SourceChanged,
        std::time::Duration::from_millis(10),
    );
    assert!(!info.hit);
    assert_eq!(info.reason, Some(CacheInvalidationReason::SourceChanged));
    assert_eq!(info.elapsed_ms, 10);
    assert_eq!(info.pass, AnalysisPass::Bind);
}

#[test]
fn test_cache_hit_info_serialization() {
    let info = CacheHitInfo::hit(AnalysisPass::Semantic, std::time::Duration::from_millis(3));
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"hit\":true"));
    assert!(json.contains("\"pass\":\"Semantic\""));
}
