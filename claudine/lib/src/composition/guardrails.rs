//! Guardrail loading for inline composition workflows.
//!
//! Guardrails are instructions appended to every inline prompt to prevent the
//! agent from rewriting frontmatter or otherwise defeating the composition
//! pipeline.  Users can customise the guardrails by placing a
//! `.claudine/inline-compose.md` file in the repository root.

use std::fs;
use std::path::Path;
use tracing::warn;

/// Relative path (from repo root) to the user-customisable guardrails file.
const GUARDRAILS_RELATIVE_PATH: &str = ".claudine/inline-compose.md";

/// Default guardrail instructions shipped with Claudine.
const DEFAULT_GUARDRAILS: &str = "\
> **IMPORTANT:**
>
> - Return the replacement Markdown body content only
> - Do not include frontmatter delimiters or frontmatter content
> - Do not edit the source file directly
";

/// Load guardrails from `.claudine/inline-compose.md` (creating it if
/// absent), or fall back to the built-in default when no repo root is known.
///
/// ## Returns
///
/// The guardrail text, ready to be appended to the composed prompt (the
/// caller is responsible for adding the leading `\n\n`).
pub fn load_or_create_guardrails(repo_root: Option<&Path>) -> String {
    let Some(root) = repo_root else {
        return DEFAULT_GUARDRAILS.to_string();
    };

    let guardrails_path = root.join(GUARDRAILS_RELATIVE_PATH);

    // Read existing file
    if guardrails_path.is_file() {
        return fs::read_to_string(&guardrails_path)
            .unwrap_or_else(|_| DEFAULT_GUARDRAILS.to_string());
    }

    // Create .claudine/ directory if needed, then write the default template
    if let Some(parent) = guardrails_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        warn!(
            "failed to create guardrails directory {}: {e}",
            biscuit_file::to_portable_string(parent)
        );
        return DEFAULT_GUARDRAILS.to_string();
    }
    if let Err(e) = fs::write(&guardrails_path, DEFAULT_GUARDRAILS) {
        warn!(
            "failed to write guardrails file {}: {e}",
            biscuit_file::to_portable_string(&guardrails_path)
        );
    }

    DEFAULT_GUARDRAILS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn returns_default_when_no_repo_root() {
        let result = load_or_create_guardrails(None);
        assert_eq!(result, DEFAULT_GUARDRAILS);
    }

    #[test]
    fn creates_file_when_absent() {
        let dir = TempDir::new().unwrap();
        let result = load_or_create_guardrails(Some(dir.path()));

        assert_eq!(result, DEFAULT_GUARDRAILS);

        // File should now exist
        let path = dir.path().join(GUARDRAILS_RELATIVE_PATH);
        assert!(path.is_file(), "guardrails file should have been created");
        let on_disk = fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, DEFAULT_GUARDRAILS);
    }

    #[test]
    fn reads_existing_custom_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(GUARDRAILS_RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let custom = "> Custom guardrails here\n";
        fs::write(&path, custom).unwrap();

        let result = load_or_create_guardrails(Some(dir.path()));
        assert_eq!(result, custom);
    }
}
