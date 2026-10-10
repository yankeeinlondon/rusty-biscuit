//! Staging prompts, with everything they transclude, into a test workspace.
//!
//! A prompt pulls shared fragments in with `::file` directives, and a fragment
//! added to one used to break every test that staged that prompt from a
//! hand-kept list of snippets. Staging follows the directives instead, so the
//! staged set always matches what the prompt reads.
//!
//! The repository's `prompts/` are internal tools, never tested by CI, so a
//! Level 1 or Level 2 test stages a frozen copy through
//! [`stage_frozen_prompts`]: `tests/fixtures/frozen_prompts/` holds them
//! byte-for-byte at their repository-relative paths, intentionally not kept in
//! step with `prompts/`. Only the opt-in `prompts` binary (`prompt-tests`)
//! reads the live tree, through [`stage_shipped_prompts`].

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// The repository root, two levels above the `claudine-cli` manifest.
pub fn workspace_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("claudine/cli parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The frozen copy of the repository's `prompts/` directory.
pub fn frozen_prompts_dir() -> PathBuf {
    biscuit_test_harness::manifest_dir!().join("tests/fixtures/frozen_prompts/prompts")
}

/// Stages frozen prompts the way [`stage_shipped_prompts`] stages live ones.
/// Each entry is relative to `prompts/` (`"implement.md"`), so the spelling
/// never names a live prompt.
pub fn stage_frozen_prompts(staged: &Path, entries: &[&str]) -> Vec<String> {
    let pending = entries.iter().map(|&entry| entry.to_owned()).collect();
    stage_from(&frozen_prompts_dir(), staged, pending)
}

/// Copies each `entries` prompt, a repository path under `prompts/`, from
/// `repository` into `staged`, which stands in for `prompts/`, together with
/// every file those prompts transclude through `::file`, recursively. Returns
/// the staged paths relative to `staged`, in order.
///
/// A directive whose target is an expression (`{{…}}`) is skipped: it names
/// nothing a test can stage ahead of time. A target outside `prompts/`, or one
/// spelled `@…`, panics, because the staged prompt could not resolve it.
/// Route targets reached through frontmatter rather than `::file` are not
/// followed; list them in `entries`.
pub fn stage_shipped_prompts(repository: &Path, staged: &Path, entries: &[&str]) -> Vec<String> {
    let pending = entries
        .iter()
        .map(|entry| {
            entry
                .strip_prefix("prompts/")
                .unwrap_or_else(|| panic!("`{entry}` is not a repository path under `prompts/`"))
                .to_owned()
        })
        .collect();
    stage_from(&repository.join("prompts"), staged, pending)
}

fn stage_from(shipped: &Path, staged: &Path, mut pending: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    while let Some(relative) = pending.pop() {
        if !seen.insert(relative.clone()) {
            continue;
        }
        let source = shipped.join(&relative);
        let content = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("read shipped prompt {}: {error}", source.display()));
        super::write(&staged.join(&relative), &content);
        let directory = Path::new(&relative).parent().unwrap_or(Path::new(""));
        for target in transcluded_paths(&content) {
            pending.push(resolve_within(directory, &target).unwrap_or_else(|| {
                panic!("`{relative}` transcludes `{target}`, which is outside `prompts/`")
            }));
        }
    }
    seen.into_iter().collect()
}

/// The static targets of a document's `::file` directives.
fn transcluded_paths(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("::file"))
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .filter_map(|rest| {
            let rest = rest.trim_start();
            let target = match rest.strip_prefix('"') {
                Some(quoted) => quoted.split('"').next()?,
                None => rest.split_whitespace().next()?,
            };
            if target.contains("{{") {
                return None;
            }
            assert!(
                !target.starts_with('@'),
                "`::file {target}` is anchored outside the prompt's directory; stage it explicitly"
            );
            Some(target.to_owned())
        })
        .collect()
}

/// `target` joined onto `directory`, with `.` and `..` folded, as a
/// `/`-separated relative path; `None` when it climbs above the root.
fn resolve_within(directory: &Path, target: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in directory.join(target).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}
