//! Integration tests for the unified offline catalog.

use std::collections::BTreeSet;

use biscuit_icon::cache::{IconCache, SetInfo};
use biscuit_icon::IconBody;
use biscuit_icon::catalog;

#[test]
fn offline_icons_merges_builtin_and_cached_ids() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
    cache.put("mdi", "home-automation", &IconBody::new("<a/>", 24, 24)).unwrap();

    let hits = catalog::offline_icons(&cache, "apple", &BTreeSet::new()).unwrap();
    assert!(hits.iter().any(|id| id == "ic:baseline-apple"));

    let hits = catalog::offline_icons(&cache, "home", &BTreeSet::new()).unwrap();
    assert!(hits.iter().any(|id| id == "mdi:home-automation"));
}

#[test]
fn offline_icons_honors_from_prefix_filter() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
    cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();
    cache.put("ic", "baseline-apple", &IconBody::new("<b/>", 24, 24)).unwrap();

    let allowed: BTreeSet<String> = ["ic".to_string()].into_iter().collect();
    let hits = catalog::offline_icons(&cache, "home", &allowed).unwrap();
    assert!(hits.iter().all(|id| id.starts_with("ic:")));
}

#[test]
fn offline_sets_includes_builtin_prefixes_and_cached_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
    cache
        .put_set(&SetInfo {
            prefix: "lucide".into(),
            title: "Lucide Icons".into(),
            license: Some("ISC".into()),
        })
        .unwrap();

    let hits = catalog::offline_sets(&cache, "").unwrap();
    assert!(hits.iter().any(|s| s.prefix == "ic"));
    assert!(hits.iter().any(|s| s.prefix == "lucide" && s.title == "Lucide Icons"));
}
