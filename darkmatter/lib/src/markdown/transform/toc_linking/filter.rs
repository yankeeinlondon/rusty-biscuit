//! Glob-based heading filter engine.

use super::types::{HeadingGlob, TocLinkingError};
use globset::{GlobBuilder, GlobMatcher};

/// Compiled heading filter that applies keep/filter glob rules.
pub struct HeadingFilter {
    keep_matchers: Vec<GlobMatcher>,
    filter_matchers: Vec<GlobMatcher>,
}

impl HeadingFilter {
    /// Compiles keep and filter glob patterns into a filter.
    pub fn new(
        keep_patterns: &[HeadingGlob],
        filter_patterns: &[HeadingGlob],
        line: usize,
    ) -> Result<Self, TocLinkingError> {
        let keep_matchers = compile_globs(keep_patterns, line)?;
        let filter_matchers = compile_globs(filter_patterns, line)?;
        Ok(Self {
            keep_matchers,
            filter_matchers,
        })
    }

    /// Returns true if the heading text passes keep/filter rules.
    ///
    /// Matching is on **raw heading text** (before cleanup).
    pub fn should_include(&self, heading_text: &str) -> bool {
        // Keep check: if keep patterns exist, heading must match at least one
        if !self.keep_matchers.is_empty()
            && !self.keep_matchers.iter().any(|m| m.is_match(heading_text))
        {
            return false;
        }

        // Filter check: heading must NOT match any filter pattern
        if self
            .filter_matchers
            .iter()
            .any(|m| m.is_match(heading_text))
        {
            return false;
        }

        true
    }

    /// Returns true if this filter has no patterns at all (pass-through).
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.keep_matchers.is_empty() && self.filter_matchers.is_empty()
    }
}

fn compile_globs(
    patterns: &[HeadingGlob],
    line: usize,
) -> Result<Vec<GlobMatcher>, TocLinkingError> {
    patterns
        .iter()
        .map(|g| {
            GlobBuilder::new(&g.pattern)
                .case_insensitive(!g.case_sensitive)
                .build()
                .map(|glob| glob.compile_matcher())
                .map_err(|e| TocLinkingError::InvalidGlob {
                    pattern: g.pattern.clone(),
                    line,
                    message: e.to_string(),
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glob(pattern: &str) -> HeadingGlob {
        HeadingGlob {
            pattern: pattern.to_string(),
            case_sensitive: false,
        }
    }

    fn glob_sensitive(pattern: &str) -> HeadingGlob {
        HeadingGlob {
            pattern: pattern.to_string(),
            case_sensitive: true,
        }
    }

    #[test]
    fn keep_whitelist() {
        let filter =
            HeadingFilter::new(&[glob("3.*")], &[], 1).unwrap();
        assert!(filter.should_include("3. Setup"));
        assert!(filter.should_include("3.2 Details"));
        assert!(!filter.should_include("4. Other"));
    }

    #[test]
    fn filter_blacklist() {
        let filter =
            HeadingFilter::new(&[], &[glob("3.*")], 1).unwrap();
        assert!(!filter.should_include("3. Setup"));
        assert!(filter.should_include("4. Other"));
    }

    #[test]
    fn combined_keep_and_filter() {
        let filter = HeadingFilter::new(
            &[glob("*important*")],
            &[glob("*draft*")],
            1,
        )
        .unwrap();
        assert!(filter.should_include("Very important topic"));
        assert!(!filter.should_include("important draft notes"));
        assert!(!filter.should_include("Unrelated topic"));
    }

    #[test]
    fn case_insensitive_default() {
        let filter =
            HeadingFilter::new(&[glob("setup*")], &[], 1).unwrap();
        assert!(filter.should_include("Setup Guide"));
        assert!(filter.should_include("setup guide"));
        assert!(filter.should_include("SETUP GUIDE"));
    }

    #[test]
    fn case_sensitive_with_caret() {
        let filter =
            HeadingFilter::new(&[glob_sensitive("Setup*")], &[], 1).unwrap();
        assert!(filter.should_include("Setup Guide"));
        assert!(!filter.should_include("setup guide"));
    }

    #[test]
    fn multiple_patterns_ored() {
        let filter = HeadingFilter::new(
            &[glob("3.*"), glob("4.*"), glob("*important*")],
            &[],
            1,
        )
        .unwrap();
        assert!(filter.should_include("3. Setup"));
        assert!(filter.should_include("4. Usage"));
        assert!(filter.should_include("Very important"));
        assert!(!filter.should_include("5. Other"));
    }

    #[test]
    fn empty_patterns_pass_all() {
        let filter = HeadingFilter::new(&[], &[], 1).unwrap();
        assert!(filter.should_include("Anything"));
        assert!(filter.is_empty());
    }
}
