//! Guardrail loading for inline composition workflows.
//!
//! Guardrails are instructions appended to every inline prompt to prevent the
//! agent from rewriting frontmatter or otherwise defeating the composition
//! pipeline. Users can customize the guardrails by placing a
//! `.claudine/inline-compose.md` file in the repository root.

use std::fs;
use std::io;
use std::path::Path;
use tracing::warn;

/// Relative path (from repo root) to the user-customizable guardrails file.
const GUARDRAILS_RELATIVE_PATH: &str = ".claudine/inline-compose.md";

/// Default guardrail instructions shipped with Claudine.
const DEFAULT_GUARDRAILS: &str = "\
> **IMPORTANT:**
>
> - Return the replacement Markdown body content in your final response
> - Do not edit the source file directly
> - If the prompt asks you to add or update frontmatter properties, put them
>   in a YAML frontmatter block (`---` fenced) at the very top of your
>   response; they will be merged into the document's frontmatter
> - Never include the `prompt` property in that block; it cannot be changed.
>   `hash` and `last_updated` are managed for you and must not be included
";

const SHIPPED_GUARDRAILS_2026_09_01: &str = "\
> **IMPORTANT:**
>
> - Return the replacement Markdown body content only
> - Do not edit the source file directly
> - If an \"Allowed response frontmatter properties\" list appears below, you
>   may put exactly those properties in a YAML frontmatter block (`---`
>   fenced) at the very top of your response; they will be merged into the
>   document
> - Frontmatter properties outside that allowed list — including `prompt` —
>   cannot be changed; do not include them in that block
";

const SHIPPED_GUARDRAILS_2026_03_17: &str = "\
> **IMPORTANT:**
>
> - Never change the `prompt` frontmatter property, that property is to read and should not be reformatted or changed in any way
> - Your task is to use the prompt from the `prompt` property to update the body of this document
> - Do not create another document and have this document link to it unless the frontmatter `prompt` explicitly tells you to
";

const SHIPPED_GUARDRAILS_2026_03_27: &str = "\
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
    load_or_create_guardrails_with(repo_root, crate::config::atomic::atomic_write)
}

fn load_or_create_guardrails_with<W>(repo_root: Option<&Path>, write: W) -> String
where
    W: Fn(&Path, &[u8]) -> io::Result<()>,
{
    let Some(root) = repo_root else {
        return DEFAULT_GUARDRAILS.to_string();
    };

    let guardrails_path = root.join(GUARDRAILS_RELATIVE_PATH);

    if guardrails_path.is_file() {
        let Ok(existing) = fs::read_to_string(&guardrails_path) else {
            return DEFAULT_GUARDRAILS.to_string();
        };
        if matches!(
            existing.as_str(),
            SHIPPED_GUARDRAILS_2026_03_17
                | SHIPPED_GUARDRAILS_2026_03_27
                | SHIPPED_GUARDRAILS_2026_09_01
        ) {
            if let Err(error) = write(&guardrails_path, DEFAULT_GUARDRAILS.as_bytes()) {
                warn!(
                    "failed to migrate guardrails file {}: {error}",
                    biscuit_file::to_portable_string(&guardrails_path)
                );
            }
            return DEFAULT_GUARDRAILS.to_string();
        }
        return existing;
    }

    if let Err(error) = write(&guardrails_path, DEFAULT_GUARDRAILS.as_bytes()) {
        warn!(
            "failed to write guardrails file {}: {error}",
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
        assert_eq!(fs::read_to_string(path).unwrap(), custom);
    }

    #[test]
    fn migrates_each_known_shipped_default_atomically() {
        for shipped in [
            SHIPPED_GUARDRAILS_2026_03_17,
            SHIPPED_GUARDRAILS_2026_03_27,
            SHIPPED_GUARDRAILS_2026_09_01,
        ] {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join(GUARDRAILS_RELATIVE_PATH);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, shipped).unwrap();

            let result = load_or_create_guardrails(Some(dir.path()));

            assert_eq!(result, DEFAULT_GUARDRAILS);
            assert_eq!(fs::read_to_string(path).unwrap(), DEFAULT_GUARDRAILS);
        }
    }

    #[tracing_test::traced_test]
    #[test]
    fn failed_migration_uses_new_protocol_without_truncating_old_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(GUARDRAILS_RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, SHIPPED_GUARDRAILS_2026_09_01).unwrap();

        let result = load_or_create_guardrails_with(Some(dir.path()), |_, _| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected"))
        });

        assert_eq!(result, DEFAULT_GUARDRAILS);
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            SHIPPED_GUARDRAILS_2026_09_01
        );
        assert!(logs_contain("failed to migrate guardrails file"));
        assert!(logs_contain("injected"));
    }
}
