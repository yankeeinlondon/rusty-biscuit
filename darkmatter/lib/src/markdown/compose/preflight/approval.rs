//! Approval-set boundary export.
//!
//! Turns the condition-blind collection into the deduped normalized approval
//! set the orchestrator boundary authorizes and hands back as the execution
//! membership source. Policy validation (builtin/user blacklist, whitelist) and
//! the single batched prompt live with the caller — Darkmatter discovers, the
//! caller authorizes — so this module only normalizes and dedupes.

use std::collections::HashSet;

use crate::markdown::compose::shell_expansion::types::ShellCommandEntry;

/// Returns the deduped normalized approval set from collected entries, in
/// first-discovery order.
///
/// Entries are already deduped by the collector, but callers may merge
/// additional sources before authorization, so dedup defensively here too.
pub fn approval_set(entries: &[ShellCommandEntry]) -> Vec<String> {
    let mut seen = HashSet::new();
    entries
        .iter()
        .filter(|entry| seen.insert(entry.normalized.clone()))
        .map(|entry| entry.normalized.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::shell_expansion::types::ShellCommandOrigin;
    use std::path::PathBuf;

    fn entry(normalized: &str) -> ShellCommandEntry {
        ShellCommandEntry {
            raw_command: normalized.to_string(),
            executable: normalized.split(' ').next().unwrap_or("").to_string(),
            args: Vec::new(),
            normalized: normalized.to_string(),
            source_file: PathBuf::from("<test>"),
            origin: ShellCommandOrigin::Body { line: 1 },
        }
    }

    #[test]
    fn dedupes_preserving_first_order() {
        let entries = vec![entry("echo a"), entry("echo b"), entry("echo a")];
        assert_eq!(
            approval_set(&entries),
            vec!["echo a".to_string(), "echo b".to_string()]
        );
    }

    #[test]
    fn empty_in_empty_out() {
        assert!(approval_set(&[]).is_empty());
    }
}
