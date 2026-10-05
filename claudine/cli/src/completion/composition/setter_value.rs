//! Committed directory / partial-path handling for the composition completer.
//!
//! [`gather_committed`] walks only inside the committed directory under the
//! roots the shared `FileReference` completion expansion supplies for the
//! token. High-profile scopes are intentionally **not** consulted — the spec
//! flips semantics once the user commits to a subtree.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;

use super::{Candidate, file_name_matches};
use crate::completion::frontmatter;
use crate::completion::fuzzy::{self, PartialLen};
use crate::completion::scopes::{ComposeMode, Scope, ScopeKind};
use crate::completion::walker;

/// CommittedDir / PartialPath: walk only inside the committed directory.
///
/// `roots` are the shared expansion's directories with `dir` already
/// appended, in resolution order (the launch directory, then the repository
/// root), so a file offered here is the file that composition resolves for
/// the same token. An earlier root ranks first, and the final dedup keeps its
/// candidate when two roots hold the same relative path.
pub(super) fn gather_committed(
    mode: ComposeMode,
    roots: &[PathBuf],
    dir: &str,
    active: &str,
) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for (rank, walk_root) in roots.iter().enumerate() {
        let rank = u8::try_from(rank).unwrap_or(u8::MAX);
        gather_committed_root(mode, walk_root, rank, dir, active, &mut seen, &mut out);
    }
    out
}

fn gather_committed_root(
    mode: ComposeMode,
    walk_root: &Path,
    source_rank: u8,
    dir: &str,
    active: &str,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Candidate>,
) {
    let partial_len = PartialLen::classify(active.chars().count());
    if !walk_root.is_dir() {
        return;
    }
    let scope = Scope {
        kind: ScopeKind::CommittedDir,
        path: walk_root.to_path_buf(),
        follow_links: true,
    };
    let entries = walker::walk_scope(&scope);

    for entry_path in entries {
        let is_dir = entry_path.is_dir();
        if is_dir && !partial_len.directories_allowed() {
            continue;
        }
        if !is_dir && !frontmatter::valid_for_mode(&entry_path, mode) {
            continue;
        }

        let canonical = std::fs::canonicalize(&entry_path).unwrap_or_else(|_| entry_path.clone());
        if !seen.insert(canonical) {
            continue;
        }
        let Some(name) = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        if partial_len.matching_enabled() {
            let matched = if is_dir {
                fuzzy::fuzzy_match(&name, active)
            } else {
                file_name_matches(&name, active)
            };
            if !matched {
                continue;
            }
        }
        let rel = match entry_path.strip_prefix(walk_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = to_portable_string(rel);
        let dir = to_portable_string(Path::new(dir));
        let insert = if dir.is_empty() {
            rel_str
        } else {
            format!("{}/{}", dir.trim_end_matches('/'), rel_str)
        };
        let insert = if is_dir {
            format!("{}/", insert.trim_end_matches('/'))
        } else {
            insert
        };
        out.push(Candidate {
            insert,
            source_rank,
        });
    }
}
