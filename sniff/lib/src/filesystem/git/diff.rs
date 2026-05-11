//! Diff helpers used during status and commit inspection.
//!
//! Contains a single-pass diff aggregator that walks a `git2::Diff` exactly
//! once, accumulating per-path line stats and optionally per-path unified
//! patch strings. This avoids the previous O(dirty_files * diff_setup_cost)
//! scaling of issuing one pathspec-restricted diff per dirty file.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::Result;

/// Per-file line stats accumulated from a single diff walk.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LineStats {
    pub(crate) added: usize,
    pub(crate) removed: usize,
}

/// Walk a `git2::Diff` exactly once, accumulating per-path line stats and
/// optionally per-path unified patch strings.
///
/// Lines are attributed to the new file path when present, falling back to
/// the old file path. This keeps deletes attributed to the deleted path and
/// renames (when rename detection is off, the default) attributed as
/// add/delete pairs against their respective paths — matching the prior
/// behavior of the per-file `pathspec`-restricted diff loop.
pub(crate) fn aggregate_diff(
    diff: &git2::Diff,
    stats: &mut HashMap<PathBuf, LineStats>,
    mut patches: Option<&mut HashMap<PathBuf, String>>,
) -> Result<()> {
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        let Some(path) = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(PathBuf::from)
        else {
            return true;
        };

        match line.origin() {
            '+' => stats.entry(path.clone()).or_default().added += 1,
            '-' => stats.entry(path.clone()).or_default().removed += 1,
            _ => {}
        }

        if let Some(patches) = patches.as_deref_mut() {
            let entry = patches.entry(path).or_default();
            // Mirror the legacy diff_to_string format: prefix only content
            // lines, leave headers untouched.
            if matches!(line.origin(), '+' | '-' | ' ') {
                entry.push(line.origin());
            }
            if let Ok(content) = std::str::from_utf8(line.content()) {
                entry.push_str(content);
            }
        }
        true
    })?;
    Ok(())
}
