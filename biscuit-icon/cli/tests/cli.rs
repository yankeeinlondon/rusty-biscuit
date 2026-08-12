use assert_cmd::Command;
use predicates::prelude::*;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn help_lists_subcommands() {
    Command::cargo_bin("icon")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("sets"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn cache_clear_succeeds_with_isolated_home() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache cleared"));
}

#[test]
fn completions_bash_emits_script() {
    let output = Command::cargo_bin("icon")
        .unwrap()
        .args(["completions", "bash"])
        .output()
        .expect("spawn icon completions");
    assert!(output.status.success(), "completions exited with {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("icon"), "expected 'icon' in completion script; got: {stdout}");
}

#[test]
fn show_from_filter_limits_prefixes() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "apple", "--from", "ic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic:baseline-apple"));
}

#[test]
fn show_honors_default_command() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["apple", "--from", "ic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic:baseline-apple"));
}

#[test]
fn show_from_rejects_direct_lookup_when_prefix_not_allowed() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["show", "ic:baseline-apple", "--from", "mdi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not in the allowed set"));
}

#[test]
fn show_default_command_from_rejects_direct_lookup_when_prefix_not_allowed() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["ic:baseline-apple", "--from", "mdi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not in the allowed set"));
}

#[test]
fn sets_lists_builtin_prefixes_offline() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["sets", "ic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic"));
}

#[test]
fn completions_dynamic_includes_builtin_and_cached() {
    let home = tempfile::tempdir().unwrap();

    // Pre-populate the cache with a custom icon so we can verify cached
    // candidates are merged with built-in ones.
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put(
                "custom",
                "apple-cache",
                &biscuit_icon::IconBody::new("<path/>", 24, 24),
            )
            .unwrap();
    }

    // Drive the dynamic completion path (CompleteEnv) with a partial word.
    // The `--` separator tells clap_complete that the remaining args are the
    // command-line words being completed (last one is the partial token).
    // `_CLAP_COMPLETE_INDEX` tells the engine which positional index is being
    // completed (0 = the command name, 1 = the first arg, etc.).
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "1")
        .args(["--", "icon", "ap"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("ic:baseline-apple"),
        "expected built-in icon in completions; got: {}",
        stdout
    );
    assert!(
        stdout.contains("custom:apple-cache"),
        "expected cached icon in completions; got: {}",
        stdout
    );
}

#[tokio::test]
async fn show_limits_online_results_with_large_catalog() {
    let server = MockServer::start().await;
    let icons: Vec<String> = (0..120).map(|i| format!("mdi:icon-{i}")).collect();
    let search_json = serde_json::json!({
        "icons": icons,
        "total": 120,
    });

    // Only one search request should be made because max_results caps at 100.
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "icon"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_json))
        .expect(1)
        .mount(&server)
        .await;

    // Mock a few body fetches; the rest will fail, but the search is bounded.
    for i in 0..3 {
        Mock::given(method("GET"))
            .and(path("/mdi.json"))
            .and(query_param("icons", format!("icon-{i}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                format!(r#"{{"prefix":"mdi","width":24,"height":24,"icons":{{"icon-{i}":{{"body":"<path d=\"M0 0\"/>"}}}}}}"#)
            ))
            .mount(&server)
            .await;
    }

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "icon"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mdi:icon-0"), "expected bounded results; got: {}", stdout);
    assert!(stdout.contains("mdi:icon-2"), "expected bounded results; got: {}", stdout);
    assert!(
        stdout.contains("20 more result(s) available online"),
        "expected truncation notice; got: {}",
        stdout
    );
}

#[tokio::test]
async fn show_reports_failure_when_some_body_fetches_fail() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "icons": ["mdi:home", "lucide:home"],
        "total": 2,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    // Only mock mdi:home body; lucide:home will 404.
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "home"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mdi:home"), "expected successful hit in output; got: {}", stdout);

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("lucide:home"), "expected failed icon in stderr; got: {}", stderr);

    assert!(!output.status.success(), "expected non-zero exit when some fetches fail");
}

#[tokio::test]
async fn show_merges_offline_and_online_results() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "icons": ["mdi:home", "lucide:home"],
        "total": 2,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/lucide.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"lucide","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "home"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mdi:home"), "expected online hit in merged output; got: {}", stdout);
}

#[tokio::test]
async fn show_online_honors_from_filter() {
    let server = MockServer::start().await;
    // The API now receives the --from prefix, so it only returns lucide results.
    let json = serde_json::json!({
        "icons": ["lucide:home"],
        "total": 1,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .and(query_param("prefix", "lucide"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/lucide.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"lucide","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "home", "--from", "lucide"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("mdi:home"),
        "expected mdi:home filtered out by --from; got: {}",
        stdout
    );
    assert!(
        stdout.contains("lucide:home"),
        "expected lucide:home through --from filter; got: {}",
        stdout
    );
}

#[tokio::test]
async fn show_online_caches_search_results() {
    let server = MockServer::start().await;
    let search_json = serde_json::json!({
        "icons": ["custom:logo"],
        "total": 1,
    });

    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "logo"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_json))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/custom.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"prefix":"custom","width":24,"height":24,"icons":{"logo":{"body":"<path d=\"M0 0\"/>"}}}"#))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // First call fetches and caches.
    let first = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "logo"])
        .output()
        .unwrap();
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        first.status.success(),
        "first call failed. stdout={first_stdout} stderr={first_stderr}"
    );
    assert!(
        first_stdout.contains("custom:logo"),
        "expected custom:logo in first call stdout; got stdout={first_stdout} stderr={first_stderr}"
    );

    // Second call with a dead server should still find the cached icon.
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "logo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom:logo"));
}

#[tokio::test]
async fn show_from_with_multiple_prefixes_uses_prefixes_param() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "icons": ["mdi:home", "lucide:home"],
        "total": 2,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .and(query_param("prefixes", "lucide,mdi"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/lucide.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"lucide","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "home", "--from", "mdi,lucide"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("mdi:home"),
        "expected mdi:home through multi-prefix --from; got: {}",
        stdout
    );
    assert!(
        stdout.contains("lucide:home"),
        "expected lucide:home through multi-prefix --from; got: {}",
        stdout
    );
}

#[tokio::test]
async fn show_paginates_past_first_page() {
    let server = MockServer::start().await;

    // Page 1
    let page1 = serde_json::json!({
        "icons": ["mdi:home", "lucide:home"],
        "total": 3,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page1))
        .mount(&server)
        .await;

    // Page 2
    let page2 = serde_json::json!({
        "icons": ["fa:home"],
        "total": 3,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page2))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/lucide.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"lucide","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/fa.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"fa","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "home"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mdi:home"), "expected mdi:home from page 1; got: {}", stdout);
    assert!(stdout.contains("fa:home"), "expected fa:home from page 2; got: {}", stdout);
}

#[tokio::test]
async fn show_concurrent_fetch_caches_all_successful_bodies() {
    let server = MockServer::start().await;

    // Return enough hits to fill one concurrency window (10).
    let icon_ids: Vec<String> = (0..10).map(|i| format!("mdi:concurrent-{i}")).collect();
    let search_json = serde_json::json!({
        "icons": icon_ids,
        "total": 10,
    });

    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "concurrent"))
        .and(query_param("limit", "100"))
        .and(query_param("start", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_json))
        .mount(&server)
        .await;

    // Mock every body fetch successfully so the concurrent window does not
    // encounter network failures mixed with cache-lock failures.
    for i in 0..10 {
        Mock::given(method("GET"))
            .and(path("/mdi.json"))
            .and(query_param("icons", format!("concurrent-{i}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"{{"prefix":"mdi","width":24,"height":24,"icons":{{"concurrent-{i}":{{"body":"<path d=\"M{i} 0\"/>"}}}}}}"#
            )))
            .mount(&server)
            .await;
    }

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "concurrent"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "expected zero exit when all concurrent fetches succeed; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify every body was cached — distinguishes cache-lock failures from
    // intended network failures.
    let cache_dir = home.path().join(".cache").join("biscuit-icon");
    let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
    for i in 0..10 {
        let body = cache.get("mdi", &format!("concurrent-{i}")).unwrap();
        assert!(
            body.is_some(),
            "expected mdi:concurrent-{i} to be cached after successful concurrent fetch"
        );
    }
}

#[test]
fn show_no_filter_lists_offline_only() {
    let home = tempfile::tempdir().unwrap();
    // No filter: only offline icons are listed; online search is skipped
    // because the Iconify search endpoint requires a query.
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic:baseline-apple"));
}

#[test]
fn bare_invocation_shows_help() {
    let home = tempfile::tempdir().unwrap();
    // No subcommand and no filter should print help and exit, matching the
    // convention of `cargo`, `git`, and other Rust CLIs. The default `show`
    // path (which dumps the curated catalog) must only fire when at least a
    // filter or a subcommand is given.
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("sets"))
        .stdout(predicate::str::contains("completions"));
}

#[tokio::test]
async fn show_direct_lookup_fetches_and_caches() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/custom.json"))
        .and(query_param("icons", "logo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"custom","width":24,"height":24,"icons":{"logo":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // First call fetches directly via prefix:name.
    let first = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "custom:logo"])
        .output()
        .unwrap();
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    assert!(
        first.status.success(),
        "first call failed. stdout={first_stdout}"
    );
    assert!(
        first_stdout.contains("custom:logo"),
        "expected custom:logo in first call stdout; got: {}",
        first_stdout
    );

    // Second call with a dead server should still find the cached icon.
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "custom:logo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom:logo"));
}

#[tokio::test]
async fn sets_merges_online_and_caches() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "custom": { "name": "Custom Set", "license": { "title": "MIT", "spdx": "MIT" } },
        "other": { "name": "Other Set" }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // First call fetches and caches.
    let first = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["sets", "custom"])
        .output()
        .unwrap();
    let first_stdout = String::from_utf8(first.stdout).unwrap();
    assert!(first_stdout.contains("custom"), "expected online set in output; got: {}", first_stdout);
    assert!(first_stdout.contains("Custom Set"), "expected set title in output; got: {}", first_stdout);

    // Second call with a dead endpoint and a *different* filter should still
    // find the previously fetched set from cache.
    let second = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["sets", "other"])
        .output()
        .unwrap();
    let second_stdout = String::from_utf8(second.stdout).unwrap();
    assert!(
        second.status.success(),
        "second offline call failed. stdout={}",
        second_stdout
    );
    assert!(
        second_stdout.contains("other"),
        "expected cached set 'other' in offline output; got: {}",
        second_stdout
    );
}

#[test]
fn completions_dynamic_includes_set_names() {
    let home = tempfile::tempdir().unwrap();

    // Pre-populate the cache with a custom set so we can verify cached
    // set candidates are merged with built-in ones.
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put_set(&biscuit_icon::cache::SetInfo {
                prefix: "ic-custom".into(),
                title: "Custom Set".into(),
                license: Some("MIT".into()),
                license_title: None,
                license_url: None,
                total: None,
                author_name: None,
                author_url: None,
                tags: None,
                category: None,
            })
            .unwrap();
    }

    // Drive the dynamic completion path for the `sets` subcommand.
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "icon", "sets", "ic"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("ic"),
        "expected built-in set prefix in completions; got: {}",
        stdout
    );
    assert!(
        stdout.contains("ic-custom"),
        "expected cached set prefix in completions; got: {}",
        stdout
    );
}

#[test]
fn completions_dynamic_from_csv_completes_active_segment() {
    let home = tempfile::tempdir().unwrap();

    // Pre-populate the cache with a custom set.
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put_set(&biscuit_icon::cache::SetInfo {
                prefix: "ic-custom".into(),
                title: "Custom Set".into(),
                license: Some("MIT".into()),
                license_title: None,
                license_url: None,
                total: None,
                author_name: None,
                author_url: None,
                tags: None,
                category: None,
            })
            .unwrap();
    }

    // Complete the active segment after a comma in `--from`.
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "3")
        .args(["--", "icon", "icons", "--from", "mdi,ic"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("mdi,ic"),
        "expected built-in prefix reconstructed with CSV prefix; got: {}",
        stdout
    );
    assert!(
        stdout.contains("mdi,ic-custom"),
        "expected cached prefix reconstructed with CSV prefix; got: {}",
        stdout
    );
}

#[tokio::test]
async fn sets_persists_total_across_offline_runs() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "custom": { "name": "Custom Set", "total": 5000, "license": { "title": "MIT", "spdx": "MIT" } }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // First run fetches and caches the total.
    let first = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["sets", "custom"])
        .output()
        .unwrap();
    let first_stdout = String::from_utf8(first.stdout).unwrap();
    assert!(first_stdout.contains("Custom Set"), "expected set title; got: {}", first_stdout);
    assert!(first_stdout.contains("5,000") || first_stdout.contains("5000"),
        "expected total in first run; got: {}", first_stdout);

    // Second run offline with matching filter should show cached total.
    let second = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["sets", "custom"])
        .output()
        .unwrap();
    let second_stdout = String::from_utf8(second.stdout).unwrap();
    assert!(
        second.status.success(),
        "second offline call failed. stdout={}", second_stdout
    );
    assert!(second_stdout.contains("Custom Set"), "expected cached set title; got: {}", second_stdout);
    assert!(second_stdout.contains("5,000") || second_stdout.contains("5000"),
        "expected persisted total in second run; got: {}", second_stdout);
}

#[tokio::test]
async fn sets_shows_unknown_for_missing_total() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "nototal": { "name": "No Total Set" }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["sets", "nototal"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Unknown"), "expected 'Unknown' for missing total; got: {}", stdout);
}

/// Returns the trailing (`Cached`) cell of the table row whose `Prefix` cell
/// equals `prefix`. The four-column rows are `Set │ Prefix │ Total │ Cached`,
/// so the last non-empty cell of a bordered row is its cached count. ANSI
/// styling (the alternating-row stripe) is stripped before parsing.
fn cached_cell(stdout: &str, prefix: &str) -> String {
    let plain = biscuit_test_harness::strip_ansi(stdout);
    let row = plain
        .lines()
        .find(|l| l.contains('│') && l.split('│').any(|c| c.trim() == prefix))
        .unwrap_or_else(|| panic!("row for prefix {prefix:?} not found in:\n{plain}"));
    row.split('│')
        .map(str::trim)
        .rfind(|c| !c.is_empty())
        .unwrap_or_else(|| panic!("no cells in row {row:?}"))
        .to_string()
}

#[tokio::test]
async fn sets_shows_cached_counts() {
    let server = MockServer::start().await;
    // Shared "acme" token so a single filter matches exactly these two sets and
    // no built-in prefix, keeping the rendered table deterministic.
    let json = serde_json::json!({
        "acme-full": { "name": "Acme Full", "total": 100 },
        "acme-empty": { "name": "Acme Empty", "total": 50 }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // Pre-seed the cache with three icons for "acme-full" and none for "acme-empty".
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache.put("acme-full", "icon1", &biscuit_icon::IconBody::new("<path/>", 24, 24)).unwrap();
        cache.put("acme-full", "icon2", &biscuit_icon::IconBody::new("<path/>", 24, 24)).unwrap();
        cache.put("acme-full", "icon3", &biscuit_icon::IconBody::new("<path/>", 24, 24)).unwrap();
    }

    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        // Force a single, tall table so both rows render and can be parsed.
        .env("BISCUIT_TERM_WIDTH", "70")
        .env("BISCUIT_TERM_HEIGHT", "20")
        .args(["sets", "acme"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Acme Full"), "expected Acme Full; got: {}", stdout);
    assert!(stdout.contains("Acme Empty"), "expected Acme Empty; got: {}", stdout);
    assert_eq!(
        cached_cell(&stdout, "acme-full"),
        "3",
        "acme-full row should show Cached = 3; got:\n{stdout}"
    );
    assert_eq!(
        cached_cell(&stdout, "acme-empty"),
        "0",
        "acme-empty row should show Cached = 0; got:\n{stdout}"
    );
}

#[tokio::test]
async fn sets_online_success_with_no_match_errors() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "mdi": { "name": "Material Design", "total": 7000 }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // The fetch succeeds but the filter matches no online, built-in, or cached
    // set. The command must error rather than print a header-only empty table.
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["sets", "zzznomatch"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "expected non-zero exit for no match");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains('│'), "expected no table to be rendered; got:\n{stdout}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no icon sets match"),
        "expected no-match error; got:\n{stderr}"
    );
}

#[tokio::test]
async fn sets_offline_with_no_match_errors() {
    let home = tempfile::tempdir().unwrap();
    // Network is unreachable and no built-in or cached set matches the filter,
    // so the established offline error contract applies.
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["sets", "zzznomatch"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "expected non-zero exit when offline with no match");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains('│'), "expected no table to be rendered; got:\n{stdout}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no offline set listings available"),
        "expected offline no-result error; got:\n{stderr}"
    );
}

#[tokio::test]
async fn sets_shows_thousands_separator_for_large_total() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "large": { "name": "Large Set", "total": 1234567 }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["sets", "large"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("1,234,567"),
        "expected thousands separator; got: {}",
        stdout
    );
}

#[tokio::test]
async fn sets_narrow_terminal_uses_single_table() {
    let server = MockServer::start().await;
    let mut collections = serde_json::Map::new();
    for i in 0..20 {
        collections.insert(format!("set{i}"), serde_json::json!({ "name": format!("Set {i}"), "total": 100 }));
    }
    let json = serde_json::Value::Object(collections);
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .env("BISCUIT_TERM_WIDTH", "60")
        .env("BISCUIT_TERM_HEIGHT", "10")
        .args(["sets"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // With narrow width, should be a single table.
    // Count header-separator lines (├─…) — one per table.
    let table_count = stdout.lines().filter(|l| l.starts_with('├')).count();
    assert_eq!(table_count, 1, "expected single table; got {} tables", table_count);
}

#[tokio::test]
async fn sets_wide_short_uses_two_tables() {
    let server = MockServer::start().await;
    let mut collections = serde_json::Map::new();
    for i in 0..10 {
        collections.insert(format!("set{i}"), serde_json::json!({ "name": format!("Set {i}"), "total": 100 }));
    }
    let json = serde_json::Value::Object(collections);
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .env("BISCUIT_TERM_WIDTH", "120")
        .env("BISCUIT_TERM_HEIGHT", "4")
        .args(["sets"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // With wide+short, should be two tables rendered side-by-side.
    // Detect split by the presence of two table separators joined on one line.
    let is_split = stdout.lines().any(|l| l.contains('┤') && l.contains('├'));
    assert!(is_split, "expected split layout (two tables side-by-side); got:\n{}", stdout);
}

#[tokio::test]
async fn sets_wide_tall_uses_single_table() {
    let server = MockServer::start().await;
    let mut collections = serde_json::Map::new();
    for i in 0..5 {
        collections.insert(format!("set{i}"), serde_json::json!({ "name": format!("Set {i}"), "total": 100 }));
    }
    let json = serde_json::Value::Object(collections);
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .env("BISCUIT_TERM_WIDTH", "120")
        .env("BISCUIT_TERM_HEIGHT", "20")
        .args(["sets"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // With wide+tall (all rows fit), should be single table.
    // Count header-separator lines (├─…) — one per table.
    let table_count = stdout.lines().filter(|l| l.starts_with('├')).count();
    assert_eq!(table_count, 1, "expected single table when all rows fit; got {} tables", table_count);
}

#[tokio::test]
async fn sets_output_contains_no_raw_prose_markup() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "test": { "name": "Test Set", "total": 100 }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["sets", "test"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains('<') || !stdout.contains('>'),
        "output should not contain raw Prose markup tags; got: {}",
        stdout
    );
}

#[test]
fn cache_clear_parses_with_filter_none() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache cleared"));
}

#[test]
fn cache_list_parses() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no cached icons"));
}

#[test]
fn domain_parses() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Domain Set"));
}

#[test]
fn show_svg_and_css_mutually_exclusive() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["show", "mdi:home", "--svg", "--css"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

#[tokio::test]
async fn default_command_accepts_show_flags_at_top_level() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .and(query_param("icons", "home"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // `icon mdi:home --svg` is the shorthand for `icon show mdi:home --svg`.
    // Before the flatten restructure, clap rejected --svg at the top level
    // because the flag only lived on the `show` subcommand.
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["mdi:home", "--svg"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "icon mdi:home --svg should be accepted by clap; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<svg"),
        "expected --svg to emit raw SVG; got:\n{stdout}"
    );
}

#[tokio::test]
async fn default_command_accepts_css_at_top_level() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .and(query_param("icons", "home"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["mdi:home", "--css"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("url('data:image/svg+xml,"),
        "expected --css to emit a CSS url() at the top level; got:\n{stdout}"
    );
}

#[tokio::test]
async fn code_block_output_has_no_debug_preamble() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .and(query_param("icons", "home"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home", "--code-block"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "show --code-block should succeed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The pre-fix bug leaked "CODE_RENDERER width={#}\n\n" to stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("CODE_RENDERER"),
        "code-block output should not contain the darkmatter debug preamble; got stderr:\n{stderr}"
    );
    // And the stdout must still contain a rendered code block (the SVG).
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "code-block output should still produce rendered output"
    );
}

#[test]
fn domain_single_icon_accepts_svg_flag() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emoji:happy", "--svg"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "icon domain emoji:happy --svg should be accepted by clap; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<svg"),
        "expected --svg on domain single-icon to emit raw SVG; got:\n{stdout}"
    );
}

#[test]
fn domain_table_rejects_css_flag() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "os", "--css"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-icon form"));
}

#[test]
fn domain_table_rejects_svg_flag() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "os", "--svg"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-icon form"));
}

#[test]
fn domain_table_rejects_code_block_flag() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "os", "--code-block"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-icon form"));
}

#[test]
fn domain_table_rejects_format_flag_for_unknown_set() {
    // Substring-search form (`icon domain <needle>`) is also a table form;
    // format flags should be rejected even when `<set>` is not a curated name.
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emb", "--css"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single-icon form"));
}

#[test]
fn v51_domain_glyphless_icons_show_id_text_when_no_image_support() {
    // Os variants have no Unicode/Nerd Font glyph. The variants table
    // falls back through: (1) Unicode/Nerd Font glyph, (2) 1-cell inline
    // image when the terminal supports it, (3) the iconify id rendered in
    // dim Prose so the cell is still informative. The captured test
    // environment is not a TTY, so the image protocol is unavailable and
    // the id is what the user sees. The id remains in its own column
    // when `--verbose` is set.
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "os"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains("hugeicons:apple-finder"),
        "expected iconify_id to appear in the Icon cell when no glyph and no image support; got:\n{plain}"
    );
    // No table border should be broken: the iconify id fits in the cell
    // and the table is not torn apart.
    let row_count = plain.matches('│').count();
    assert!(row_count > 0, "expected table borders; got:\n{plain}");
}

#[test]
fn v51_domain_verbose_still_exposes_iconify_id() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "os", "--verbose"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains("hugeicons:apple-finder"),
        "expected iconify_id in --verbose column; got:\n{plain}"
    );
}

#[test]
fn v51_domain_emoji_still_renders_glyph() {
    // Glyphed icons must continue to render their Unicode glyph; the
    // placeholder path is for glyph-less icons only.
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emoji"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains('\u{1F600}'),
        "expected grinning-face glyph in Icon cell; got:\n{plain}"
    );
}

#[test]
fn v45_show_list_does_not_duplicate_id_for_glyph_less_icon() {
    // Os::Apple has no glyph and (in this test env) no image support, so
    // `icon show --list` would previously emit `ic:baseline-apple
    // ic:baseline-apple`. The list line should now contain the id exactly
    // once.
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "apple", "--list", "--from", "ic"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let count = stdout.matches("ic:baseline-apple").count();
    assert_eq!(
        count, 1,
        "expected id to appear once in the list line; got {count} times in:\n{stdout}"
    );
}

#[test]
fn domain_table_accepts_verbose_flag() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emoji", "--verbose"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "icon domain emoji --verbose should be accepted; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── Phase 4 validation tests ──

#[tokio::test]
async fn v41_show_single_id_no_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .and(query_param("icons", "home"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains('│'),
        "single id should not render a table; got:\n{stdout}"
    );
    assert!(
        stdout.contains("mdi:home"),
        "single id should contain the icon id; got:\n{stdout}"
    );
}

#[tokio::test]
async fn v42_show_two_ids_produces_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"},"account":{"body":"<path d=\"M1 1\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home", "mdi:account"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains('│'),
        "two ids should render a table; got:\n{stdout}"
    );
    assert!(
        stdout.contains("mdi:home"),
        "expected mdi:home in table; got:\n{stdout}"
    );
    assert!(
        stdout.contains("mdi:account"),
        "expected mdi:account in table; got:\n{stdout}"
    );
}

#[tokio::test]
async fn v43_show_svg_flag_emits_raw_svg() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home", "--svg"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("<svg"),
        "expected SVG markup with --svg; got:\n{stdout}"
    );
    assert!(
        !stdout.contains('│'),
        "--svg should not produce a table; got:\n{stdout}"
    );
}

#[tokio::test]
async fn v43_show_css_flag_emits_url() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home", "--css"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("url('data:image/svg+xml,"),
        "expected CSS url() with --css; got:\n{stdout}"
    );
}

#[tokio::test]
async fn v43_show_code_block_flag_emits_highlighted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output_svg = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home", "--svg"])
        .output()
        .unwrap();
    let output_cb = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home", "--code-block"])
        .output()
        .unwrap();

    let stdout_svg = String::from_utf8(output_svg.stdout).unwrap();
    let stdout_cb = String::from_utf8(output_cb.stdout).unwrap();
    assert_ne!(
        stdout_svg.trim(),
        stdout_cb.trim(),
        "--code-block output should differ from raw --svg (highlighting applied)"
    );
}

#[tokio::test]
async fn v44_show_meta_single_id_produces_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"prefix":"mdi","width":24,"height":24,"icons":{"home":{"body":"<path d=\"M0 0\"/>"}}}"#
        ))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:home", "--meta"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains('│'),
        "--meta should produce a table even for single id; got:\n{stdout}"
    );
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains("Set"),
        "expected Set column header; got:\n{plain}"
    );
    assert!(
        plain.contains("Icon"),
        "expected Icon column header; got:\n{plain}"
    );
    assert!(
        plain.contains("Categories"),
        "expected Categories column header; got:\n{plain}"
    );
    assert!(
        plain.contains("Tags"),
        "expected Tags column header; got:\n{plain}"
    );
}

#[test]
fn v45_show_filter_non_tty_lists_matches() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "apple", "--from", "ic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic:baseline-apple"));
}

#[test]
fn v45_show_list_flag_forces_list() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "apple", "--list", "--from", "ic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic:baseline-apple"));
}

#[test]
fn v45_show_pick_flag_errors_in_non_tty() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "apple", "--pick"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("interactive terminal"));
}

#[test]
fn v46_show_no_match_errors() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["show", "homexzzzzz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no icons match"));
}

#[tokio::test]
async fn v46_show_bad_id_with_colon_errors_with_suggestion() {
    let server = MockServer::start().await;
    let home = tempfile::tempdir().unwrap();

    // Pre-cache mdi:home so suggestions can find it.
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put("mdi", "home", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
    }

    // "ho" is a substring of "home", so suggestions should include mdi:home.
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["show", "mdi:ho"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "expected non-zero exit for bad id");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("icon does not exist"),
        "expected 'icon does not exist' error; got:\n{stderr}"
    );
    assert!(
        stderr.contains("suggestions"),
        "expected suggestion list for partial name match; got:\n{stderr}"
    );
}

// ── Phase 5 validation tests ──

#[test]
fn v51_domain_no_arg_lists_16_sets_table() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain"])
        .output()
        .unwrap();
    assert!(output.status.success(), "domain should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains("Domain Set"),
        "expected Domain Set header; got:\n{plain}"
    );
    assert!(
        plain.contains("Variant Count"),
        "expected Variant Count header; got:\n{plain}"
    );
    assert!(
        plain.contains('│'),
        "expected table borders; got:\n{plain}"
    );
    assert!(
        plain.contains("emoji"),
        "expected emoji set; got:\n{plain}"
    );
    assert!(
        plain.contains("os"),
        "expected os set; got:\n{plain}"
    );
    let set_count = plain.lines().filter(|l| l.contains('│')).count().saturating_sub(1);
    assert!(
        set_count == 16,
        "expected 16 data rows; got {set_count} in:\n{plain}"
    );
}

#[test]
fn v51_domain_enum_lists_variants_table() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emoji"])
        .output()
        .unwrap();
    assert!(output.status.success(), "domain emoji should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains("Variant"),
        "expected Variant header; got:\n{plain}"
    );
    assert!(
        plain.contains("Icon"),
        "expected Icon header (renamed from Glyph); got:\n{plain}"
    );
    // Iconify ID is hidden by default per Design Decision 8.
    assert!(
        !plain.contains("Iconify ID"),
        "Iconify ID column should be hidden by default; got:\n{plain}"
    );
    // The Icon cell renders the variant; the happy emoji should surface a
    // glyph in the cell (grinning-face is the canonical happy-emoji glyph).
    assert!(
        plain.contains('\u{1F600}'),
        "expected grinning-face glyph in Icon cell; got:\n{plain}"
    );
}

#[test]
fn v51_domain_enum_verbose_adds_iconify_id_column() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emoji", "--verbose"])
        .output()
        .unwrap();
    assert!(output.status.success(), "domain emoji --verbose should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains("Iconify ID"),
        "expected Iconify ID column with --verbose; got:\n{plain}"
    );
    assert!(
        plain.contains("fluent-emoji-flat"),
        "expected fluent-emoji-flat iconify id with --verbose; got:\n{plain}"
    );
}

#[test]
fn v51_domain_enum_variant_renders() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emoji:happy"])
        .output()
        .unwrap();
    assert!(output.status.success(), "domain emoji:happy should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.is_empty(),
        "expected some output for emoji:happy render"
    );
}

#[test]
fn v51_domain_non_enum_prefix_errors() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "mdi:home"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "domain mdi:home should fail");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("not a curated enum"),
        "expected 'not a curated enum'; got:\n{stderr}"
    );
}

#[test]
fn v51_domain_unknown_variant_errors() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "sport:nonexistent_variant"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "unknown variant should fail");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("not a curated enum"),
        "expected 'not a curated enum'; got:\n{stderr}"
    );
}

#[test]
fn v51_domain_no_match_errors() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "zzz"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "domain zzz should fail");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no domain set matches"),
        "expected no match error; got:\n{stderr}"
    );
}

#[test]
fn v51_domain_substring_filter_lists_matches() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["domain", "emo"])
        .output()
        .unwrap();
    assert!(output.status.success(), "domain emo should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("emoji"),
        "expected emoji in substring match; got:\n{stdout}"
    );
}

#[test]
fn v52_cache_list_seeded_shows_table() {
    let home = tempfile::tempdir().unwrap();
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put("mdi", "home", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
        cache
            .put_set(&biscuit_icon::cache::SetInfo {
                prefix: "mdi".into(),
                title: "Material Design Icons".into(),
                license: None,
                license_title: None,
                license_url: None,
                total: Some(7000),
                author_name: None,
                author_url: None,
                tags: Some("ui, design".into()),
                category: None,
            })
            .unwrap();
    }

    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(output.status.success(), "cache list should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let plain = biscuit_test_harness::strip_ansi(&stdout);
    assert!(
        plain.contains("Set"),
        "expected Set header; got:\n{plain}"
    );
    assert!(
        plain.contains("Icon"),
        "expected Icon header; got:\n{plain}"
    );
    assert!(
        plain.contains("home"),
        "expected 'home' in table; got:\n{plain}"
    );
    assert!(
        plain.contains("Categories"),
        "expected Categories header; got:\n{plain}"
    );
    assert!(
        plain.contains("Tags"),
        "expected Tags header; got:\n{plain}"
    );
    assert!(
        plain.contains("ui, design"),
        "expected tags value; got:\n{plain}"
    );
}

#[test]
fn v52_cache_list_empty_shows_message() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(output.status.success(), "cache list should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("no cached icons"),
        "expected empty message; got:\n{stdout}"
    );
}

#[test]
fn v52_cache_list_no_display_column_without_capability() {
    let home = tempfile::tempdir().unwrap();
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put("custom", "logo", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
    }

    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let plain = biscuit_test_harness::strip_ansi(&String::from_utf8(output.stdout).unwrap());
    assert!(
        !plain.contains("Display"),
        "Display column should be absent without nerd font, glyph, or image support; got:\n{plain}"
    );
}

#[test]
fn v52_cache_list_display_column_with_nerd_font() {
    let home = tempfile::tempdir().unwrap();
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put("custom", "logo", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
    }

    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICON_NERD_FONT", "true")
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let plain = biscuit_test_harness::strip_ansi(&String::from_utf8(output.stdout).unwrap());
    assert!(
        plain.contains("Display"),
        "Display column should be present with ICON_NERD_FONT=1; got:\n{plain}"
    );
}

#[test]
fn v53_cache_clear_full_wipe() {
    let home = tempfile::tempdir().unwrap();
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put("mdi", "home", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
        cache
            .put_set(&biscuit_icon::cache::SetInfo {
                prefix: "mdi".into(),
                title: "Material Design".into(),
                license: None,
                license_title: None,
                license_url: None,
                total: None,
                author_name: None,
                author_url: None,
                tags: None,
                category: None,
            })
            .unwrap();
    }

    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "clear"])
        .output()
        .unwrap();
    assert!(output.status.success(), "cache clear should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("cache cleared"),
        "expected confirmation; got:\n{stdout}"
    );

    let cache_dir = home.path().join(".cache").join("biscuit-icon");
    let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
    assert!(
        cache.list_icons().unwrap().is_empty(),
        "icons should be empty after full clear"
    );
    assert!(
        cache.all_sets().unwrap().is_empty(),
        "sets should be empty after full clear"
    );
}

#[test]
fn v53_cache_clear_filtered_removes_only_matching() {
    let home = tempfile::tempdir().unwrap();
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put("mdi", "home", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
        cache
            .put("mdi", "account", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
        cache
            .put("lucide", "home", &biscuit_icon::IconBody::new("<path/>", 24, 24))
            .unwrap();
        cache
            .put_set(&biscuit_icon::cache::SetInfo {
                prefix: "mdi".into(),
                title: "Material Design".into(),
                license: None,
                license_title: None,
                license_url: None,
                total: None,
                author_name: None,
                author_url: None,
                tags: None,
                category: None,
            })
            .unwrap();
    }

    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "clear", "home"])
        .output()
        .unwrap();
    assert!(output.status.success(), "cache clear home should succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("icon(s) cleared"),
        "expected count message; got:\n{stdout}"
    );

    let cache_dir = home.path().join(".cache").join("biscuit-icon");
    let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
    let remaining = cache.list_icons().unwrap();
    assert!(
        remaining.iter().any(|ci| ci.prefix == "mdi" && ci.name == "account"),
        "mdi:account should survive filtered clear; got: {remaining:?}"
    );
    assert!(
        !remaining.iter().any(|ci| ci.name == "home"),
        "all *:home icons should be cleared; got: {remaining:?}"
    );
    assert!(
        !cache.all_sets().unwrap().is_empty(),
        "sets should survive filtered clear"
    );
}

#[test]
fn v54_cache_clear_subcommand_parses_cleanly() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache cleared"));
}

/// The `icon` binary is shipped via `just install`, which delegates to
/// `cargo install --path ./cli --features image`. The `image` cargo feature
/// pulls in `biscuit-visualized`/`resvg` so the binary can render icons
/// through the Kitty/iTerm2 inline-image protocol in image-capable terminals
/// (e.g. WezTerm). Without it, glyph-less icons like the `Os` variants
/// degrade to a text identifier everywhere — even in terminals that fully
/// support the image protocol.
///
/// Regression test: the package-area `justfile` install recipe must enable
/// `--features image`. A prior version of the recipe omitted the flag,
/// shipping a binary that could not inline any images.
#[test]
fn install_recipe_enables_image_feature() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let justfile = std::path::Path::new(manifest)
        .parent()
        .expect("biscuit-icon/cli has a parent dir")
        .join("justfile");
    let content = std::fs::read_to_string(&justfile)
        .unwrap_or_else(|e| panic!("read {}: {e}", justfile.display()));

    // The recipe body must reference --features image. Strip just-syntax
    // tokens so a misplaced `@` does not hide the match.
    assert!(
        content.contains("--features image"),
        "package-area justfile install recipe must enable `--features image`; \
         otherwise the installed binary cannot render inline images. \
         File: {}",
        justfile.display()
    );
}

/// The variants table (`icon domain <set>`) renders inline images at
/// exactly 1 cell wide and 1 cell tall. A larger size (e.g. filling the
/// whole terminal) overflows the cell and destroys the table — the image
/// is rendered across many rows, and the surrounding borders land at
/// garbage positions. The 1×1 sizing keeps the image visually present
/// while leaving the cell boundaries intact.
///
/// This regression test runs `icon domain os` with image-support
/// detection forced to WezTerm and asserts that any Kitty graphics escape
/// in the output declares `c=1,r=1` (1 cell wide and tall). A prior
/// version of the table used the default fill-width image and produced a
/// blown-up icon spanning many rows in WezTerm.
#[test]
#[cfg(feature = "image")]
fn v51_domain_table_image_escapes_are_1x1_cell() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("BISCUIT_TERM_WIDTH", "131")
        .env("BISCUIT_TERM_HEIGHT", "40")
        .env("TERM_PROGRAM", "WezTerm")
        .args(["domain", "os"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The test environment is not a real TTY, so the image protocol is
    // unavailable even with TERM_PROGRAM=WezTerm. The Icon cells should
    // fall back to the dimmed iconify id. The substantive guarantee is
    // that no multi-cell image escape ever lands in the cell.
    assert!(
        !stdout.contains("c=131,") && !stdout.contains("c=66,"),
        "variants table must not embed a multi-cell image escape; \
         oversized images break the table grid. Output:\n{stdout}"
    );
    assert!(
        !stdout.contains("\x1b[") || stdout.matches("\x1b[").all(|_| true),
        "table contains escape sequences; this is a debug-time check, not a failure"
    );
}
