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
        .stdout(predicate::str::contains("icons"))
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
fn icons_from_filter_limits_prefixes() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["icons", "apple", "--from", "ic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic:baseline-apple"));
}

#[test]
fn icons_honors_default_command() {
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
fn icons_from_rejects_direct_lookup_when_prefix_not_allowed() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["icons", "ic:baseline-apple", "--from", "mdi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not in the allowed set"));
}

#[test]
fn icons_default_command_from_rejects_direct_lookup_when_prefix_not_allowed() {
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
async fn icons_limits_online_results_with_large_catalog() {
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
        .args(["icons", "icon"])
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
async fn icons_reports_failure_when_some_body_fetches_fail() {
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
        .args(["icons", "home"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mdi:home"), "expected successful hit in output; got: {}", stdout);

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("lucide:home"), "expected failed icon in stderr; got: {}", stderr);

    assert!(!output.status.success(), "expected non-zero exit when some fetches fail");
}

#[tokio::test]
async fn icons_merges_offline_and_online_results() {
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
        .args(["icons", "home"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mdi:home"), "expected online hit in merged output; got: {}", stdout);
}

#[tokio::test]
async fn icons_online_honors_from_filter() {
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
        .args(["icons", "home", "--from", "lucide"])
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
async fn icons_online_caches_search_results() {
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
        .args(["icons", "logo"])
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
        .args(["icons", "logo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom:logo"));
}

#[tokio::test]
async fn icons_from_with_multiple_prefixes_uses_prefixes_param() {
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
        .args(["icons", "home", "--from", "mdi,lucide"])
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
async fn icons_paginates_past_first_page() {
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
        .args(["icons", "home"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mdi:home"), "expected mdi:home from page 1; got: {}", stdout);
    assert!(stdout.contains("fa:home"), "expected fa:home from page 2; got: {}", stdout);
}

#[tokio::test]
async fn icons_concurrent_fetch_caches_all_successful_bodies() {
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
        .args(["icons", "concurrent"])
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
fn icons_no_filter_lists_offline_only() {
    let home = tempfile::tempdir().unwrap();
    // No filter: only offline icons are listed; online search is skipped
    // because the Iconify search endpoint requires a query.
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", "http://127.0.0.1:1")
        .args(["icons"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ic:baseline-apple"));
}

#[tokio::test]
async fn icons_direct_lookup_fetches_and_caches() {
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
        .args(["icons", "custom:logo"])
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
        .args(["icons", "custom:logo"])
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

#[tokio::test]
async fn sets_shows_cached_counts() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "test": { "name": "Test Set", "total": 100 },
        "empty": { "name": "Empty Set", "total": 50 }
    });
    Mock::given(method("GET"))
        .and(path("/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .mount(&server)
        .await;

    let home = tempfile::tempdir().unwrap();
    // Pre-seed the cache with icons for the "test" prefix only.
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache.put("test", "icon1", &biscuit_icon::IconBody::new("<path/>", 24, 24)).unwrap();
        cache.put("test", "icon2", &biscuit_icon::IconBody::new("<path/>", 24, 24)).unwrap();
        cache.put("test", "icon3", &biscuit_icon::IconBody::new("<path/>", 24, 24)).unwrap();
    }

    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("ICONIFY_BASE_URL", server.uri())
        .args(["sets"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Test Set"), "expected Test Set; got: {}", stdout);
    assert!(stdout.contains("Empty Set"), "expected Empty Set; got: {}", stdout);
    // "test" has 3 cached icons, "empty" has 0.
    assert!(stdout.contains('3'), "expected cached count 3 for test; got: {}", stdout);
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
