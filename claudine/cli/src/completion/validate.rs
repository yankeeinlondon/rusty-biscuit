//! Mode-specific file validation for completion candidates.
//!
//! Phase 3 of the `2026-04-17-file-completion` feature implements:
//!
//! - a shared markdown-extension gate (`.md` / `.markdown`) and a shared
//!   size/UTF-8 guard used by every validator;
//! - `compose` validation as an extension-only filter (no frontmatter
//!   parsing), mirroring the runtime gate in
//!   [`claudine::composition::resolve_composition_source`];
//! - `inline-compose` validation requiring a non-empty string `prompt:`
//!   frontmatter value via [`darkmatter::markdown::Markdown`];
//! - `sequence` validation via a temporary
//!   [`claudine::composition::ResolvedCompositionSource`] +
//!   [`claudine::composition::resolve_sequence_plan`] call that matches the
//!   real sequence parser (inline list, inline object list, or external
//!   YAML reference).
//!
//! All validators are fail-closed: any I/O, UTF-8, parse, or size failure
//! becomes "not a candidate", never a shell-visible diagnostic.
//!
//! Directories are never validated here; [`super::command_factory`] routes
//! directory entries straight through to the shell so a single `<TAB>` can
//! keep descending into a tree.
//!
//! The shared gates defer to [`file_reference::MAX_FRONTMATTER_BYTES`] so
//! every size-sensitive check shares one tunable constant.
//!
//! [`file_reference::MAX_FRONTMATTER_BYTES`]: super::file_reference::MAX_FRONTMATTER_BYTES

use std::fs;
use std::path::Path;

use claudine::composition::{ResolvedCompositionSource, resolve_sequence_plan};
use darkmatter::markdown::Markdown;

use super::command_factory::ComposeMode;
use super::file_reference::MAX_FRONTMATTER_BYTES;

/// Validate a candidate file against the rules for `mode`.
///
/// Returns `true` when the file should be emitted as a completion
/// candidate, `false` when it must be silently omitted. Any I/O or parse
/// failure yields `false` — completion never surfaces diagnostics.
pub(crate) fn is_valid_for_mode(path: &Path, mode: ComposeMode) -> bool {
    match mode {
        ComposeMode::Compose => is_valid_compose(path),
        ComposeMode::InlineCompose => is_valid_inline_compose(path),
        ComposeMode::Sequence => is_valid_sequence(path),
    }
}

/// `compose` mirrors [`claudine::composition::resolve_composition_source`]'s
/// extension gate. No frontmatter parsing, no I/O beyond extension inspection.
pub(crate) fn is_valid_compose(path: &Path) -> bool {
    has_markdown_extension(path)
}

/// `inline-compose` candidates must parse as Markdown and expose a non-empty
/// string `prompt:` field. Completion is stricter than runtime by design —
/// `prepare_inline` accepts the empty string, but surfacing empty-prompt
/// files in completion would be misleading.
pub(crate) fn is_valid_inline_compose(path: &Path) -> bool {
    if !has_markdown_extension(path) {
        return false;
    }
    let Some(text) = read_text_within_size_cap(path) else {
        return false;
    };
    let markdown: Markdown = text.into();
    let Some(prompt_value) = markdown.frontmatter().as_map().get("prompt") else {
        return false;
    };
    match prompt_value {
        serde_json::Value::String(s) => !s.trim().is_empty(),
        _ => false,
    }
}

/// `sequence` candidates must be markdown files whose frontmatter resolves
/// to a real sequence plan — inline scalars, inline objects, or an external
/// YAML reference. Any parse failure or unresolved external file drops the
/// candidate silently.
///
/// Defense-in-depth: the validator also requires at least one resolved step,
/// even though [`resolve_sequence_plan`] today rejects empty lists via its
/// internal normalizer. Pinning the invariant here keeps completion aligned
/// with the spirit of the spec if the library ever accepts empty plans as
/// `Ok(Some(_))`.
pub(crate) fn is_valid_sequence(path: &Path) -> bool {
    if !has_markdown_extension(path) {
        return false;
    }
    let Some(text) = read_text_within_size_cap(path) else {
        return false;
    };
    let markdown: Markdown = text.clone().into();
    let source = ResolvedCompositionSource {
        original_ref: path.display().to_string(),
        resolved_path: path.to_path_buf(),
        original_text: text,
        markdown,
    };
    match resolve_sequence_plan(&source) {
        Ok(Some(plan)) => !plan.steps.is_empty(),
        _ => false,
    }
}

/// Accept `.md` and `.markdown` extensions case-insensitively.
fn has_markdown_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

/// Read the file if — and only if — it is at most
/// [`MAX_FRONTMATTER_BYTES`] bytes long and valid UTF-8. Returns `None`
/// for every failure mode so callers can treat every error class
/// (missing file, too-big file, non-UTF-8, unreadable) identically.
fn read_text_within_size_cap(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    if metadata.len() > MAX_FRONTMATTER_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    // -- shared gates ------------------------------------------------------

    #[test]
    fn extension_gate_accepts_md_and_markdown_case_insensitive() {
        assert!(has_markdown_extension(&PathBuf::from("a.md")));
        assert!(has_markdown_extension(&PathBuf::from("a.MD")));
        assert!(has_markdown_extension(&PathBuf::from("a.markdown")));
        assert!(has_markdown_extension(&PathBuf::from("a.Markdown")));
        assert!(!has_markdown_extension(&PathBuf::from("a.txt")));
        assert!(!has_markdown_extension(&PathBuf::from("a")));
        assert!(!has_markdown_extension(&PathBuf::from("a.yaml")));
    }

    #[test]
    fn size_gate_rejects_files_larger_than_cap() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("big.md");
        // One byte over the cap is enough.
        let content = "a".repeat((MAX_FRONTMATTER_BYTES + 1) as usize);
        write(&path, &content);
        assert!(read_text_within_size_cap(&path).is_none());
    }

    #[test]
    fn size_gate_accepts_files_at_or_below_cap() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ok.md");
        write(&path, "hello");
        assert!(read_text_within_size_cap(&path).is_some());
    }

    #[test]
    fn size_gate_rejects_missing_files() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.md");
        assert!(read_text_within_size_cap(&path).is_none());
    }

    // -- compose -----------------------------------------------------------

    #[test]
    fn compose_accepts_any_markdown_file_regardless_of_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("doc.md");
        write(&path, "# Hello\n\nSome body.\n");
        assert!(is_valid_compose(&path));
    }

    #[test]
    fn compose_rejects_non_markdown_extensions() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("doc.txt");
        write(&path, "# Hello");
        assert!(!is_valid_compose(&path));
    }

    #[test]
    fn compose_does_not_require_readability() {
        // `compose` is extension-only — even a file that fails subsequent
        // I/O should still pass the cheap gate.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("unread.md");
        // Nothing created — extension check still accepts.
        assert!(is_valid_compose(&path));
    }

    // -- inline-compose ----------------------------------------------------

    #[test]
    fn inline_compose_accepts_file_with_non_empty_string_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inline.md");
        write(&path, "---\nprompt: Write a poem\n---\nbody\n");
        assert!(is_valid_inline_compose(&path));
    }

    #[test]
    fn inline_compose_rejects_empty_string_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.md");
        write(&path, "---\nprompt: \"\"\n---\nbody\n");
        assert!(!is_valid_inline_compose(&path));
    }

    #[test]
    fn inline_compose_rejects_whitespace_only_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ws.md");
        write(&path, "---\nprompt: \"   \\t  \"\n---\nbody\n");
        assert!(!is_valid_inline_compose(&path));
    }

    #[test]
    fn inline_compose_rejects_non_string_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("int.md");
        write(&path, "---\nprompt: 42\n---\nbody\n");
        assert!(!is_valid_inline_compose(&path));
    }

    #[test]
    fn inline_compose_rejects_missing_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nope.md");
        write(&path, "---\ntitle: Hi\n---\nbody\n");
        assert!(!is_valid_inline_compose(&path));
    }

    #[test]
    fn inline_compose_rejects_malformed_frontmatter_silently() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("malformed.md");
        // Unclosed fence + invalid YAML — the `From<String> for Markdown`
        // impl falls back to treating the whole thing as content, so
        // the `prompt` key is simply missing.
        write(&path, "---\nprompt: [unterminated\n\nbody without fence\n");
        assert!(!is_valid_inline_compose(&path));
    }

    #[test]
    fn inline_compose_rejects_non_markdown_extension() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inline.txt");
        write(&path, "---\nprompt: hi\n---\nbody\n");
        assert!(!is_valid_inline_compose(&path));
    }

    #[test]
    fn inline_compose_rejects_oversized_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("big.md");
        let padding = "a".repeat((MAX_FRONTMATTER_BYTES + 1) as usize);
        let content = format!("---\nprompt: hi\n---\n{padding}");
        write(&path, &content);
        assert!(!is_valid_inline_compose(&path));
    }

    // -- sequence ----------------------------------------------------------

    #[test]
    fn sequence_accepts_inline_scalar_list() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.md");
        write(&path, "---\nsequence:\n  - one\n  - two\n---\nBody\n");
        assert!(is_valid_sequence(&path));
    }

    #[test]
    fn sequence_accepts_inline_object_list() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.md");
        write(
            &path,
            "---\nsequence:\n  - name: alpha\n  - name: beta\n---\nBody\n",
        );
        assert!(is_valid_sequence(&path));
    }

    #[test]
    fn sequence_accepts_external_reference_that_loads() {
        let tmp = TempDir::new().unwrap();
        let yaml = tmp.path().join("steps.yaml");
        write(&yaml, "sequence:\n  - alpha\n  - beta\n");
        let md = tmp.path().join("seq.md");
        write(&md, "---\nsequence: steps.yaml\n---\nBody\n");
        assert!(is_valid_sequence(&md));
    }

    #[test]
    fn sequence_rejects_file_without_sequence_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("prose.md");
        write(&path, "---\ntitle: Hi\n---\nBody\n");
        assert!(!is_valid_sequence(&path));
    }

    #[test]
    fn sequence_rejects_broken_external_reference() {
        let tmp = TempDir::new().unwrap();
        let md = tmp.path().join("seq.md");
        // Points to a missing YAML file — `resolve_sequence_plan` fails,
        // which the validator turns into "not a candidate".
        write(&md, "---\nsequence: missing.yaml\n---\nBody\n");
        assert!(!is_valid_sequence(&md));
    }

    #[test]
    fn sequence_rejects_empty_list() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.md");
        write(&path, "---\nsequence: []\n---\nBody\n");
        assert!(!is_valid_sequence(&path));
    }

    #[test]
    fn sequence_rejects_non_markdown_extension() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.txt");
        write(&path, "---\nsequence:\n  - one\n---\nBody\n");
        assert!(!is_valid_sequence(&path));
    }

    #[test]
    fn sequence_rejects_oversized_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("big.md");
        let padding = "a".repeat((MAX_FRONTMATTER_BYTES + 1) as usize);
        let content = format!("---\nsequence:\n  - one\n---\n{padding}");
        write(&path, &content);
        assert!(!is_valid_sequence(&path));
    }

    // -- dispatcher --------------------------------------------------------

    #[test]
    fn dispatcher_routes_compose_extension_only() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("x.md");
        write(&path, "no frontmatter at all\n");
        assert!(is_valid_for_mode(&path, ComposeMode::Compose));
    }

    #[test]
    fn dispatcher_routes_inline_compose_to_prompt_check() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("x.md");
        write(&path, "no frontmatter\n");
        assert!(!is_valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    #[test]
    fn dispatcher_routes_sequence_to_sequence_check() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("x.md");
        write(&path, "---\nprompt: hi\n---\nBody\n");
        // Has a prompt but no sequence — inline-compose passes, sequence
        // fails.
        assert!(is_valid_for_mode(&path, ComposeMode::InlineCompose));
        assert!(!is_valid_for_mode(&path, ComposeMode::Sequence));
    }
}
