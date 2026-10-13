//! The cache rows: a file reached through a changed-file scan and an
//! ordinary scan shares one persistent cache entry — the selection never
//! enters the cache key.

use crate::{Repo, cache_stats, json_of};

/// A repository whose only changed file is `lib.rs`, edited after the
/// initial commit.
fn changed_fixture() -> Repo {
    let repo = Repo::committed(&[("lib.rs", "fn one() {}\n")]);
    repo.write("lib.rs", "fn one() {}\nfn two() {}\n");
    repo
}

#[test]
fn an_ordinary_scan_then_a_changed_scan_hits_the_same_entry() {
    let repo = changed_fixture();

    // Cold ordinary scan: one parse, one miss.
    let cold_args = ["--verbose", "--json", "symbols", "lib.rs"];
    let cold = repo.hug(&cold_args);
    assert!(cold.status.success(), "cold ordinary scan failed");
    assert!(
        cache_stats(&cold).starts_with("cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"),
        "{}",
        cache_stats(&cold)
    );

    // The changed scan reads the same file from the cache: a complete hit
    // that parses nothing.
    let warm_args = ["--verbose", "--json", "symbols", "--changed"];
    let warm = repo.hug(&warm_args);
    assert!(warm.status.success(), "warm changed scan failed");
    assert!(
        cache_stats(&warm).starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"),
        "{}",
        cache_stats(&warm)
    );

    // And the two runs report the same records with the same identities.
    let ordinary = json_of(&String::from_utf8(cold.stdout).unwrap());
    let changed = json_of(&String::from_utf8(warm.stdout).unwrap());
    assert_eq!(ordinary["files"], changed["files"]);
    assert_eq!(changed["errors"], serde_json::json!([]));
    let repeated = repo.hug(&warm_args);
    assert!(repeated.status.success());
    assert!(cache_stats(&repeated).starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"));
    assert_eq!(json_of(&String::from_utf8(repeated.stdout).unwrap())["files"], changed["files"]);
}

#[test]
fn a_changed_scan_then_an_ordinary_scan_hits_the_same_entry() {
    let repo = changed_fixture();
    // A fresh cache directory, so the changed scan below is the cold one.
    let cold_cache = tempfile::TempDir::new().unwrap();

    let cold_args = ["--verbose", "--json", "symbols", "--changed"];
    let cold = repo.hug_with_temp(cold_cache.path(), &cold_args);
    assert!(cold.status.success(), "cold changed scan failed");
    assert!(
        cache_stats(&cold).starts_with("cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"),
        "{}",
        cache_stats(&cold)
    );

    let warm_args = ["--verbose", "--json", "symbols", "lib.rs"];
    let warm = repo.hug_with_temp(cold_cache.path(), &warm_args);
    assert!(warm.status.success(), "warm ordinary scan failed");
    assert!(
        cache_stats(&warm).starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"),
        "{}",
        cache_stats(&warm)
    );

    // Same records again: the entry one scan warmed, the other reused.
    let changed = json_of(&String::from_utf8(cold.stdout).unwrap());
    let ordinary = json_of(&String::from_utf8(warm.stdout).unwrap());
    assert_eq!(changed["files"], ordinary["files"]);
    assert_eq!(ordinary["errors"], serde_json::json!([]));
    let repeated = repo.hug_with_temp(cold_cache.path(), &warm_args);
    assert!(repeated.status.success());
    assert!(cache_stats(&repeated).starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"));
    assert_eq!(json_of(&String::from_utf8(repeated.stdout).unwrap())["files"], ordinary["files"]);
}
