//! Rule 1: provider boolean flags → `--provider <slug>`.
//!
//! On composition subcommands only, rewrites `--claude`, `--codex`, … to
//! `--provider claude`, `--provider codex`, etc.

use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};

/// Returns the [`Provider`] that a boolean flag token selects, if any.
///
/// The boolean flag surface is derived from [`Provider::cli_aliases`]: the
/// first alias of each provider (`claude`, `codex`, `gemini`, `goose`,
/// `kimi`, `opencode`, `qwen`) is the user-facing flag name, and
/// the normalizer rewrites `--<first-alias>` to `--provider <as_slug()>`.
/// Keeping the mapping derived means a new provider added with a matching
/// clap boolean flag declaration in `SharedComposeArgs` automatically
/// inherits Rule 1 coverage.
pub(crate) fn provider_for_boolean_flag(token: &str) -> Option<Provider> {
    let name = token.strip_prefix("--")?;
    if name.is_empty() || name.starts_with('-') {
        return None;
    }
    PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .find(|provider| provider.cli_aliases().first().copied() == Some(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    #[test]
    fn rule_1_rewrites_claude_boolean_to_provider_slug() {
        let input = argv(&["claudine", "compose", "file.md", "--claude"]);
        let expected = argv(&["claudine", "compose", "file.md", "--provider", "claude"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_1_rewrites_gemini_boolean_in_interleaved_position() {
        let input = argv(&["claudine", "compose", "--gemini", "file.md"]);
        let expected = argv(&["claudine", "compose", "--provider", "gemini", "file.md"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_1_preserves_canonical_kimi_slug() {
        let input = argv(&["claudine", "compose", "--kimi"]);
        let expected = argv(&["claudine", "compose", "--provider", "kimi"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_1_preserves_canonical_opencode_slug() {
        let input = argv(&["claudine", "compose", "--opencode"]);
        let expected = argv(&["claudine", "compose", "--provider", "opencode"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_1_preserves_canonical_qwen_slug() {
        let input = argv(&["claudine", "compose", "--qwen"]);
        let expected = argv(&["claudine", "compose", "--provider", "qwen"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_1_rewrites_every_provider_boolean() {
        let booleans = [
            ("--claude", "claude"),
            ("--codex", "codex"),
            ("--gemini", "gemini"),
            ("--goose", "goose"),
            ("--kimi", "kimi"),
            ("--opencode", "opencode"),
            ("--qwen", "qwen"),
        ];
        for (flag, slug) in booleans {
            let input = argv(&["claudine", "compose", flag]);
            let expected = argv(&["claudine", "compose", "--provider", slug]);
            assert_eq!(crate::argv::normalize(input), expected, "flag {flag}");
        }
    }

    #[test]
    fn rule_1_duplicates_survive_to_clap() {
        let input = argv(&["claudine", "compose", "--claude", "--gemini"]);
        let expected = argv(&[
            "claudine",
            "compose",
            "--provider",
            "claude",
            "--provider",
            "gemini",
        ]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_1_coexists_with_explicit_provider_flag() {
        let input = argv(&[
            "claudine",
            "compose",
            "--provider",
            "claude",
            "--gemini",
            "file.md",
        ]);
        let expected = argv(&[
            "claudine",
            "compose",
            "--provider",
            "claude",
            "--provider",
            "gemini",
            "file.md",
        ]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_1_does_not_fuzzy_match_near_miss_flags() {
        let input = argv(&["claudine", "compose", "--claud", "file.md"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_1_ignores_provider_booleans_after_dash_dash() {
        let input = argv(&["claudine", "claude", "--", "--gemini", "file.md"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn provider_for_boolean_flag_matches_every_entry() {
        for provider in PROVIDERS_DISPLAY_ORDER {
            let first_alias = provider
                .cli_aliases()
                .first()
                .copied()
                .expect("every Provider exposes at least one cli alias");
            let flag = format!("--{first_alias}");
            assert_eq!(
                provider_for_boolean_flag(&flag),
                Some(provider),
                "flag {flag}"
            );
        }
    }

    #[test]
    fn provider_for_boolean_flag_rejects_unknown() {
        assert_eq!(provider_for_boolean_flag("--claud"), None);
        assert_eq!(provider_for_boolean_flag("--anthropic"), None);
        assert_eq!(provider_for_boolean_flag("--plain"), None);
    }
}
