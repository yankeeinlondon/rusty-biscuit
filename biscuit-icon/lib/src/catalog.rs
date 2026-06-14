//! Unified offline catalog: built-in domain icons plus the local SQLite cache.

use std::collections::{BTreeMap, BTreeSet};

use crate::cache::{CachedIcon, IconCache, SetInfo};
use crate::error::Result;
use crate::icon::{Icon, Source};

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

/// Suggests icon ids whose **name part** contains `needle` (case-insensitive).
///
/// Matches are drawn from the offline catalog (curated domain ids) and the
/// local cache; no network requests are made. Results are sorted and
/// deduplicated.
///
/// ## Errors
/// [`IconError::Cache`] on SQLite failure.
pub fn suggestions(cache: &IconCache, needle: &str) -> Result<Vec<String>> {
    let mut ids = BTreeSet::new();

    for id in crate::domain::all_iconify_ids() {
        if name_contains(id, needle) {
            ids.insert(id.to_string());
        }
    }

    // Cache.search_names already filters by the *name* part substring, so the
    // results fit the "name contains needle" rule directly.
    for id in cache.search_names(needle)? {
        ids.insert(id);
    }

    Ok(ids.into_iter().collect())
}

fn name_contains(id: &str, needle: &str) -> bool {
    let needle_lower = needle.to_lowercase();
    id.split_once(':')
        .map(|(_, name)| name.to_lowercase().contains(&needle_lower))
        .unwrap_or(false)
}

/// Returns set metadata from the built-in domain catalog and the local cache.
/// When `needle` is non-empty, filters by prefix/title substring. Results are
/// deduplicated by prefix.
///
/// ## Errors
/// [`IconError::Cache`] on SQLite failure.
pub fn offline_sets(cache: &IconCache, needle: &str) -> Result<Vec<SetInfo>> {
    let mut out: BTreeMap<String, SetInfo> = BTreeMap::new();

    for id in crate::domain::all_iconify_ids() {
        if let Some((prefix, _)) = id.split_once(':') {
            let title = prefix.to_string();
            if needle.is_empty() || prefix.contains(needle) || title.contains(needle) {
                out.entry(prefix.to_string()).or_insert(SetInfo { prefix: prefix.to_string(), title, license: None, license_title: None, license_url: None, total: None, author_name: None, author_url: None, tags: None, category: None });
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

/// Tabular metadata for an icon used by `--meta` and `cache list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconMeta {
    /// Human-readable set title, falling back to the prefix.
    pub set: String,
    /// The local icon name.
    pub icon: String,
    /// Curated enum-set name, or `"N/A"` for non-curated icons.
    pub categories: String,
    /// Set-level tags from the cache, empty when unknown.
    pub tags: String,
    /// Cached set author, when populated.
    pub author: Option<String>,
    /// Cached set license, when populated.
    pub license: Option<String>,
}

/// Resolves tabular metadata for `icon` using the cache for set-level fields.
///
/// ## Errors
/// [`IconError::Cache`] on SQLite failure.
pub fn icon_meta(cache: &IconCache, icon: &Icon) -> Result<IconMeta> {
    let id = icon.id();
    let (prefix, name) = id.split_once(':').unwrap_or((id, ""));

    let set_title = cache.set_title(prefix)?;
    let cached_set = cache
        .search_sets(prefix)?
        .into_iter()
        .find(|s| s.prefix == prefix);

    let categories = if icon.source() == Source::Embedded {
        crate::domain::domain_set_name_for_prefix(prefix)
            .map(String::from)
            .unwrap_or_else(|| "N/A".into())
    } else {
        "N/A".into()
    };

    Ok(IconMeta {
        set: set_title.unwrap_or_else(|| prefix.into()),
        icon: name.into(),
        categories,
        tags: cached_set.as_ref().and_then(|s| s.tags.clone()).unwrap_or_default(),
        author: cached_set.as_ref().and_then(|s| s.author_name.clone()),
        license: cached_set
            .as_ref()
            .and_then(|s| s.license.clone().filter(|l| !l.is_empty())),
    })
}

/// Resolves tabular metadata for a cached icon using the cache for set-level fields.
///
/// ## Errors
/// [`IconError::Cache`] on SQLite failure.
pub fn cached_icon_meta(cache: &IconCache, cached: &CachedIcon) -> Result<IconMeta> {
    let CachedIcon { prefix, name } = cached;

    let set_title = cache.set_title(prefix)?;
    let cached_set = cache
        .search_sets(prefix)?
        .into_iter()
        .find(|s| s.prefix == *prefix);

    let categories = crate::domain::domain_set_name_for_prefix(prefix)
        .map(String::from)
        .unwrap_or_else(|| "N/A".into());

    Ok(IconMeta {
        set: set_title.unwrap_or_else(|| prefix.clone()),
        icon: name.clone(),
        categories,
        tags: cached_set.as_ref().and_then(|s| s.tags.clone()).unwrap_or_default(),
        author: cached_set.as_ref().and_then(|s| s.author_name.clone()),
        license: cached_set
            .as_ref()
            .and_then(|s| s.license.clone().filter(|l| !l.is_empty())),
    })
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

    #[test]
    fn suggestions_match_name_substring() {
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        cache.put("mdi", "home-automation", &crate::body::IconBody::new("<a/>", 24, 24)).unwrap();
        cache.put("mdi", "alert", &crate::body::IconBody::new("<b/>", 24, 24)).unwrap();

        let hits = suggestions(&cache, "home").unwrap();
        assert!(hits.iter().any(|id| id == "mdi:home-automation"), "expected mdi:home-automation; got: {hits:?}");
        assert!(hits.iter().any(|id| id == "material-symbols:home"), "expected material-symbols:home; got: {hits:?}");
        assert!(!hits.iter().any(|id| id == "mdi:alert"), "alert should not match home; got: {hits:?}");
        assert!(!hits.iter().any(|id| id == "fluent-emoji-flat:grinning-face"), "grinning-face should not match home; got: {hits:?}");
    }

    #[test]
    fn suggestions_empty_for_nonsense_needle() {
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        let hits = suggestions(&cache, "zzzzzzzz").unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn icon_meta_for_curated_icon_uses_enum_set_category() {
        use crate::domain::DomainIcon;
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();

        let icon = crate::domain::Emoji::Happy.icon();
        let meta = icon_meta(&cache, &icon).unwrap();
        assert_eq!(meta.icon, "grinning-face");
        assert_eq!(meta.categories, "emoji");
    }

    #[test]
    fn icon_meta_for_network_icon_uses_cached_set_metadata() {
        use crate::body::IconBody;
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        cache
            .put_set(&SetInfo {
                prefix: "mdi".into(),
                title: "Material Design".into(),
                license: None,
                license_title: None,
                license_url: None,
                total: None,
                author_name: Some("Templarian".into()),
                author_url: None,
                tags: Some("ui, design".into()),
                category: None,
            })
            .unwrap();

        let icon = Icon::from_network("mdi:home", IconBody::new("<path/>", 24, 24));
        let meta = icon_meta(&cache, &icon).unwrap();
        assert_eq!(meta.set, "Material Design");
        assert_eq!(meta.icon, "home");
        assert_eq!(meta.categories, "N/A");
        assert_eq!(meta.tags, "ui, design");
        assert_eq!(meta.author, Some("Templarian".into()));
    }

    #[test]
    fn cached_icon_meta_falls_back_to_prefix_when_set_unknown() {
        use crate::body::IconBody;
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        cache.put("unknown-set", "settings", &IconBody::new("<a/>", 24, 24)).unwrap();

        let cached = crate::cache::CachedIcon { prefix: "unknown-set".into(), name: "settings".into() };
        let meta = cached_icon_meta(&cache, &cached).unwrap();
        assert_eq!(meta.set, "unknown-set");
        assert_eq!(meta.icon, "settings");
        assert_eq!(meta.categories, "N/A");
    }
}
