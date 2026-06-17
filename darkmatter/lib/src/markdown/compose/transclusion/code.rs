//! Code transclusion formatting helpers.

use crate::markdown::language_grammar::LanguageGrammar;
use std::path::Path;

/// Infers markdown code-fence language from a file path.
///
/// The path is validated against [`LanguageGrammar`] so the lookup uses
/// Darkmatter's full two-face grammar set rather than syntect's smaller
/// default set. When the path resolves, the returned token is the lowercase
/// file extension (or basename for extensionless paths) so that existing
/// output such as `main.rs` -> `rs` is preserved.
pub fn infer_language(path: &Path, fallback: &str) -> String {
    let raw = path.to_string_lossy();

    if LanguageGrammar::from_filename(raw.as_ref()).is_err() {
        return fallback.to_string();
    }

    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        return ext.to_ascii_lowercase();
    }

    if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
        return name.to_ascii_lowercase();
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
    fn two_face_only_extension_emits_real_extension() {
        assert_eq!(infer_language(Path::new("component.ts"), "txt"), "ts");
    }

    #[test]
    fn unknown_extension_uses_fallback() {
        assert_eq!(infer_language(Path::new("sample.weird"), "txt"), "txt");
    }

    #[test]
    fn recognizes_extensionless_well_known_filename() {
        // `Makefile` has no extension but is a supported source filename; the
        // returned fence token is the lowercase basename so composed Markdown
        // surfaces the language hint instead of falling back to plain text.
        assert_eq!(infer_language(Path::new("Makefile"), "txt"), "makefile");
        assert_eq!(infer_language(Path::new("Dockerfile"), "txt"), "dockerfile");
    }

    #[test]
    fn fallback_language_when_no_extension_and_unknown_basename() {
        assert_eq!(infer_language(Path::new("not-a-known-name"), "txt"), "txt");
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
