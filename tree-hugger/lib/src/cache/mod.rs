use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_hash::blake3_hash;
use serde::{Deserialize, Serialize};

use crate::shared::{
    AnalysisPass, Diagnostic, ExportRecord, FileSymbolIndex, ImportRecord, ProgrammingLanguage,
    SchemaVersion, SymbolRecord,
};

/// Fingerprint dimensions that determine cache validity.
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

/// Key for per-file cache snapshots.
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

/// Cached file-level snapshot (L2 foundation).
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

/// Lightweight in-memory cache for symbol snapshots.
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

/// Basic cache usage counters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
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
                hits: guard.hits,
                misses: guard.misses,
            },
            Err(_) => CacheStats {
                entries: 0,
                hits: 0,
                misses: 0,
            },
        }
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
        assert_eq!(cache.stats().misses, 1);

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
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.entries, 1);
    }
}
