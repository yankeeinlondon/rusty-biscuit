//! Ambient `ctx.*` capture parity.
//!
//! Both tests compare a full `ComposeContext::capture()` against the ambient
//! `ComposeOptions::new()` path. They run inside a purpose-built fixture
//! repository rather than the rusty-biscuit checkout: a full capture walks the
//! whole repository (structure, documents, git status), which costs seconds on a
//! developer machine and over a minute on a cold two-core CI guest, and none of
//! that size adds coverage. The fixture is a two-package Cargo workspace with a
//! commit, staged and dirty files, documents, and a skill, so every catalog
//! category has at least one value to compare.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::context::catalog::context_variable_descriptors;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    /// The directory a compose runs from: inside the `alpha` package.
    cwd: PathBuf,
}

impl Fixture {
    fn build() -> Self {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("repo");
        let write = |relative: &str, contents: &str| {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        };

        write(
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = [\"alpha/lib\", \"alpha/cli\"]\n",
        );
        write(
            "alpha/lib/Cargo.toml",
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        );
        write("alpha/lib/src/lib.rs", "pub fn alpha() {}\n");
        write("alpha/lib/README.md", "# alpha\n");
        write(
            "alpha/cli/Cargo.toml",
            "[package]\nname = \"alpha-cli\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nalpha = { path = \"../lib\" }\n",
        );
        write("alpha/cli/src/main.rs", "fn main() {}\n");
        write("README.md", "# fixture\n");
        write("docs/guide.md", "# guide\n");
        write(
            ".claude/skills/alpha/SKILL.md",
            "---\nname: alpha\ndescription: fixture skill\n---\n# alpha\n",
        );

        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "fixture"]);
        // One dirty tracked file, one staged change, one untracked file, so the
        // file-change variables have values in both captures.
        write("alpha/lib/src/lib.rs", "pub fn alpha() {}\npub fn beta() {}\n");
        write("alpha/cli/src/main.rs", "fn main() { alpha::alpha(); }\n");
        git(&root, &["add", "alpha/cli/src/main.rs"]);
        write("alpha/cli/notes.md", "untracked\n");

        Self {
            cwd: root.join("alpha").join("lib"),
            _temp: temp,
        }
    }
}

/// Runs git with the repository-local identity only, so a caller's hook
/// environment or signing configuration cannot reach the fixture.
fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed");
}

/// Enters a directory for the ambient capture and restores the previous one on
/// drop. Tests that use it are `#[serial]` because the current directory is
/// process-wide.
struct CwdGuard(PathBuf);

impl CwdGuard {
    fn enter(dir: &Path) -> Self {
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(dir).expect("enter fixture");
        Self(previous)
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn compose_ambient(content: &str) -> String {
    let md: Markdown = content.into();
    let (composed, _report) = md
        .compose_with(ComposeOptions::new())
        .expect("compose must succeed");
    composed.content().trim().to_string()
}

fn compose_full_capture(content: &str) -> String {
    let md: Markdown = content.into();
    let options = ComposeOptions::new_with_context(ComposeContext::capture());
    let (composed, _report) = md
        .compose_with(options)
        .expect("compose must succeed");
    composed.content().trim().to_string()
}

#[test]
#[serial_test::serial]
fn ambient_options_resolve_date_time_without_discovery() {
    let fixture = Fixture::build();
    let _cwd = CwdGuard::enter(&fixture.cwd);
    let content = "today={{ ctx.today }}\n";

    assert_eq!(compose_ambient(content), compose_full_capture(content));
}

fn render_every_variable(options: ComposeOptions) -> HashMap<String, String> {
    let document: String = context_variable_descriptors()
        .iter()
        .map(|descriptor| format!("<{0}={{{{ ctx.{0} }}}}>", descriptor.name))
        .collect::<Vec<_>>()
        .join(" ");

    let md: Markdown = document.as_str().into();
    let (composed, _report) = md.compose_with(options).expect("compose must succeed");
    let content = composed.content().to_string();

    content
        .split('<')
        .filter_map(|segment| segment.split_once('>'))
        .filter_map(|(pair, _)| pair.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

/// The 2026-08-02 regression blanked `ctx.repo_root` and `ctx.os` while the
/// whole suite passed, because the tests that interpolate `ctx.*` overwhelmingly
/// pin the context with `ComposeContext::fixed_for_testing()`. A pinned context
/// already holds every value, so those tests verify that interpolation reads a
/// map — never that capture put anything in it. Only a real capture can fail
/// that way, and only two integration tests used one.
///
/// Driving this from `context_variable_descriptors()` rather than a hand-picked
/// list is the point: the regression touched two keys, and a list written by the
/// person who just fixed those two would have covered exactly those two. A new
/// `ctx.*` variable joins this test by existing.
#[test]
#[serial_test::serial]
fn every_catalog_variable_survives_ambient_options() {
    let fixture = Fixture::build();
    let _cwd = CwdGuard::enter(&fixture.cwd);

    let expected = render_every_variable(ComposeOptions::new_with_context(
        ComposeContext::capture(),
    ));
    let ambient = render_every_variable(ComposeOptions::new());

    // The fixture must give the comparison something to compare: every catalog
    // category needs at least one variable the full capture rendered non-empty,
    // or a blanked group could hide behind an empty-equals-empty pass.
    let mut categories: Vec<&str> = context_variable_descriptors()
        .iter()
        .map(|descriptor| descriptor.category)
        .collect();
    categories.sort_unstable();
    categories.dedup();
    let silent: Vec<&str> = categories
        .into_iter()
        .filter(|category| {
            !context_variable_descriptors()
                .iter()
                .filter(|descriptor| descriptor.category == *category)
                .any(|descriptor| expected.get(descriptor.name).is_some_and(|v| !v.is_empty()))
        })
        .collect();
    assert!(
        silent.is_empty(),
        "the fixture rendered no value for any variable in these categories: {silent:?}"
    );

    let blanked: Vec<&str> = context_variable_descriptors()
        .iter()
        .map(|descriptor| descriptor.name)
        .filter(|name| match (expected.get(*name), ambient.get(*name)) {
            (Some(want), Some(got)) => !want.is_empty() && got.is_empty(),
            _ => false,
        })
        .collect();

    assert!(
        blanked.is_empty(),
        "ambient options rendered these `ctx.*` variables empty where a full \
         capture rendered a value: {blanked:?}"
    );
}

// The other half of the rule — that a document naming no `ctx.*` group still
// captures nothing — is asserted at construction by
// `compose::context::options::tests::new_captures_no_discovery_derived_group`,
// which reads the crate-private context directly instead of inferring the
// captured set from rendered output.
