//! The shipped CLI's projection of lockfile observations
//! (`2026-09-26-lockfile-corroboration`, AC5).
//!
//! Each case copies a real pnpm 10.32.1 or Composer 2.10.3 fixture from the
//! library's `tests/fixtures/lockfiles` into a temporary directory: in place, this
//! monorepo's `.git` and workspaces would take over. No package-manager binary
//! runs. No CLI command displays a layer from a request that declines
//! corroboration, so `not_requested` is covered by the renderer's unit tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::{Value, json};

use crate::common;

fn lockfile_fixtures() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("the sniff package area")
        .join("lib/tests/fixtures/lockfiles")
}

fn pnpm_fixture(case: &str) -> PathBuf {
    lockfile_fixtures().join("pnpm-10.32.1").join(case)
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create fixture copy directory");
    for entry in fs::read_dir(from).expect("read fixture directory") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("fixture entry type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}

/// A disposable copy of one pnpm fixture, rearranged by `edit`.
fn copied(case: &str, edit: fn(&Path)) -> (tempfile::TempDir, PathBuf) {
    let source = pnpm_fixture(case);
    assert!(source.is_dir(), "missing fixture {}", source.display());
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    copy_tree(&source, &root);
    edit(&root);
    (dir, root)
}

fn unchanged(_: &Path) {}

fn lockfile_removed(root: &Path) {
    fs::remove_file(root.join("pnpm-lock.yaml")).expect("remove lockfile");
}

fn lockfile_replaced_by_directory(root: &Path) {
    lockfile_removed(root);
    fs::create_dir(root.join("pnpm-lock.yaml")).expect("directory where the lockfile goes");
}

fn non_string_member_declared(root: &Path) {
    let manifest = root.join("pnpm-workspace.yaml");
    let content = fs::read_to_string(&manifest).expect("read pnpm-workspace.yaml");
    fs::write(&manifest, format!("{content}  - 123\n")).expect("write pnpm-workspace.yaml");
}

/// Review 5's reproduction: a present `packages` field that is not a list.
fn wrong_type_members_declared(root: &Path) {
    fs::write(root.join("pnpm-workspace.yaml"), "packages: 123\n")
        .expect("write pnpm-workspace.yaml");
}

fn run(root: &Path, args: &[&str]) -> Output {
    common::owned_sniff_command()
        .args(["--base", root.to_str().expect("UTF-8 fixture path")])
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run sniff {args:?}: {error}"))
}

/// Exit 0 and nothing on stderr: an observation, even `mismatch` or
/// `unreadable`, is a fact about the repository, not a CLI failure.
fn assert_quiet_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label}: exit {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{label}: stderr must be empty: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The whole of stdout parsed as one JSON document, which rejects any line
/// mixed in before or after it.
fn json_stdout(output: &Output, label: &str) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("UTF-8 stdout");
    serde_json::from_str(stdout)
        .unwrap_or_else(|error| panic!("{label}: stdout must be one JSON document: {error}\n{stdout}"))
}

/// Plain human stdout with every whitespace run collapsed to one space, so an
/// assertion is independent of where the terminal width wraps a line.
fn human_stdout(output: &Output, label: &str) -> String {
    let stdout = std::str::from_utf8(&output.stdout).expect("UTF-8 stdout");
    assert!(
        serde_json::from_str::<Value>(stdout).is_err(),
        "{label}: human mode must not print JSON"
    );
    stdout.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `(label, fixture, edit, expected lockfile object, expected human lines)`.
type Case = (&'static str, &'static str, fn(&Path), Value, &'static [&'static str]);

fn cases() -> Vec<Case> {
    let observation = |status: &str, paths: &[&str], reason: Value, extra: &[&str], missing: &[&str]| {
        json!({
            "status": status,
            "paths": paths,
            "reason": reason,
            "extra": extra,
            "missing": missing,
        })
    };
    vec![
        (
            "match",
            "workspace",
            unchanged,
            observation("match", &["pnpm-lock.yaml"], Value::Null, &[], &[]),
            &[
                "pnpm workspaces (.): match — the lockfile's members match the manifest",
                "- lockfile: pnpm-lock.yaml",
            ],
        ),
        (
            "mismatch-missing",
            "workspace-edited-missing",
            unchanged,
            observation("mismatch", &["pnpm-lock.yaml"], Value::Null, &[], &["packages/beta"]),
            &[
                "pnpm workspaces (.): mismatch — the lockfile's members differ from the manifest",
                "- lockfile: pnpm-lock.yaml",
                "- missing from the lockfile: packages/beta",
            ],
        ),
        (
            "mismatch-extra",
            "workspace-edited-stale-extra",
            unchanged,
            observation("mismatch", &["pnpm-lock.yaml"], Value::Null, &["packages/gamma"], &[]),
            &[
                "pnpm workspaces (.): mismatch",
                "- only in the lockfile: packages/gamma",
            ],
        ),
        (
            "unreadable-parse",
            "workspace-edited-malformed-trailing",
            unchanged,
            observation("unreadable", &["pnpm-lock.yaml"], json!("parse_failed"), &[], &[]),
            &[
                "pnpm workspaces (.): unreadable — the lockfile is not valid for its format",
                "- lockfile: pnpm-lock.yaml",
            ],
        ),
        (
            "unreadable-directory",
            "workspace",
            lockfile_replaced_by_directory,
            observation("unreadable", &["pnpm-lock.yaml"], json!("read_failed"), &[], &[]),
            &["pnpm workspaces (.): unreadable — the lockfile could not be read"],
        ),
        (
            "unverifiable-invalid-member",
            "workspace",
            non_string_member_declared,
            observation(
                "unverifiable",
                &["pnpm-lock.yaml"],
                json!("incomplete_manifest_discovery"),
                &[],
                &[],
            ),
            &[
                "pnpm workspaces (.): unverifiable — the manifest's member list may be incomplete, so it was not compared",
                "- lockfile: pnpm-lock.yaml",
            ],
        ),
        (
            "unverifiable-wrong-type-members",
            "workspace",
            wrong_type_members_declared,
            observation(
                "unverifiable",
                &["pnpm-lock.yaml"],
                json!("incomplete_manifest_discovery"),
                &[],
                &[],
            ),
            &[
                "pnpm workspaces (.): unverifiable — the manifest's member list may be incomplete, so it was not compared",
                "- lockfile: pnpm-lock.yaml",
            ],
        ),
        (
            "absent",
            "workspace",
            lockfile_removed,
            observation("absent", &[], Value::Null, &[], &[]),
            &["pnpm workspaces (.): absent — no lockfile was found"],
        ),
    ]
}

#[test]
fn structure_json_carries_each_layer_lockfile_observation() {
    for (label, fixture, edit, expected, _) in cases() {
        let (_dir, root) = copied(fixture, edit);
        let output = run(&root, &["repo", "structure", "--json"]);
        assert_quiet_success(&output, label);
        let json = json_stdout(&output, label);

        let layers = json["monorepo_layers"].as_array().expect("monorepo_layers");
        assert_eq!(layers.len(), 1, "{label}: {json}");
        assert_eq!(layers[0]["lockfile"], expected, "{label}: {json}");
        // Only an exact match upgrades the layer and its packages.
        let provenance = if label == "match" { "lockfile" } else { "globbed" };
        assert_eq!(layers[0]["provenance"], provenance, "{label}: {json}");
        assert!(
            json["packages"]
                .as_array()
                .expect("packages")
                .iter()
                .all(|package| package["provenance"] == provenance),
            "{label}: {json}"
        );
        assert!(
            json.get("standalone_lockfiles").is_none(),
            "{label}: an empty standalone list is omitted: {json}"
        );
    }
}

#[test]
fn structure_human_output_names_each_layer_status_paths_and_members() {
    for (label, fixture, edit, _, lines) in cases() {
        let (_dir, root) = copied(fixture, edit);
        let output = run(&root, &["--plain", "repo", "structure"]);
        assert_quiet_success(&output, label);
        let stdout = human_stdout(&output, label);

        assert!(stdout.contains("Lockfiles -"), "{label}: {stdout}");
        for line in lines {
            assert!(stdout.contains(line), "{label}: missing `{line}` in: {stdout}");
        }
    }
}

/// A Poetry lockfile at a workspace package root is a repository-level
/// observation (ruling R2), in JSON and in the human section.
#[test]
fn a_standalone_lockfile_is_reported_in_json_and_human_output() {
    fn with_poetry_lock(root: &Path) {
        fs::copy(
            lockfile_fixtures().join("poetry-2.5.1/single-project/poetry.lock"),
            root.join("packages/alpha/poetry.lock"),
        )
        .expect("copy poetry.lock");
    }
    let (_dir, root) = copied("workspace", with_poetry_lock);

    let output = run(&root, &["repo", "structure", "--json"]);
    assert_quiet_success(&output, "json");
    let json = json_stdout(&output, "json");
    assert_eq!(
        json["standalone_lockfiles"],
        json!([{
            "root": "packages/alpha",
            "tool": "poetry",
            "status": "unverifiable",
            "paths": ["poetry.lock"],
            "reason": "no_membership_data",
            "extra": [],
            "missing": [],
        }]),
        "{json}"
    );
    assert_eq!(json["monorepo_layers"][0]["lockfile"]["status"], "match", "{json}");

    let output = run(&root, &["--plain", "repo", "structure"]);
    assert_quiet_success(&output, "human");
    let stdout = human_stdout(&output, "human");
    for line in [
        "pnpm workspaces (.): match",
        "Poetry (packages/alpha): unverifiable — this lockfile does not record which packages are workspace members",
        "- lockfile: poetry.lock",
    ] {
        assert!(stdout.contains(line), "missing `{line}` in: {stdout}");
    }
}

/// Bare `sniff repo --json` projects the same observation, and its
/// `structure` always carries `standalone_lockfiles`, as it does
/// `monorepo_layers`.
#[test]
fn aggregate_json_carries_the_layer_lockfile_and_standalone_list() {
    let (_dir, root) = copied("workspace-edited-missing", unchanged);
    git2::Repository::init(&root).expect("git init");

    let output = run(&root, &["repo", "--json"]);
    assert_quiet_success(&output, "aggregate");
    let json = json_stdout(&output, "aggregate");
    let structure = &json["structure"];
    assert_eq!(
        structure["monorepo_layers"][0]["lockfile"],
        json!({
            "status": "mismatch",
            "paths": ["pnpm-lock.yaml"],
            "reason": null,
            "extra": [],
            "missing": ["packages/beta"],
        }),
        "{json}"
    );
    assert_eq!(structure["standalone_lockfiles"], json!([]), "{json}");
}

/// A PHP-only project (`composer.json` and `composer.lock`, nothing else) is a
/// single-package repository whose Composer lockfile is a repository-level
/// observation (ruling R2), in JSON and in the human section.
#[test]
fn a_php_only_project_reports_its_composer_lockfile() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    copy_tree(&lockfile_fixtures().join("composer-2.10.3/single-project"), &root);

    let output = run(&root, &["repo", "structure", "--json"]);
    assert_quiet_success(&output, "json");
    let json = json_stdout(&output, "json");
    assert_eq!(json["is_monorepo"], false, "{json}");
    assert!(
        json.get("monorepo_layers").is_none(),
        "an empty layer list is omitted: {json}"
    );
    assert_eq!(
        json["standalone_lockfiles"],
        json!([{
            "root": "",
            "tool": "composer",
            "status": "unverifiable",
            "paths": ["composer.lock"],
            "reason": "no_membership_data",
            "extra": [],
            "missing": [],
        }]),
        "{json}"
    );
    assert_eq!(json["packages"][0]["name"], "fixture/fixture-root", "{json}");

    let output = run(&root, &["--plain", "repo", "structure"]);
    assert_quiet_success(&output, "human");
    let stdout = human_stdout(&output, "human");
    for line in [
        "Type: Single-package",
        "Composer (.): unverifiable — this lockfile does not record which packages are workspace members",
        "- lockfile: composer.lock",
    ] {
        assert!(stdout.contains(line), "missing `{line}` in: {stdout}");
    }
    assert!(!stdout.contains("workspaces"), "no layer is rendered: {stdout}");
}
