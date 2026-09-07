//! Guardrail loading for inline composition workflows.
//!
//! Guardrails are instructions appended to every inline prompt that tell the
//! agent it is editing the document named in the prompt header, which
//! frontmatter properties the closure owns, and that its final response is a
//! summary rather than document content. Users can customize the guardrails by
//! placing a `.claudine/inline-compose.md` file in the repository root; the
//! text may reference the active document with [`DOCUMENT_PATH_PLACEHOLDER`].
//! A customization that still carries the retired "do not edit" contract is
//! preserved and used, but preparation emits a migration warning.

use std::fs;
use std::io;
use std::path::Path;
use tracing::warn;

/// Relative path (from repo root) to the user-customizable guardrails file.
const GUARDRAILS_RELATIVE_PATH: &str = ".claudine/inline-compose.md";

/// Token a guardrail template may use for the active document's native
/// absolute path; [`render_guardrails`] replaces it with a code span.
pub const DOCUMENT_PATH_PLACEHOLDER: &str = "{document_path}";

/// Default guardrail instructions shipped with Claudine.
const DEFAULT_GUARDRAILS: &str = "\
> **IMPORTANT:**
>
> - The document you are updating is {document_path}. Write the
>   requested content into its body and any requested properties into its
>   frontmatter, then re-read the file to confirm it is well-formed.
> - Never modify the `prompt`, `hash`, or `last_updated` frontmatter
>   properties. They are owned by the caller and will be restored if changed.
> - If the document declares a `$schema`, every property you set must match
>   its declared type.
> - Your final response is a summary of what you did (two or three short
>   paragraphs), not the document content. Do not repeat the body in your
>   response.
";

const SHIPPED_GUARDRAILS_2026_09_05: &str = "\
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

/// Every guardrail text Claudine has ever shipped as its default, oldest
/// first. A materialized file byte-equal to one of these is Claudine's own
/// and migrates to the current default; any other content is the user's.
const HISTORICAL_SHIPPED_GUARDRAILS: &[&str] = &[
    SHIPPED_GUARDRAILS_2026_03_17,
    SHIPPED_GUARDRAILS_2026_03_27,
    SHIPPED_GUARDRAILS_2026_09_01,
    SHIPPED_GUARDRAILS_2026_09_05,
];

/// Load the guardrail template from `.claudine/inline-compose.md` (creating it
/// if absent), or fall back to the built-in default when no repo root is
/// known.
///
/// ## Returns
///
/// The guardrail template, which may still contain
/// [`DOCUMENT_PATH_PLACEHOLDER`]; pass it through [`render_guardrails`] before
/// delivering it.
pub fn load_or_create_guardrails(repo_root: Option<&Path>) -> String {
    load_or_create_guardrails_with(repo_root, crate::config::atomic::atomic_write)
}

/// Identify a customized guardrail file that still directs the agent to use
/// the retired response-harvesting workflow.
pub(super) fn retired_custom_guardrails_path(
    repo_root: Option<&Path>,
    template: &str,
) -> Option<std::path::PathBuf> {
    let root = repo_root?;
    let normalized = template.to_ascii_lowercase();
    (!template.contains(DOCUMENT_PATH_PLACEHOLDER)
        && normalized.contains("do not edit the source file directly"))
    .then(|| root.join(GUARDRAILS_RELATIVE_PATH))
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
        if HISTORICAL_SHIPPED_GUARDRAILS.contains(&existing.as_str()) {
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

/// Bind a guardrail template to the active document.
///
/// Every [`DOCUMENT_PATH_PLACEHOLDER`] becomes the document's native absolute
/// path in a code span (see [`document_path_span`]); a customized template
/// without the placeholder is returned unchanged.
pub fn render_guardrails(template: &str, document_path: &Path) -> String {
    template.replace(DOCUMENT_PATH_PLACEHOLDER, &document_path_span(document_path))
}

/// The document's native absolute path as the agent must see it.
///
/// Native separators are kept as they are and nothing is JSON-escaped:
/// doubled backslashes are a known way to make an agent write to the wrong
/// path. A Windows verbatim prefix (`\\?\`) is reduced to the legacy spelling
/// the provider's tools accept.
pub fn native_document_path(path: &Path) -> String {
    let native = path.display().to_string();
    if let Some(rest) = native.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    match native.strip_prefix(r"\\?\") {
        Some(rest)
            if rest
                .as_bytes()
                .get(1)
                .is_some_and(|byte| *byte == b':') =>
        {
            rest.to_string()
        }
        _ => native,
    }
}

/// [`native_document_path`] inside a Markdown code span, so spaces are
/// delimited unambiguously. A path containing a backtick is fenced with a
/// double-backtick span.
pub fn document_path_span(path: &Path) -> String {
    let native = native_document_path(path);
    if native.contains('`') {
        format!("`` {native} ``")
    } else {
        format!("`{native}`")
    }
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
    fn default_guardrails_state_the_file_aware_contract() {
        assert!(DEFAULT_GUARDRAILS.contains(DOCUMENT_PATH_PLACEHOLDER));
        assert!(DEFAULT_GUARDRAILS.contains("re-read the file"));
        assert!(DEFAULT_GUARDRAILS.contains("`prompt`, `hash`, or `last_updated`"));
        assert!(DEFAULT_GUARDRAILS.contains("must match\n>   its declared type"));
        assert!(DEFAULT_GUARDRAILS.contains("summary of what you did"));
        for retired in [
            "Return the replacement Markdown body",
            "Do not edit the source file directly",
            "Allowed response frontmatter properties",
            "YAML frontmatter block",
        ] {
            assert!(
                !DEFAULT_GUARDRAILS.contains(retired),
                "response-harvesting language must not survive in the default: {retired}"
            );
        }
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
    fn a_customized_file_that_differs_by_one_byte_is_preserved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(GUARDRAILS_RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let customized = format!("{SHIPPED_GUARDRAILS_2026_09_05}> - Also cite sources\n");
        fs::write(&path, &customized).unwrap();

        let result = load_or_create_guardrails(Some(dir.path()));
        assert_eq!(result, customized);
        assert_eq!(fs::read_to_string(path).unwrap(), customized);
        assert_eq!(
            retired_custom_guardrails_path(Some(dir.path()), &result),
            Some(dir.path().join(GUARDRAILS_RELATIVE_PATH))
        );
    }

    #[test]
    fn current_custom_guardrails_do_not_request_migration() {
        let custom = "> Edit {document_path} and summarize the changes.\n";
        assert_eq!(
            retired_custom_guardrails_path(Some(Path::new("/repo")), custom),
            None
        );
    }

    #[test]
    fn migrates_each_known_shipped_default_atomically() {
        for shipped in HISTORICAL_SHIPPED_GUARDRAILS {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join(GUARDRAILS_RELATIVE_PATH);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, shipped).unwrap();

            let result = load_or_create_guardrails(Some(dir.path()));

            assert_eq!(result, DEFAULT_GUARDRAILS);
            assert_eq!(fs::read_to_string(path).unwrap(), DEFAULT_GUARDRAILS);
        }
    }

    #[test]
    fn every_historical_default_is_distinct_and_retired() {
        for (index, shipped) in HISTORICAL_SHIPPED_GUARDRAILS.iter().enumerate() {
            assert_ne!(*shipped, DEFAULT_GUARDRAILS, "entry {index} is the live default");
            for later in &HISTORICAL_SHIPPED_GUARDRAILS[index + 1..] {
                assert_ne!(shipped, later, "entry {index} is duplicated");
            }
        }
    }

    #[tracing_test::traced_test]
    #[test]
    fn failed_migration_uses_new_protocol_without_truncating_old_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(GUARDRAILS_RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, SHIPPED_GUARDRAILS_2026_09_05).unwrap();

        let result = load_or_create_guardrails_with(Some(dir.path()), |_, _| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected"))
        });

        assert_eq!(result, DEFAULT_GUARDRAILS);
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            SHIPPED_GUARDRAILS_2026_09_05
        );
        assert!(logs_contain("failed to migrate guardrails file"));
        assert!(logs_contain("injected"));
    }

    #[test]
    fn render_binds_the_placeholder_to_a_native_code_span() {
        let path = Path::new("/Users/ken/My Docs/voip.md");
        let rendered = render_guardrails(DEFAULT_GUARDRAILS, path);
        assert!(rendered.contains("is `/Users/ken/My Docs/voip.md`."));
        assert!(!rendered.contains(DOCUMENT_PATH_PLACEHOLDER));
    }

    #[test]
    fn render_leaves_a_template_without_the_placeholder_unchanged() {
        let custom = "> Custom guardrails here\n";
        assert_eq!(render_guardrails(custom, Path::new("/tmp/x.md")), custom);
    }

    #[test]
    fn native_document_path_reduces_windows_verbatim_spellings() {
        assert_eq!(
            native_document_path(Path::new(r"\\?\C:\Users\Ken\voip.md")),
            r"C:\Users\Ken\voip.md"
        );
        assert_eq!(
            native_document_path(Path::new(r"\\?\UNC\nas\share\voip.md")),
            r"\\nas\share\voip.md"
        );
        assert_eq!(
            native_document_path(Path::new("/Users/ken/voip.md")),
            "/Users/ken/voip.md"
        );
    }

    #[test]
    fn document_path_span_keeps_backslashes_and_spaces_verbatim() {
        let windows = Path::new(r"C:\Users\Ken\My Docs\voip.md");
        assert_eq!(
            document_path_span(windows),
            "`C:\\Users\\Ken\\My Docs\\voip.md`"
        );
        let with_backtick = Path::new("/tmp/we`ird.md");
        assert_eq!(document_path_span(with_backtick), "`` /tmp/we`ird.md ``");
    }
}
