/// Parsed resource filter with optional negation and exact-match modes.
///
/// ## Syntax
///
/// | Input      | Pattern  | Negated | Exact |
/// |------------|----------|---------|-------|
/// | `rust`     | `rust`   | false   | false |
/// | `rust!`    | `rust`   | false   | true  |
/// | `-rust`    | `rust`   | true    | false |
/// | `!rust`    | `rust`   | true    | false |
/// | `-rust!`   | `rust`   | true    | true  |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceFilter {
    /// Lowercased pattern (prefix/suffix markers stripped).
    pub pattern: String,
    /// Whether this is a negation filter (prefixed with `!` or `-`).
    pub negated: bool,
    /// Whether this requires an exact name match (suffixed with `!`).
    pub exact: bool,
}

impl ResourceFilter {
    /// Parse a raw filter string into a `ResourceFilter`.
    ///
    /// Returns `None` if the pattern is empty after stripping markers.
    pub fn parse(raw: &str) -> Option<Self> {
        let mut s = raw.trim();
        let negated = if s.starts_with('!') || s.starts_with('-') {
            s = &s[1..];
            true
        } else {
            false
        };
        let exact = if s.ends_with('!') {
            s = &s[..s.len() - 1];
            true
        } else {
            false
        };
        let pattern = s.to_lowercase();
        if pattern.is_empty() {
            return None;
        }
        Some(Self {
            pattern,
            negated,
            exact,
        })
    }

    /// Parse a slice of raw filter strings, discarding any that are empty.
    pub fn parse_all(raw: &[String]) -> Vec<Self> {
        raw.iter().filter_map(|s| Self::parse(s)).collect()
    }

    /// Test whether this filter matches the given resource name.
    pub fn matches(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        if self.exact {
            lower == self.pattern
        } else {
            lower.contains(&self.pattern)
        }
    }

    /// Decide whether a resource name should be retained given a set of filters.
    ///
    /// - Negation always wins: if any negative filter matches, the name is excluded.
    /// - If positive filters exist, at least one must match.
    /// - If only negative filters exist, everything not excluded is kept.
    pub fn retain(filters: &[Self], name: &str) -> bool {
        // Check negations first — they always win
        for f in filters {
            if f.negated && f.matches(name) {
                return false;
            }
        }
        // Check positive filters
        let has_positive = filters.iter().any(|f| !f.negated);
        if has_positive {
            filters.iter().any(|f| !f.negated && f.matches(name))
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_pattern() {
        let f = ResourceFilter::parse("rust").unwrap();
        assert_eq!(f.pattern, "rust");
        assert!(!f.negated);
        assert!(!f.exact);
    }

    #[test]
    fn parse_negated_prefix_bang() {
        let f = ResourceFilter::parse("!rust").unwrap();
        assert!(f.negated);
        assert!(!f.exact);
    }

    #[test]
    fn parse_negated_prefix_dash() {
        let f = ResourceFilter::parse("-rust").unwrap();
        assert!(f.negated);
    }

    #[test]
    fn parse_exact_suffix() {
        let f = ResourceFilter::parse("rust!").unwrap();
        assert!(!f.negated);
        assert!(f.exact);
    }

    #[test]
    fn parse_negated_and_exact() {
        let f = ResourceFilter::parse("-rust!").unwrap();
        assert!(f.negated);
        assert!(f.exact);
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(ResourceFilter::parse("").is_none());
        assert!(ResourceFilter::parse("!").is_none());
        assert!(ResourceFilter::parse("-!").is_none());
    }

    #[test]
    fn matches_substring() {
        let f = ResourceFilter::parse("rus").unwrap();
        assert!(f.matches("rust"));
        assert!(f.matches("RUST"));
        assert!(!f.matches("python"));
    }

    #[test]
    fn matches_exact() {
        let f = ResourceFilter::parse("rust!").unwrap();
        assert!(f.matches("rust"));
        assert!(f.matches("RUST"));
        assert!(!f.matches("rusty"));
    }

    #[test]
    fn retain_positive_only() {
        let filters = ResourceFilter::parse_all(&["rust".into(), "py".into()]);
        assert!(ResourceFilter::retain(&filters, "rust"));
        assert!(ResourceFilter::retain(&filters, "python"));
        assert!(!ResourceFilter::retain(&filters, "go"));
    }

    #[test]
    fn retain_negation_wins() {
        let filters = ResourceFilter::parse_all(&["rust".into(), "-rusty".into()]);
        assert!(ResourceFilter::retain(&filters, "rust"));
        assert!(!ResourceFilter::retain(&filters, "rusty-biscuit"));
    }

    #[test]
    fn retain_negation_only() {
        let filters = ResourceFilter::parse_all(&["-test".into()]);
        assert!(ResourceFilter::retain(&filters, "rust"));
        assert!(!ResourceFilter::retain(&filters, "testing"));
    }
}
