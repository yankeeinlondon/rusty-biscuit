use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let fixture = CliProcessFixture::named("test_help_flag");
    fixture
        .command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("markdown"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("compose"))
        .stdout(predicate::str::contains("toc"))
        .stdout(predicate::str::contains("delta"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("hash"))
        .stdout(predicate::str::contains("graph"));
}

/// Root help is the command list, the shared controls, `--style`, and the
/// sentence that `md` is `md render`; every other render option is on
/// `md render --help`.
#[test]
fn root_help_points_at_render_help_for_render_options() {
    let fixture = CliProcessFixture::named("root_help_points_at_render_help_for_render_options");
    let help = |args: &[&str]| {
        let output = fixture.command().args(args).output().expect("md help runs");
        assert!(output.status.success(), "{args:?}: {output:?}");
        // Help wraps at the detected width; compare on collapsed whitespace.
        String::from_utf8_lossy(&output.stdout).split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let root = help(&["--help"]);
    for expected in [
        "`md [ARGS] [OPTIONS]` is `md render [ARGS] [OPTIONS]`; see `md render --help`.",
        "Commands:",
        "Shared controls:",
        "--width <WIDTH>",
        "--verbose",
        "--magic-root <DIR>",
        "--style <FILE>",
    ] {
        assert!(root.contains(expected), "`md --help` lacks {expected:?}:\n{root}");
    }
    for hidden in ["--output", "--show", "--indent", "--deny", "--feature", "--mermaid"] {
        assert!(!root.contains(&format!("{hidden} ")), "`md --help` lists {hidden}:\n{root}");
    }

    let render = help(&["render", "--help"]);
    for expected in ["[ARGS]...", "--style <FILE>", "--output <TARGET>", "--show", "--indent", "--deny <ID>", "--feature <NAME>", "--mermaid", "--width <WIDTH>"] {
        assert!(render.contains(expected), "`md render --help` lacks {expected:?}:\n{render}");
    }
    // Removed, so listed by neither help.
    for removed in ["--max-width", "--margin", "--theme", "--code-theme", "--fill", "--align-", "--line-numbers", "--page-bg", "--save", "--list-themes"] {
        assert!(!root.contains(removed), "`md --help` lists {removed}:\n{root}");
        assert!(!render.contains(removed), "`md render --help` lists {removed}:\n{render}");
    }
    assert!(!render.contains("is `md render"), "the alias sentence is root-only:\n{render}");
}

#[test]
fn test_version_flag() {
    let fixture = CliProcessFixture::named("test_version_flag");
    fixture
        .command()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("md"));
}

#[test]
fn test_removed_flags_are_rejected() {
    let fixture = CliProcessFixture::named("test_removed_flags_are_rejected");
    for flag in [
        "--html",
        "--show-html",
        "--ast",
        "--json",
        "--no-images",
        "--toc",
        "--delta",
        "--clean",
        "--clean-save",
        "--fm-merge-with",
        "--fm-defaults",
    ] {
        fixture
            .command()
            .args([flag, "-"])
            .write_stdin("# Test")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }
}

#[test]
fn test_subcommand_rejects_render_options() {
    let fixture = CliProcessFixture::named("test_subcommand_rejects_render_options");
    fixture
        .command()
        .args(["--output", "html", "toc", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommands cannot be combined"));
}

#[test]
fn test_list_themes() {
    let fixture = CliProcessFixture::named("test_list_themes");
    fixture
        .command()
        .args(["list-themes", "--output", "plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available themes"))
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("solarized"))
        .stdout(predicate::str::contains("style.page.theme=NAME"))
        .stdout(predicate::str::contains("--theme").not());
}

#[test]
fn test_completions_bash() {
    let fixture = CliProcessFixture::named("test_completions_bash");
    fixture
        .command()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_zsh() {
    let fixture = CliProcessFixture::named("test_completions_zsh");
    fixture
        .command()
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_fish() {
    let fixture = CliProcessFixture::named("test_completions_fish");
    fixture
        .command()
        .args(["--completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

/// Acceptance criterion 7 (content-policy-no-cache): `md compose --help` names
/// the one artifact class `--cache-root` persists in the same
/// semantic-result / transport-artifact vocabulary as the library docs and
/// the caching topic, and describes each freshness mode as the code behaves.
#[test]
fn test_compose_help_states_the_transport_cache_boundary() {
    let fixture = CliProcessFixture::named("test_compose_help_states_the_transport_cache_boundary");
    let output = fixture
        .command()
        .args(["compose", "--help"])
        .output()
        .expect("md compose --help runs");
    assert!(output.status.success(), "md compose --help failed: {output:?}");
    // Help wraps at the detected width; compare on collapsed whitespace.
    let help = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    for expected in [
        // --cache-root: the persisted class, the prohibited class, laziness,
        // `no-store`, and the absence of any network grant.
        "Transport-artifact cache root",
        "persists raw remote URL response bodies only",
        "Semantic results (composed documents, `::file` children, `::code` / `::toc-linking` output) are never persisted",
        "Nothing is created on disk until a storable remote response is written",
        "A `Cache-Control: no-store` response is never written",
        "A cache root never authorizes a host",
        // --remote-ttl never turns a zero TTL into `no-store`.
        "never makes a `no-store` response storable or a `no-cache` response fresh",
        // --remote-refresh
        "Revalidate every transport-cached remote body",
        // --remote-freshness, per mode.
        "Serve within the freshness lifetime; past it, revalidate with a conditional GET and fail if revalidation fails",
        "serve the stale body when revalidation fails",
        "`no-cache` responses are always revalidated",
    ] {
        assert!(help.contains(expected), "missing {expected:?} in:\n{help}");
    }
    for drifted in [
        "Always revalidate with a conditional GET",
        "composed output is never persisted",
    ] {
        assert!(!help.contains(drifted), "drifted wording {drifted:?} in:\n{help}");
    }
}
