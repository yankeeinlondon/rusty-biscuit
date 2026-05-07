//! Rule 2: fuzzy `--provider <value>` canonicalization.
//!
//! Rewrites `--provider cl` → `--provider claude`, etc.

/// True when a `--provider` value token is a candidate for fuzzy rewrite.
///
/// Empty values and hyphen-prefixed tokens (which look like flags) are left
/// alone so clap can emit its native "a value is required" / invalid-value
/// errors instead of the normalizer silently swallowing them.
pub(crate) fn is_fuzzy_provider_value(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    #[test]
    fn rule_2_rewrites_cl_to_claude_via_space_form() {
        let input = argv(&["claudine", "compose", "--provider", "cl"]);
        let expected = argv(&["claudine", "compose", "--provider", "claude"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_2_rewrites_gem_to_gemini_via_space_form() {
        let input = argv(&["claudine", "compose", "--provider", "gem"]);
        let expected = argv(&["claudine", "compose", "--provider", "gemini"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_2_rewrites_gem_via_equals_form() {
        let input = argv(&["claudine", "compose", "--provider=gem"]);
        let expected = argv(&["claudine", "compose", "--provider=gemini"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_2_canonicalizes_exact_slug_unchanged() {
        let input = argv(&["claudine", "compose", "--provider", "claude"]);
        let expected = argv(&["claudine", "compose", "--provider", "claude"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_2_leaves_unknown_value_untouched() {
        let input = argv(&["claudine", "compose", "--provider", "nonesuch"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_leaves_missing_value_untouched() {
        let input = argv(&["claudine", "compose", "--provider"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_leaves_empty_equals_value_untouched() {
        let input = argv(&["claudine", "compose", "--provider="]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_leaves_hyphen_prefixed_next_token_untouched() {
        let input = argv(&["claudine", "compose", "--provider", "-x"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_does_not_cross_dash_dash() {
        let input = argv(&["claudine", "claude", "--", "--provider", "cl"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_skips_value_after_dash_dash_boundary() {
        // `--provider` immediately before a `--` still advertises a value token;
        // the `--` is after the scan range, so the flag is treated as
        // value-less and the separator passes through untouched.
        let input = argv(&["claudine", "compose", "--provider", "--", "name=Ken"]);
        let expected = argv(&["claudine", "compose", "--provider", "--", "name=Ken"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn is_fuzzy_provider_value_accepts_typical_tokens() {
        assert!(is_fuzzy_provider_value("cl"));
        assert!(is_fuzzy_provider_value("claude"));
        assert!(is_fuzzy_provider_value("open_code"));
    }

    #[test]
    fn is_fuzzy_provider_value_rejects_empty_and_flag_like() {
        assert!(!is_fuzzy_provider_value(""));
        assert!(!is_fuzzy_provider_value("-x"));
        assert!(!is_fuzzy_provider_value("--x"));
    }
}
