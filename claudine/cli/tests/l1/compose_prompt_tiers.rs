//! `@` prompt references launched from a directory nested under `$HOME`.
//!
//! Mirrors running `claudine compose @prompts/commit.md` from `~/config/sh`:
//! the launch directory is below the user's home, is a plain Git repository
//! or no repository at all, and has no Cargo workspace. The user prompt tier
//! `~/.claudine/prompts/` must answer both the path-shaped `@prompts/<x>`
//! form and the concise `@<x>` form, while every local match — repository,
//! its `.claudine` tiers, or a plain launch directory — wins over any home
//! match (`2026-09-23-local-before-home`).
//!
//! Tests whose subject is the user tier itself are `cfg(not(windows))`: native
//! Windows resolves the home directory through the known-folder API and
//! ignores the fixture's `HOME`/`USERPROFILE` (D11; `os` skill, windows.md), so
//! a fixture home cannot stand in for it there. The ordering they prove is
//! lexical and is covered on every OS by the snapshot-home unit tests in
//! biscuit-file and `claudine::composition::resolve`.

use std::path::{Path, PathBuf};

use crate::common;

use common::{CliProcessFixture, init_git_repo, strip_ansi, write};

/// Stage `$HOME/config/sh` as the launch directory and a user-tier prompt.
fn stage(prefix: &str, repository: bool) -> (CliProcessFixture, PathBuf) {
    stage_at(prefix, &["config", "sh"], repository)
}

/// Stage the launch directory at `$HOME/<segments>` (`$HOME` itself when
/// `segments` is empty) and a user-tier prompt.
fn stage_at(prefix: &str, segments: &[&str], repository: bool) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(prefix);
    let launch = segments
        .iter()
        .fold(fixture.home().to_path_buf(), |path, segment| path.join(segment));
    std::fs::create_dir_all(&launch).unwrap();
    if repository {
        assert!(init_git_repo(&launch), "git init failed at {}", launch.display());
    }
    write(
        &fixture.home().join(".claudine").join("prompts").join("probe.md"),
        "Tier=[user]\n",
    );
    (fixture, launch)
}

fn compose_dry_run(fixture: &CliProcessFixture, launch: &Path, reference: &str) -> String {
    let output = fixture
        .command_builder()
        .ambient_context(launch)
        .build()
        .args(["compose", "--dry-run", reference])
        .assert()
        .success()
        .get_output()
        .clone();
    strip_ansi(&String::from_utf8_lossy(&output.stdout))
}

/// Run a `compose --dry-run` expected to fail and return its plain stderr.
fn compose_dry_run_failure(fixture: &CliProcessFixture, launch: &Path, reference: &str) -> String {
    let output = fixture
        .command_builder()
        .ambient_context(launch)
        .build()
        // A wide terminal keeps each search root on one line.
        .env("COLUMNS", "500")
        .args(["compose", "--dry-run", reference])
        .assert()
        .failure()
        .get_output()
        .clone();
    strip_ansi(&String::from_utf8_lossy(&output.stderr))
}

#[cfg(not(windows))] // fixture home cannot replace the known folder
#[test]
fn path_shaped_reference_reaches_user_tier_from_plain_repository_under_home() {
    let (fixture, launch) = stage("prompt-tiers-repo-path", true);
    let stdout = compose_dry_run(&fixture, &launch, "@prompts/probe.md");
    assert!(stdout.contains("Tier=[user]"), "{stdout}");
}

#[cfg(not(windows))] // fixture home cannot replace the known folder
#[test]
fn concise_reference_reaches_user_tier_from_plain_repository_under_home() {
    let (fixture, launch) = stage("prompt-tiers-repo-concise", true);
    let stdout = compose_dry_run(&fixture, &launch, "@probe.md");
    assert!(stdout.contains("Tier=[user]"), "{stdout}");
}

#[cfg(not(windows))] // fixture home cannot replace the known folder
#[test]
fn both_reference_forms_reach_user_tier_outside_any_repository() {
    let (fixture, launch) = stage("prompt-tiers-no-repo", false);
    for reference in ["@prompts/probe.md", "@probe.md"] {
        let stdout = compose_dry_run(&fixture, &launch, reference);
        assert!(stdout.contains("Tier=[user]"), "{reference}: {stdout}");
    }
}

#[test]
fn repository_prompt_wins_over_user_tier_for_both_forms() {
    let (fixture, launch) = stage("prompt-tiers-local-wins", true);
    write(&launch.join("prompts").join("probe.md"), "Tier=[repo]\n");
    write(
        &launch.join(".claudine").join("prompts").join("probe.md"),
        "Tier=[repo-claudine]\n",
    );
    for reference in ["@prompts/probe.md", "@probe.md"] {
        let stdout = compose_dry_run(&fixture, &launch, reference);
        assert!(stdout.contains("Tier=[repo]"), "{reference}: {stdout}");
    }
}

#[test]
fn repository_claudine_tier_wins_over_user_tier_for_path_shaped_form() {
    let (fixture, launch) = stage("prompt-tiers-repo-claudine", true);
    write(
        &launch.join(".claudine").join("prompts").join("probe.md"),
        "Tier=[repo-claudine]\n",
    );
    let stdout = compose_dry_run(&fixture, &launch, "@prompts/probe.md");
    assert!(stdout.contains("Tier=[repo-claudine]"), "{stdout}");
}

// ---------------------------------------------------------------------
// Local roots before home (2026-09-23-local-before-home)
// ---------------------------------------------------------------------

#[test]
fn repository_root_file_wins_over_user_prompt_tier_for_concise_form() {
    // Defect 2, row 1: `~/.claudine/prompts` was a prepend, so it outranked
    // the repository root for `@x.md`.
    let (fixture, launch) = stage("prompt-tiers-repo-root-wins", true);
    write(&launch.join("probe.md"), "Tier=[repo-root]\n");
    let stdout = compose_dry_run(&fixture, &launch, "@probe.md");
    assert!(stdout.contains("Tier=[repo-root]"), "{stdout}");
    assert!(!stdout.contains("Tier=[user]"), "{stdout}");
}

#[test]
fn repository_claudine_tier_wins_over_home_prompts_directory() {
    // Defect 2, row 2: `<repo>/.claudine` was an append, so the intrinsic home
    // root's `~/prompts/x.md` outranked `<repo>/.claudine/prompts/x.md`.
    let (fixture, launch) = stage("prompt-tiers-repo-claudine-vs-home", true);
    write(
        &launch.join(".claudine").join("prompts").join("probe.md"),
        "Tier=[repo-claudine]\n",
    );
    write(&fixture.home().join("prompts").join("probe.md"), "Tier=[home]\n");
    let stdout = compose_dry_run(&fixture, &launch, "@prompts/probe.md");
    assert!(stdout.contains("Tier=[repo-claudine]"), "{stdout}");
    assert!(!stdout.contains("Tier=[home]"), "{stdout}");
    assert!(!stdout.contains("Tier=[user]"), "{stdout}");
}

#[test]
fn plain_launch_directory_under_home_wins_for_both_forms() {
    // Defect 4: outside a repository the launch directory was never an `@`
    // root, so only home-based candidates were tried.
    let (fixture, launch) = stage_at("prompt-tiers-scratch", &["scratch"], false);
    write(&launch.join("prompts").join("probe.md"), "Tier=[local]\n");
    for reference in ["@prompts/probe.md", "@probe.md"] {
        let stdout = compose_dry_run(&fixture, &launch, reference);
        assert!(stdout.contains("Tier=[local]"), "{reference}: {stdout}");
        assert!(!stdout.contains("Tier=[user]"), "{reference}: {stdout}");
    }
}

#[cfg(not(windows))] // fixture home cannot replace the known folder
#[test]
fn local_root_file_wins_over_user_prompt_tier_when_the_local_tree_is_home() {
    // Ruling 1: when the launch directory, or its repository, *is* `$HOME`,
    // containment alone would make `~/.claudine/prompts` local; its explicit
    // user tier keeps it behind the local root's own `~/probe.md`.
    for (prefix, repository) in [
        ("prompt-tiers-launch-is-home", false),
        ("prompt-tiers-repo-is-home", true),
    ] {
        let (fixture, launch) = stage_at(prefix, &[], repository);
        assert_eq!(launch, fixture.home());
        write(&launch.join("probe.md"), "Tier=[local-root]\n");
        let stdout = compose_dry_run(&fixture, &launch, "@probe.md");
        assert!(stdout.contains("Tier=[local-root]"), "{prefix}: {stdout}");
        assert!(!stdout.contains("Tier=[user]"), "{prefix}: {stdout}");

        // A local convention directory still wins, too.
        write(&launch.join("prompts").join("probe.md"), "Tier=[local-prompts]\n");
        for reference in ["@probe.md", "@prompts/probe.md"] {
            let stdout = compose_dry_run(&fixture, &launch, reference);
            assert!(
                stdout.contains("Tier=[local-prompts]"),
                "{prefix} {reference}: {stdout}"
            );
        }
    }
}

/// Write one single-transclusion document per reference form into `dir`,
/// returning `(reference, document)` pairs.
fn stage_transclusion_routers(dir: &Path, forms: &[&str]) -> Vec<(String, PathBuf)> {
    forms
        .iter()
        .enumerate()
        .map(|(index, form)| {
            let reference = format!("{form}snippet.md");
            let document = dir.join(format!("router-{index}.md"));
            write(&document, &format!("---\n---\n::file {reference}\n"));
            (reference, document)
        })
        .collect()
}

#[cfg(not(windows))] // fixture home cannot replace the known folder
#[test]
fn nested_magic_reference_in_a_user_prompt_resolves_from_the_launch_tree() {
    // Ruling 2: a prompt loaded from `~/.claudine/prompts` keeps the launch
    // tree as the local root for its nested `@` references, while `./` and
    // bare references stay beside the prompt that authored them.
    let (fixture, launch) = stage("prompt-tiers-nested-user", true);
    let user_prompts = fixture.home().join(".claudine").join("prompts");
    write(&launch.join("snippet.md"), "Snippet=[launch]\n");
    write(&user_prompts.join("snippet.md"), "Snippet=[source-dir]\n");

    let expected = [("@", "launch"), ("./", "source-dir"), ("", "source-dir")];
    let forms: Vec<&str> = expected.iter().map(|(form, _)| *form).collect();
    for ((reference, document), (_, winner)) in stage_transclusion_routers(&user_prompts, &forms)
        .into_iter()
        .zip(expected)
    {
        // Launch through the user tier, the way a user reaches such a prompt.
        let name = document.file_name().unwrap().to_str().unwrap();
        let stdout = compose_dry_run(&fixture, &launch, &format!("@{name}"));
        assert!(
            stdout.contains(&format!("Snippet=[{winner}]")),
            "`::file {reference}` should resolve to the {winner} copy: {stdout}"
        );
    }
}

#[test]
fn nested_magic_reference_in_another_repository_resolves_from_the_launch_tree() {
    // Ruling 2 across repositories: `@` follows the launch tree; `&` and `^`
    // keep the source document's repository; `./` and bare stay beside it.
    let (fixture, launch) = stage("prompt-tiers-nested-other-repo", true);
    let other = fixture.cwd().join("other");
    std::fs::create_dir_all(&other).unwrap();
    assert!(init_git_repo(&other), "git init failed at {}", other.display());
    let other_prompts = other.join("prompts");
    write(&launch.join("snippet.md"), "Snippet=[launch]\n");
    write(&other.join("snippet.md"), "Snippet=[source-repo]\n");
    write(&other_prompts.join("snippet.md"), "Snippet=[source-dir]\n");

    let expected = [
        ("@", "launch"),
        ("&", "source-repo"),
        ("^", "source-repo"),
        ("./", "source-dir"),
        ("", "source-dir"),
    ];
    let forms: Vec<&str> = expected.iter().map(|(form, _)| *form).collect();
    for ((reference, document), (_, winner)) in stage_transclusion_routers(&other_prompts, &forms)
        .into_iter()
        .zip(expected)
    {
        let stdout = compose_dry_run(&fixture, &launch, document.to_str().unwrap());
        assert!(
            stdout.contains(&format!("Snippet=[{winner}]")),
            "`::file {reference}` should resolve to the {winner} copy: {stdout}"
        );
    }
}

/// Search-root lines of a rendered `@` miss, as `(path below the fixture
/// workspace, configured-root marker)` pairs.
///
/// The workspace-relative form absorbs the launch directory's physical
/// spelling (`/private/var/…` on macOS) against `$HOME`'s authored one.
#[cfg(not(windows))]
fn rendered_search_roots(stderr: &str, fixture: &CliProcessFixture) -> Vec<(String, bool)> {
    let workspace = fixture.workspace_path().file_name().unwrap().to_str().unwrap();
    let anchor = format!("{workspace}/");
    stderr
        .lines()
        .map(|line| line.trim_start_matches(|c: char| c == '┃' || c.is_whitespace()))
        .filter_map(|line| line.strip_prefix("- `"))
        .map(|line| {
            let (path, marker) = line.split_once('`').expect("closing backtick");
            let relative = path
                .split_once(&anchor)
                .unwrap_or_else(|| panic!("root `{path}` outside the fixture workspace"))
                .1;
            (relative.to_string(), marker.trim() == "(*)")
        })
        .collect()
}

#[cfg(not(windows))] // fixture home cannot replace the known folder
#[test]
fn magic_miss_lists_ordered_search_roots_instead_of_joined_candidates() {
    // Defect 3, the reported input: the human-readable miss names the payload
    // once and lists the directories `@` searched, local before home.
    let (fixture, launch) = stage("prompt-tiers-magic-miss", true);
    let stderr = compose_dry_run_failure(&fixture, &launch, "@prompts/missing.md");

    assert!(
        stderr.contains("`prompts/missing.md` was not found under any directory an `@` reference searches:"),
        "{stderr}"
    );
    // Every joined candidate would repeat the payload, so a single occurrence
    // also proves none (such as `…/prompts/prompts/missing.md`) is listed.
    assert_eq!(stderr.matches("prompts/missing.md").count(), 1, "{stderr}");
    for label in ["Tried:", "magic:", "repository:", "home:", "Cannot resolve"] {
        assert!(!stderr.contains(label), "`{label}` leaked into the `@` miss: {stderr}");
    }

    let roots = rendered_search_roots(&stderr, &fixture);
    let first_user = roots
        .iter()
        .position(|(path, _)| !path.starts_with("home/config/sh"))
        .expect("a home-tier root");
    assert!(
        roots[first_user..].iter().all(|(path, _)| !path.starts_with("home/config/sh")),
        "every local root must precede the first home root: {roots:?}"
    );
    let position = |wanted: &str| {
        roots
            .iter()
            .position(|(path, _)| path == wanted)
            .unwrap_or_else(|| panic!("root `{wanted}` missing: {roots:?}"))
    };
    let ordered = [
        "home/config/sh/prompts",
        "home/config/sh/.claudine/prompts",
        "home/config/sh",
        "home/config/sh/.claudine",
        "home/.claudine/prompts",
        "home",
        "home/.claudine",
    ]
    .map(position);
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]), "{roots:?}");

    // Only configured roots carry `(*)`: the intrinsic local and home roots
    // are the unmarked ones.
    let unmarked: Vec<&str> = roots
        .iter()
        .filter(|(_, marked)| !marked)
        .map(|(path, _)| path.as_str())
        .collect();
    assert_eq!(unmarked, ["home/config/sh", "home"], "{roots:?}");
    assert!(
        stderr.contains("(*) searched in addition to the standard `@` roots, for this context"),
        "{stderr}"
    );
}

#[cfg(not(windows))] // fixture home cannot replace the known folder
#[test]
fn magic_miss_outside_any_repository_lists_the_launch_directory_first() {
    let (fixture, launch) = stage_at("prompt-tiers-scratch-miss", &["scratch"], false);
    let stderr = compose_dry_run_failure(&fixture, &launch, "@prompts/missing.md");
    let roots = rendered_search_roots(&stderr, &fixture);
    let unmarked: Vec<&str> = roots
        .iter()
        .filter(|(_, marked)| !marked)
        .map(|(path, _)| path.as_str())
        .collect();
    assert_eq!(unmarked, ["home/scratch", "home"], "{roots:?}");
    assert_eq!(roots[0].0, "home/scratch/prompts", "{roots:?}");
}

#[test]
fn separated_magic_miss_names_the_payload_without_the_separator() {
    // Review-1 Medium: `@/<x>` is the same payload as `@<x>`, so the miss
    // line must not echo the optional `/` separator.
    let (fixture, launch) = stage("prompt-tiers-separated-miss", true);
    let stderr = compose_dry_run_failure(&fixture, &launch, "@/absent-payload-probe.md");

    assert!(
        stderr.contains(
            "`absent-payload-probe.md` was not found under any directory an `@` reference searches:"
        ),
        "{stderr}"
    );
    assert_eq!(stderr.matches("`absent-payload-probe.md`").count(), 1, "{stderr}");
    assert!(!stderr.contains("`/absent-payload-probe.md`"), "{stderr}");
}

#[test]
fn absolute_and_bare_misses_keep_their_existing_reports() {
    let (fixture, launch) = stage("prompt-tiers-other-miss", true);
    let absolute = fixture.home().join("missing.md");
    let stderr = compose_dry_run_failure(&fixture, &launch, absolute.to_str().unwrap());
    assert!(stderr.contains("Cannot resolve `"), "{stderr}");
    assert!(stderr.contains("Tried:"), "{stderr}");
    assert!(stderr.contains("- absolute: `"), "{stderr}");
    assert!(!stderr.contains("an `@` reference searches"), "{stderr}");

    // A bare miss still reaches the non-interactive autocomplete gate.
    let stderr = compose_dry_run_failure(&fixture, &launch, "missing.md");
    assert!(stderr.contains("autocomplete not available"), "{stderr}");
    assert!(!stderr.contains("an `@` reference searches"), "{stderr}");
}

#[cfg(unix)] // fixture home cannot replace the known folder; unix symlink API
#[test]
fn repository_claudine_symlinked_to_user_claudine_keeps_local_priority() {
    // Review-1 High: a repository whose `.claudine` is a symlink to
    // `~/.claudine` is lexically local, so `<repo>/.claudine/prompts/<x>`
    // must still outrank the intrinsic home root's `~/prompts/<x>` even
    // though both `.claudine` rows name the same physical directory.
    let (fixture, launch) = stage_at("prompt-tiers-symlinked-claudine", &["project"], true);
    write(
        &fixture.home().join(".claudine").join("prompts").join("probe.md"),
        "Tier=[symlinked-claudine]\n",
    );
    write(&fixture.home().join("prompts").join("probe.md"), "Tier=[home]\n");
    std::os::unix::fs::symlink(fixture.home().join(".claudine"), launch.join(".claudine")).unwrap();

    let stdout = compose_dry_run(&fixture, &launch, "@prompts/probe.md");
    assert!(stdout.contains("Tier=[symlinked-claudine]"), "{stdout}");
    assert!(!stdout.contains("Tier=[home]"), "{stdout}");
}
