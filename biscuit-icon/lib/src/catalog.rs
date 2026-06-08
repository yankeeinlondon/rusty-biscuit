//! Unified offline catalog: built-in domain icons plus the local SQLite cache.

use std::collections::{BTreeMap, BTreeSet};

use crate::cache::{IconCache, SetInfo};
use crate::error::Result;

/// Returns `prefix:name` identifiers from the built-in domain catalog and the
/// local cache that match `needle`. When `allowed_prefixes` is non-empty, only
/// identifiers whose prefix is in the set are returned. Results are
/// deduplicated and sorted.
///
/// ## Errors
/// [`IconError::Cache`] on SQLite failure.
pub fn offline_icons(cache: &IconCache, needle: &str, allowed_prefixes: &BTreeSet<String>) -> Result<Vec<String>> {
    let mut ids = BTreeSet::new();

    for id in crate::domain::all_iconify_ids() {
        if id_contains(id, needle) && prefix_allowed(id, allowed_prefixes) {
            ids.insert(id.to_string());
        }
    }

    for id in cache.search_names(needle)? {
        if prefix_allowed(&id, allowed_prefixes) {
            ids.insert(id);
        }
    }

    Ok(ids.into_iter().collect())
}

/// Returns set metadata from the built-in domain catalog and the local cache.
/// When `needle` is non-empty, filters by prefix/title substring. Results are
/// deduplicated by prefix.
///
/// ## Errors
/// [`IconError::Cache`] on SQLite failure.
pub fn offline_sets(cache: &IconCache, needle: &str) -> Result<Vec<SetInfo>> {
    let mut out: BTreeMap<String, SetInfo> = BTreeMap::new();

    // Derive built-in set prefixes from curated domain ids.
    for id in crate::domain::all_iconify_ids() {
        if let Some((prefix, _)) = id.split_once(':') {
            let title = prefix.to_string();
            if needle.is_empty() || prefix.contains(needle) || title.contains(needle) {
                out.entry(prefix.to_string()).or_insert(SetInfo { prefix: prefix.to_string(), title, license: None, license_title: None, license_url: None });
            }
        }
    }

    for set in cache.search_sets(needle)? {
        out.insert(set.prefix.clone(), set);
    }

    if needle.is_empty() {
        for set in cache.all_sets()? {
            out.insert(set.prefix.clone(), set);
        }
    }

    Ok(out.into_values().collect())
}

fn id_contains(id: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    id.to_lowercase().contains(&needle.to_lowercase())
}

fn prefix_allowed(id: &str, allowed: &BTreeSet<String>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    id.split_once(':')
        .map(|(prefix, _)| allowed.contains(prefix))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_icons_includes_builtin_and_cached() {
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        cache.put("mdi", "home-automation", &crate::body::IconBody::new("<a/>", 24, 24)).unwrap();

        let hits = offline_icons(&cache, "apple", &BTreeSet::new()).unwrap();
        assert!(hits.iter().any(|id| id == "ic:baseline-apple"));

        let hits = offline_icons(&cache, "home", &BTreeSet::new()).unwrap();
        assert!(hits.iter().any(|id| id == "mdi:home-automation"));
    }

    #[test]
    fn offline_icons_filters_by_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        let allowed: BTreeSet<String> = ["ic".to_string()].into_iter().collect();
        let hits = offline_icons(&cache, "apple", &allowed).unwrap();
        assert!(hits.iter().all(|id| id.starts_with("ic:")));
    }
}
