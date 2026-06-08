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
    Command::cargo_bin("icon")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("icon"));
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
async fn icons_merges_offline_and_online_results() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "icons": ["mdi:home", "lucide:home"],
        "total": 2,
        "limit": 20,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "20"))
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
    let json = serde_json::json!({
        "icons": ["mdi:home", "lucide:home"],
        "total": 2,
        "limit": 20,
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "home"))
        .and(query_param("limit", "20"))
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
        "limit": 20,
    });

    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "logo"))
        .and(query_param("limit", "20"))
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
async fn sets_merges_online_and_caches() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "custom": { "name": "Custom Set", "license": { "title": "MIT", "spdx": "MIT" } }
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
        .args(["sets", "custom"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("custom"), "expected online set in output; got: {}", stdout);
    assert!(stdout.contains("Custom Set"), "expected set title in output; got: {}", stdout);
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

#[test]
fn completions_dynamic_default_command_from_csv_completes_active_segment() {
    let home = tempfile::tempdir().unwrap();

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
            })
            .unwrap();
    }

    // Complete the active segment after a comma in default-command `--from`.
    let output = Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "icon", "--from", "mdi,ic"])
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
