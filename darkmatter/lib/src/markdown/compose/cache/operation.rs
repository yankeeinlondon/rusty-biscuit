//! Operation-level caching with parameter buckets.
//!
//! Formalizes the conditional/pre/variant/post parameter model for
//! cacheable compose operations. Variant parameters determine the
//! cached core result; post parameters are applied after cache lookup.

// `CodeOperation` and `TocLinkingOperation` are wired into the compose
// pipeline today. `FileOperation` remains available as the bucket model for
// future compose-core refactoring, so this module still carries some unused
// items in the current build.
#![allow(dead_code)]

use biscuit_hash::xx_hash;
use serde_json::Value;
use std::path::Path;

use super::hashing::{canonical_json_sorted, compose_cache_key, operation_entry_key};
use super::types::ArtifactClass;
use crate::markdown::compose::toc_linking::TocLinkingOptions;
use crate::markdown::compose::transclusion::{BlockOptions, ReplaceOption};

/// Classification of directive parameters into cache-relevant buckets.
///
/// - **conditional**: Control whether the operation runs (e.g., `when`).
/// - **pre**: Applied before core computation.
/// - **variant**: Determine the cached result — different variant params produce different entries.
/// - **post**: Applied after cache lookup (cheap transforms like wrappers).
#[derive(Debug, Clone, Default)]
pub(crate) struct ParamBuckets {
    pub conditional: Vec<(String, String)>,
    pub pre: Vec<(String, String)>,
    pub variant: Vec<(String, String)>,
    pub post: Vec<(String, String)>,
}

impl ParamBuckets {
    /// Computes a hash of the variant parameters for use in cache keys.
    pub fn variant_hash(&self) -> u64 {
        if self.variant.is_empty() {
            return 0;
        }
        let parts: Vec<String> = self
            .variant
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        xx_hash(&parts.join("\0"))
    }
}

/// A compose operation whose core result can be cached.
pub(crate) trait CacheableOperation {
    /// Classifies directive options into parameter buckets.
    fn split_params(&self, options: &BlockOptions) -> ParamBuckets;

    /// Computes a variant cache key from source identity and parameter buckets.
    fn variant_cache_key(&self, source_id: u64, buckets: &ParamBuckets) -> u64;

    /// Returns the artifact class for persistent storage.
    fn artifact_class(&self) -> ArtifactClass;
}

// ── File operation ───────────────────────────────────────────────────

/// `::file` markdown transclusion operation.
pub(crate) struct FileOperation;

impl CacheableOperation for FileOperation {
    fn split_params(&self, options: &BlockOptions) -> ParamBuckets {
        let mut buckets = ParamBuckets::default();

        if let Some(when) = &options.when_expr {
            buckets.conditional.push(("when".to_string(), when.clone()));
        }

        // Variant: replace behavior affects the composed core
        match &options.replace {
            ReplaceOption::InheritDefault => {
                buckets
                    .variant
                    .push(("replace".to_string(), "inherit".to_string()));
            }
            ReplaceOption::ParentWins => {
                buckets
                    .variant
                    .push(("replace".to_string(), "parent_wins".to_string()));
            }
            ReplaceOption::OneOff(map) => {
                let canonical = canonical_json_sorted(&Value::Object(map.clone()));
                buckets.variant.push(("replace".to_string(), canonical));
            }
        }

        // Post: applied after cache lookup
        for exclude in &options.exclude {
            buckets.post.push(("exclude".to_string(), exclude.clone()));
        }
        if let Some(q) = &options.quotation {
            buckets.post.push(("quotation".to_string(), q.clone()));
        }
        if let Some(d) = &options.disclosure {
            buckets.post.push(("disclosure".to_string(), d.clone()));
        }

        buckets
    }

    fn variant_cache_key(&self, source_id: u64, buckets: &ParamBuckets) -> u64 {
        operation_entry_key("file", source_id, buckets.variant_hash())
    }

    fn artifact_class(&self) -> ArtifactClass {
        ArtifactClass::ComposeDocumentCore
    }
}

// ── Code operation ───────────────────────────────────────────────────

/// `::code` source-code transclusion operation.
pub(crate) struct CodeOperation;

impl CacheableOperation for CodeOperation {
    fn split_params(&self, options: &BlockOptions) -> ParamBuckets {
        let mut buckets = ParamBuckets::default();

        if let Some(when) = &options.when_expr {
            buckets.conditional.push(("when".to_string(), when.clone()));
        }

        // Variant: replace map and language affect the fenced core
        match &options.replace {
            ReplaceOption::InheritDefault => {
                buckets
                    .variant
                    .push(("replace".to_string(), "inherit".to_string()));
            }
            ReplaceOption::ParentWins => {
                buckets
                    .variant
                    .push(("replace".to_string(), "parent_wins".to_string()));
            }
            ReplaceOption::OneOff(map) => {
                let canonical = canonical_json_sorted(&Value::Object(map.clone()));
                buckets.variant.push(("replace".to_string(), canonical));
            }
        }

        // Post: wrappers
        if let Some(q) = &options.quotation {
            buckets.post.push(("quotation".to_string(), q.clone()));
        }
        if let Some(d) = &options.disclosure {
            buckets.post.push(("disclosure".to_string(), d.clone()));
        }

        buckets
    }

    fn variant_cache_key(&self, source_id: u64, buckets: &ParamBuckets) -> u64 {
        operation_entry_key("code", source_id, buckets.variant_hash())
    }

    fn artifact_class(&self) -> ArtifactClass {
        ArtifactClass::OperationResult
    }
}

// ── TOC linking operation ────────────────────────────────────────────

/// `::toc-linking` TOC link generation operation.
///
/// Since `::toc-linking` uses `TocLinkingOptions` rather than `BlockOptions`,
/// cache key computation is provided via standalone methods instead of the
/// `CacheableOperation` trait.
pub(crate) struct TocLinkingOperation;

impl TocLinkingOperation {
    /// Classifies TOC linking options into cache-relevant buckets.
    pub fn split_params(options: &TocLinkingOptions) -> ParamBuckets {
        let mut buckets = ParamBuckets::default();

        let mut levels: Vec<u8> = options.levels.levels.iter().map(|l| l.as_u8()).collect();
        levels.sort();
        buckets
            .variant
            .push(("levels".to_string(), format!("{:?}", levels)));

        let cleanups: Vec<String> = options
            .cleanup_services
            .iter()
            .map(|c| format!("{:?}", c))
            .collect();
        buckets
            .variant
            .push(("cleanup".to_string(), cleanups.join(",")));

        for p in &options.keep_patterns {
            buckets.variant.push((
                "keep".to_string(),
                format!("{}:{}", p.case_sensitive, p.pattern),
            ));
        }
        for p in &options.filter_patterns {
            buckets.variant.push((
                "filter".to_string(),
                format!("{}:{}", p.case_sensitive, p.pattern),
            ));
        }

        if let Some(text) = &options.empty_text {
            buckets.variant.push(("empty".to_string(), text.clone()));
        }

        buckets
    }

    /// Computes a variant cache key from source identity and parameter buckets.
    pub fn variant_cache_key(source_id: u64, buckets: &ParamBuckets) -> u64 {
        operation_entry_key("toc-linking", source_id, buckets.variant_hash())
    }

    /// Computes a string cache key for a TOC linking operation.
    pub fn cache_key_string(path: &Path, options: &TocLinkingOptions) -> String {
        let source_key = compose_cache_key(path);
        let source_id = xx_hash(&source_key);
        let buckets = Self::split_params(options);
        let entry_key = Self::variant_cache_key(source_id, &buckets);
        format!("toc:{}:{:016x}", source_key, entry_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;

    #[test]
    fn code_split_params_variant_includes_replace() {
        let op = CodeOperation;
        let mut one_off = Map::new();
        one_off.insert("foo".into(), Value::from("bar"));

        let options = BlockOptions {
            replace: ReplaceOption::OneOff(one_off),
            quotation: Some("Author".to_string()),
            ..Default::default()
        };

        let buckets = op.split_params(&options);
        assert_eq!(buckets.variant.len(), 1);
        assert_eq!(buckets.variant[0].0, "replace");
        assert!(buckets.variant[0].1.contains("foo"));

        assert_eq!(buckets.post.len(), 1);
        assert_eq!(buckets.post[0].0, "quotation");
    }

    #[test]
    fn code_split_params_conditional_when() {
        let op = CodeOperation;
        let options = BlockOptions {
            when_expr: Some("env.CI == true".to_string()),
            ..Default::default()
        };

        let buckets = op.split_params(&options);
        assert_eq!(buckets.conditional.len(), 1);
        assert_eq!(buckets.conditional[0].1, "env.CI == true");
    }

    #[test]
    fn file_split_params_post_includes_exclude_and_wrappers() {
        let op = FileOperation;
        let options = BlockOptions {
            exclude: vec!["Examples".to_string()],
            quotation: Some("".to_string()),
            disclosure: Some("Details".to_string()),
            ..Default::default()
        };

        let buckets = op.split_params(&options);
        assert_eq!(buckets.post.len(), 3);
        let post_keys: Vec<&str> = buckets.post.iter().map(|(k, _)| k.as_str()).collect();
        assert!(post_keys.contains(&"exclude"));
        assert!(post_keys.contains(&"quotation"));
        assert!(post_keys.contains(&"disclosure"));
    }

    #[test]
    fn variant_hash_deterministic() {
        let mut buckets = ParamBuckets::default();
        buckets
            .variant
            .push(("replace".to_string(), "inherit".to_string()));

        let h1 = buckets.variant_hash();
        let h2 = buckets.variant_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn variant_hash_sensitive_to_values() {
        let mut b1 = ParamBuckets::default();
        b1.variant
            .push(("replace".to_string(), "inherit".to_string()));

        let mut b2 = ParamBuckets::default();
        b2.variant
            .push(("replace".to_string(), "parent_wins".to_string()));

        assert_ne!(b1.variant_hash(), b2.variant_hash());
    }

    #[test]
    fn variant_cache_key_different_per_operation() {
        let code_op = CodeOperation;
        let file_op = FileOperation;
        let buckets = ParamBuckets::default();

        let code_key = code_op.variant_cache_key(12345, &buckets);
        let file_key = file_op.variant_cache_key(12345, &buckets);
        assert_ne!(code_key, file_key);
    }

    #[test]
    fn toc_cache_key_deterministic() {
        let path = Path::new("/tmp/test.md");
        let options = TocLinkingOptions::default();

        let k1 = TocLinkingOperation::cache_key_string(path, &options);
        let k2 = TocLinkingOperation::cache_key_string(path, &options);
        assert_eq!(k1, k2);
    }

    #[test]
    fn toc_cache_key_sensitive_to_options() {
        let path = Path::new("/tmp/test.md");

        // Default options
        let opts1 = TocLinkingOptions::default();

        // Modified empty_text (accessible without private types)
        let opts2 = TocLinkingOptions {
            empty_text: Some("nothing here".to_string()),
            ..Default::default()
        };

        assert_ne!(
            TocLinkingOperation::cache_key_string(path, &opts1),
            TocLinkingOperation::cache_key_string(path, &opts2),
        );
    }

    #[test]
    fn code_cache_key_deterministic() {
        let path = Path::new("/tmp/main.rs");
        let op = CodeOperation;
        let options = BlockOptions::default();
        let source_id = xx_hash(&compose_cache_key(path));
        let mut buckets = op.split_params(&options);
        buckets
            .variant
            .push(("language".to_string(), "rust".to_string()));

        let k1 = op.variant_cache_key(source_id, &buckets);
        let k2 = op.variant_cache_key(source_id, &buckets);
        assert_eq!(k1, k2);
    }

    #[test]
    fn code_cache_key_sensitive_to_language() {
        let path = Path::new("/tmp/main.rs");
        let op = CodeOperation;
        let options = BlockOptions::default();
        let source_id = xx_hash(&compose_cache_key(path));
        let mut rust_buckets = op.split_params(&options);
        rust_buckets
            .variant
            .push(("language".to_string(), "rust".to_string()));
        let mut python_buckets = op.split_params(&options);
        python_buckets
            .variant
            .push(("language".to_string(), "python".to_string()));

        assert_ne!(
            op.variant_cache_key(source_id, &rust_buckets),
            op.variant_cache_key(source_id, &python_buckets),
        );
    }

    #[test]
    fn code_cache_key_sensitive_to_replace_map() {
        let path = Path::new("/tmp/main.rs");
        let op = CodeOperation;
        let source_id = xx_hash(&compose_cache_key(path));

        let options1 = BlockOptions::default();
        let mut options2 = BlockOptions::default();
        let mut replace_map = Map::new();
        replace_map.insert("key".into(), Value::from("value"));
        options2.replace = ReplaceOption::OneOff(replace_map);
        let mut buckets1 = op.split_params(&options1);
        buckets1
            .variant
            .push(("language".to_string(), "rust".to_string()));
        let mut buckets2 = op.split_params(&options2);
        buckets2
            .variant
            .push(("language".to_string(), "rust".to_string()));

        assert_ne!(
            op.variant_cache_key(source_id, &buckets1),
            op.variant_cache_key(source_id, &buckets2),
        );
    }
}
