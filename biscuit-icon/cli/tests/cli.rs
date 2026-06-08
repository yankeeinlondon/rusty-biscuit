use assert_cmd::Command;
use predicates::prelude::*;

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
