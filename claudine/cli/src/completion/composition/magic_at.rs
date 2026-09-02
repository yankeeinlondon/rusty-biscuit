//! Magic (`@...`) path handling for the composition completer.
//!
//! Magic mode enumerates the ordered roots supplied by
//! [`biscuit_file::FileReference::complete_partial_in_context`], applies Claudine's
//! frontmatter gate, and renders the shared completion prefix plus each
//! basename. Runtime composition registers the identical roots, so the emitted
//! value selects the file completion inspected. Directories are intentionally
//! not surfaced under `@`.

use std::collections::HashSet;

use biscuit_file::PartialCompletion;

use super::{Candidate, file_name_matches};
use crate::completion::frontmatter;
use crate::completion::fuzzy::PartialLen;
use crate::completion::scopes::ComposeMode;

/// Entry point for magic (`@...`) resolution.
///
/// Walks the shared completion roots in priority order. The first root to
/// contribute a basename owns its rank; later duplicates are discarded.
pub(super) fn gather_magic(
    mode: ComposeMode,
    completion: &PartialCompletion,
    active_override: Option<&str>,
) -> Vec<Candidate> {
    let active = active_override.unwrap_or_else(|| completion.active_segment());
    let partial_len = PartialLen::classify(active.chars().count());

    let mut out: Vec<Candidate> = Vec::new();
    // Deduplicate by lowercased basename so a filename present in several
    // scopes surfaces once; the closest scope (iterated first) keeps it.
    let mut seen_basenames: HashSet<String> = HashSet::new();

    for (rank, walk_root) in completion.roots().iter().enumerate() {
        let Ok(entries) = std::fs::read_dir(walk_root) else {
            continue;
        };
        let rank = rank.min(u8::MAX as usize) as u8;
        for entry_path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            if entry_path.is_dir() {
                continue;
            }
            if !frontmatter::valid_for_mode(&entry_path, mode) {
                continue;
            }
            let Some(basename) = entry_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if partial_len.matching_enabled() && !file_name_matches(basename, active) {
                continue;
            }
            if !seen_basenames.insert(basename.to_ascii_lowercase()) {
                continue;
            }
            out.push(Candidate {
                insert: format!("{}{basename}", completion.rendered_prefix()),
                source_rank: rank,
            });
        }
    }
    out
}
