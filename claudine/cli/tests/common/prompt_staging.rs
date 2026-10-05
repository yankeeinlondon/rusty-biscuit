//! Staging shipped prompts, with everything they transclude, into a test
//! workspace.
//!
//! A shipped prompt pulls shared fragments in with `::file` directives, and a
//! fragment added to one used to break every test that staged that prompt from
//! a hand-kept list of snippets. [`stage_shipped_prompts`] follows the
//! directives instead, so the staged set always matches what the prompt reads.
//!
//! Call sites bind `let repository = workspace_root();` and name each entry
//! by its full repository path (`"prompts/implement.md"`). CI's test-input
//! index (`docs/cicd/test-inputs.md`) reads that spelling as a read of the
//! entry, and follows the entry's `::file` directives, so a change to the entry
//! or to any fragment it transcludes schedules the test.

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
    let shipped = repository.join("prompts");
    let mut seen = BTreeSet::new();
    let mut pending: Vec<String> = entries
        .iter()
        .map(|entry| {
            entry
                .strip_prefix("prompts/")
                .unwrap_or_else(|| panic!("`{entry}` is not a repository path under `prompts/`"))
                .to_owned()
        })
        .collect();
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
