//! Committed directory / partial-path handling for the composition completer.
//!
//! [`gather_committed`] walks only inside the committed directory relative
//! to the repo root (or cwd when no repo). High-profile scopes are
//! intentionally **not** consulted — the spec flips semantics once the
//! user commits to a subtree.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;

use super::{Candidate, file_name_matches};
use crate::completion::frontmatter;
use crate::completion::fuzzy::{self, PartialLen};
use crate::completion::scopes::{self, ComposeMode, Scope, ScopeContext, ScopeKind};
use crate::completion::walker;

/// CommittedDir / PartialPath: walk only inside the committed directory
/// relative to the repo root (or cwd when no repo). High-profile scopes
/// are intentionally **not** consulted — the spec flips semantics once the
/// user commits to a subtree.
pub(super) fn gather_committed(
    mode: ComposeMode,
    ctx: &ScopeContext,
    dir: &str,
    active: &str,
) -> Vec<Candidate> {
    let partial_len = PartialLen::classify(active.chars().count());

    let base = scopes::effective_repo_root(ctx)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ctx.cwd.clone());
    let walk_root = base.join(dir);
    if !walk_root.is_dir() {
        return Vec::new();
    }
    let scope = Scope {
        kind: ScopeKind::CommittedDir,
        path: walk_root.clone(),
        follow_links: true,
    };
    let entries = walker::walk_scope(&scope);
    let mut out: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

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
        let rel = match entry_path.strip_prefix(&walk_root) {
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
            source_rank: 0,
        });
    }
    out
}
