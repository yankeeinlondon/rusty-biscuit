//! Persistent state for system prompt change detection.
//!
//! Records the hash of the most recently composed system prompt per scope
//! (repo root or launch CWD) under `~/.claudine/state/system_prompts/`.
//! A subsequent run can determine whether the composed prompt has changed
//! by comparing against the stored hash, which controls whether the header
//! is shown in the default precedence branch.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use biscuit_hash::{blake3_hash, xx_hash};

use crate::reporting::paths::claudine_home_dir;

/// Compute a stable hash key for the composed prompt content.
///
/// Combines the primary composed markdown with any non-interactive appendix
/// so that changing only the appendix still counts as a change.
pub fn compute_prompt_hash(composed_markdown: &str, appendix: Option<&str>) -> String {
    let combined = match appendix {
        Some(a) => format!("{}\n---\n{}", composed_markdown, a),
        None => composed_markdown.to_string(),
    };
    blake3_hash(&combined)
}

/// Resolve the on-disk state file for a given scope path.
fn state_file_for_scope(scope: &Path) -> io::Result<PathBuf> {
    let home = claudine_home_dir().map_err(io::Error::other)?;
    let dir = home.join("state").join("system_prompts");
    fs::create_dir_all(&dir)?;
    let scope_key = xx_hash(&scope.to_string_lossy());
    Ok(dir.join(format!("{scope_key:016x}.txt")))
}

/// Read the previously stored hash for the given scope (if any).
pub fn read_last_hash(scope: &Path) -> Option<String> {
    let path = state_file_for_scope(scope).ok()?;
    fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
}

/// Persist a new hash for the given scope, overwriting any previous value.
pub fn write_hash(scope: &Path, hash: &str) -> io::Result<()> {
    let path = state_file_for_scope(scope)?;
    fs::write(&path, hash)
}

/// Determine whether the composed prompt is unchanged relative to the last
/// recorded hash for `scope`, then persist the new hash for future runs.
///
/// Returns `true` when the prompt is byte-for-byte identical to the last
/// observed value at this scope. Returns `false` on first run, on change,
/// or when reading/writing state fails.
pub fn check_and_record(scope: &Path, composed_markdown: &str, appendix: Option<&str>) -> bool {
    let new_hash = compute_prompt_hash(composed_markdown, appendix);
    let unchanged = read_last_hash(scope)
        .map(|prev| prev == new_hash)
        .unwrap_or(false);
    let _ = write_hash(scope, &new_hash);
    unchanged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let a = compute_prompt_hash("hello", Some("world"));
        let b = compute_prompt_hash("hello", Some("world"));
        assert_eq!(a, b);
    }

    #[test]
    fn hash_changes_with_appendix() {
        let a = compute_prompt_hash("hello", None);
        let b = compute_prompt_hash("hello", Some("appendix"));
        assert_ne!(a, b);
    }

    #[test]
    fn hash_changes_with_content() {
        let a = compute_prompt_hash("hello", None);
        let b = compute_prompt_hash("world", None);
        assert_ne!(a, b);
    }
}
