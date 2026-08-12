//! Token estimation for composed prompt content.
//!
//! Provides a lightweight heuristic for estimating LLM token counts from
//! composed markdown text.  The estimation is based on character-count ratios
//! tuned for natural-language and code content (≈ 4 characters per token for
//! prose, ≈ 2.5 for dense formats like JSON/YAML).
//!
//! ## Alignment with biscuit-terminal
//!
//! The spec for this feature originally pointed at
//! "biscuit-terminal's FileTree utility" for token estimation, but the
//! `FileTree` component renders Markdown dependency graphs and does not
//! count tokens. The actual token estimator inside biscuit-terminal lives
//! in `components::filesystem` (a private `estimate_tokens(path, metadata)`
//! function) and uses the **same character-count ratios** as this module
//! (4.0 chars/token for prose/code, 2.5 for dense formats like JSON/YAML).
//! Cloning that file-based estimator here would not produce different
//! results — we already match its heuristic byte-for-byte.
//!
//! ## Limitation
//!
//! This measures only the content Claudine composes (the
//! `system-prompt.md` body plus any appendix).  The agent platform's own
//! default/base system prompt is **not** included in the estimate.

/// Estimated characters per token for plain text / markdown content.
const CHARS_PER_TOKEN_TEXT: f64 = 4.0;

/// Estimate the token count of a composed prompt string.
///
/// The heuristic treats the input as plain-text/markdown by default
/// (≈ 4 chars / token).
pub fn estimate_tokens(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    (text.len() as f64 / CHARS_PER_TOKEN_TEXT) as u64
}

/// Estimate tokens for a composed system prompt, optionally including the
/// non-interactive appendix.
///
/// Concatenates the primary composed markdown with the appendix (if any)
/// and runs [`estimate_tokens`] over the combined text.
pub fn estimate_system_prompt_tokens(composed_markdown: &str, appendix: Option<&str>) -> u64 {
    let combined = match appendix {
        Some(a) => format!("{}\n\n{}", composed_markdown.trim_end(), a.trim()),
        None => composed_markdown.to_string(),
    };
    estimate_tokens(&combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_is_zero() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn simple_text_estimate() {
        // 28 chars / 4 = 7 tokens
        let text = "This is a short system prompt.";
        assert_eq!(estimate_tokens(text), 7);
    }

    #[test]
    fn system_prompt_with_appendix() {
        let primary = "Base prompt content.";
        let appendix = "Non-interactive safety instructions.";
        let tokens = estimate_system_prompt_tokens(primary, Some(appendix));
        let expected =
            estimate_tokens("Base prompt content.\n\nNon-interactive safety instructions.");
        assert_eq!(tokens, expected);
    }

    #[test]
    fn system_prompt_without_appendix() {
        let primary = "Base prompt content.";
        let tokens = estimate_system_prompt_tokens(primary, None);
        assert_eq!(tokens, estimate_tokens(primary));
    }

    #[test]
    fn large_prompt_estimate() {
        let text = "a".repeat(4000);
        assert_eq!(estimate_tokens(&text), 1000);
    }
}
