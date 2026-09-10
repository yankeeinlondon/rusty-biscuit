//! Compiled exit-expression set + per-line matching.
//!
//! Phase 2 keeps this module independent of the config layer (Phase 3) by
//! accepting a small local input struct, [`ExitExpressionInput`], at
//! compile time. Phase 3's resolved config maps `ExitExpressionEntry →
//! ExitExpressionInput` with a one-line conversion, so the detector never
//! depends on the config crate.

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{ClaudineError, Result};

/// Whether an [`ExitExpressionInput`] is matched by literal substring or by
/// regex.
///
/// Literal is the default kind to avoid metacharacter surprises (e.g. the
/// regex `STOP.` matching `STOPS`); regex is the opt-in for anchors,
/// alternation, and inline flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    /// Literal substring match (uses `str::contains`).
    #[default]
    Literal,
    /// Regex `is_match` test against the compiled pattern.
    Regex,
}

/// Minimal input shape for compiling an exit-expression entry. Decouples
/// Phase 2 from Phase 3's resolved config: Phase 3 builds these from
/// `ExitExpressionEntry` after layer resolution, then hands them to
/// [`CompiledExitExpressions::compile`].
///
/// One input may carry multiple `patterns` (they share a single
/// `kind`/`ignore_case`/`scope`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitExpressionInput {
    /// One or more patterns sharing the entry's kind/scope.
    pub patterns: Vec<String>,
    /// Literal-vs-regex selection.
    pub kind: PatternKind,
    /// Literal-only case folding (`ignore_case`). Regex defers to inline
    /// flags (`(?i)`); this flag is ignored when `kind == Regex`.
    pub ignore_case: bool,
    /// Optional scope string verbatim
    /// (e.g. `"opencode/kimi-for-coding/k2p7"`); `None` is the global
    /// wildcard.
    pub scope: Option<String>,
}

/// One compiled entry held inside [`CompiledExitExpressions`].
#[derive(Debug, Clone)]
enum CompiledEntry {
    /// Literal substring (already lowercased on both sides at match time
    /// when `ignore_case` is set).
    Literal {
        needle: String,
        ignore_case: bool,
        pattern: String,
        scope: Option<String>,
    },
    /// Precompiled regex.
    Regex {
        regex: Regex,
        pattern: String,
        scope: Option<String>,
    },
}

/// Resolved, precompiled exit-expression set. Built **once** before
/// streaming (never per line) and matched per completed line via
/// [`Self::matches_line`].
///
/// An empty set is a no-op: [`Self::matches_line`] always returns `None`
/// with zero per-line cost.
#[derive(Debug, Clone, Default)]
pub struct CompiledExitExpressions {
    entries: Vec<CompiledEntry>,
}

impl CompiledExitExpressions {
    /// Compile every entry, precompiling regexes so the per-line match
    /// path is cheap. Invalid regex surfaces here (at config-load / wiring
    /// time), never mid-stream.
    pub fn compile(entries: &[ExitExpressionInput]) -> Result<Self> {
        let mut compiled: Vec<CompiledEntry> = Vec::with_capacity(entries.len());
        for entry in entries {
            if entry.patterns.is_empty() {
                return Err(ClaudineError::ConfigValidation(
                    "exit-expression entry has an empty `patterns` list".to_string(),
                ));
            }
            for pattern in &entry.patterns {
                match entry.kind {
                    PatternKind::Literal => compiled.push(CompiledEntry::Literal {
                        needle: pattern.clone(),
                        ignore_case: entry.ignore_case,
                        pattern: pattern.clone(),
                        scope: entry.scope.clone(),
                    }),
                    PatternKind::Regex => {
                        let regex = Regex::new(pattern).map_err(|source| {
                            ClaudineError::ProtectRuleParse {
                                pattern: pattern.clone(),
                                source,
                            }
                        })?;
                        compiled.push(CompiledEntry::Regex {
                            regex,
                            pattern: pattern.clone(),
                            scope: entry.scope.clone(),
                        });
                    }
                }
            }
        }
        Ok(Self { entries: compiled })
    }

    /// Build an empty set (matches nothing).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether the set is empty (no per-line cost when true).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Test a completed line against every entry in declaration order;
    /// return the first match's `(pattern, scope)` or `None`.
    ///
    /// Literal entries use `str::contains`, honoring `ignore_case` by
    /// lowercasing both sides. Regex entries defer to the regex's
    /// `is_match`, so case sensitivity is controlled by inline flags
    /// (`(?i)`).
    pub fn matches_line(&self, line: &str) -> Option<(&str, &Option<String>)> {
        for entry in &self.entries {
            let matched = match entry {
                CompiledEntry::Literal {
                    needle,
                    ignore_case,
                    ..
                } => {
                    if *ignore_case {
                        line.to_lowercase().contains(&needle.to_lowercase())
                    } else {
                        line.contains(needle.as_str())
                    }
                }
                CompiledEntry::Regex { regex, .. } => regex.is_match(line),
            };
            if matched {
                let (pattern, scope) = match entry {
                    CompiledEntry::Literal {
                        pattern, scope, ..
                    } => (pattern.as_str(), scope),
                    CompiledEntry::Regex {
                        pattern, scope, ..
                    } => (pattern.as_str(), scope),
                };
                return Some((pattern, scope));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(pattern: &str, scope: Option<&str>, ignore_case: bool) -> ExitExpressionInput {
        ExitExpressionInput {
            patterns: vec![pattern.to_string()],
            kind: PatternKind::Literal,
            ignore_case,
            scope: scope.map(|s| s.to_string()),
        }
    }

    fn re(pattern: &str, scope: Option<&str>) -> ExitExpressionInput {
        ExitExpressionInput {
            patterns: vec![pattern.to_string()],
            kind: PatternKind::Regex,
            ignore_case: false,
            scope: scope.map(|s| s.to_string()),
        }
    }

    #[test]
    fn empty_set_matches_nothing() {
        let set = CompiledExitExpressions::empty();
        assert!(set.is_empty());
        assert!(set.matches_line("anything").is_none());
    }

    #[test]
    fn literal_substring_matches_case_sensitive() {
        let set = CompiledExitExpressions::compile(&[lit("STOP.", None, false)]).unwrap();
        assert!(set.matches_line("the model said STOP.").is_some());
        // Case-sensitive default: `StopS` does NOT match `STOP.`
        // (the metacharacter-surprise guardrail — E3a).
        assert!(set.matches_line("the model said StopS").is_none());
    }

    #[test]
    fn literal_ignore_case_matches() {
        let set = CompiledExitExpressions::compile(&[lit("STOP.", None, true)]).unwrap();
        assert!(set.matches_line("the model said stop.").is_some());
        assert!(set.matches_line("STOP.").is_some());
    }

    #[test]
    fn regex_inline_flags_match_without_ignore_case() {
        let set = CompiledExitExpressions::compile(&[re(r"(?i)stop\.", None)]).unwrap();
        assert!(set.matches_line("ok stop.").is_some());
        assert!(set.matches_line("ok STOP.").is_some());
    }

    #[test]
    fn regex_default_is_case_sensitive() {
        let set = CompiledExitExpressions::compile(&[re(r"stop\.", None)]).unwrap();
        assert!(set.matches_line("ok stop.").is_some());
        assert!(set.matches_line("ok STOP.").is_none());
    }

    #[test]
    fn first_matching_entry_wins_and_carries_scope() {
        let set = CompiledExitExpressions::compile(&[
            lit("nomatch", Some("opencode"), false),
            lit("STOP.", Some("opencode/kimi-for-coding/k2p7"), false),
            lit("late", None, false),
        ])
        .unwrap();
        let (pattern, scope) = set.matches_line("STOP.").expect("second entry should match");
        assert_eq!(pattern, "STOP.");
        assert_eq!(scope.as_deref(), Some("opencode/kimi-for-coding/k2p7"));
    }

    #[test]
    fn empty_patterns_list_rejected_at_compile() {
        let bad = ExitExpressionInput {
            patterns: vec![],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        };
        assert!(CompiledExitExpressions::compile(&[bad]).is_err());
    }

    #[test]
    fn invalid_regex_rejected_at_compile() {
        let result = CompiledExitExpressions::compile(&[re("[", None)]);
        assert!(result.is_err(), "invalid regex must fail at compile time");
    }

    #[test]
    fn multiple_patterns_in_one_entry_share_kind_and_scope() {
        let entry = ExitExpressionInput {
            patterns: vec!["STOP.".to_string(), "Bye.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: Some("opencode".to_string()),
        };
        let set = CompiledExitExpressions::compile(&[entry]).unwrap();
        let (p1, s1) = set.matches_line("ok STOP.").expect("first pattern matches");
        assert_eq!(p1, "STOP.");
        assert_eq!(s1.as_deref(), Some("opencode"));
        let (p2, _) = set.matches_line("ok Bye.").expect("second pattern matches");
        assert_eq!(p2, "Bye.");
    }
}
