use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use biscuit_hash::blake3_hash;
use serde::{Deserialize, Serialize};

use crate::shared::{
    AnalysisPass, Diagnostic, ExportRecord, FileSymbolIndex, ImportRecord, ProgrammingLanguage,
    SchemaVersion, SymbolRecord,
};

/// Dimensions that determine cache validity for a specific analysis pass.
///
/// This struct includes all inputs that affect the output of a pass:
/// source content, grammar version, query hashes, rule metadata, config,
/// and external tool versions where relevant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PassFingerprint {
    pub schema_version: SchemaVersion,
    pub tree_hugger_version: String,
    pub grammar_version: String,
    pub grammar_id: String,
    pub query_hash: String,
    pub rule_metadata_hash: String,
    pub enabled_rules_hash: String,
    pub config_hash: String,
    pub external_tool_version: Option<String>,
}

impl PassFingerprint {
    /// Computes a deterministic digest for the fingerprint fields.
    pub fn digest(&self) -> String {
        let canonical = format!(
            "{}.{}{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version.major,
            self.schema_version.minor,
            self.tree_hugger_version,
            self.grammar_version,
            self.grammar_id,
            self.query_hash,
            self.rule_metadata_hash,
            self.enabled_rules_hash,
            self.config_hash,
            self.external_tool_version.as_deref().unwrap_or("none")
        );
        blake3_hash(&canonical)
    }
}

/// Legacy fingerprint for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnalyzerFingerprint {
    pub schema_version: SchemaVersion,
    pub tree_hugger_version: String,
    pub grammar_fingerprint: String,
    pub query_fingerprint: String,
    pub config_fingerprint: String,
}

impl AnalyzerFingerprint {
    /// Returns a deterministic digest for the fingerprint fields.
    pub fn digest(&self) -> String {
        let canonical = format!(
            "{}.{}|{}|{}|{}|{}",
            self.schema_version.major,
            self.schema_version.minor,
            self.tree_hugger_version,
            self.grammar_fingerprint,
            self.query_fingerprint,
            self.config_fingerprint
        );
        blake3_hash(&canonical)
    }
}

/// Key for per-file cache entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileCacheKey {
    pub file_path: PathBuf,
    pub language: ProgrammingLanguage,
    pub file_hash: String,
    pub analyzer_fingerprint: AnalyzerFingerprint,
}

impl FileCacheKey {
    /// Returns a deterministic key string for map/database lookup.
    pub fn stable_key(&self) -> String {
        let canonical = format!(
            "{}|{}|{}|{}",
            normalize_path(&self.file_path),
            self.language.query_name(),
            self.file_hash,
            self.analyzer_fingerprint.digest()
        );
        blake3_hash(&canonical)
    }
}

/// Individual cache unit types, split by analysis stage.
///
/// Each unit can be cached independently, allowing granular invalidation
/// when only one pass's inputs change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheUnit {
    /// Parse tree and source text fingerprint.
    Parse {
        fingerprint: PassFingerprint,
        source_hash: String,
        grammar_id: String,
    },
    /// Symbol records from the parse pass.
    Symbols {
        fingerprint: PassFingerprint,
        symbols: Vec<SymbolRecord>,
    },
    /// Import records from the bind pass.
    Imports {
        fingerprint: PassFingerprint,
        imports: Vec<ImportRecord>,
    },
    /// Export records from the bind pass.
    Exports {
        fingerprint: PassFingerprint,
        exports: Vec<ExportRecord>,
    },
    /// Reference records from the bind pass.
    References {
        fingerprint: PassFingerprint,
        // References are not yet stored in FileSymbolIndex; placeholder
    },
    /// Diagnostics from any pass.
    Diagnostics {
        fingerprint: PassFingerprint,
        diagnostics: Vec<Diagnostic>,
    },
    /// Comments and ignore directives.
    Comments {
        fingerprint: PassFingerprint,
        // Ignore directives are ephemeral; placeholder for future use
    },
    /// Project graph snapshot.
    ProjectGraph {
        fingerprint: PassFingerprint,
        // Project graphs are not yet implemented; placeholder
    },
}

/// Reason why a cache entry was invalidated or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheInvalidationReason {
    SourceChanged,
    QueryChanged,
    GrammarChanged,
    RuleMetadataChanged,
    ConfigChanged,
    ExternalToolChanged,
    CorruptEntry,
    PassOptionsChanged,
    CacheDisabled,
}

impl std::fmt::Display for CacheInvalidationReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::SourceChanged => "source changed",
            Self::QueryChanged => "query changed",
            Self::GrammarChanged => "grammar changed",
            Self::RuleMetadataChanged => "rule metadata changed",
            Self::ConfigChanged => "config changed",
            Self::ExternalToolChanged => "external tool changed",
            Self::CorruptEntry => "corrupt entry",
            Self::PassOptionsChanged => "pass options changed",
            Self::CacheDisabled => "cache disabled",
        };
        formatter.write_str(label)
    }
}

/// Information about a cache hit or miss, including timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHitInfo {
    pub hit: bool,
    pub reason: Option<CacheInvalidationReason>,
    pub elapsed_ms: u64,
    pub pass: AnalysisPass,
}

impl CacheHitInfo {
    /// Creates a hit record with the given timing.
    pub fn hit(pass: AnalysisPass, elapsed: Duration) -> Self {
        Self {
            hit: true,
            reason: None,
            elapsed_ms: elapsed.as_millis() as u64,
            pass,
        }
    }

    /// Creates a miss record with the given reason.
    pub fn miss(pass: AnalysisPass, reason: CacheInvalidationReason, elapsed: Duration) -> Self {
        Self {
            hit: false,
            reason: Some(reason),
            elapsed_ms: elapsed.as_millis() as u64,
            pass,
        }
    }
}

/// Lightweight in-process cache for parsed file artifacts.
///
/// Caches parse trees, symbol records, and diagnostics keyed by file hash
/// and pass fingerprint. This cache is scoped to a single command invocation
/// and is discarded when the process exits.
#[derive(Debug)]
pub struct InProcessCache {
    parse_trees: Mutex<HashMap<String, Arc<ParsedFileSnapshot>>>,
    symbol_indexes: Mutex<HashMap<String, Arc<FileSymbolIndex>>>,
    hit_info: Mutex<Vec<CacheHitInfo>>,
    capacity: usize,
}

/// Snapshot of a parsed file, suitable for caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFileSnapshot {
    pub file_path: PathBuf,
    pub language: ProgrammingLanguage,
    pub source_hash: String,
    pub grammar_id: String,
    pub source_text: String,
}

/// Cache usage counters and timing stats.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub parse_hits: u64,
    pub parse_misses: u64,
    pub symbol_hits: u64,
    pub symbol_misses: u64,
    pub diagnostic_hits: u64,
    pub diagnostic_misses: u64,
    pub entries: usize,
}

/// Configuration controlling cache behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheConfig {
    pub enabled: bool,
    pub persistent: bool,
    pub capacity: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            persistent: false,
            capacity: 256,
        }
    }
}

impl InProcessCache {
    /// Creates a new in-process cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            parse_trees: Mutex::new(HashMap::new()),
            symbol_indexes: Mutex::new(HashMap::new()),
            hit_info: Mutex::new(Vec::new()),
            capacity: capacity.max(1),
        }
    }

    /// Creates a cache configured for the given settings.
    pub fn with_config(config: CacheConfig) -> Self {
        Self::new(config.capacity)
    }

    /// Looks up a parsed file snapshot by key.
    pub fn get_parse_tree(&self, key: &str) -> Option<Arc<ParsedFileSnapshot>> {
        let guard = self.parse_trees.lock().ok()?;
        guard.get(key).cloned()
    }

    /// Stores a parsed file snapshot.
    pub fn put_parse_tree(&self, key: String, snapshot: ParsedFileSnapshot) {
        let mut guard = match self.parse_trees.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if guard.len() >= self.capacity
            && let Some(first_key) = guard.keys().next().cloned()
        {
            guard.remove(&first_key);
        }
        guard.insert(key, Arc::new(snapshot));
    }

    /// Looks up a symbol index by key.
    pub fn get_symbol_index(&self, key: &str) -> Option<Arc<FileSymbolIndex>> {
        let guard = self.symbol_indexes.lock().ok()?;
        guard.get(key).cloned()
    }

    /// Stores a symbol index.
    pub fn put_symbol_index(&self, key: String, index: FileSymbolIndex) {
        let mut guard = match self.symbol_indexes.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if guard.len() >= self.capacity
            && let Some(first_key) = guard.keys().next().cloned()
        {
            guard.remove(&first_key);
        }
        guard.insert(key, Arc::new(index));
    }

    /// Records a cache hit/miss event.
    pub fn record_hit(&self, info: CacheHitInfo) {
        if let Ok(mut guard) = self.hit_info.lock() {
            guard.push(info);
        }
    }

    /// Returns current cache counters.
    pub fn stats(&self) -> CacheStats {
        let parse_guard = match self.parse_trees.lock() {
            Ok(guard) => guard,
            Err(_) => return CacheStats::default(),
        };
        let symbol_guard = match self.symbol_indexes.lock() {
            Ok(guard) => guard,
            Err(_) => return CacheStats::default(),
        };

        let hit_guard = match self.hit_info.lock() {
            Ok(guard) => guard,
            Err(_) => return CacheStats::default(),
        };

        let mut stats = CacheStats {
            entries: parse_guard.len() + symbol_guard.len(),
            ..Default::default()
        };

        for info in hit_guard.iter() {
            match info.pass {
                AnalysisPass::Parse => {
                    if info.hit {
                        stats.parse_hits += 1;
                    } else {
                        stats.parse_misses += 1;
                    }
                }
                AnalysisPass::Bind => {
                    if info.hit {
                        stats.symbol_hits += 1;
                    } else {
                        stats.symbol_misses += 1;
                    }
                }
                AnalysisPass::Semantic | AnalysisPass::Docs => {
                    if info.hit {
                        stats.diagnostic_hits += 1;
                    } else {
                        stats.diagnostic_misses += 1;
                    }
                }
            }
        }

        stats
    }

    /// Returns recorded hit info for debugging.
    pub fn hit_history(&self) -> Vec<CacheHitInfo> {
        match self.hit_info.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Clears all cached entries.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.parse_trees.lock() {
            guard.clear();
        }
        if let Ok(mut guard) = self.symbol_indexes.lock() {
            guard.clear();
        }
        if let Ok(mut guard) = self.hit_info.lock() {
            guard.clear();
        }
    }
}

/// Persistent on-disk cache for cross-invocation caching.
///
/// Stores serialized cache entries under a tool-owned user cache directory.
/// Entries are keyed by project identity and file fingerprint. Any mismatch
/// in the fingerprint causes recomputation.
#[derive(Debug)]
pub struct PersistentCache {
    cache_dir: PathBuf,
    enabled: bool,
}

impl PersistentCache {
    /// Opens or creates the persistent cache at the given directory.
    pub fn open(cache_dir: PathBuf, enabled: bool) -> Self {
        if enabled && !cache_dir.exists() {
            let _ = std::fs::create_dir_all(&cache_dir);
        }
        Self {
            cache_dir,
            enabled,
        }
    }

    /// Opens the default user cache directory for tree-hugger.
    pub fn default_cache(project_id: &str, enabled: bool) -> Self {
        let cache_dir = std::env::temp_dir().join("tree-hugger").join("cache").join(project_id);
        Self::open(cache_dir, enabled)
    }

    /// Reads a cached entry by key.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        if !self.enabled {
            return None;
        }
        let path = self.entry_path(key);
        let bytes = std::fs::read(&path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Writes a cache entry.
    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), CacheError> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.entry_path(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(value)?;
        std::fs::write(&path, bytes)?;
        Ok(())
    }

    /// Invalidates a specific entry.
    pub fn invalidate(&self, key: &str) {
        let path = self.entry_path(key);
        let _ = std::fs::remove_file(&path);
    }

    /// Invalidates all entries.
    pub fn invalidate_all(&self) {
        let _ = std::fs::remove_dir_all(&self.cache_dir);
    }

    fn entry_path(&self, key: &str) -> PathBuf {
        // Shard by first two chars of key to avoid too many files in one dir
        let shard = if key.len() >= 2 {
            &key[..2]
        } else {
            "00"
        };
        self.cache_dir.join(shard).join(format!("{key}.json"))
    }
}

/// Errors that can occur when interacting with the persistent cache.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Cached file-level snapshot (L2 foundation, backward-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSnapshot {
    pub key: FileCacheKey,
    pub completed_passes: Vec<AnalysisPass>,
    pub symbols: Vec<SymbolRecord>,
    pub imports: Vec<ImportRecord>,
    pub exports: Vec<ExportRecord>,
    pub diagnostics: Vec<Diagnostic>,
    pub created_at_epoch_ms: u64,
}

impl From<FileSymbolIndex> for SymbolSnapshot {
    fn from(index: FileSymbolIndex) -> Self {
        let now = crate::shared::now_epoch_ms();
        Self {
            key: FileCacheKey {
                file_path: index.file,
                language: index.language,
                file_hash: index.file_hash,
                analyzer_fingerprint: AnalyzerFingerprint {
                    schema_version: index.schema_version,
                    tree_hugger_version: env!("CARGO_PKG_VERSION").to_string(),
                    grammar_fingerprint: "tree-sitter-unknown".to_string(),
                    query_fingerprint: "query-fingerprint-unknown".to_string(),
                    config_fingerprint: "default".to_string(),
                },
            },
            completed_passes: index.completed_passes,
            symbols: index.symbols,
            imports: index.imports,
            exports: index.exports,
            diagnostics: index.diagnostics,
            created_at_epoch_ms: now,
        }
    }
}

impl From<SymbolSnapshot> for FileSymbolIndex {
    fn from(snapshot: SymbolSnapshot) -> Self {
        Self {
            schema_version: snapshot.key.analyzer_fingerprint.schema_version,
            file: snapshot.key.file_path,
            language: snapshot.key.language,
            file_hash: snapshot.key.file_hash,
            completed_passes: snapshot.completed_passes,
            symbols: snapshot.symbols,
            imports: snapshot.imports,
            exports: snapshot.exports,
            diagnostics: snapshot.diagnostics,
        }
    }
}

/// Lightweight in-memory cache for symbol snapshots (backward-compatible).
#[derive(Debug)]
pub struct InMemorySymbolCache {
    capacity: usize,
    state: Mutex<CacheState>,
}

#[derive(Debug, Default)]
struct CacheState {
    entries: HashMap<String, Arc<SymbolSnapshot>>,
    hits: u64,
    misses: u64,
}

impl InMemorySymbolCache {
    /// Creates a cache with bounded entry count.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Mutex::new(CacheState::default()),
        }
    }

    /// Gets a snapshot by key.
    pub fn get(&self, key: &FileCacheKey) -> Option<Arc<SymbolSnapshot>> {
        let stable = key.stable_key();
        let mut guard = match self.state.lock() {
            Ok(guard) => guard,
            Err(_) => return None,
        };

        if let Some(snapshot) = guard.entries.get(&stable).cloned() {
            guard.hits += 1;
            Some(snapshot)
        } else {
            guard.misses += 1;
            None
        }
    }

    /// Inserts or replaces a snapshot.
    pub fn put(&self, snapshot: SymbolSnapshot) {
        let key = snapshot.key.stable_key();
        let mut guard = match self.state.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        if guard.entries.len() >= self.capacity
            && let Some(first_key) = guard.entries.keys().next().cloned()
        {
            guard.entries.remove(&first_key);
        }

        guard.entries.insert(key, Arc::new(snapshot));
    }

    /// Clears all cached snapshots.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.state.lock() {
            guard.entries.clear();
        }
    }

    /// Returns current cache counters.
    pub fn stats(&self) -> CacheStats {
        match self.state.lock() {
            Ok(guard) => CacheStats {
                entries: guard.entries.len(),
                parse_hits: guard.hits,
                parse_misses: guard.misses,
                ..Default::default()
            },
            Err(_) => CacheStats::default(),
        }
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Computes a fingerprint for a query source.
pub fn query_fingerprint(query_source: &str) -> String {
    blake3_hash(query_source)
}

/// Computes a fingerprint for a set of enabled rules.
pub fn enabled_rules_fingerprint(rules: &[String]) -> String {
    let canonical = rules.join(",");
    blake3_hash(&canonical)
}

/// Computes a fingerprint for rule metadata.
pub fn rule_metadata_fingerprint(registry_hash: &str) -> String {
    blake3_hash(registry_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fingerprint() -> AnalyzerFingerprint {
        AnalyzerFingerprint {
            schema_version: SchemaVersion::V2_0,
            tree_hugger_version: "0.1.0".to_string(),
            grammar_fingerprint: "g1".to_string(),
            query_fingerprint: "q1".to_string(),
            config_fingerprint: "c1".to_string(),
        }
    }

    fn sample_key() -> FileCacheKey {
        FileCacheKey {
            file_path: PathBuf::from("src/lib.rs"),
            language: ProgrammingLanguage::Rust,
            file_hash: "abc".to_string(),
            analyzer_fingerprint: sample_fingerprint(),
        }
    }

    #[test]
    fn cache_key_is_deterministic() {
        let first = sample_key().stable_key();
        let second = sample_key().stable_key();
        assert_eq!(first, second);
    }

    #[test]
    fn cache_key_changes_with_fingerprint_mutation() {
        let base = sample_key().stable_key();
        let mut key = sample_key();
        key.analyzer_fingerprint.query_fingerprint = "q2".to_string();
        assert_ne!(base, key.stable_key());
    }

    #[test]
    fn in_memory_cache_hit_miss_tracking() {
        let cache = InMemorySymbolCache::new(8);
        let key = sample_key();

        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().parse_misses, 1);

        let snapshot = SymbolSnapshot {
            key: key.clone(),
            completed_passes: vec![AnalysisPass::Parse],
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            diagnostics: Vec::new(),
            created_at_epoch_ms: 0,
        };
        cache.put(snapshot);

        assert!(cache.get(&key).is_some());
        let stats = cache.stats();
        assert_eq!(stats.parse_hits, 1);
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn pass_fingerprint_digest_is_deterministic() {
        let fp = PassFingerprint {
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
        let first = fp.digest();
        let second = fp.digest();
        assert_eq!(first, second);
    }

    #[test]
    fn pass_fingerprint_changes_with_tool_version() {
        let base = PassFingerprint {
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
        let mut changed = base.clone();
        changed.external_tool_version = Some("2.0.0".to_string());
        assert_ne!(base.digest(), changed.digest());
    }

    #[test]
    fn in_process_cache_tracks_hits() {
        let cache = InProcessCache::new(8);
        let key = "test_key";
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
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn cache_invalidation_reason_display() {
        assert_eq!(
            format!("{}", CacheInvalidationReason::SourceChanged),
            "source changed"
        );
        assert_eq!(
            format!("{}", CacheInvalidationReason::CorruptEntry),
            "corrupt entry"
        );
    }

    #[test]
    fn cache_hit_info_serialization() {
        let info = CacheHitInfo::hit(AnalysisPass::Parse, Duration::from_millis(5));
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"hit\":true"));
        assert!(json.contains("\"pass\":\"Parse\""));
    }

    #[test]
    fn persistent_cache_disabled_returns_none() {
        let cache = PersistentCache::open(PathBuf::from("/tmp/test-cache-5"), false);
        assert!(cache.get::<SymbolSnapshot>("key").is_none());
    }

    #[test]
    fn query_fingerprint_is_deterministic() {
        let fp1 = query_fingerprint("(identifier) @id");
        let fp2 = query_fingerprint("(identifier) @id");
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn enabled_rules_fingerprint_is_deterministic() {
        let rules = vec!["unwrap-call".to_string(), "dbg-macro".to_string()];
        let fp1 = enabled_rules_fingerprint(&rules);
        let fp2 = enabled_rules_fingerprint(&rules);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn enabled_rules_fingerprint_changes_with_order() {
        let fp1 = enabled_rules_fingerprint(&[
            "unwrap-call".to_string(),
            "dbg-macro".to_string(),
        ]);
        let fp2 = enabled_rules_fingerprint(&[
            "dbg-macro".to_string(),
            "unwrap-call".to_string(),
        ]);
        // Order matters because it's a comma-joined string
        assert_ne!(fp1, fp2);
    }
}
