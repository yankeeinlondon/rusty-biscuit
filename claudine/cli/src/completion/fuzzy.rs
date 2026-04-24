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
//! a boolean predicate plus a simple tiebreak score, both of which a
//! hand-rolled 20-line matcher covers cleanly.
//!
//! The score returned by [`fuzzy_score`] is a tie-breaker only: lower is
//! better, computed as the distance the query's characters had to span in
//! the target (first match position + gap count). Callers that need a
//! stable sort should break ties by candidate string before sorting by
//! score.

// Phase 3 scaffolding. Callers land in `composition.rs`; tests exercise
// every public item today.
#![allow(dead_code)]

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

/// Subsequence match with a tiebreaker score.
///
/// Returns `Some(score)` when `query` is a subsequence of `target`, where
/// lower scores mean a tighter match. `None` means no match.
///
/// The score is the span (0-indexed last-match position − first-match
/// position). Contiguous matches score 0; widely spread matches score
/// higher. A mismatch on a character immediately aborts.
pub(crate) fn fuzzy_score(target: &str, query: &str) -> Option<u32> {
    if query.is_empty() {
        return Some(0);
    }
    let target_lower = target.to_ascii_lowercase();
    let query_lower = query.to_ascii_lowercase();
    let target_chars: Vec<char> = target_lower.chars().collect();

    let mut first: Option<usize> = None;
    let mut last: usize = 0;
    let mut cursor = 0usize;
    for q in query_lower.chars() {
        let mut found: Option<usize> = None;
        while cursor < target_chars.len() {
            if target_chars[cursor] == q {
                found = Some(cursor);
                cursor += 1;
                break;
            }
            cursor += 1;
        }
        match found {
            Some(pos) => {
                if first.is_none() {
                    first = Some(pos);
                }
                last = pos;
            }
            None => return None,
        }
    }
    let first = first.unwrap_or(0);
    Some((last - first) as u32)
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

impl PartialLen {
    /// Classify by the character count of the active segment.
    pub(crate) fn classify(active_chars: usize) -> Self {
        match active_chars {
            0 => Self::Empty,
            1..=2 => Self::Short,
            _ => Self::Long,
        }
    }

    /// Whether directories should be emitted at this prefix length.
    pub(crate) fn directories_allowed(self) -> bool {
        matches!(self, Self::Long)
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
    fn fuzzy_score_empty_query_is_zero() {
        assert_eq!(fuzzy_score("anything", ""), Some(0));
    }

    #[test]
    fn fuzzy_score_contiguous_match_is_smaller_than_spread() {
        let tight = fuzzy_score("plan", "pla").unwrap();
        let loose = fuzzy_score("p.....a", "pa").unwrap();
        assert!(
            tight < loose,
            "tight={tight} should be less than loose={loose}"
        );
    }

    #[test]
    fn fuzzy_score_returns_none_on_mismatch() {
        assert_eq!(fuzzy_score("plan", "xyz"), None);
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
}
