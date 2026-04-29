//! Prefix and fuzzy subsequence matching for completion candidates.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. The
//! composition completer gates between three matching regimes based on
//! typed-prefix length (see spec §5.3):
//!
//! - **0 characters** — no matching; every candidate passes.
//! - **1–2 characters** — fuzzy subsequence match on filenames only.
//!   Directories are elided at this length.
//! - **3+ characters** — fuzzy subsequence match on both filenames **and**
//!   directory names.
//!
//! Matching is always case-insensitive. The subsequence test mirrors the
//! classic Skim / fzf behavior — every character of the query must appear
//! in the target in order, but not necessarily adjacently. This is a
//! deliberate choice over a Skim-crate dependency: the behavior we need is
//! a boolean predicate, which a hand-rolled 20-line matcher covers cleanly.

/// Boolean subsequence match.
///
/// Returns `true` when every character of `query` appears in `target` in
/// order, case-insensitively. An empty query always matches — the caller
/// is expected to apply the length-0 rule before calling in that case.
pub(crate) fn fuzzy_match(target: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let target_lower = target.to_ascii_lowercase();
    let query_lower = query.to_ascii_lowercase();
    let mut target_iter = target_lower.chars();
    for q in query_lower.chars() {
        let mut found = false;
        for t in target_iter.by_ref() {
            if t == q {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

/// Case-insensitive prefix test.
pub(crate) fn prefix_match(target: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    target
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
}

/// Classification of a typed partial by character count.
///
/// The pipeline in `composition.rs` consults this to decide whether to
/// surface directories, and whether matching is fuzzy or a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartialLen {
    /// No characters typed (empty string).
    Empty,
    /// 1–2 characters typed.
    Short,
    /// 3 or more characters typed.
    Long,
}

/// Directory matching regime for the repo-wide directory walk.
///
/// The composition pipeline performs a second walk over the repo / CWD
/// root (independent of the high-profile file scopes) to surface
/// directory candidates. The matching strategy varies by typed-prefix
/// length: short prefixes use case-insensitive starting-substring
/// matches so 1–2 character partials are useful; longer prefixes use
/// fuzzy subsequence matching to align with the file-matching contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirMatchMode {
    /// No directory matching — empty partial.
    None,
    /// Case-insensitive starting-substring match (1–2 character partials).
    Prefix,
    /// Case-insensitive fuzzy subsequence match (3+ character partials).
    Fuzzy,
}

impl PartialLen {
    /// Classify by the character count of the active segment.
    pub(crate) fn classify(active_chars: usize) -> Self {
        match active_chars {
            0 => Self::Empty,
            1..=2 => Self::Short,
            _ => Self::Long,
        }
    }

    /// Whether directories should be emitted at this prefix length under
    /// the **legacy high-profile** scope set.
    ///
    /// Kept for the magic-path and committed-directory pipelines, which
    /// retain the original "directories only at Long" contract. The
    /// repo-wide directory walk added by the review-plan uses
    /// [`Self::dir_match_mode`] instead so 1–2 character prefixes still
    /// surface directories.
    pub(crate) fn directories_allowed(self) -> bool {
        matches!(self, Self::Long)
    }

    /// Directory match mode for the repo-wide walk.
    ///
    /// `Empty` partials emit no directory candidates at all; `Short`
    /// prefixes use prefix matching so the user does not have to type
    /// every character of a deeply-named directory; `Long` prefixes
    /// switch to fuzzy matching to align with file-matching behavior.
    pub(crate) fn dir_match_mode(self) -> DirMatchMode {
        match self {
            Self::Empty => DirMatchMode::None,
            Self::Short => DirMatchMode::Prefix,
            Self::Long => DirMatchMode::Fuzzy,
        }
    }

    /// Whether matching should apply at all (vs. accepting every entry).
    pub(crate) fn matching_enabled(self) -> bool {
        !matches!(self, Self::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_match_empty_query_matches_anything() {
        assert!(fuzzy_match("anything", ""));
        assert!(fuzzy_match("", ""));
    }

    #[test]
    fn fuzzy_match_exact_substring_matches() {
        assert!(fuzzy_match("plan.md", "plan"));
        assert!(fuzzy_match("compose-plan", "compose"));
    }

    #[test]
    fn fuzzy_match_subsequence_matches() {
        // 'p','l','n' all appear in order in "plan.md"
        assert!(fuzzy_match("plan.md", "pln"));
        // same for a longer target
        assert!(fuzzy_match("implementation-plan", "impl"));
        assert!(fuzzy_match("implementation-plan", "ipn"));
    }

    #[test]
    fn fuzzy_match_is_case_insensitive() {
        assert!(fuzzy_match("Plan.md", "pln"));
        assert!(fuzzy_match("plan.md", "PLN"));
        assert!(fuzzy_match("IMPLementation", "imp"));
    }

    #[test]
    fn fuzzy_match_no_subsequence_fails() {
        // Order matters — 'n','p' in 'plan' is backwards
        assert!(!fuzzy_match("plan", "np"));
        // missing characters
        assert!(!fuzzy_match("plan", "plax"));
    }

    #[test]
    fn fuzzy_match_consumes_characters_in_order() {
        // Must NOT reuse a character; 'a' in "aa" can only match once.
        assert!(!fuzzy_match("a", "aa"));
        assert!(fuzzy_match("aa", "aa"));
    }

    #[test]
    fn prefix_match_respects_case() {
        assert!(prefix_match("Plan.md", "pla"));
        assert!(prefix_match("plan.md", "PLA"));
        assert!(!prefix_match("plan.md", "lan"));
    }

    #[test]
    fn prefix_match_empty_prefix_always_passes() {
        assert!(prefix_match("anything", ""));
    }

    #[test]
    fn partial_len_classify_boundaries() {
        assert_eq!(PartialLen::classify(0), PartialLen::Empty);
        assert_eq!(PartialLen::classify(1), PartialLen::Short);
        assert_eq!(PartialLen::classify(2), PartialLen::Short);
        assert_eq!(PartialLen::classify(3), PartialLen::Long);
        assert_eq!(PartialLen::classify(100), PartialLen::Long);
    }

    #[test]
    fn partial_len_directories_only_at_long() {
        assert!(!PartialLen::Empty.directories_allowed());
        assert!(!PartialLen::Short.directories_allowed());
        assert!(PartialLen::Long.directories_allowed());
    }

    #[test]
    fn partial_len_matching_disabled_only_at_empty() {
        assert!(!PartialLen::Empty.matching_enabled());
        assert!(PartialLen::Short.matching_enabled());
        assert!(PartialLen::Long.matching_enabled());
    }

    #[test]
    fn partial_len_dir_match_mode_progression() {
        // Empty → no matching, Short → prefix, Long → fuzzy.
        assert_eq!(PartialLen::Empty.dir_match_mode(), DirMatchMode::None);
        assert_eq!(PartialLen::Short.dir_match_mode(), DirMatchMode::Prefix);
        assert_eq!(PartialLen::Long.dir_match_mode(), DirMatchMode::Fuzzy);
    }
}
