//! Code transclusion formatting helpers.

use lazy_static::lazy_static;
use std::path::Path;
use syntect::parsing::SyntaxSet;

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
}

/// Infers markdown code-fence language from file extension.
pub fn infer_language(path: &Path, fallback: &str) -> String {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return fallback.to_string();
    };

    if SYNTAX_SET.find_syntax_by_extension(ext).is_some() {
        return ext.to_ascii_lowercase();
    }

    fallback.to_string()
}

/// Generates a backtick fence safe for embedded source content.
pub fn generate_safe_fence(content: &str) -> String {
    let mut longest = 0usize;
    let mut run = 0usize;

    for ch in content.chars() {
        if ch == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }

    "`".repeat(longest.saturating_add(1).max(3))
}

/// Wraps raw source content in a fenced markdown code block.
pub fn wrap_in_code_block(content: &str, language: &str) -> String {
    let fence = generate_safe_fence(content);
    format!("{fence}{language}\n{content}\n{fence}")
}

/// Ensures exactly one blank line above and below block content.
pub fn ensure_vertical_spacing(content: &str) -> String {
    let trimmed = content.trim_matches('\n');
    format!("\n\n{trimmed}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_language_from_extension() {
        assert_eq!(infer_language(Path::new("main.rs"), "txt"), "rs");
    }

    #[test]
    fn unknown_extension_uses_fallback() {
        assert_eq!(infer_language(Path::new("sample.weird"), "txt"), "txt");
    }

    #[test]
    fn fallback_language_when_no_extension() {
        assert_eq!(infer_language(Path::new("Makefile"), "txt"), "txt");
    }

    #[test]
    fn fence_expands_for_backticks() {
        let fence = generate_safe_fence("hello ``` world");
        assert_eq!(fence, "````");
    }

    #[test]
    fn wraps_code_block() {
        let wrapped = wrap_in_code_block("fn main() {}", "rust");
        assert!(wrapped.starts_with("```rust"));
        assert!(wrapped.ends_with("```"));
    }

    #[test]
    fn ensures_vertical_spacing() {
        let spaced = ensure_vertical_spacing("content");
        assert_eq!(spaced, "\n\ncontent\n\n");
    }
}
